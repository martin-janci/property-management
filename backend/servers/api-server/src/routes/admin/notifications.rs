//! Admin notification delivery analytics (Epic 2B, Story 2B-C.3 / gap #969-4).
//!
//! Reads aggregates from `notification_events` (written asynchronously by the
//! notification pipeline) and exposes per-channel delivery/failure rates plus a
//! **>5% failure-rate alert** for operators.
//!
//! Mounted under `/api/v1/admin/notifications`. Capability-gated with
//! [`Capability::AuditRead`] — this is platform-operator observability of the
//! delivery infrastructure, the same audience and risk class as the audit-log
//! viewer (no per-tenant data is returned, only aggregate counts).
//!
//! ## Query parameters (`GET /analytics`)
//!
//! * `from=<iso8601>` / `to=<iso8601>` — absolute window bounds.
//! * `after=<24h|7d|15m>` — relative lower bound; ignored when `from` is set.
//! * `channel=push|email|in_app` — restrict to one channel.
//!
//! Defaults: `to = now`, `from = now − 24h`.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use db::models::ChannelEventCounts;
use db::repositories::{total_counts, NotificationEventRepository};

use crate::state::AppState;

/// Failure-rate threshold above which the alert fires (Story 2B-C.3: >5%).
const FAILURE_RATE_THRESHOLD: f64 = 0.05;

/// Minimum attempts in the window before the alert can fire. Guards against a
/// noisy 1-failed-of-1 (100%) alert on a near-idle window; the alert is about a
/// sustained delivery problem, not a single failure.
const ALERT_MIN_ATTEMPTS: i64 = 20;

/// Default lookback when no `from`/`after` is supplied.
const DEFAULT_WINDOW_HOURS: i64 = 24;

/// Valid channel filter values (mirror `NotificationChannel::as_str()`).
const VALID_CHANNELS: [&str; 3] = ["push", "email", "in_app"];

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/analytics",
        get(get_analytics).layer(require_capability(Capability::AuditRead)),
    )
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// Absolute lower bound on `occurred_at`. Wins over `after`.
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    /// Absolute upper bound on `occurred_at`. Defaults to now.
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    /// Relative lower-bound alias (`24h`, `7d`, `15m`). Ignored when `from` set.
    #[serde(default)]
    pub after: Option<String>,
    /// Restrict to a single channel (`push` | `email` | `in_app`).
    #[serde(default)]
    pub channel: Option<String>,
}

/// Per-channel (or total) analytics row returned to the operator.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChannelAnalytics {
    pub channel: String,
    pub sent: i64,
    pub delivered: i64,
    pub failed: i64,
    pub skipped: i64,
    pub opened: i64,
    pub clicked: i64,
    /// `sent + failed` — deliveries that resolved to success or failure.
    pub attempted: i64,
    /// `failed / attempted` in `[0,1]`; `0` for an empty window.
    pub failure_rate: f64,
}

impl From<&ChannelEventCounts> for ChannelAnalytics {
    fn from(c: &ChannelEventCounts) -> Self {
        Self {
            channel: c.channel.clone(),
            sent: c.sent,
            delivered: c.delivered,
            failed: c.failed,
            skipped: c.skipped,
            opened: c.opened,
            clicked: c.clicked,
            attempted: c.attempted(),
            failure_rate: c.failure_rate(),
        }
    }
}

/// The >5% failure-rate alert state for the window. Computed once per query (per
/// window) rather than emitted per event.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FailureRateAlert {
    /// `true` when `attempted >= min_attempts` and `failure_rate > threshold`.
    pub firing: bool,
    pub threshold: f64,
    pub failure_rate: f64,
    pub attempted: i64,
    pub min_attempts: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotificationAnalyticsResponse {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    /// Echoes the `channel` filter, if any.
    pub channel_filter: Option<String>,
    /// Aggregate across all channels in the window.
    pub totals: ChannelAnalytics,
    /// One row per channel that had events in the window.
    pub by_channel: Vec<ChannelAnalytics>,
    pub alert: FailureRateAlert,
}

/// Parse a relative duration suffixed with `h`/`d`/`m`. Bounded so an
/// attacker-controlled value cannot panic or wrap (`try_*` constructors + a
/// generous upper bound). Returns `None` on any malformed input.
fn parse_relative_duration(s: &str) -> Option<Duration> {
    const MAX_DAYS: i64 = 36_500; // ~100y
    const MAX_HOURS: i64 = MAX_DAYS * 24;
    const MAX_MINUTES: i64 = MAX_HOURS * 60;

    let s = s.trim();
    if let Some(val) = s.strip_suffix('h') {
        let n = val.parse::<i64>().ok()?;
        if !(0..=MAX_HOURS).contains(&n) {
            return None;
        }
        Duration::try_hours(n)
    } else if let Some(val) = s.strip_suffix('d') {
        let n = val.parse::<i64>().ok()?;
        if !(0..=MAX_DAYS).contains(&n) {
            return None;
        }
        Duration::try_days(n)
    } else if let Some(val) = s.strip_suffix('m') {
        let n = val.parse::<i64>().ok()?;
        if !(0..=MAX_MINUTES).contains(&n) {
            return None;
        }
        Duration::try_minutes(n)
    } else {
        None
    }
}

/// GET /api/v1/admin/notifications/analytics
async fn get_analytics(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<NotificationAnalyticsResponse>, (StatusCode, String)> {
    if let Some(ch) = q.channel.as_deref() {
        if !VALID_CHANNELS.contains(&ch) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("invalid channel `{ch}` (expected one of {VALID_CHANNELS:?})"),
            ));
        }
    }

    let now = Utc::now();
    let to = q.to.unwrap_or(now);
    let from = q
        .from
        .or_else(|| {
            q.after
                .as_deref()
                .and_then(parse_relative_duration)
                .map(|d| now - d)
        })
        .unwrap_or_else(|| now - Duration::hours(DEFAULT_WINDOW_HOURS));

    if from > to {
        return Err((
            StatusCode::BAD_REQUEST,
            "`from` must be earlier than or equal to `to`".to_string(),
        ));
    }

    let repo = NotificationEventRepository::new(state.db.clone());
    let rows = repo
        .aggregate_by_channel(from, to, q.channel.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let totals = ChannelAnalytics::from(&total_counts(&rows));
    let by_channel: Vec<ChannelAnalytics> = rows.iter().map(ChannelAnalytics::from).collect();

    let firing =
        totals.attempted >= ALERT_MIN_ATTEMPTS && totals.failure_rate > FAILURE_RATE_THRESHOLD;
    let alert = FailureRateAlert {
        firing,
        threshold: FAILURE_RATE_THRESHOLD,
        failure_rate: totals.failure_rate,
        attempted: totals.attempted,
        min_attempts: ALERT_MIN_ATTEMPTS,
    };

    Ok(Json(NotificationAnalyticsResponse {
        from,
        to,
        channel_filter: q.channel,
        totals,
        by_channel,
        alert,
    }))
}

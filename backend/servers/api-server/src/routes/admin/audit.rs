//! Admin endpoints for the audit-log viewer. Phase 5.
//!
//! Reads from `audit_logs`. The append-only trigger (migration 00138)
//! protects the underlying rows; we only expose SELECTs here.
//!
//! ## Query parameters
//!
//! * `since=<iso8601>` — existing: filter rows to `created_at >= since`.
//! * `after=<duration>` — new alias: relative duration string (`24h`, `7d`,
//!   `15m`, `1h`, `30d`) subtracted from `now()`. When both `since` and
//!   `after` are provided, `since` wins.
//! * `severity=high` — new: when set, restricts results to capabilities in
//!   the platform-level high-risk list.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(list_audit_events).layer(require_capability(Capability::AuditRead)),
    )
}

/// High-risk capability action strings used for `severity=high` filtering.
/// These match the `action::text` values written by `AuditWriter::record`.
const HIGH_RISK_ACTIONS: &[&str] = &[
    "tenant_purge",
    "principal_kind_escalate",
    "grant_principal_kind_escalate",
    "tenant_restore",
];

/// Parse a relative duration string like `24h`, `7d`, `15m`, `1h`, `30d`
/// into a `chrono::Duration`. Returns `None` for unrecognised formats.
fn parse_relative_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if let Some(val) = s.strip_suffix('h') {
        val.parse::<i64>().ok().map(Duration::hours)
    } else if let Some(val) = s.strip_suffix('d') {
        val.parse::<i64>().ok().map(Duration::days)
    } else if let Some(val) = s.strip_suffix('m') {
        val.parse::<i64>().ok().map(Duration::minutes)
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    pub actor_id: Option<Uuid>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_id: Option<Uuid>,
    /// Absolute lower bound on `created_at` (ISO 8601). Takes precedence
    /// over `after` when both are provided.
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
    /// Relative duration alias for `since`. Supports `24h`, `7d`, `15m`,
    /// `1h`, `30d`. Ignored when `since` is also present.
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<u32>,
    /// When set to `"high"`, restricts results to the platform high-risk
    /// action set: tenant_purge, principal_kind_escalate,
    /// grant_principal_kind_escalate, tenant_restore.
    #[serde(default)]
    pub severity: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// GET /admin/audit
async fn list_audit_events(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditRow>>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(100).min(500) as i64;

    // Resolve the effective lower-bound timestamp.
    // `since` wins over `after` when both are supplied.
    let effective_since: Option<DateTime<Utc>> = if q.since.is_some() {
        q.since
    } else if let Some(ref after_str) = q.after {
        parse_relative_duration(after_str)
            .map(|dur| Utc::now() - dur)
            .or_else(|| {
                tracing::warn!(after = %after_str, "unrecognised `after` duration; ignoring");
                None
            })
    } else {
        None
    };

    let high_severity = q.severity.as_deref() == Some("high");

    // Filter dynamically. We use a single SQL with `$N IS NULL OR …`
    // patterns rather than dynamic SQL — avoids string concat / injection.
    //
    // The high-severity filter passes the action list as a Postgres array;
    // when `$8` is FALSE the `= ANY(…)` clause is skipped entirely.
    let rows = sqlx::query_as::<_, AuditRow>(
        r#"
        SELECT id, user_id, action::text AS action, resource_type, resource_id,
               details, ip_address, user_agent, created_at
        FROM audit_logs
        WHERE ($1::uuid IS NULL OR user_id = $1)
          AND ($2::text IS NULL OR action::text = $2)
          AND ($3::text IS NULL OR resource_type = $3)
          AND ($4::uuid IS NULL OR resource_id = $4)
          AND ($5::timestamptz IS NULL OR created_at >= $5)
          AND ($6::timestamptz IS NULL OR created_at <= $6)
          AND (NOT $7 OR action::text = ANY($8))
        ORDER BY created_at DESC
        LIMIT $9
        "#,
    )
    .bind(q.actor_id)
    .bind(q.action.as_deref())
    .bind(q.target_type.as_deref())
    .bind(q.target_id)
    .bind(effective_since)
    .bind(q.until)
    .bind(high_severity)
    .bind(HIGH_RISK_ACTIONS)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_hours() {
        let d = parse_relative_duration("24h").unwrap();
        assert_eq!(d, Duration::hours(24));
    }

    #[test]
    fn parse_duration_days() {
        let d = parse_relative_duration("7d").unwrap();
        assert_eq!(d, Duration::days(7));
    }

    #[test]
    fn parse_duration_minutes() {
        let d = parse_relative_duration("15m").unwrap();
        assert_eq!(d, Duration::minutes(15));
    }

    #[test]
    fn parse_duration_unknown_returns_none() {
        assert!(parse_relative_duration("1w").is_none());
        assert!(parse_relative_duration("abc").is_none());
        assert!(parse_relative_duration("").is_none());
    }
}

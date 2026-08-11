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
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    // Routes are siblings under the parent `/audit` nest.
    Router::new()
        .route(
            "/csv",
            get(export_csv).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/",
            get(list_audit_events).layer(require_capability(Capability::AuditRead)),
        )
}

/// Sanitize a string cell for CSV output to prevent spreadsheet formula
/// injection. The `csv` crate already handles quoting/escaping of commas,
/// quotes, and newlines — we only need to neutralize the `=`, `+`, `-`,
/// `@` characters that some spreadsheet apps interpret as formulas.
///
/// Two checks:
///   1. The classic START-of-cell rule (leading `=+-@`).
///   2. The "anywhere" rule (M5 fix): a cell whose body contains one of
///      these chars also gets the quote prefix. This matters for the
///      JSON `details` blob, which starts with `{` (so the leading-char
///      check passes) but can embed `=cmd|...` that Excel still parses
///      when the operator copy-pastes the cell contents.
///
/// We prepend a single quote (`'`) which is the standard mitigation: it
/// forces the cell to be treated as text without being visible in most
/// spreadsheet UIs. False positives (a legit cell containing `-`) are
/// acceptable for an audit export — the prefix is harmless when pasted
/// into a viewer and the operator can strip it.
///
/// `pub(crate)` so other CSV exporters (e.g. the reports export in
/// [`crate::routes::reports`]) reuse this single implementation instead of
/// re-deriving their own formula-injection guard.
pub(crate) fn sanitize_csv_cell(value: &str) -> String {
    let dangerous = value.chars().any(|c| matches!(c, '=' | '+' | '-' | '@'));
    if dangerous {
        let mut out = String::with_capacity(value.len() + 1);
        out.push('\'');
        out.push_str(value);
        out
    } else {
        value.to_string()
    }
}

/// High-risk capability names used for `severity=high` filtering.
///
/// `PgAuditWriter::record` writes every capability invocation as
/// `action = 'resource_accessed'` with the actual capability stashed in
/// `details->>'capability'`. So we filter on the JSONB capability string,
/// NOT on `action::text`.
const HIGH_RISK_CAPABILITIES: &[&str] = &[
    "tenant_purge",
    "principal_kind_escalate",
    "grant_principal_kind_escalate",
    "tenant_restore",
];

/// Parse a relative duration string like `24h`, `7d`, `15m`, `1h`, `30d`
/// into a `chrono::Duration`. Returns `None` for unrecognised formats OR
/// for values that would overflow chrono's `Duration` (e.g.
/// `9223372036854775807d`).
///
/// We use the `try_*` constructors so an attacker-controlled `after` query
/// param cannot panic the handler. As an additional belt-and-braces guard
/// we cap each unit at a clearly-sane upper bound (100 years).
fn parse_relative_duration(s: &str) -> Option<Duration> {
    // ~100 years in each unit — more than enough for an audit-log lookback.
    const MAX_DAYS: i64 = 36_500;
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

/// Shared fetcher used by both the JSON and CSV endpoints. Resolves the
/// effective lower-bound timestamp from `since` / `after`, applies the
/// high-severity capability filter, and returns the matched rows.
async fn fetch_rows(
    state: &AppState,
    q: &AuditQuery,
    limit: i64,
) -> Result<Vec<AuditRow>, sqlx::Error> {
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
    // The high-severity filter passes the capability list as a Postgres
    // array; when `$7` is FALSE the `= ANY(…)` clause is skipped entirely.
    sqlx::query_as::<_, AuditRow>(
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
          AND (NOT $7 OR (action::text = 'resource_accessed'
                          AND details->>'capability' = ANY($8)))
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
    .bind(HIGH_RISK_CAPABILITIES)
    .bind(limit)
    .fetch_all(&state.db)
    .await
}

/// GET /admin/audit
async fn list_audit_events(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditRow>>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(100).min(500) as i64;
    let rows = fetch_rows(&state, &q, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

/// GET /admin/audit/csv
///
/// Same filter shape as the JSON endpoint. Builds the CSV in-memory and
/// returns it as the response body (one header row + one row per audit
/// event). Default limit raised to 10_000 because operators usually export
/// the full filtered range; capped at 50_000 so memory stays bounded.
/// Client can still narrow via `since` / `until` / `limit` query params.
async fn export_csv(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Response, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(10_000).min(50_000) as i64;
    let rows = fetch_rows(&state, &q, limit)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Build CSV in memory. With limit ≤50k the buffer stays well under
    // 10 MB even at 200 chars/row — far below the gateway timeout window.
    let mut buf: Vec<u8> = Vec::with_capacity(rows.len() * 256);
    {
        let mut w = csv::Writer::from_writer(&mut buf);
        w.write_record([
            "id",
            "user_id",
            "action",
            "resource_type",
            "resource_id",
            "ip_address",
            "user_agent",
            "created_at",
            "details",
        ])
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        for r in &rows {
            // UUIDs and timestamps are well-formed and not attacker-controlled,
            // but every free-form string column passes through `sanitize_csv_cell`
            // to neutralize spreadsheet-formula injection.
            w.write_record([
                r.id.to_string(),
                r.user_id.map(|u| u.to_string()).unwrap_or_default(),
                sanitize_csv_cell(&r.action),
                sanitize_csv_cell(r.resource_type.as_deref().unwrap_or("")),
                r.resource_id.map(|u| u.to_string()).unwrap_or_default(),
                sanitize_csv_cell(r.ip_address.as_deref().unwrap_or("")),
                sanitize_csv_cell(r.user_agent.as_deref().unwrap_or("")),
                r.created_at.to_rfc3339(),
                sanitize_csv_cell(
                    &r.details
                        .as_ref()
                        .map(|j| j.to_string())
                        .unwrap_or_default(),
                ),
            ])
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }
        w.flush()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    let filename = format!("audit-{}.csv", Utc::now().format("%Y%m%d"));
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
    );
    Ok((headers, buf).into_response())
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

    #[test]
    fn sanitize_csv_cell_leading_equals() {
        // Legacy behaviour: leading `=` gets a quote prefix.
        assert_eq!(sanitize_csv_cell("=cmd|/c calc"), "'=cmd|/c calc");
    }

    #[test]
    fn sanitize_csv_cell_plain_text_untouched() {
        assert_eq!(sanitize_csv_cell("ordinary"), "ordinary");
        assert_eq!(sanitize_csv_cell(""), "");
    }

    #[test]
    fn sanitize_csv_cell_json_blob_with_inner_equals() {
        // M5 fix: a JSON blob starts with `{` (leading-char rule passes)
        // but embeds `=cmd|...` which Excel parses when the operator
        // copy-pastes the cell. The "anywhere" check catches it.
        let json = r#"{"action":"=cmd|/c calc"}"#;
        let sanitized = sanitize_csv_cell(json);
        assert!(
            sanitized.starts_with('\''),
            "JSON cell with inner `=` must be prefixed; got {sanitized}"
        );
        // The original content must be preserved (just prefixed).
        assert_eq!(&sanitized[1..], json);
    }

    #[test]
    fn sanitize_csv_cell_json_blob_with_inner_at_sign() {
        // `@` inside the JSON should also trigger the prefix.
        let json = r#"{"actor":"@SUM(A1:A9)"}"#;
        assert!(sanitize_csv_cell(json).starts_with('\''));
    }

    #[test]
    fn parse_duration_overflow_returns_none_does_not_panic() {
        // i64::MAX days would panic the legacy `Duration::days` constructor.
        // Our checked + clamped variant must reject it cleanly.
        assert!(parse_relative_duration("9223372036854775807d").is_none());
        assert!(parse_relative_duration("9223372036854775807h").is_none());
        assert!(parse_relative_duration("9223372036854775807m").is_none());
        // Negative values are also rejected.
        assert!(parse_relative_duration("-1d").is_none());
    }
}

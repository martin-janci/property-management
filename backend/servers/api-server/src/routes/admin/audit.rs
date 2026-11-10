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

use std::borrow::Cow;

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

/// Sanitize a string cell for CSV output. Closes two cell-integrity classes
/// in one call so every hand-rolled exporter can rely on a single guard:
///
///   A. **Record-separator neutralization (CR/LF).** Every embedded `\r` and
///      `\n` is collapsed to a single space. Not all callers run their cells
///      through the `csv` crate's `Writer` (which would quote newlines) — the
///      reports export ([`crate::routes::reports`]) is hand-rolled `format!`
///      with a raw `\n` row terminator and no quoting, so a free-form cell
///      containing `\n`/`\r\n` would otherwise terminate the row early and let
///      attacker-controlled content be parsed as a *new record* (CSV
///      row-injection / report tampering). Stripping the separator here means
///      no cell can carry one regardless of how the caller frames the row. For
///      `csv`-crate callers this is a harmless single-line normalization.
///
///   B. **Spreadsheet-formula neutralization (`= + - @`).** Some spreadsheet
///      apps interpret a cell that contains one of these as a formula. Two
///      checks:
///        1. The classic START-of-cell rule (leading `=+-@`).
///        2. The "anywhere" rule (M5 fix): a cell whose body contains one of
///           these chars also gets the quote prefix. This matters for the
///           JSON `details` blob, which starts with `{` (so the leading-char
///           check passes) but can embed `=cmd|...` that Excel still parses
///           when the operator copy-pastes the cell contents.
///      We prepend a single quote (`'`), the standard mitigation: it forces
///      the cell to be treated as text without being visible in most
///      spreadsheet UIs. False positives (a legit cell containing `-`) are
///      acceptable for an export — the prefix is harmless when pasted into a
///      viewer and the operator can strip it.
///
/// Note this guard does NOT escape the field delimiter (`,`): callers that
/// frame rows by hand still own that dimension (e.g. `voting_csv_row` replaces
/// `,` with `;`), and `csv`-crate callers must keep raw commas for the writer
/// to quote — folding comma-escaping in here would corrupt those exports.
///
/// `pub(crate)` so other CSV exporters (e.g. the reports export in
/// [`crate::routes::reports`]) reuse this single implementation instead of
/// re-deriving their own injection guards.
pub(crate) fn sanitize_csv_cell(value: &str) -> String {
    // A. Collapse embedded record separators so no cell can carry a raw
    //    CR/LF (see rustdoc class A above). Only allocates when needed.
    let value = if value.contains(['\r', '\n']) {
        Cow::Owned(value.replace(['\r', '\n'], " "))
    } else {
        Cow::Borrowed(value)
    };

    // B. Neutralize spreadsheet formula triggers.
    let dangerous = value.chars().any(|c| matches!(c, '=' | '+' | '-' | '@'));
    if dangerous {
        let mut out = String::with_capacity(value.len() + 1);
        out.push('\'');
        out.push_str(&value);
        out
    } else {
        value.into_owned()
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
    fn sanitize_csv_cell_collapses_embedded_crlf() {
        // #2822: a free-form cell containing a raw record separator must not
        // be able to terminate the row. Every CR/LF collapses to a space so
        // hand-rolled exporters (no `csv`-crate quoting) cannot be tricked
        // into parsing injected content as a new record.
        let out = sanitize_csv_cell("Roof\r\nInjected row");
        assert!(
            !out.contains('\r') && !out.contains('\n'),
            "no raw CR/LF may survive; got {out:?}"
        );
        // Each separator char maps to one space, so CRLF becomes two spaces.
        assert_eq!(out, "Roof  Injected row");

        // A lone LF and a lone CR are both neutralized.
        assert_eq!(sanitize_csv_cell("a\nb"), "a b");
        assert_eq!(sanitize_csv_cell("a\rb"), "a b");
    }

    #[test]
    fn sanitize_csv_cell_crlf_and_formula_trigger_combine() {
        // Both classes in one cell: CR/LF collapsed AND the leading `=` is
        // prefixed with `'`.
        let out = sanitize_csv_cell("=SUM(A1)\r\nrow2");
        assert!(!out.contains('\r') && !out.contains('\n'));
        // CRLF -> two spaces, then the leading `=` gets the `'` prefix.
        assert_eq!(out, "'=SUM(A1)  row2");
    }

    // -------------------------------------------------------------------
    // Fuzz / property coverage for `sanitize_csv_cell` (#2827 / #2822).
    //
    // The three tests below assert the sanitizer's *invariants* rather than
    // hand-picked input→output pairs, so any future edit to the guard that
    // reopens the CR / LF / CRLF record-injection or the spreadsheet
    // formula-injection vector fails here regardless of the exact payload.
    // Dependency-free: `proptest` is not a workspace dependency and crates.io
    // egress is not guaranteed in CI, so we ship a self-contained matrix +
    // xorshift fuzz loop instead of a `proptest!` macro.
    // -------------------------------------------------------------------

    /// The formula triggers the guard neutralizes at any position.
    const FORMULA_TRIGGERS: [char; 4] = ['=', '+', '-', '@'];

    /// Assert every invariant `sanitize_csv_cell` must uphold for `input`.
    ///
    /// * P1 record-safety — no raw `\r`/`\n` survives (row-injection closed).
    /// * P2 formula-safety — the cell never *begins* with a formula trigger.
    /// * P3 content-preservation — output is exactly the CR/LF-collapsed input,
    ///   optionally with a single leading `'`; nothing else is added or lost.
    /// * P4 trigger⇒prefix — a trigger anywhere forces the `'` prefix.
    fn assert_csv_cell_invariants(input: &str) {
        let out = sanitize_csv_cell(input);
        // The reference "collapsed" form: every CR/LF becomes one space.
        let collapsed = input.replace(['\r', '\n'], " ");
        let has_trigger = collapsed.chars().any(|c| FORMULA_TRIGGERS.contains(&c));

        // P1 — record-separator safety.
        assert!(
            !out.contains('\r') && !out.contains('\n'),
            "P1 record-safety violated: {input:?} -> {out:?}"
        );
        // P2 — never begins with a formula trigger.
        assert!(
            out.chars()
                .next()
                .is_none_or(|c| !FORMULA_TRIGGERS.contains(&c)),
            "P2 formula-safety violated: {input:?} -> {out:?}"
        );
        // P3 — exact content preservation (collapsed, optional single quote).
        let expected = if has_trigger {
            format!("'{collapsed}")
        } else {
            collapsed.clone()
        };
        assert_eq!(out, expected, "P3 content-preservation violated: {input:?}");
        // P4 — a trigger anywhere forces exactly one `'` prefix.
        if has_trigger {
            assert!(
                out.starts_with('\'') && out.len() == collapsed.len() + 1,
                "P4 trigger⇒prefix violated: {input:?} -> {out:?}"
            );
        }
    }

    #[test]
    fn sanitize_csv_cell_fuzz_column_matrix() {
        // "Every export column type" that funnels a free-form, attacker-
        // influenced string through `sanitize_csv_cell`: the audit-log CSV
        // columns (action / resource_type / ip_address / user_agent / details
        // JSON blob) and the reports voting-participation `title`. Each is
        // represented by a plausible base payload so a regression is reported
        // against a realistic cell shape, not just an abstract fuzz string.
        let columns: [(&str, &str); 6] = [
            ("action", "resource_accessed"),
            ("resource_type", "fault"),
            ("ip_address", "203.0.113.7"),
            ("user_agent", "Mozilla/5.0 (X11; Linux x86_64)"),
            ("details", r#"{"capability":"tenant_purge","actor":"u"}"#),
            ("voting_title", "Q3 roof-repair assessment vote"),
        ];
        // The record-separator payloads the ticket calls out explicitly.
        let separators = ["\r", "\n", "\r\n", "\n\r", "\r\r", "\n\n"];

        for (_col, base) in columns {
            // The base payload alone must round-trip cleanly.
            assert_csv_cell_invariants(base);

            for sep in separators {
                // (a) separator embedded in the middle — a second record must
                //     not be smuggled in through a hand-rolled exporter.
                assert_csv_cell_invariants(&format!("{base}{sep}injected,evil,row"));
                // (b) trailing separator.
                assert_csv_cell_invariants(&format!("{base}{sep}"));
                // (c) leading separator (could shift a trigger to the front).
                assert_csv_cell_invariants(&format!("{sep}{base}"));

                for trig in FORMULA_TRIGGERS {
                    // (d) classic leading formula trigger + separator payload.
                    assert_csv_cell_invariants(&format!("{trig}{base}{sep}=1+1"));
                    // (e) trigger *after* a separator — the collapse must not
                    //     leave the trigger exposed at a record boundary.
                    assert_csv_cell_invariants(&format!("{base}{sep}{trig}cmd|'/c calc'!A0"));
                    // (f) trigger buried mid-cell (the "anywhere" M5 rule).
                    assert_csv_cell_invariants(&format!("{base}{sep}mid{trig}dle"));
                }
            }
        }
    }

    #[test]
    fn sanitize_csv_cell_fuzz_random_hostile_inputs() {
        // Deterministic xorshift PRNG — reproducible, no external crate.
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        // A hostile alphabet weighted toward the dangerous bytes: record
        // separators, every formula trigger, CSV framing chars, plus a couple
        // of ordinary/unicode chars so the "safe" path is exercised too.
        let alphabet: [char; 16] = [
            '\r', '\n', '=', '+', '-', '@', ',', ';', '"', '\'', '{', '}', ' ', 'a', 'Z', 'ř',
        ];

        for _ in 0..20_000 {
            let len = (next() % 12) as usize;
            let mut s = String::new();
            for _ in 0..len {
                let idx = (next() as usize) % alphabet.len();
                s.push(alphabet[idx]);
            }
            assert_csv_cell_invariants(&s);
        }
    }

    #[test]
    fn sanitize_csv_cell_fuzz_every_trigger_and_separator_pairing() {
        // Exhaustive cross-product of {each formula trigger} × {CR, LF, CRLF}
        // at the cell boundary — the precise combinations named in the ticket
        // (#2827 CR/LF/CRLF × formula-injection prefixes). Each pairing must
        // be neutralized: no raw separator survives, and the leading trigger is
        // always quoted.
        for trig in FORMULA_TRIGGERS {
            for sep in ["\r", "\n", "\r\n"] {
                let payload = format!("{trig}HYPERLINK(0){sep}2ND ROW");
                let out = sanitize_csv_cell(&payload);
                assert!(
                    !out.contains('\r') && !out.contains('\n'),
                    "separator {sep:?} survived for trigger {trig:?}: {out:?}"
                );
                assert!(
                    out.starts_with('\''),
                    "leading trigger {trig:?} not quoted: {out:?}"
                );
                // The original trigger char is preserved right after the quote.
                assert_eq!(out.chars().nth(1), Some(trig));
                assert_csv_cell_invariants(&payload);
            }
        }
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

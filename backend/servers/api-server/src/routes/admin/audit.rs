//! Admin endpoints for the audit-log viewer. Phase 5.
//!
//! Reads from `audit_logs`. The append-only trigger (migration 00138)
//! protects the underlying rows; we only expose SELECTs here.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
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
/// quotes, and newlines — we only need to neutralize leading `=`, `+`,
/// `-`, `@` characters that some spreadsheet apps interpret as formulas.
///
/// We prepend a single quote (`'`) which is the standard mitigation: it
/// forces the cell to be treated as text without being visible in most
/// spreadsheet UIs.
fn sanitize_csv_cell(value: &str) -> String {
    if matches!(value.chars().next(), Some('=' | '+' | '-' | '@')) {
        let mut out = String::with_capacity(value.len() + 1);
        out.push('\'');
        out.push_str(value);
        out
    } else {
        value.to_string()
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
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
    #[serde(default)]
    pub until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub limit: Option<u32>,
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

async fn fetch_rows(
    state: &AppState,
    q: &AuditQuery,
    limit: i64,
) -> Result<Vec<AuditRow>, sqlx::Error> {
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
        ORDER BY created_at DESC
        LIMIT $7
        "#,
    )
    .bind(q.actor_id)
    .bind(q.action.as_deref())
    .bind(q.target_type.as_deref())
    .bind(q.target_id)
    .bind(q.since)
    .bind(q.until)
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

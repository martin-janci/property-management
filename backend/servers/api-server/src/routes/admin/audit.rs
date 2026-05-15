//! Admin endpoints for the audit-log viewer. Phase 5.
//!
//! Reads from `audit_logs`. The append-only trigger (migration 00138)
//! protects the underlying rows; we only expose SELECTs here.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/",
        get(list_audit_events).layer(require_capability(Capability::AuditRead)),
    )
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

/// GET /admin/audit
async fn list_audit_events(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditRow>>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(100).min(500) as i64;

    // Filter dynamically. We use a single SQL with `$N IS NULL OR …`
    // patterns rather than dynamic SQL — avoids string concat / injection.
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
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(rows))
}

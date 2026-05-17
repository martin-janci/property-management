//! Admin metrics summary endpoint. Phase 5.
//!
//! Routes:
//!   * `GET /summary` — returns a quick-glance operations summary (gated by `AuditRead`).
//!
//! Mounted under `/api/v1/admin/metrics` via `admin::router()`.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/summary",
        get(metrics_summary).layer(require_capability(Capability::AuditRead)),
    )
}

#[derive(Debug, Serialize)]
pub struct MetricsSummary {
    /// Organizations that are not soft-deleted.
    pub tenants_active: i64,
    /// Distinct users with an active session in the last 5 minutes.
    /// TODO(N4-followup): wire to actual session tracking if sessions table
    /// gains a `last_seen_at` column; currently returns a placeholder.
    pub operators_online: i64,
    /// Unresolved portal-user merge collisions (from migration 00131).
    pub pending_merges: i64,
    /// High-risk capability actions performed in the last 24 hours.
    pub high_risk_24h: i64,
}

/// GET /admin/metrics/summary
async fn metrics_summary(
    _cap: RequireCapability,
    State(state): State<AppState>,
) -> Result<Json<MetricsSummary>, (StatusCode, String)> {
    let tenants_active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM organizations WHERE soft_deleted_at IS NULL")
            .fetch_one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO(N4-followup): sessions table does not yet have a `last_seen_at`
    // column tracked per-request. Return 0 until that infrastructure lands.
    let operators_online: i64 = 0;

    let pending_merges: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_merge_collisions WHERE resolved_at IS NULL")
            .fetch_one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // High-risk capabilities: those whose invocation should be surfaced
    // immediately to operators. `PgAuditWriter::record` writes every
    // capability invocation as `action = 'resource_accessed'` with the actual
    // capability name stashed in `details->>'capability'`, so we filter on
    // the JSONB capability string — NOT on `action::text`.
    let high_risk_24h: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM audit_logs
        WHERE created_at > NOW() - INTERVAL '24 hours'
          AND action::text = 'resource_accessed'
          AND details->>'capability' IN (
              'tenant_purge',
              'principal_kind_escalate',
              'grant_principal_kind_escalate',
              'tenant_restore'
          )
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(MetricsSummary {
        tenants_active,
        operators_online,
        pending_merges,
        high_risk_24h,
    }))
}

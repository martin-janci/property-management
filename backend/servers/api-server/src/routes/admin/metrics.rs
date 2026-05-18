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
#[serde(rename_all = "camelCase")]
pub struct MetricsSummary {
    /// Organizations whose `status` is not `'deleted'`.
    ///
    /// The `organizations` table tracks deletion via a `status` column
    /// (with value `'deleted'` for soft-deleted rows) — there is no
    /// `soft_deleted_at` column. The previous query against
    /// `soft_deleted_at` would have failed with an undefined-column
    /// error at runtime.
    pub tenants_active: i64,
    /// Heuristic: distinct users who emitted any audit-log row in the last
    /// 5 minutes. There is no presence / heartbeat table yet, so this
    /// is an approximation rather than a true "currently online" count.
    /// `None` would be ideal but we keep `i64` so the frontend tile keeps
    /// rendering; callers should treat this as a lower-bound activity
    /// signal, not a live presence count.
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
    // The organizations table tracks deletion via `status = 'deleted'`,
    // not a `soft_deleted_at` column — see migrations / sibling queries in
    // `routes/tenant_config.rs` and `routes/admin/mfa/enroll.rs`.
    let tenants_active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM organizations WHERE status != 'deleted'")
            .fetch_one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Heuristic "operators online" — counts distinct user_ids that produced
    // an audit-log row in the last 5 minutes. There is no presence /
    // heartbeat table yet, so this is a lower-bound activity proxy, not a
    // true live-session count. When/if a presence table lands, swap this.
    let operators_online: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)
        FROM audit_logs
        WHERE user_id IS NOT NULL
          AND created_at > NOW() - INTERVAL '5 minutes'
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // `user_merge_collisions` (migration 00131) tracks unresolved rows via
    // `status = 'pending'` — there is no `resolved_at` column. The previous
    // query referenced a nonexistent column and would 500 at runtime.
    let pending_merges: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_merge_collisions WHERE status = 'pending'")
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

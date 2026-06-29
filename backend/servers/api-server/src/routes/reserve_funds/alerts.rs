//! Fund-alert route surface (list / acknowledge / resolve).

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use db::models::reserve_funds::FundAlert;
use uuid::Uuid;

use super::shared::{internal_error, repo_error, ApiResult};
use crate::state::AppState;

/// Alert sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/alerts", get(list_alerts))
        .route("/alerts/{alert_id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{alert_id}/resolve", post(resolve_alert))
}

/// List active alerts.
async fn list_alerts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> ApiResult<Json<Vec<FundAlert>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_active_alerts(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list alerts: {}", e)));
    rls.release().await;
    out
}

/// Acknowledge an alert.
async fn acknowledge_alert(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<FundAlert>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .acknowledge_alert(&mut **rls.conn(), org_id, alert_id, user_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("acknowledge alert", "Alert not found", e));
    rls.release().await;
    out
}

/// Resolve an alert.
async fn resolve_alert(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<FundAlert>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .resolve_alert(&mut **rls.conn(), org_id, alert_id, user_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("resolve alert", "Alert not found", e));
    rls.release().await;
    out
}

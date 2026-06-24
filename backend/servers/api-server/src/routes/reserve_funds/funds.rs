//! Fund CRUD plus the dashboard and per-fund health-report read surfaces.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use db::models::reserve_funds::{
    CreateReserveFund, FundDashboard, FundHealthReport, ReserveFund, UpdateReserveFund,
};
use uuid::Uuid;

use super::shared::{internal_error, not_found_error, repo_error, ApiResult, FundListQuery};
use crate::state::AppState;

/// Fund CRUD / dashboard / health sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_funds).post(create_fund))
        .route("/dashboard", get(get_dashboard))
        .route("/{fund_id}", get(get_fund).put(update_fund))
        .route("/{fund_id}/health", get(get_fund_health))
}

/// List reserve funds.
async fn list_funds(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<FundListQuery>,
) -> ApiResult<Json<Vec<ReserveFund>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_funds(
            &mut **rls.conn(),
            org_id,
            query.fund_type,
            query.building_id,
            query.active_only.unwrap_or(false),
        )
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to list funds: {}", e)));
    rls.release().await;
    out
}

/// Create a reserve fund.
async fn create_fund(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateReserveFund>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .reserve_fund_repo
        .create_fund(&mut **rls.conn(), org_id, req, user_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to create fund: {}", e)));
    rls.release().await;
    out
}

/// Get a reserve fund by ID.
async fn get_fund(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_fund(&mut **rls.conn(), org_id, fund_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get fund: {}", e)))
        .and_then(|f| f.map(Json).ok_or_else(|| not_found_error("Fund not found")));
    rls.release().await;
    out
}

/// Update a reserve fund.
async fn update_fund(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<UpdateReserveFund>,
) -> ApiResult<Json<ReserveFund>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .update_fund(&mut **rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("update fund", "Fund not found", e));
    rls.release().await;
    out
}

/// Get fund dashboard.
async fn get_dashboard(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> ApiResult<Json<FundDashboard>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_fund_dashboard(rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| internal_error(&format!("Failed to get dashboard: {}", e)));
    rls.release().await;
    out
}

/// Get fund health report.
async fn get_fund_health(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<FundHealthReport>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_fund_health_report(rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get health report", "Fund not found", e));
    rls.release().await;
    out
}

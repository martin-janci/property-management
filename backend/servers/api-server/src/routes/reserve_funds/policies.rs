//! Investment-policy route surface.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use db::models::reserve_funds::{CreateInvestmentPolicy, FundInvestmentPolicy};
use uuid::Uuid;

use super::shared::{insert_child_error, repo_error, ApiResult};
use crate::state::AppState;

/// Investment-policy sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/{fund_id}/policies",
            get(list_policies).post(create_policy),
        )
        .route("/{fund_id}/policies/active", get(get_active_policy))
}

/// List investment policies.
async fn list_policies(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Vec<FundInvestmentPolicy>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_investment_policies(rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("list policies", "Fund not found", e));
    rls.release().await;
    out
}

/// Create an investment policy.
async fn create_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateInvestmentPolicy>,
) -> ApiResult<Json<FundInvestmentPolicy>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_investment_policy(rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| insert_child_error("create policy", "Fund not found", e));
    rls.release().await;
    out
}

/// Get active investment policy.
async fn get_active_policy(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Option<FundInvestmentPolicy>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_active_investment_policy(rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get active policy", "Fund not found", e));
    rls.release().await;
    out
}

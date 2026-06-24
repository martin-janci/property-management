//! Fund-projection route surface (projections + their line items).

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use db::models::reserve_funds::{
    CreateFundProjection, CreateProjectionItem, FundProjection, FundProjectionItem,
};
use uuid::Uuid;

use super::shared::{insert_child_error, repo_error, ApiResult};
use crate::state::AppState;

/// Projection sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/{fund_id}/projections", post(create_projection))
        .route(
            "/{fund_id}/projections/current",
            get(get_current_projection),
        )
        .route(
            "/{fund_id}/projections/{projection_id}/items",
            get(get_projection_items).post(add_projection_items),
        )
}

/// Create a projection.
async fn create_projection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateFundProjection>,
) -> ApiResult<Json<FundProjection>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_projection(rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| insert_child_error("create projection", "Fund not found", e));
    rls.release().await;
    out
}

/// Get current projection.
async fn get_current_projection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Option<FundProjection>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_current_projection(rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get projection", "Fund not found", e));
    rls.release().await;
    out
}

/// Get projection items.
async fn get_projection_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_fund_id, projection_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Vec<FundProjectionItem>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .get_projection_items(rls.conn(), org_id, projection_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("get projection items", "Projection not found", e));
    rls.release().await;
    out
}

/// Add projection items.
async fn add_projection_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_fund_id, projection_id)): Path<(Uuid, Uuid)>,
    Json(items): Json<Vec<CreateProjectionItem>>,
) -> ApiResult<Json<Vec<FundProjectionItem>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .add_projection_items(rls.conn(), org_id, projection_id, items)
        .await
        .map(Json)
        .map_err(|e| insert_child_error("add projection items", "Projection not found", e));
    rls.release().await;
    out
}

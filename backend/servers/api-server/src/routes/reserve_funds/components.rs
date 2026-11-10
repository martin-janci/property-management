//! Fund-component route surface.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use db::models::reserve_funds::{CreateFundComponent, FundComponent, UpdateFundComponent};
use db::repositories::reserve_funds::FundChildKind;
use uuid::Uuid;

use super::shared::{insert_child_error, repo_error, require_manager, ApiResult};
use crate::state::AppState;

/// Component sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/{fund_id}/components",
            get(list_components).post(create_component),
        )
        .route(
            "/{fund_id}/components/{component_id}",
            put(update_component),
        )
}

/// List components.
async fn list_components(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
) -> ApiResult<Json<Vec<FundComponent>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_components(rls.conn(), org_id, fund_id)
        .await
        .map(Json)
        .map_err(|e| repo_error("list components", "Fund not found", e));
    rls.release().await;
    out
}

/// Create a component.
async fn create_component(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateFundComponent>,
) -> ApiResult<Json<FundComponent>> {
    require_manager(&mut rls).await?;
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_component(rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| insert_child_error("create component", "Fund not found", e));
    rls.release().await;
    out
}

/// Update a component.
async fn update_component(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((fund_id, component_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateFundComponent>,
) -> ApiResult<Json<FundComponent>> {
    require_manager(&mut rls).await?;
    let org_id = rls.tenant_id();
    // The child must belong to the fund named in the path (and the fund to
    // the caller's org), not just to some fund of the org.
    if let Err(e) = state
        .reserve_fund_repo
        .ensure_child_in_fund(
            &mut **rls.conn(),
            org_id,
            fund_id,
            FundChildKind::Component,
            component_id,
        )
        .await
    {
        rls.release().await;
        return Err(repo_error("update component", "Component not found", e));
    }
    let out = state
        .reserve_fund_repo
        .update_component(&mut **rls.conn(), org_id, component_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("update component", "Component not found", e));
    rls.release().await;
    out
}

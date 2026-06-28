//! Contribution-schedule route surface.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    routing::{get, put},
    Json, Router,
};
use db::models::reserve_funds::{
    CreateContributionSchedule, FundContributionSchedule, UpdateContributionSchedule,
};
use uuid::Uuid;

use super::shared::{insert_child_error, repo_error, ActiveOnlyQuery, ApiResult};
use crate::state::AppState;

/// Contribution-schedule sub-router.
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/{fund_id}/schedules",
            get(list_schedules).post(create_schedule),
        )
        .route("/{fund_id}/schedules/{schedule_id}", put(update_schedule))
}

/// List contribution schedules.
async fn list_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Query(query): Query<ActiveOnlyQuery>,
) -> ApiResult<Json<Vec<FundContributionSchedule>>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .list_contribution_schedules(
            rls.conn(),
            org_id,
            fund_id,
            query.active_only.unwrap_or(false),
        )
        .await
        .map(Json)
        .map_err(|e| repo_error("list schedules", "Fund not found", e));
    rls.release().await;
    out
}

/// Create a contribution schedule.
async fn create_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(fund_id): Path<Uuid>,
    Json(req): Json<CreateContributionSchedule>,
) -> ApiResult<Json<FundContributionSchedule>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .create_contribution_schedule(rls.conn(), org_id, fund_id, req)
        .await
        .map(Json)
        .map_err(|e| insert_child_error("create schedule", "Fund not found", e));
    rls.release().await;
    out
}

/// Update a contribution schedule.
async fn update_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_fund_id, schedule_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateContributionSchedule>,
) -> ApiResult<Json<FundContributionSchedule>> {
    let org_id = rls.tenant_id();
    let out = state
        .reserve_fund_repo
        .update_contribution_schedule(&mut **rls.conn(), org_id, schedule_id, req)
        .await
        .map(Json)
        .map_err(|e| repo_error("update schedule", "Schedule not found", e));
    rls.release().await;
    out
}

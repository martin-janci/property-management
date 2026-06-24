//! Vendor contract endpoints (Story 21.3).
//!
//! See [`super`] for the RLS contract that every handler upholds.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{ExpiringContract, UpdateVendorContract, VendorContract};
use uuid::Uuid;

use super::shared::{
    db_error, load_contract_for_user, not_found, CreateContractRequest, ExpiringQuery,
    ListContractsQuery,
};
use crate::state::AppState;

pub(super) async fn create_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateContractRequest>,
) -> Result<(StatusCode, Json<VendorContract>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .create_contract(&mut **rls.conn(), org_id, payload.data)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(|e| db_error("Failed to create contract", e));
    rls.release().await;
    out
}

pub(super) async fn list_contracts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListContractsQuery>,
) -> Result<Json<Vec<VendorContract>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list_contracts(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list contracts", e));
    rls.release().await;
    out
}

pub(super) async fn get_expiring_contracts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<ExpiringContract>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_expiring_contracts(&mut **rls.conn(), org_id, query.days.unwrap_or(30))
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get expiring contracts", e));
    rls.release().await;
    out
}

pub(super) async fn get_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorContract>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_contract_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

pub(super) async fn update_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendorContract>,
) -> Result<Json<VendorContract>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_contract_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .update_contract(&mut **rls.conn(), id, org_id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update contract", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn delete_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_contract_for_user(&state, &mut rls, id).await?;
        let deleted = state
            .vendor_repo
            .delete_contract(&mut **rls.conn(), id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete contract", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Contract not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

//! Core vendor endpoints (Story 21.1).
//!
//! See [`super`] for the RLS contract that every handler upholds.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{UpdateVendor, Vendor, VendorStatistics, VendorWithDetails};
use uuid::Uuid;

use super::shared::{
    db_error, load_vendor_for_user, not_found, ListVendorsQuery, OrgQuery, PreferredRequest,
};
use crate::state::AppState;

pub(super) async fn create_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<super::shared::CreateVendorRequest>,
) -> Result<(StatusCode, Json<Vendor>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .create(&mut **rls.conn(), org_id, payload.data)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| db_error("Failed to create vendor", e));
    rls.release().await;
    out
}

pub(super) async fn list_vendors(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListVendorsQuery>,
) -> Result<Json<Vec<Vendor>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list vendors", e));
    rls.release().await;
    out
}

pub(super) async fn list_vendors_with_details(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListVendorsQuery>,
) -> Result<Json<Vec<VendorWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list_with_details(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list vendors", e));
    rls.release().await;
    out
}

pub(super) async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_query): Query<OrgQuery>,
) -> Result<Json<VendorStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_statistics(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get statistics", e));
    rls.release().await;
    out
}

pub(super) async fn get_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_vendor_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

pub(super) async fn update_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendor>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        let org_id = rls.tenant_id();
        state
            .vendor_repo
            .update(&mut **rls.conn(), id, org_id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update vendor", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn delete_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        let org_id = rls.tenant_id();
        let deleted = state
            .vendor_repo
            .delete(&mut **rls.conn(), id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete vendor", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Vendor not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn set_preferred(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<PreferredRequest>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .set_preferred(&mut **rls.conn(), id, org_id, data.is_preferred)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to set preferred", e))
    }
    .await;
    rls.release().await;
    out
}

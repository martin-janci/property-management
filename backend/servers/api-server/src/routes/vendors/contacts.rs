//! Vendor contacts and ratings endpoints (Stories 21.1 / 21.2).
//!
//! See [`super`] for the RLS contract that every handler upholds.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{CreateVendorContact, CreateVendorRating, VendorContact, VendorRating};
use uuid::Uuid;

use super::shared::{db_error, load_vendor_for_user, not_found, PaginationQuery};
use crate::routes::pagination::clamp_limit;
use crate::state::AppState;

// ==================== Vendor Contacts ====================

pub(super) async fn add_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateVendorContact>,
) -> Result<(StatusCode, Json<VendorContact>), (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .add_contact(&mut **rls.conn(), id, data)
            .await
            .map(|c| (StatusCode::CREATED, Json(c)))
            .map_err(|e| db_error("Failed to add contact", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn list_contacts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VendorContact>>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .list_contacts(&mut **rls.conn(), id)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to list contacts", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn delete_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(contact_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        // Resolve the owning vendor, keyed to the caller's org (PAP-129: the
        // org join holds even where RLS does not bind). A contact whose vendor
        // is owned by another org resolves to `None` → 404. We then load the
        // vendor to confirm tenant visibility before deleting.
        let vendor_id = state
            .vendor_repo
            .find_contact_vendor_id(&mut **rls.conn(), contact_id, org_id)
            .await
            .map_err(|e| db_error("Failed to resolve contact", e))?
            .ok_or_else(|| not_found("Contact not found"))?;
        load_vendor_for_user(&state, &mut rls, vendor_id).await?;

        let deleted = state
            .vendor_repo
            .delete_contact(&mut **rls.conn(), contact_id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete contact", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Contact not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

// ==================== Vendor Ratings ====================

pub(super) async fn add_rating(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateVendorRating>,
) -> Result<(StatusCode, Json<VendorRating>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .add_rating(&mut **rls.conn(), id, user_id, data)
            .await
            .map(|r| (StatusCode::CREATED, Json(r)))
            .map_err(|e| db_error("Failed to add rating", e))
    }
    .await;
    rls.release().await;
    out
}

pub(super) async fn list_ratings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<VendorRating>>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .list_ratings(
                &mut **rls.conn(),
                id,
                clamp_limit(query.limit.map(i64::from), 50) as i32,
                query.offset.unwrap_or(0),
            )
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to list ratings", e))
    }
    .await;
    rls.release().await;
    out
}

//! Neighbor routes (Epic 6, Story 6.6: Neighbor Information).

use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{NeighborView, PrivacySettings, ProfileVisibility, UpdatePrivacySettings};
use db::repositories::UserRepository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Response Types
// ============================================================================

/// Response for neighbor list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NeighborsResponse {
    pub neighbors: Vec<NeighborView>,
    pub count: usize,
    pub total: i64,
}

/// Response for privacy settings.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PrivacySettingsResponse {
    pub settings: PrivacySettings,
}

/// Generic success response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NeighborSuccessResponse {
    pub message: String,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request to update privacy settings.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePrivacySettingsRequest {
    pub profile_visibility: Option<ProfileVisibility>,
    pub show_contact_info: Option<bool>,
}

// ============================================================================
// Router
// ============================================================================

/// Create the neighbors router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Neighbor endpoints
        .route("/buildings/{building_id}/neighbors", get(list_neighbors))
        // Privacy settings
        .route("/users/me/privacy", get(get_privacy_settings))
        .route("/users/me/privacy", put(update_privacy_settings))
}

// ============================================================================
// Neighbor Handlers
// ============================================================================

/// List neighbors in the same building.
#[utoipa::path(
    get,
    path = "/api/v1/buildings/{building_id}/neighbors",
    params(
        ("building_id" = Uuid, Path, description = "Building ID"),
    ),
    responses(
        (status = 200, description = "Neighbors retrieved successfully", body = NeighborsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Not a resident of this building", body = ErrorResponse),
    ),
    tag = "neighbors"
)]
async fn list_neighbors(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth_user: AuthUser,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
) -> Result<Json<NeighborsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Security: Verify building belongs to current tenant (Critical 1.4 fix)
    let building_org: Option<(Uuid,)> =
        sqlx::query_as("SELECT organization_id FROM buildings WHERE id = $1")
            .bind(building_id)
            .fetch_optional(&mut **rls.conn())
            .await
            .map_err(|e| {
                tracing::error!("Failed to verify building: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Failed to verify building")),
                )
            })?;

    match building_org {
        None => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Building not found")),
            ));
        }
        Some((org_id,)) if org_id != tenant.tenant_id => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "Cannot access neighbors in buildings outside your organization",
                )),
            ));
        }
        _ => {} // Building is in user's org, continue
    }

    let repo = UserRepository::new(state.db.clone());

    // Get neighbors (the repository will return empty if user is not a resident)
    let neighbors = repo
        .get_neighbors(auth_user.user_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get neighbors: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let total = repo
        .count_neighbors(auth_user.user_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count neighbors: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    // Release RLS context before returning connection to pool
    rls.release().await;

    Ok(Json(NeighborsResponse {
        count: neighbors.len(),
        neighbors,
        total,
    }))
}

// ============================================================================
// Privacy Settings Handlers
// ============================================================================

/// Get current user's privacy settings.
#[utoipa::path(
    get,
    path = "/api/v1/users/me/privacy",
    responses(
        (status = 200, description = "Privacy settings retrieved", body = PrivacySettingsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "neighbors"
)]
async fn get_privacy_settings(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<PrivacySettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = UserRepository::new(state.db.clone());

    let settings = repo
        .get_privacy_settings(auth_user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get privacy settings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(PrivacySettingsResponse { settings }))
}

/// Update current user's privacy settings.
#[utoipa::path(
    put,
    path = "/api/v1/users/me/privacy",
    request_body = UpdatePrivacySettingsRequest,
    responses(
        (status = 200, description = "Privacy settings updated", body = PrivacySettingsResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "neighbors"
)]
async fn update_privacy_settings(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(body): Json<UpdatePrivacySettingsRequest>,
) -> Result<Json<PrivacySettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = UserRepository::new(state.db.clone());

    let settings = repo
        .update_privacy_settings(
            auth_user.user_id,
            UpdatePrivacySettings {
                profile_visibility: body.profile_visibility,
                show_contact_info: body.show_contact_info,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update privacy settings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(PrivacySettingsResponse { settings }))
}

//! Registry routes (Epic 57: Building Registries).
//!
//! Provides endpoints for pet and vehicle registrations,
//! parking spots, and building registry rules.

use crate::state::AppState;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    BuildingRegistryRules, CreateParkingSpot, CreatePetRegistration, CreateVehicleRegistration,
    ParkingSpot, ParkingSpotQuery, ParkingSpotWithDetails, PetRegistration, PetRegistrationQuery,
    PetRegistrationSummary, PetRegistrationWithDetails, RegistryStatistics, ReviewRegistration,
    UpdateParkingSpot, UpdatePetRegistration, UpdateRegistryRules, UpdateVehicleRegistration,
    VehicleRegistration, VehicleRegistrationQuery, VehicleRegistrationSummary,
    VehicleRegistrationWithDetails,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Response Types
// ============================================================================

/// Response for pet registration list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PetRegistrationListResponse {
    pub registrations: Vec<PetRegistrationSummary>,
    pub total: i64,
}

/// Response for pet registration detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PetRegistrationDetailResponse {
    pub registration: PetRegistrationWithDetails,
}

/// Response for pet registration action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PetRegistrationActionResponse {
    pub message: String,
    pub registration: PetRegistration,
}

/// Response for vehicle registration list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VehicleRegistrationListResponse {
    pub registrations: Vec<VehicleRegistrationSummary>,
    pub total: i64,
}

/// Response for vehicle registration detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VehicleRegistrationDetailResponse {
    pub registration: VehicleRegistrationWithDetails,
}

/// Response for vehicle registration action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VehicleRegistrationActionResponse {
    pub message: String,
    pub registration: VehicleRegistration,
}

/// Response for parking spot list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ParkingSpotListResponse {
    pub spots: Vec<ParkingSpotWithDetails>,
    pub total: i64,
}

/// Response for parking spot detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ParkingSpotDetailResponse {
    pub spot: ParkingSpot,
}

/// Response for parking spot action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ParkingSpotActionResponse {
    pub message: String,
    pub spot: ParkingSpot,
}

/// Response for registry rules.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegistryRulesResponse {
    pub rules: BuildingRegistryRules,
}

/// Response for registry statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RegistryStatisticsResponse {
    pub statistics: RegistryStatistics,
}

// ============================================================================
// Pet Registration Endpoints
// ============================================================================

/// Creates a new pet registration.
#[utoipa::path(
    post,
    path = "/api/v1/registry/pets",
    request_body = CreatePetRegistration,
    responses(
        (status = 201, description = "Pet registration created", body = PetRegistrationActionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn create_pet_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Json(data): Json<CreatePetRegistration>,
) -> Result<(StatusCode, Json<PetRegistrationActionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .registry_repo
        .create_pet_registration(tenant.tenant_id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create pet registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CREATE_FAILED",
                    "Failed to create pet registration",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(PetRegistrationActionResponse {
            message: "Pet registration created successfully".to_string(),
            registration,
        }),
    ))
}

/// Gets a pet registration by ID.
#[utoipa::path(
    get,
    path = "/api/v1/registry/pets/{id}",
    params(
        ("id" = Uuid, Path, description = "Pet registration ID")
    ),
    responses(
        (status = 200, description = "Pet registration found", body = PetRegistrationDetailResponse),
        (status = 404, description = "Pet registration not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn get_pet_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<PetRegistrationDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .registry_repo
        .get_pet_registration_with_details(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get pet registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get pet registration",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Pet registration not found",
                )),
            )
        })?;

    Ok(Json(PetRegistrationDetailResponse { registration }))
}

/// Lists pet registrations.
#[utoipa::path(
    get,
    path = "/api/v1/registry/pets",
    params(
        ("building_id" = Option<Uuid>, Query, description = "Filter by building"),
        ("unit_id" = Option<Uuid>, Query, description = "Filter by unit"),
        ("owner_id" = Option<Uuid>, Query, description = "Filter by owner"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("pet_type" = Option<String>, Query, description = "Filter by pet type"),
        ("limit" = Option<i64>, Query, description = "Limit results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Pet registrations list", body = PetRegistrationListResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn list_pet_registrations(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Query(query): Query<PetRegistrationQuery>,
) -> Result<Json<PetRegistrationListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (registrations, total) = state
        .registry_repo
        .list_pet_registrations(tenant.tenant_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list pet registrations: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LIST_FAILED",
                    "Failed to list pet registrations",
                )),
            )
        })?;

    Ok(Json(PetRegistrationListResponse {
        registrations,
        total,
    }))
}

/// Updates a pet registration.
#[utoipa::path(
    put,
    path = "/api/v1/registry/pets/{id}",
    params(
        ("id" = Uuid, Path, description = "Pet registration ID")
    ),
    request_body = UpdatePetRegistration,
    responses(
        (status = 200, description = "Pet registration updated", body = PetRegistrationActionResponse),
        (status = 404, description = "Pet registration not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn update_pet_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePetRegistration>,
) -> Result<Json<PetRegistrationActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .registry_repo
        .update_pet_registration(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update pet registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update pet registration",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Pet registration not found",
                )),
            )
        })?;

    Ok(Json(PetRegistrationActionResponse {
        message: "Pet registration updated successfully".to_string(),
        registration,
    }))
}

/// Reviews (approves/rejects) a pet registration.
#[utoipa::path(
    post,
    path = "/api/v1/registry/pets/{id}/review",
    params(
        ("id" = Uuid, Path, description = "Pet registration ID")
    ),
    request_body = ReviewRegistration,
    responses(
        (status = 200, description = "Pet registration reviewed", body = PetRegistrationActionResponse),
        (status = 404, description = "Pet registration not found"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized to review"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn review_pet_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ReviewRegistration>,
) -> Result<Json<PetRegistrationActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .registry_repo
        .review_pet_registration(tenant.tenant_id, id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to review pet registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "REVIEW_FAILED",
                    "Failed to review pet registration",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Pet registration not found",
                )),
            )
        })?;

    let action = if registration.status == "approved" {
        "approved"
    } else {
        "rejected"
    };

    Ok(Json(PetRegistrationActionResponse {
        message: format!("Pet registration {} successfully", action),
        registration,
    }))
}

/// Deletes a pet registration.
#[utoipa::path(
    delete,
    path = "/api/v1/registry/pets/{id}",
    params(
        ("id" = Uuid, Path, description = "Pet registration ID")
    ),
    responses(
        (status = 204, description = "Pet registration deleted"),
        (status = 404, description = "Pet registration not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn delete_pet_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .registry_repo
        .delete_pet_registration(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete pet registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DELETE_FAILED",
                    "Failed to delete pet registration",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Pet registration not found",
            )),
        ))
    }
}

// ============================================================================
// Vehicle Registration Endpoints
// ============================================================================

/// Creates a new vehicle registration.
#[utoipa::path(
    post,
    path = "/api/v1/registry/vehicles",
    request_body = CreateVehicleRegistration,
    responses(
        (status = 201, description = "Vehicle registration created", body = VehicleRegistrationActionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn create_vehicle_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Json(data): Json<CreateVehicleRegistration>,
) -> Result<(StatusCode, Json<VehicleRegistrationActionResponse>), (StatusCode, Json<ErrorResponse>)>
{
    let registration = state
        .registry_repo
        .create_vehicle_registration(tenant.tenant_id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create vehicle registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CREATE_FAILED",
                    "Failed to create vehicle registration",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(VehicleRegistrationActionResponse {
            message: "Vehicle registration created successfully".to_string(),
            registration,
        }),
    ))
}

/// Gets a vehicle registration by ID.
#[utoipa::path(
    get,
    path = "/api/v1/registry/vehicles/{id}",
    params(
        ("id" = Uuid, Path, description = "Vehicle registration ID")
    ),
    responses(
        (status = 200, description = "Vehicle registration found", body = VehicleRegistrationDetailResponse),
        (status = 404, description = "Vehicle registration not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn get_vehicle_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<VehicleRegistrationDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .registry_repo
        .get_vehicle_registration_with_details(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get vehicle registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get vehicle registration",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Vehicle registration not found",
                )),
            )
        })?;

    Ok(Json(VehicleRegistrationDetailResponse { registration }))
}

/// Lists vehicle registrations.
#[utoipa::path(
    get,
    path = "/api/v1/registry/vehicles",
    params(
        ("building_id" = Option<Uuid>, Query, description = "Filter by building"),
        ("unit_id" = Option<Uuid>, Query, description = "Filter by unit"),
        ("owner_id" = Option<Uuid>, Query, description = "Filter by owner"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("vehicle_type" = Option<String>, Query, description = "Filter by vehicle type"),
        ("limit" = Option<i64>, Query, description = "Limit results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Vehicle registrations list", body = VehicleRegistrationListResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn list_vehicle_registrations(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Query(query): Query<VehicleRegistrationQuery>,
) -> Result<Json<VehicleRegistrationListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (registrations, total) = state
        .registry_repo
        .list_vehicle_registrations(tenant.tenant_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list vehicle registrations: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LIST_FAILED",
                    "Failed to list vehicle registrations",
                )),
            )
        })?;

    Ok(Json(VehicleRegistrationListResponse {
        registrations,
        total,
    }))
}

/// Updates a vehicle registration.
#[utoipa::path(
    put,
    path = "/api/v1/registry/vehicles/{id}",
    params(
        ("id" = Uuid, Path, description = "Vehicle registration ID")
    ),
    request_body = UpdateVehicleRegistration,
    responses(
        (status = 200, description = "Vehicle registration updated", body = VehicleRegistrationActionResponse),
        (status = 404, description = "Vehicle registration not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn update_vehicle_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVehicleRegistration>,
) -> Result<Json<VehicleRegistrationActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .registry_repo
        .update_vehicle_registration(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update vehicle registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update vehicle registration",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Vehicle registration not found",
                )),
            )
        })?;

    Ok(Json(VehicleRegistrationActionResponse {
        message: "Vehicle registration updated successfully".to_string(),
        registration,
    }))
}

/// Reviews (approves/rejects) a vehicle registration.
#[utoipa::path(
    post,
    path = "/api/v1/registry/vehicles/{id}/review",
    params(
        ("id" = Uuid, Path, description = "Vehicle registration ID")
    ),
    request_body = ReviewRegistration,
    responses(
        (status = 200, description = "Vehicle registration reviewed", body = VehicleRegistrationActionResponse),
        (status = 404, description = "Vehicle registration not found"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized to review"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn review_vehicle_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ReviewRegistration>,
) -> Result<Json<VehicleRegistrationActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .registry_repo
        .review_vehicle_registration(tenant.tenant_id, id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to review vehicle registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "REVIEW_FAILED",
                    "Failed to review vehicle registration",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Vehicle registration not found",
                )),
            )
        })?;

    let action = if registration.status == "approved" {
        "approved"
    } else {
        "rejected"
    };

    Ok(Json(VehicleRegistrationActionResponse {
        message: format!("Vehicle registration {} successfully", action),
        registration,
    }))
}

/// Deletes a vehicle registration.
#[utoipa::path(
    delete,
    path = "/api/v1/registry/vehicles/{id}",
    params(
        ("id" = Uuid, Path, description = "Vehicle registration ID")
    ),
    responses(
        (status = 204, description = "Vehicle registration deleted"),
        (status = 404, description = "Vehicle registration not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn delete_vehicle_registration(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .registry_repo
        .delete_vehicle_registration(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete vehicle registration: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DELETE_FAILED",
                    "Failed to delete vehicle registration",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Vehicle registration not found",
            )),
        ))
    }
}

// ============================================================================
// Parking Spot Endpoints
// ============================================================================

/// Creates a new parking spot.
#[utoipa::path(
    post,
    path = "/api/v1/registry/parking-spots",
    request_body = CreateParkingSpot,
    responses(
        (status = 201, description = "Parking spot created", body = ParkingSpotActionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn create_parking_spot(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Json(data): Json<CreateParkingSpot>,
) -> Result<(StatusCode, Json<ParkingSpotActionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let spot = state
        .registry_repo
        .create_parking_spot(tenant.tenant_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create parking spot: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CREATE_FAILED",
                    "Failed to create parking spot",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(ParkingSpotActionResponse {
            message: "Parking spot created successfully".to_string(),
            spot,
        }),
    ))
}

/// Gets a parking spot by ID.
#[utoipa::path(
    get,
    path = "/api/v1/registry/parking-spots/{id}",
    params(
        ("id" = Uuid, Path, description = "Parking spot ID")
    ),
    responses(
        (status = 200, description = "Parking spot found", body = ParkingSpotDetailResponse),
        (status = 404, description = "Parking spot not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn get_parking_spot(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<ParkingSpotDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let spot = state
        .registry_repo
        .get_parking_spot(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get parking spot: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get parking spot",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Parking spot not found")),
            )
        })?;

    Ok(Json(ParkingSpotDetailResponse { spot }))
}

/// Lists parking spots.
#[utoipa::path(
    get,
    path = "/api/v1/registry/parking-spots",
    params(
        ("building_id" = Option<Uuid>, Query, description = "Filter by building"),
        ("is_available" = Option<bool>, Query, description = "Filter by availability"),
        ("is_covered" = Option<bool>, Query, description = "Filter by covered status"),
        ("limit" = Option<i64>, Query, description = "Limit results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Parking spots list", body = ParkingSpotListResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn list_parking_spots(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Query(query): Query<ParkingSpotQuery>,
) -> Result<Json<ParkingSpotListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (spots, total) = state
        .registry_repo
        .list_parking_spots(tenant.tenant_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list parking spots: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LIST_FAILED",
                    "Failed to list parking spots",
                )),
            )
        })?;

    Ok(Json(ParkingSpotListResponse { spots, total }))
}

/// Updates a parking spot.
#[utoipa::path(
    put,
    path = "/api/v1/registry/parking-spots/{id}",
    params(
        ("id" = Uuid, Path, description = "Parking spot ID")
    ),
    request_body = UpdateParkingSpot,
    responses(
        (status = 200, description = "Parking spot updated", body = ParkingSpotActionResponse),
        (status = 404, description = "Parking spot not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn update_parking_spot(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateParkingSpot>,
) -> Result<Json<ParkingSpotActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let spot = state
        .registry_repo
        .update_parking_spot(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update parking spot: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update parking spot",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Parking spot not found")),
            )
        })?;

    Ok(Json(ParkingSpotActionResponse {
        message: "Parking spot updated successfully".to_string(),
        spot,
    }))
}

/// Deletes a parking spot.
#[utoipa::path(
    delete,
    path = "/api/v1/registry/parking-spots/{id}",
    params(
        ("id" = Uuid, Path, description = "Parking spot ID")
    ),
    responses(
        (status = 204, description = "Parking spot deleted"),
        (status = 404, description = "Parking spot not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn delete_parking_spot(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .registry_repo
        .delete_parking_spot(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete parking spot: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DELETE_FAILED",
                    "Failed to delete parking spot",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Parking spot not found")),
        ))
    }
}

// ============================================================================
// Registry Rules Endpoints
// ============================================================================

/// Gets registry rules for a building.
#[utoipa::path(
    get,
    path = "/api/v1/registry/buildings/{building_id}/rules",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    responses(
        (status = 200, description = "Registry rules found", body = RegistryRulesResponse),
        (status = 404, description = "Registry rules not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn get_registry_rules(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
) -> Result<Json<RegistryRulesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let rules = state
        .registry_repo
        .get_registry_rules(tenant.tenant_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get registry rules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get registry rules",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Registry rules not found")),
            )
        })?;

    Ok(Json(RegistryRulesResponse { rules }))
}

/// Updates (or creates) registry rules for a building.
#[utoipa::path(
    put,
    path = "/api/v1/registry/buildings/{building_id}/rules",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    request_body = UpdateRegistryRules,
    responses(
        (status = 200, description = "Registry rules updated", body = RegistryRulesResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not authorized"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn update_registry_rules(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
    Json(data): Json<UpdateRegistryRules>,
) -> Result<Json<RegistryRulesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let rules = state
        .registry_repo
        .upsert_registry_rules(tenant.tenant_id, building_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update registry rules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update registry rules",
                )),
            )
        })?;

    Ok(Json(RegistryRulesResponse { rules }))
}

// ============================================================================
// Statistics Endpoints
// ============================================================================

/// Gets registry statistics for a building.
#[utoipa::path(
    get,
    path = "/api/v1/registry/buildings/{building_id}/statistics",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    responses(
        (status = 200, description = "Registry statistics", body = RegistryStatisticsResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Registry"
)]
async fn get_registry_statistics(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
) -> Result<Json<RegistryStatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let statistics = state
        .registry_repo
        .get_statistics(tenant.tenant_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get registry statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get registry statistics",
                )),
            )
        })?;

    Ok(Json(RegistryStatisticsResponse { statistics }))
}

// ============================================================================
// Router
// ============================================================================

/// Creates the registry router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Pet registrations
        .route("/pets", post(create_pet_registration))
        .route("/pets", get(list_pet_registrations))
        .route("/pets/{id}", get(get_pet_registration))
        .route("/pets/{id}", put(update_pet_registration))
        .route("/pets/{id}", delete(delete_pet_registration))
        .route("/pets/{id}/review", post(review_pet_registration))
        // Vehicle registrations
        .route("/vehicles", post(create_vehicle_registration))
        .route("/vehicles", get(list_vehicle_registrations))
        .route("/vehicles/{id}", get(get_vehicle_registration))
        .route("/vehicles/{id}", put(update_vehicle_registration))
        .route("/vehicles/{id}", delete(delete_vehicle_registration))
        .route("/vehicles/{id}/review", post(review_vehicle_registration))
        // Parking spots
        .route("/parking-spots", post(create_parking_spot))
        .route("/parking-spots", get(list_parking_spots))
        .route("/parking-spots/{id}", get(get_parking_spot))
        .route("/parking-spots/{id}", put(update_parking_spot))
        .route("/parking-spots/{id}", delete(delete_parking_spot))
        // Building registry rules and statistics
        .route("/buildings/{building_id}/rules", get(get_registry_rules))
        .route("/buildings/{building_id}/rules", put(update_registry_rules))
        .route(
            "/buildings/{building_id}/statistics",
            get(get_registry_statistics),
        )
}

//! Package and Visitor routes (Epic 58: Package & Visitor Management).
//!
//! Provides endpoints for package tracking, visitor pre-registration,
//! and access code verification.

use crate::state::AppState;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use common::TenantRole;
use db::models::PackageSummary;
use db::models::{
    AccessCodeVerification, BuildingPackageSettings, BuildingVisitorSettings, CheckInVisitor,
    CheckOutVisitor, CreatePackage, CreateVisitor, Package, PackageQuery, PackageStatistics,
    PackageWithDetails, PickupPackage, ReceivePackage, UpdateBuildingPackageSettings,
    UpdateBuildingVisitorSettings, UpdatePackage, UpdateVisitor, VerifyAccessCode, Visitor,
    VisitorQuery, VisitorStatistics, VisitorSummary, VisitorWithDetails,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Authorization Helpers
// ============================================================================

/// Check if user has staff or manager role.
/// Staff-level operations require at least Manager, TechnicalManager, OrgAdmin, or SuperAdmin.
fn require_staff_role(auth: &AuthUser) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    match &auth.role {
        Some(TenantRole::Manager)
        | Some(TenantRole::TechnicalManager)
        | Some(TenantRole::OrgAdmin)
        | Some(TenantRole::SuperAdmin)
        | Some(TenantRole::PropertyManager) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "This action requires staff or manager privileges",
            )),
        )),
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for package list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageListResponse {
    pub packages: Vec<PackageSummary>,
    pub total: i64,
}

/// Response for package detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageDetailResponse {
    pub package: PackageWithDetails,
}

/// Response for package action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageActionResponse {
    pub message: String,
    pub package: Package,
}

/// Response for visitor list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VisitorListResponse {
    pub visitors: Vec<VisitorSummary>,
    pub total: i64,
}

/// Response for visitor detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VisitorDetailResponse {
    pub visitor: VisitorWithDetails,
}

/// Response for visitor action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VisitorActionResponse {
    pub message: String,
    pub visitor: Visitor,
}

/// Response for package settings.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageSettingsResponse {
    pub settings: BuildingPackageSettings,
}

/// Response for visitor settings.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VisitorSettingsResponse {
    pub settings: BuildingVisitorSettings,
}

/// Response for package statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PackageStatisticsResponse {
    pub statistics: PackageStatistics,
}

/// Response for visitor statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VisitorStatisticsResponse {
    pub statistics: VisitorStatistics,
}

// ============================================================================
// Package Endpoints
// ============================================================================

/// Creates a new package registration.
#[utoipa::path(
    post,
    path = "/api/v1/packages",
    request_body = CreatePackage,
    responses(
        (status = 201, description = "Package registered", body = PackageActionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn create_package(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Json(data): Json<CreatePackage>,
) -> Result<(StatusCode, Json<PackageActionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let package = state
        .package_visitor_repo
        .create_package(tenant.tenant_id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create package: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CREATE_FAILED",
                    "Failed to register package",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(PackageActionResponse {
            message: "Package registered successfully".to_string(),
            package,
        }),
    ))
}

/// Gets a package by ID.
#[utoipa::path(
    get,
    path = "/api/v1/packages/{id}",
    params(
        ("id" = Uuid, Path, description = "Package ID")
    ),
    responses(
        (status = 200, description = "Package found", body = PackageDetailResponse),
        (status = 404, description = "Package not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn get_package(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<PackageDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let package = state
        .package_visitor_repo
        .get_package_with_details(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get package: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("FETCH_FAILED", "Failed to get package")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Package not found")),
            )
        })?;

    Ok(Json(PackageDetailResponse { package }))
}

/// Lists packages.
#[utoipa::path(
    get,
    path = "/api/v1/packages",
    params(
        ("building_id" = Option<Uuid>, Query, description = "Filter by building"),
        ("unit_id" = Option<Uuid>, Query, description = "Filter by unit"),
        ("resident_id" = Option<Uuid>, Query, description = "Filter by resident"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("carrier" = Option<String>, Query, description = "Filter by carrier"),
        ("limit" = Option<i64>, Query, description = "Limit results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Package list", body = PackageListResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn list_packages(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Query(query): Query<PackageQuery>,
) -> Result<Json<PackageListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (packages, total) = state
        .package_visitor_repo
        .list_packages(tenant.tenant_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list packages: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("LIST_FAILED", "Failed to list packages")),
            )
        })?;

    Ok(Json(PackageListResponse { packages, total }))
}

/// Updates a package.
#[utoipa::path(
    put,
    path = "/api/v1/packages/{id}",
    params(
        ("id" = Uuid, Path, description = "Package ID")
    ),
    request_body = UpdatePackage,
    responses(
        (status = 200, description = "Package updated", body = PackageActionResponse),
        (status = 404, description = "Package not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn update_package(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePackage>,
) -> Result<Json<PackageActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let package = state
        .package_visitor_repo
        .update_package(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update package: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update package",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Package not found")),
            )
        })?;

    Ok(Json(PackageActionResponse {
        message: "Package updated successfully".to_string(),
        package,
    }))
}

/// Marks a package as received (staff only).
#[utoipa::path(
    post,
    path = "/api/v1/packages/{id}/receive",
    params(
        ("id" = Uuid, Path, description = "Package ID")
    ),
    request_body = ReceivePackage,
    responses(
        (status = 200, description = "Package received", body = PackageActionResponse),
        (status = 404, description = "Package not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn receive_package(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ReceivePackage>,
) -> Result<Json<PackageActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only staff/managers can mark packages as received
    require_staff_role(&auth)?;

    let package = state
        .package_visitor_repo
        .receive_package(tenant.tenant_id, id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to receive package: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "RECEIVE_FAILED",
                    "Failed to log package receipt",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Package not found or already received",
                )),
            )
        })?;

    Ok(Json(PackageActionResponse {
        message: "Package marked as received".to_string(),
        package,
    }))
}

/// Marks a package as picked up.
#[utoipa::path(
    post,
    path = "/api/v1/packages/{id}/pickup",
    params(
        ("id" = Uuid, Path, description = "Package ID")
    ),
    request_body = PickupPackage,
    responses(
        (status = 200, description = "Package picked up", body = PackageActionResponse),
        (status = 404, description = "Package not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn pickup_package(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<PickupPackage>,
) -> Result<Json<PackageActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let package = state
        .package_visitor_repo
        .pickup_package(tenant.tenant_id, id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to pickup package: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "PICKUP_FAILED",
                    "Failed to log package pickup",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Package not found or not ready for pickup",
                )),
            )
        })?;

    Ok(Json(PackageActionResponse {
        message: "Package picked up successfully".to_string(),
        package,
    }))
}

/// Deletes a package.
#[utoipa::path(
    delete,
    path = "/api/v1/packages/{id}",
    params(
        ("id" = Uuid, Path, description = "Package ID")
    ),
    responses(
        (status = 204, description = "Package deleted"),
        (status = 404, description = "Package not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn delete_package(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .package_visitor_repo
        .delete_package(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete package: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DELETE_FAILED",
                    "Failed to delete package",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Package not found")),
        ))
    }
}

/// Gets package statistics for a building.
#[utoipa::path(
    get,
    path = "/api/v1/packages/buildings/{building_id}/statistics",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    responses(
        (status = 200, description = "Package statistics", body = PackageStatisticsResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn get_package_statistics(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
) -> Result<Json<PackageStatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let statistics = state
        .package_visitor_repo
        .get_package_statistics(tenant.tenant_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get package statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get package statistics",
                )),
            )
        })?;

    Ok(Json(PackageStatisticsResponse { statistics }))
}

// ============================================================================
// Visitor Endpoints
// ============================================================================

/// Pre-registers a visitor.
#[utoipa::path(
    post,
    path = "/api/v1/visitors",
    request_body = CreateVisitor,
    responses(
        (status = 201, description = "Visitor registered", body = VisitorActionResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn create_visitor(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Json(data): Json<CreateVisitor>,
) -> Result<(StatusCode, Json<VisitorActionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let visitor = state
        .package_visitor_repo
        .create_visitor(tenant.tenant_id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create visitor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CREATE_FAILED",
                    "Failed to pre-register visitor",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(VisitorActionResponse {
            message: format!("Visitor registered. Access code: {}", visitor.access_code),
            visitor,
        }),
    ))
}

/// Gets a visitor by ID.
#[utoipa::path(
    get,
    path = "/api/v1/visitors/{id}",
    params(
        ("id" = Uuid, Path, description = "Visitor ID")
    ),
    responses(
        (status = 200, description = "Visitor found", body = VisitorDetailResponse),
        (status = 404, description = "Visitor not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn get_visitor(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<VisitorDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visitor = state
        .package_visitor_repo
        .get_visitor_with_details(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get visitor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("FETCH_FAILED", "Failed to get visitor")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Visitor not found")),
            )
        })?;

    Ok(Json(VisitorDetailResponse { visitor }))
}

/// Lists visitors.
#[utoipa::path(
    get,
    path = "/api/v1/visitors",
    params(
        ("building_id" = Option<Uuid>, Query, description = "Filter by building"),
        ("unit_id" = Option<Uuid>, Query, description = "Filter by unit"),
        ("host_id" = Option<Uuid>, Query, description = "Filter by host"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("purpose" = Option<String>, Query, description = "Filter by purpose"),
        ("today_only" = Option<bool>, Query, description = "Only today's visitors"),
        ("limit" = Option<i64>, Query, description = "Limit results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Visitor list", body = VisitorListResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn list_visitors(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Query(query): Query<VisitorQuery>,
) -> Result<Json<VisitorListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (visitors, total) = state
        .package_visitor_repo
        .list_visitors(tenant.tenant_id, query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list visitors: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("LIST_FAILED", "Failed to list visitors")),
            )
        })?;

    Ok(Json(VisitorListResponse { visitors, total }))
}

/// Updates a visitor registration.
#[utoipa::path(
    put,
    path = "/api/v1/visitors/{id}",
    params(
        ("id" = Uuid, Path, description = "Visitor ID")
    ),
    request_body = UpdateVisitor,
    responses(
        (status = 200, description = "Visitor updated", body = VisitorActionResponse),
        (status = 404, description = "Visitor not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn update_visitor(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVisitor>,
) -> Result<Json<VisitorActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visitor = state
        .package_visitor_repo
        .update_visitor(tenant.tenant_id, id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update visitor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update visitor",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Visitor not found or already checked in",
                )),
            )
        })?;

    Ok(Json(VisitorActionResponse {
        message: "Visitor updated successfully".to_string(),
        visitor,
    }))
}

/// Checks in a visitor (staff only).
#[utoipa::path(
    post,
    path = "/api/v1/visitors/{id}/check-in",
    params(
        ("id" = Uuid, Path, description = "Visitor ID")
    ),
    request_body = CheckInVisitor,
    responses(
        (status = 200, description = "Visitor checked in", body = VisitorActionResponse),
        (status = 404, description = "Visitor not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn check_in_visitor(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CheckInVisitor>,
) -> Result<Json<VisitorActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only staff/managers can check in visitors
    require_staff_role(&auth)?;

    let visitor = state
        .package_visitor_repo
        .check_in_visitor(tenant.tenant_id, id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check in visitor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CHECK_IN_FAILED",
                    "Failed to check in visitor",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Visitor not found or already checked in",
                )),
            )
        })?;

    Ok(Json(VisitorActionResponse {
        message: "Visitor checked in successfully".to_string(),
        visitor,
    }))
}

/// Checks out a visitor (staff only).
#[utoipa::path(
    post,
    path = "/api/v1/visitors/{id}/check-out",
    params(
        ("id" = Uuid, Path, description = "Visitor ID")
    ),
    request_body = CheckOutVisitor,
    responses(
        (status = 200, description = "Visitor checked out", body = VisitorActionResponse),
        (status = 404, description = "Visitor not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn check_out_visitor(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CheckOutVisitor>,
) -> Result<Json<VisitorActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only staff/managers can check out visitors
    require_staff_role(&auth)?;

    let visitor = state
        .package_visitor_repo
        .check_out_visitor(tenant.tenant_id, id, auth.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check out visitor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CHECK_OUT_FAILED",
                    "Failed to check out visitor",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Visitor not found or not checked in",
                )),
            )
        })?;

    Ok(Json(VisitorActionResponse {
        message: "Visitor checked out successfully".to_string(),
        visitor,
    }))
}

/// Cancels a visitor registration.
#[utoipa::path(
    post,
    path = "/api/v1/visitors/{id}/cancel",
    params(
        ("id" = Uuid, Path, description = "Visitor ID")
    ),
    responses(
        (status = 200, description = "Visitor cancelled", body = VisitorActionResponse),
        (status = 404, description = "Visitor not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn cancel_visitor(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<VisitorActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visitor = state
        .package_visitor_repo
        .cancel_visitor(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to cancel visitor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CANCEL_FAILED",
                    "Failed to cancel visitor",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Visitor not found or already active",
                )),
            )
        })?;

    Ok(Json(VisitorActionResponse {
        message: "Visitor registration cancelled".to_string(),
        visitor,
    }))
}

/// Verifies a visitor access code.
#[utoipa::path(
    post,
    path = "/api/v1/visitors/verify-code",
    request_body = VerifyAccessCode,
    responses(
        (status = 200, description = "Access code verification result", body = AccessCodeVerification),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn verify_access_code(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Json(data): Json<VerifyAccessCode>,
) -> Result<Json<AccessCodeVerification>, (StatusCode, Json<ErrorResponse>)> {
    let verification = state
        .package_visitor_repo
        .verify_access_code(tenant.tenant_id, data.building_id, &data.access_code)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify access code: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "VERIFY_FAILED",
                    "Failed to verify access code",
                )),
            )
        })?;

    Ok(Json(verification))
}

/// Deletes a visitor registration.
#[utoipa::path(
    delete,
    path = "/api/v1/visitors/{id}",
    params(
        ("id" = Uuid, Path, description = "Visitor ID")
    ),
    responses(
        (status = 204, description = "Visitor deleted"),
        (status = 404, description = "Visitor not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn delete_visitor(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .package_visitor_repo
        .delete_visitor(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete visitor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DELETE_FAILED",
                    "Failed to delete visitor",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Visitor not found")),
        ))
    }
}

/// Gets visitor statistics for a building.
#[utoipa::path(
    get,
    path = "/api/v1/visitors/buildings/{building_id}/statistics",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    responses(
        (status = 200, description = "Visitor statistics", body = VisitorStatisticsResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn get_visitor_statistics(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
) -> Result<Json<VisitorStatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let statistics = state
        .package_visitor_repo
        .get_visitor_statistics(tenant.tenant_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get visitor statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get visitor statistics",
                )),
            )
        })?;

    Ok(Json(VisitorStatisticsResponse { statistics }))
}

// ============================================================================
// Settings Endpoints
// ============================================================================

/// Gets package settings for a building.
#[utoipa::path(
    get,
    path = "/api/v1/packages/buildings/{building_id}/settings",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    responses(
        (status = 200, description = "Package settings", body = PackageSettingsResponse),
        (status = 404, description = "Settings not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn get_package_settings(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
) -> Result<Json<PackageSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state
        .package_visitor_repo
        .get_package_settings(tenant.tenant_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get package settings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get package settings",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Package settings not found",
                )),
            )
        })?;

    Ok(Json(PackageSettingsResponse { settings }))
}

/// Updates package settings for a building.
#[utoipa::path(
    put,
    path = "/api/v1/packages/buildings/{building_id}/settings",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    request_body = UpdateBuildingPackageSettings,
    responses(
        (status = 200, description = "Package settings updated", body = PackageSettingsResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Packages"
)]
async fn update_package_settings(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
    Json(data): Json<UpdateBuildingPackageSettings>,
) -> Result<Json<PackageSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state
        .package_visitor_repo
        .upsert_package_settings(tenant.tenant_id, building_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update package settings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update package settings",
                )),
            )
        })?;

    Ok(Json(PackageSettingsResponse { settings }))
}

/// Gets visitor settings for a building.
#[utoipa::path(
    get,
    path = "/api/v1/visitors/buildings/{building_id}/settings",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    responses(
        (status = 200, description = "Visitor settings", body = VisitorSettingsResponse),
        (status = 404, description = "Settings not found"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn get_visitor_settings(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
) -> Result<Json<VisitorSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state
        .package_visitor_repo
        .get_visitor_settings(tenant.tenant_id, building_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get visitor settings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "FETCH_FAILED",
                    "Failed to get visitor settings",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Visitor settings not found",
                )),
            )
        })?;

    Ok(Json(VisitorSettingsResponse { settings }))
}

/// Updates visitor settings for a building.
#[utoipa::path(
    put,
    path = "/api/v1/visitors/buildings/{building_id}/settings",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    request_body = UpdateBuildingVisitorSettings,
    responses(
        (status = 200, description = "Visitor settings updated", body = VisitorSettingsResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
    tag = "Visitors"
)]
async fn update_visitor_settings(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(building_id): Path<Uuid>,
    Json(data): Json<UpdateBuildingVisitorSettings>,
) -> Result<Json<VisitorSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let settings = state
        .package_visitor_repo
        .upsert_visitor_settings(tenant.tenant_id, building_id, data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update visitor settings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "UPDATE_FAILED",
                    "Failed to update visitor settings",
                )),
            )
        })?;

    Ok(Json(VisitorSettingsResponse { settings }))
}

// ============================================================================
// Router
// ============================================================================

/// Creates the packages router.
pub fn packages_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_package))
        .route("/", get(list_packages))
        .route("/{id}", get(get_package))
        .route("/{id}", put(update_package))
        .route("/{id}", delete(delete_package))
        .route("/{id}/receive", post(receive_package))
        .route("/{id}/pickup", post(pickup_package))
        .route(
            "/buildings/{building_id}/settings",
            get(get_package_settings),
        )
        .route(
            "/buildings/{building_id}/settings",
            put(update_package_settings),
        )
        .route(
            "/buildings/{building_id}/statistics",
            get(get_package_statistics),
        )
}

/// Creates the visitors router.
pub fn visitors_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_visitor))
        .route("/", get(list_visitors))
        .route("/verify-code", post(verify_access_code))
        .route("/{id}", get(get_visitor))
        .route("/{id}", put(update_visitor))
        .route("/{id}", delete(delete_visitor))
        .route("/{id}/check-in", post(check_in_visitor))
        .route("/{id}/check-out", post(check_out_visitor))
        .route("/{id}/cancel", post(cancel_visitor))
        .route(
            "/buildings/{building_id}/settings",
            get(get_visitor_settings),
        )
        .route(
            "/buildings/{building_id}/settings",
            put(update_visitor_settings),
        )
        .route(
            "/buildings/{building_id}/statistics",
            get(get_visitor_statistics),
        )
}

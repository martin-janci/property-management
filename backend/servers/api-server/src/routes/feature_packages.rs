//! Feature package routes (Epic 108).
//!
//! API routes for managing feature packages, including admin CRUD operations
//! and public endpoints for package listings and comparisons.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::feature_package::{
    BatchAddFeatures, CreateFeaturePackage, CreateOrganizationPackage, FeatureComparisonRow,
    FeaturePackage, FeaturePackageItem, FeaturePackageQuery, FeaturePackageSummary,
    FeaturePackageWithFeatures, OrganizationPackage, OrganizationPackageWithDetails,
    PackageComparison, PublicPackage, UpdateFeaturePackage,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

// ==================== Authorization Helpers ====================

/// Super admin role names for platform-level operations.
const SUPER_ADMIN_ROLES: &[&str] = &[
    "SuperAdministrator",
    "super_admin",
    "superadmin",
    "platform_admin",
];

/// Check if the user has super admin role.
fn has_super_admin_role(roles: &Option<Vec<String>>) -> bool {
    match roles {
        Some(user_roles) => user_roles.iter().any(|r| {
            SUPER_ADMIN_ROLES
                .iter()
                .any(|admin| r.eq_ignore_ascii_case(admin))
        }),
        None => false,
    }
}

/// Require super admin role for platform-level operations.
fn require_super_admin(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    let token = &auth_header[7..];
    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|e| {
            tracing::debug!(error = %e, "Invalid access token");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            )
        })?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    if !has_super_admin_role(&claims.roles) {
        tracing::warn!(
            user_id = %user_id,
            email = %claims.email,
            roles = ?claims.roles,
            "Unauthorized feature package admin access attempt"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_PERMISSIONS",
                "Super Admin role required for feature package management",
            )),
        ));
    }

    Ok(user_id)
}

/// Verify user has access to the organization.
#[allow(dead_code)]
async fn verify_org_access(
    rls: &mut RlsConnection,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_member: Option<(bool,)> = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM organization_members WHERE user_id = $1 AND organization_id = $2)",
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_optional(&mut **rls.conn())
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to check org membership");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Database error")),
        )
    })?;

    match is_member {
        Some((true,)) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have access to this organization",
            )),
        )),
    }
}

/// Create combined routes for feature packages.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(admin_router())
        .nest("/public", public_router())
}

/// Create admin routes for feature packages.
pub fn admin_router() -> Router<AppState> {
    Router::new()
        // Feature Package CRUD
        .route("/", get(list_packages))
        .route("/", post(create_package))
        .route("/{id}", get(get_package))
        .route("/{id}", put(update_package))
        .route("/{id}", delete(delete_package))
        // Feature management
        .route("/{id}/features", post(add_features))
        .route("/{id}/features/{fid}", delete(remove_feature))
        // Organization packages
        .route("/organizations/{org_id}", get(get_org_packages))
        .route("/organizations/{org_id}/assign", post(assign_package))
        .route(
            "/organizations/{org_id}/packages/{pid}",
            delete(deactivate_org_package),
        )
}

/// Create public routes for feature packages.
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_public_packages))
        .route("/compare", get(compare_packages))
        .route("/{id}", get(get_public_package))
}

// ==================== Request/Response Types ====================

/// Query parameters for listing packages.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct ListPackagesQuery {
    pub package_type: Option<String>,
    pub is_active: Option<bool>,
    pub is_public: Option<bool>,
    pub linked_plan_id: Option<Uuid>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<ListPackagesQuery> for FeaturePackageQuery {
    fn from(q: ListPackagesQuery) -> Self {
        FeaturePackageQuery {
            package_type: q.package_type,
            is_active: q.is_active,
            is_public: q.is_public,
            linked_plan_id: q.linked_plan_id,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Query parameters for comparing packages.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ComparePackagesQuery {
    /// Comma-separated list of package IDs to compare
    pub ids: String,
}

/// Assign package request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignPackageRequest {
    pub package_id: Uuid,
    pub source: String,
    pub subscription_id: Option<Uuid>,
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
}

// ==================== Admin Routes ====================

/// List all feature packages (admin).
#[utoipa::path(
    get,
    path = "/api/v1/admin/feature-packages",
    params(ListPackagesQuery),
    responses(
        (status = 200, description = "Packages retrieved", body = Vec<FeaturePackageSummary>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn list_packages(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<ListPackagesQuery>,
) -> Result<Json<Vec<FeaturePackageSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let packages = state
        .feature_package_repo
        .list_packages(query.into())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list packages");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(packages))
}

/// Create a feature package (admin).
#[utoipa::path(
    post,
    path = "/api/v1/admin/feature-packages",
    request_body = CreateFeaturePackage,
    responses(
        (status = 201, description = "Package created", body = FeaturePackage),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn create_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(data): Json<CreateFeaturePackage>,
) -> Result<(StatusCode, Json<FeaturePackage>), (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let package = state
        .feature_package_repo
        .create_package(data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create package");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok((StatusCode::CREATED, Json(package)))
}

/// Get a feature package by ID (admin).
#[utoipa::path(
    get,
    path = "/api/v1/admin/feature-packages/{id}",
    params(("id" = Uuid, Path, description = "Package ID")),
    responses(
        (status = 200, description = "Package retrieved", body = FeaturePackageWithFeatures),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn get_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<FeaturePackageWithFeatures>, (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let package = state
        .feature_package_repo
        .get_package_with_features(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get package");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Package not found")),
            )
        })?;

    Ok(Json(package))
}

/// Update a feature package (admin).
#[utoipa::path(
    put,
    path = "/api/v1/admin/feature-packages/{id}",
    params(("id" = Uuid, Path, description = "Package ID")),
    request_body = UpdateFeaturePackage,
    responses(
        (status = 200, description = "Package updated", body = FeaturePackage),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn update_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateFeaturePackage>,
) -> Result<Json<FeaturePackage>, (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let package = state
        .feature_package_repo
        .update_package(id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update package");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(package))
}

/// Delete a feature package (soft delete, admin).
#[utoipa::path(
    delete,
    path = "/api/v1/admin/feature-packages/{id}",
    params(("id" = Uuid, Path, description = "Package ID")),
    responses(
        (status = 204, description = "Package deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn delete_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let deleted = state
        .feature_package_repo
        .soft_delete(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete package");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
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

/// Add features to a package (admin).
#[utoipa::path(
    post,
    path = "/api/v1/admin/feature-packages/{id}/features",
    params(("id" = Uuid, Path, description = "Package ID")),
    request_body = BatchAddFeatures,
    responses(
        (status = 201, description = "Features added", body = Vec<FeaturePackageItem>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn add_features(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(data): Json<BatchAddFeatures>,
) -> Result<(StatusCode, Json<Vec<FeaturePackageItem>>), (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let items = state
        .feature_package_repo
        .add_features_batch(id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to add features");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok((StatusCode::CREATED, Json(items)))
}

/// Remove a feature from a package (admin).
#[utoipa::path(
    delete,
    path = "/api/v1/admin/feature-packages/{id}/features/{fid}",
    params(
        ("id" = Uuid, Path, description = "Package ID"),
        ("fid" = Uuid, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 204, description = "Feature removed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 404, description = "Feature not found in package"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn remove_feature(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((id, fid)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let removed = state
        .feature_package_repo
        .remove_feature(id, fid)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to remove feature");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    if removed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Feature not found in package",
            )),
        ))
    }
}

/// Get packages for an organization (admin).
#[utoipa::path(
    get,
    path = "/api/v1/admin/feature-packages/organizations/{org_id}",
    params(("org_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization packages retrieved", body = Vec<OrganizationPackageWithDetails>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn get_org_packages(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<Vec<OrganizationPackageWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let packages = state
        .feature_package_repo
        .get_organization_packages(org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get org packages");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(packages))
}

/// Assign a package to an organization (admin).
#[utoipa::path(
    post,
    path = "/api/v1/admin/feature-packages/organizations/{org_id}/assign",
    params(("org_id" = Uuid, Path, description = "Organization ID")),
    request_body = AssignPackageRequest,
    responses(
        (status = 201, description = "Package assigned", body = OrganizationPackage),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn assign_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(org_id): Path<Uuid>,
    Json(data): Json<AssignPackageRequest>,
) -> Result<(StatusCode, Json<OrganizationPackage>), (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let org_package = state
        .feature_package_repo
        .assign_to_organization(CreateOrganizationPackage {
            organization_id: org_id,
            package_id: data.package_id,
            source: data.source,
            subscription_id: data.subscription_id,
            valid_from: None,
            valid_until: data.valid_until,
            metadata: None,
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to assign package");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok((StatusCode::CREATED, Json(org_package)))
}

/// Deactivate an organization's package (admin).
#[utoipa::path(
    delete,
    path = "/api/v1/admin/feature-packages/organizations/{org_id}/packages/{pid}",
    params(
        ("org_id" = Uuid, Path, description = "Organization ID"),
        ("pid" = Uuid, Path, description = "Organization package ID")
    ),
    responses(
        (status = 204, description = "Package deactivated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - platform admin only"),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages Admin"
)]
async fn deactivate_org_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((_org_id, pid)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let _admin_id = require_super_admin(&headers, &state)?;

    let deactivated = state
        .feature_package_repo
        .deactivate_organization_package(pid)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to deactivate package");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    if deactivated {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Package not found")),
        ))
    }
}

// ==================== Public Routes ====================

/// List public feature packages.
///
/// This endpoint is intentionally public to allow potential customers to view
/// available packages without authentication.
#[utoipa::path(
    get,
    path = "/api/v1/features/packages",
    responses(
        (status = 200, description = "Public packages retrieved", body = Vec<PublicPackage>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages"
)]
async fn list_public_packages(
    State(state): State<AppState>,
) -> Result<Json<Vec<PublicPackage>>, (StatusCode, Json<ErrorResponse>)> {
    let packages = state
        .feature_package_repo
        .list_public_packages()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list public packages");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(packages))
}

/// Get a public package with features.
#[utoipa::path(
    get,
    path = "/api/v1/features/packages/{id}",
    params(("id" = Uuid, Path, description = "Package ID")),
    responses(
        (status = 200, description = "Package retrieved", body = FeaturePackageWithFeatures),
        (status = 404, description = "Package not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages"
)]
async fn get_public_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<FeaturePackageWithFeatures>, (StatusCode, Json<ErrorResponse>)> {
    let package = state
        .feature_package_repo
        .get_public_package_with_features(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get public package");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Package not found")),
            )
        })?;

    Ok(Json(package))
}

/// Compare multiple packages.
#[utoipa::path(
    get,
    path = "/api/v1/features/packages/compare",
    params(ComparePackagesQuery),
    responses(
        (status = 200, description = "Package comparison retrieved", body = PackageComparison),
        (status = 400, description = "Invalid package IDs"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Feature Packages"
)]
async fn compare_packages(
    State(state): State<AppState>,
    Query(query): Query<ComparePackagesQuery>,
) -> Result<Json<PackageComparison>, (StatusCode, Json<ErrorResponse>)> {
    // Parse comma-separated IDs
    let package_ids: Result<Vec<Uuid>, _> =
        query.ids.split(',').map(|s| s.trim().parse()).collect();

    let package_ids = package_ids.map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_IDS", "Invalid package IDs")),
        )
    })?;

    if package_ids.is_empty() || package_ids.len() > 5 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_IDS", "Provide 1-5 package IDs")),
        ));
    }

    let (packages, all_features) = state
        .feature_package_repo
        .compare_packages(package_ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to compare packages");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    // Build feature comparison rows
    let mut feature_map: std::collections::HashMap<String, FeatureComparisonRow> =
        std::collections::HashMap::new();

    for feature in all_features {
        let entry = feature_map
            .entry(feature.feature_key.clone())
            .or_insert_with(|| FeatureComparisonRow {
                feature_key: feature.feature_key.clone(),
                feature_name: feature.feature_name.clone(),
                feature_description: feature.feature_description.clone(),
                packages: serde_json::json!({}),
            });

        // Add this package's inclusion details
        if let Some(obj) = entry.packages.as_object_mut() {
            obj.insert(
                feature.package_id.to_string(),
                serde_json::json!({
                    "included": true,
                    "usage_limit": feature.usage_limit,
                    "usage_unit": feature.usage_unit,
                    "custom_description": feature.custom_description
                }),
            );
        }
    }

    // Fill in missing packages for each feature
    for row in feature_map.values_mut() {
        if let Some(obj) = row.packages.as_object_mut() {
            for pkg in &packages {
                if !obj.contains_key(&pkg.id.to_string()) {
                    obj.insert(pkg.id.to_string(), serde_json::Value::Null);
                }
            }
        }
    }

    let features: Vec<FeatureComparisonRow> = feature_map.into_values().collect();

    Ok(Json(PackageComparison { packages, features }))
}

//! Platform-admin feature-flag endpoints.
//!
//! Story 10B.2 — feature flag CRUD, per-scope overrides, and the public
//! resolved-flags endpoint. Behaviour is identical to the original
//! `platform_admin.rs`; this is a pure structural move.

use admin_core::RequireCapability;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{CreateFeatureFlagOverrideRequest, CreateFeatureFlagRequest, FeatureFlag};
use db::repositories::{FeatureFlagWithCount, FeatureFlagWithOverrides, ResolvedFeatureFlag};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::extract_super_admin_token;
use crate::state::AppState;

// ==================== Feature Flag Endpoints (Story 10B.2) ====================

/// Feature flag response types.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListFeatureFlagsResponse {
    pub flags: Vec<FeatureFlagWithCount>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureFlagResponse {
    pub flag: FeatureFlag,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureFlagDetailResponse {
    pub flag: FeatureFlagWithOverrides,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResolvedFlagsResponse {
    pub flags: Vec<ResolvedFeatureFlag>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFeatureFlagRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_enabled: Option<bool>,
}

/// List all feature flags.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/feature-flags",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Feature flags retrieved", body = ListFeatureFlagsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_feature_flags(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListFeatureFlagsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let flags = state.feature_flag_repo.list_all().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list feature flags");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to list feature flags",
            )),
        )
    })?;

    Ok(Json(ListFeatureFlagsResponse { flags }))
}

/// Create a new feature flag.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/feature-flags",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    request_body = CreateFeatureFlagRequest,
    responses(
        (status = 201, description = "Feature flag created", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 409, description = "Flag key already exists", body = ErrorResponse)
    )
)]
pub async fn create_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateFeatureFlagRequest>,
) -> Result<(StatusCode, Json<FeatureFlagResponse>), (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag = state
        .feature_flag_repo
        .create(
            &req.key,
            &req.name,
            req.description.as_deref(),
            req.is_enabled,
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("unique") || e.to_string().contains("duplicate") {
                (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(
                        "DUPLICATE_KEY",
                        "Feature flag key already exists",
                    )),
                )
            } else {
                tracing::error!(error = %e, "Failed to create feature flag");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to create feature flag",
                    )),
                )
            }
        })?;

    tracing::info!(
        flag_key = %req.key,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag created"
    );

    Ok((StatusCode::CREATED, Json(FeatureFlagResponse { flag })))
}

/// Get a feature flag with all overrides.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/feature-flags/{id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag retrieved", body = FeatureFlagDetailResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn get_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<FeatureFlagDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let flag = state
        .feature_flag_repo
        .get_by_id(flag_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get feature flag",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FLAG_NOT_FOUND",
                    "Feature flag not found",
                )),
            )
        })?;

    Ok(Json(FeatureFlagDetailResponse { flag }))
}

/// Update a feature flag.
#[utoipa::path(
    put,
    path = "/api/v1/platform-admin/feature-flags/{id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    request_body = UpdateFeatureFlagRequest,
    responses(
        (status = 200, description = "Feature flag updated", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn update_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateFeatureFlagRequest>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let flag = state
        .feature_flag_repo
        .update(
            flag_id,
            req.name.as_deref(),
            req.description.as_deref(),
            req.is_enabled,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update feature flag",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FLAG_NOT_FOUND",
                    "Feature flag not found",
                )),
            )
        })?;

    tracing::info!(
        flag_id = %flag_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag updated"
    );

    Ok(Json(FeatureFlagResponse { flag }))
}

/// Delete a feature flag.
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/feature-flags/{id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 204, description = "Feature flag deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn delete_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let deleted = state.feature_flag_repo.delete(flag_id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete feature flag");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to delete feature flag",
            )),
        )
    })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "FLAG_NOT_FOUND",
                "Feature flag not found",
            )),
        ));
    }

    tracing::info!(
        flag_id = %flag_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag deleted"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Toggle a feature flag's global enabled state.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/feature-flags/{id}/toggle",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    responses(
        (status = 200, description = "Feature flag toggled", body = FeatureFlagResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn toggle_feature_flag(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let flag = state
        .feature_flag_repo
        .toggle(flag_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to toggle feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to toggle feature flag",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FLAG_NOT_FOUND",
                    "Feature flag not found",
                )),
            )
        })?;

    tracing::info!(
        flag_id = %flag_id,
        new_state = flag.is_enabled,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag toggled"
    );

    Ok(Json(FeatureFlagResponse { flag }))
}

/// Create a feature flag override.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/feature-flags/{id}/overrides",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID")
    ),
    request_body = CreateFeatureFlagOverrideRequest,
    responses(
        (status = 201, description = "Override created", body = db::models::FeatureFlagOverride),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Feature flag not found", body = ErrorResponse)
    )
)]
pub async fn create_feature_flag_override(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<CreateFeatureFlagOverrideRequest>,
) -> Result<(StatusCode, Json<db::models::FeatureFlagOverride>), (StatusCode, Json<ErrorResponse>)>
{
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    // Verify flag exists
    if state
        .feature_flag_repo
        .get_by_id(flag_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check feature flag");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check feature flag",
                )),
            )
        })?
        .is_none()
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "FLAG_NOT_FOUND",
                "Feature flag not found",
            )),
        ));
    }

    let override_record = state
        .feature_flag_repo
        .create_override(
            flag_id,
            req.scope_type.clone(),
            req.scope_id,
            req.is_enabled,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create override");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create override",
                )),
            )
        })?;

    tracing::info!(
        flag_id = %flag_id,
        scope_type = %req.scope_type,
        scope_id = %req.scope_id,
        is_enabled = req.is_enabled,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag override created"
    );

    Ok((StatusCode::CREATED, Json(override_record)))
}

/// Delete a feature flag override.
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/feature-flags/{id}/overrides/{override_id}",
    tag = "Platform Admin - Feature Flags",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "Feature flag ID"),
        ("override_id" = String, Path, description = "Override ID")
    ),
    responses(
        (status = 204, description = "Override deleted"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Override not found", body = ErrorResponse)
    )
)]
pub async fn delete_feature_flag_override(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((id, override_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let _flag_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid feature flag ID format",
            )),
        )
    })?;

    let override_uuid: Uuid = override_id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ID",
                "Invalid override ID format",
            )),
        )
    })?;

    let deleted = state
        .feature_flag_repo
        .delete_override(override_uuid)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete override");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete override",
                )),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "OVERRIDE_NOT_FOUND",
                "Override not found",
            )),
        ));
    }

    tracing::info!(
        override_id = %override_uuid,
        admin_id = %admin_id,
        admin_email = %admin_email,
        "Feature flag override deleted"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Get resolved feature flags for current user context (public endpoint).
#[utoipa::path(
    get,
    path = "/api/v1/feature-flags",
    tag = "Feature Flags",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Resolved flags retrieved", body = ResolvedFlagsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_resolved_feature_flags(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ResolvedFlagsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract user context from token (any authenticated user)
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
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

    // Get user's org context from token if available
    let org_id = claims.org_id.and_then(|t| t.parse().ok());

    let flags = state
        .feature_flag_repo
        .resolve_all_for_context(Some(user_id), org_id, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to resolve feature flags");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to resolve feature flags",
                )),
            )
        })?;

    Ok(Json(ResolvedFlagsResponse { flags }))
}

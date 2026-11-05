//! User Onboarding routes (Epic 10B, Story 10B.6).
//!
//! Routes for user onboarding tour progress tracking and management.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::repositories::{OnboardingTour, TourWithProgress, UserOnboardingProgress};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create onboarding router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tours", get(get_user_tours))
        .route("/tours/{tour_id}", get(get_tour))
        .route("/tours/{tour_id}/start", post(start_tour))
        .route(
            "/tours/{tour_id}/steps/{step_id}/complete",
            post(complete_step),
        )
        .route("/tours/{tour_id}/complete", post(complete_tour))
        .route("/tours/{tour_id}/skip", post(skip_tour))
        .route("/tours/{tour_id}/reset", post(reset_tour))
        .route("/status", get(get_onboarding_status))
}

// ==================== Types ====================

/// Response for tour progress.
#[derive(Debug, Serialize, ToSchema)]
pub struct TourProgressResponse {
    pub progress: UserOnboardingProgress,
}

/// Response for onboarding status.
#[derive(Debug, Serialize, ToSchema)]
pub struct OnboardingStatusResponse {
    /// Whether user has any incomplete tours
    pub needs_onboarding: bool,
    /// List of tours with their progress
    pub tours: Vec<TourWithProgress>,
}

/// Request to complete a step.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteStepRequest {
    /// Optional data captured during the step
    pub data: Option<serde_json::Value>,
}

// ==================== Helper Functions ====================

/// Extract user from token.
///
/// Roles used for role-targeted tours come from the caller's active
/// membership in the **current** organization when one is given (the
/// `X-Tenant-ID` header or the token's `org_id` claim; they must agree) — a
/// manager elsewhere is not a manager here. Without an org context (login
/// tokens carry no org claim and the onboarding client sends no header) the
/// user's active memberships across orgs are used. The token's `roles` claim
/// is never consulted. Each matching
/// membership contributes its `role_type`, its role name and the display
/// name of the canonical role (e.g. "Manager"), which is how
/// `onboarding_tours.target_roles` is written.
async fn extract_user_token(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<(Uuid, Vec<String>), (StatusCode, Json<ErrorResponse>)> {
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

    let header_org: Option<Uuid> = headers
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    let claim_org: Option<Uuid> = claims.org_id.as_ref().and_then(|id| id.parse().ok());
    if let (Some(h), Some(c)) = (header_org, claim_org) {
        if h != c {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("TENANT_MISMATCH", "Tenant mismatch")),
            ));
        }
    }
    // With an org context, only that org's membership counts. Without one
    // (login tokens carry no org claim and the onboarding client sends no
    // X-Tenant-ID), fall back to all of the user's active memberships.
    let org_scope = header_org.or(claim_org);

    let mut roles = Vec::new();
    let memberships = state
        .org_member_repo
        .get_user_memberships(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to load memberships for onboarding roles");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;
    for m in memberships
        .into_iter()
        .filter(|m| m.status == "active" && org_scope.is_none_or(|org| m.organization_id == org))
    {
        if let Some(role) = common::TenantRole::from_role_type(&m.role_type) {
            roles.push(role.to_string());
        }
        if let Some(name) = m.role_name {
            roles.push(name);
        }
        roles.push(m.role_type);
    }
    roles.sort();
    roles.dedup();

    Ok((user_id, roles))
}

// ==================== Endpoints ====================

/// Get onboarding status.
#[utoipa::path(
    get,
    path = "/api/v1/onboarding/status",
    responses(
        (status = 200, description = "Onboarding status retrieved", body = OnboardingStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn get_onboarding_status(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<OnboardingStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, roles) = extract_user_token(&headers, &state).await?;

    let needs_onboarding = state
        .onboarding_repo
        .needs_onboarding(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check onboarding status");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check onboarding status",
                )),
            )
        })?;

    let tours = state
        .onboarding_repo
        .get_tours_for_user(user_id, &roles)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get tours");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get tours")),
            )
        })?;

    Ok(Json(OnboardingStatusResponse {
        needs_onboarding,
        tours,
    }))
}

/// Get all available tours for user.
#[utoipa::path(
    get,
    path = "/api/v1/onboarding/tours",
    responses(
        (status = 200, description = "Tours retrieved", body = Vec<TourWithProgress>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn get_user_tours(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<TourWithProgress>>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, roles) = extract_user_token(&headers, &state).await?;

    let tours = state
        .onboarding_repo
        .get_tours_for_user(user_id, &roles)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get tours");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get tours")),
            )
        })?;

    Ok(Json(tours))
}

/// Get a specific tour.
#[utoipa::path(
    get,
    path = "/api/v1/onboarding/tours/{tour_id}",
    params(
        ("tour_id" = String, Path, description = "Tour ID")
    ),
    responses(
        (status = 200, description = "Tour retrieved", body = OnboardingTour),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tour not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn get_tour(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(tour_id): Path<String>,
) -> Result<Json<OnboardingTour>, (StatusCode, Json<ErrorResponse>)> {
    let _ = extract_user_token(&headers, &state).await?;

    let tour = state
        .onboarding_repo
        .get_tour(&tour_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tour_id = %tour_id, "Failed to get tour");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get tour")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("TOUR_NOT_FOUND", "Tour not found")),
            )
        })?;

    Ok(Json(tour))
}

/// Start or resume a tour.
#[utoipa::path(
    post,
    path = "/api/v1/onboarding/tours/{tour_id}/start",
    params(
        ("tour_id" = String, Path, description = "Tour ID")
    ),
    responses(
        (status = 200, description = "Tour started", body = TourProgressResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn start_tour(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(tour_id): Path<String>,
) -> Result<Json<TourProgressResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, _) = extract_user_token(&headers, &state).await?;

    let progress = state
        .onboarding_repo
        .start_tour(user_id, &tour_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tour_id = %tour_id, "Failed to start tour");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to start tour")),
            )
        })?;

    Ok(Json(TourProgressResponse { progress }))
}

/// Complete a step in a tour.
#[utoipa::path(
    post,
    path = "/api/v1/onboarding/tours/{tour_id}/steps/{step_id}/complete",
    params(
        ("tour_id" = String, Path, description = "Tour ID"),
        ("step_id" = String, Path, description = "Step ID")
    ),
    responses(
        (status = 200, description = "Step completed", body = TourProgressResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tour not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn complete_step(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((tour_id, step_id)): Path<(String, String)>,
) -> Result<Json<TourProgressResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, _) = extract_user_token(&headers, &state).await?;

    let progress = state
        .onboarding_repo
        .complete_step(user_id, &tour_id, &step_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tour_id = %tour_id, step_id = %step_id, "Failed to complete step");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to complete step")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("TOUR_NOT_FOUND", "Tour not found or not started")),
            )
        })?;

    Ok(Json(TourProgressResponse { progress }))
}

/// Complete a tour.
#[utoipa::path(
    post,
    path = "/api/v1/onboarding/tours/{tour_id}/complete",
    params(
        ("tour_id" = String, Path, description = "Tour ID")
    ),
    responses(
        (status = 200, description = "Tour completed", body = TourProgressResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tour not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn complete_tour(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(tour_id): Path<String>,
) -> Result<Json<TourProgressResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, _) = extract_user_token(&headers, &state).await?;

    let progress = state
        .onboarding_repo
        .complete_tour(user_id, &tour_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tour_id = %tour_id, "Failed to complete tour");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to complete tour",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "TOUR_NOT_FOUND",
                    "Tour not found or not started",
                )),
            )
        })?;

    tracing::info!(user_id = %user_id, tour_id = %tour_id, "User completed onboarding tour");

    Ok(Json(TourProgressResponse { progress }))
}

/// Skip a tour.
#[utoipa::path(
    post,
    path = "/api/v1/onboarding/tours/{tour_id}/skip",
    params(
        ("tour_id" = String, Path, description = "Tour ID")
    ),
    responses(
        (status = 200, description = "Tour skipped", body = TourProgressResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tour not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn skip_tour(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(tour_id): Path<String>,
) -> Result<Json<TourProgressResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, _) = extract_user_token(&headers, &state).await?;

    // Ensure progress record exists
    let _ = state.onboarding_repo.start_tour(user_id, &tour_id).await;

    let progress = state
        .onboarding_repo
        .skip_tour(user_id, &tour_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tour_id = %tour_id, "Failed to skip tour");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to skip tour")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("TOUR_NOT_FOUND", "Tour not found")),
            )
        })?;

    tracing::info!(user_id = %user_id, tour_id = %tour_id, "User skipped onboarding tour");

    Ok(Json(TourProgressResponse { progress }))
}

/// Reset a tour (restart from beginning).
#[utoipa::path(
    post,
    path = "/api/v1/onboarding/tours/{tour_id}/reset",
    params(
        ("tour_id" = String, Path, description = "Tour ID")
    ),
    responses(
        (status = 200, description = "Tour reset", body = TourProgressResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Tour not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Onboarding"
)]
pub async fn reset_tour(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(tour_id): Path<String>,
) -> Result<Json<TourProgressResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user_id, _) = extract_user_token(&headers, &state).await?;

    let progress = state
        .onboarding_repo
        .reset_tour(user_id, &tour_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tour_id = %tour_id, "Failed to reset tour");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to reset tour")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "TOUR_NOT_FOUND",
                    "Tour not found or not started",
                )),
            )
        })?;

    Ok(Json(TourProgressResponse { progress }))
}

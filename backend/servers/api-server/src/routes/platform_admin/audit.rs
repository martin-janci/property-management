//! Platform-admin audit / support-tooling endpoints.
//!
//! Story 10B.5 (support user diagnostics + aggregated support data) and
//! Story 10B.6 (onboarding tour configuration). Behaviour is identical to the
//! original `platform_admin.rs`; this is a pure structural move.

use admin_core::RequireCapability;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::repositories::{
    OnboardingTour, SupportActivityLog, SupportData, SupportDataViewedProps,
    SupportSessionsRevokedProps, SupportToolingEventKind, SupportUserInfo, SupportUserMembership,
    SupportUserSearchedProps, SupportUserSession,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::extract_super_admin_token;
use super::tenants::{default_page, default_page_size};
use crate::state::AppState;

// ==================== Support Data Access Handlers (Story 10B.5) ====================

/// Query parameters for user search.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct SearchUsersQuery {
    /// Search query (email, name)
    pub query: Option<String>,
    /// Filter by status
    pub status: Option<String>,
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Page size (max 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// Response for user search.
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchUsersResponse {
    pub users: Vec<SupportUserInfo>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// User detail response for support.
#[derive(Debug, Serialize, ToSchema)]
pub struct SupportUserDetailResponse {
    pub user: SupportUserInfo,
    pub memberships: Vec<SupportUserMembership>,
    pub active_sessions: Vec<SupportUserSession>,
}

/// Response for session revocation.
#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeSessionsResponse {
    pub message: String,
    pub revoked_count: i64,
}

/// Query parameters for activity log.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct ActivityQuery {
    /// Maximum number of entries to return
    #[serde(default = "default_activity_limit")]
    pub limit: i64,
}

fn default_activity_limit() -> i64 {
    50
}

/// Search users for support purposes.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users",
    params(SearchUsersQuery),
    responses(
        (status = 200, description = "Users found", body = SearchUsersResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn search_users_for_support(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<SearchUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let page_size = query.page_size.clamp(1, 100);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as i64;

    let (users, total) = state
        .platform_admin_repo
        .search_users_for_support(
            query.query.as_deref(),
            query.status.as_deref(),
            page_size as i64,
            offset,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to search users");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to search users",
                )),
            )
        })?;

    // Emit support_user_searched analytics event (#635).
    // Fire-and-forget: a tracking failure must not fail the API response.
    let props = serde_json::to_value(SupportUserSearchedProps {
        query_length: query.query.as_deref().map(|q| q.len() as i64),
        status_filter: query.status.clone(),
        result_count: total,
    })
    .unwrap_or_default();
    if let Err(e) = state
        .platform_admin_repo
        .log_support_tooling_event(
            admin_user_id,
            SupportToolingEventKind::SupportUserSearched,
            props,
        )
        .await
    {
        tracing::warn!(
            error = %e,
            admin_user_id = %admin_user_id,
            "support_user_searched event: failed to persist (non-fatal)"
        );
    }

    Ok(Json(SearchUsersResponse {
        users,
        total,
        page,
        page_size,
    }))
}

/// Get user details for support.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User details retrieved", body = SupportUserDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_for_support(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<SupportUserDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let user = state
        .platform_admin_repo
        .get_user_for_support(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get user")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("USER_NOT_FOUND", "User not found")),
            )
        })?;

    let memberships = state
        .platform_admin_repo
        .get_user_memberships(id)
        .await
        .unwrap_or_default();

    let active_sessions = state
        .platform_admin_repo
        .get_user_sessions(id)
        .await
        .unwrap_or_default();

    Ok(Json(SupportUserDetailResponse {
        user,
        memberships,
        active_sessions,
    }))
}

/// Get user organization memberships.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}/memberships",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Memberships retrieved", body = Vec<SupportUserMembership>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_memberships(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SupportUserMembership>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let memberships = state
        .platform_admin_repo
        .get_user_memberships(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get memberships");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get memberships",
                )),
            )
        })?;

    Ok(Json(memberships))
}

/// Get user active sessions.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}/sessions",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Sessions retrieved", body = Vec<SupportUserSession>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_sessions(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SupportUserSession>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let sessions = state
        .platform_admin_repo
        .get_user_sessions(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get sessions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get sessions",
                )),
            )
        })?;

    Ok(Json(sessions))
}

/// Revoke all user sessions.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/support/users/{id}/sessions/revoke",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Sessions revoked", body = RevokeSessionsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn revoke_user_sessions(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<RevokeSessionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_super_admin_token(&headers, &state)?;

    let revoked_count = state
        .platform_admin_repo
        .revoke_user_sessions(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to revoke sessions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to revoke sessions",
                )),
            )
        })?;

    tracing::info!(
        user_id = %id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        revoked_count = revoked_count,
        "User sessions revoked by support"
    );

    // Emit support_sessions_revoked analytics event (#635).
    // Fire-and-forget: a tracking failure must not fail the API response.
    let props = serde_json::to_value(SupportSessionsRevokedProps {
        target_user_id: id,
        revoked_count,
    })
    .unwrap_or_default();
    if let Err(e) = state
        .platform_admin_repo
        .log_support_tooling_event(
            admin_id,
            SupportToolingEventKind::SupportSessionsRevoked,
            props,
        )
        .await
    {
        tracing::warn!(
            error = %e,
            admin_user_id = %admin_id,
            target_user_id = %id,
            "support_sessions_revoked event: failed to persist (non-fatal)"
        );
    }

    Ok(Json(RevokeSessionsResponse {
        message: format!("{} session(s) revoked", revoked_count),
        revoked_count,
    }))
}

/// Get user activity log.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support/users/{id}/activity",
    params(
        ("id" = Uuid, Path, description = "User ID"),
        ActivityQuery
    ),
    responses(
        (status = 200, description = "Activity log retrieved", body = Vec<SupportActivityLog>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_user_activity(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<ActivityQuery>,
) -> Result<Json<Vec<SupportActivityLog>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let limit = query.limit.clamp(1, 500);

    let activity = state
        .platform_admin_repo
        .get_user_activity_log(id, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %id, "Failed to get activity log");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get activity log",
                )),
            )
        })?;

    Ok(Json(activity))
}

// ==================== Support Data Aggregation Handler (Story 10B.5) ====================

/// Response body for `GET /api/v1/platform-admin/support-data`.
#[derive(Debug, Serialize, ToSchema)]
pub struct SupportDataResponse {
    /// Platform-wide tenant diagnostics.
    pub data: SupportData,
}

/// Get platform tenant diagnostics (support data).
///
/// Returns aggregated user counts, active session count, and fault status
/// breakdown across the whole platform.  Intended for the admin-web Support
/// Data page and platform support engineers.
///
/// Requires `AuditRead` capability (SuperAdmin role is additionally enforced
/// by `extract_super_admin_token` inside the handler body).
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/support-data",
    responses(
        (status = 200, description = "Support data retrieved", body = SupportDataResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Support"
)]
pub async fn get_support_data(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<SupportDataResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _admin_email) = extract_super_admin_token(&headers, &state)?;

    let data = state
        .platform_admin_repo
        .get_support_data()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get support data");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve support data",
                )),
            )
        })?;

    // Emit support_data_viewed analytics event (#635).
    // Fire-and-forget: a tracking failure must not fail the API response.
    let props = serde_json::to_value(SupportDataViewedProps {
        tenant_count: data.total_orgs,
        fault_total: data.total_faults,
    })
    .unwrap_or_default();
    if let Err(e) = state
        .platform_admin_repo
        .log_support_tooling_event(
            admin_user_id,
            SupportToolingEventKind::SupportDataViewed,
            props,
        )
        .await
    {
        tracing::warn!(
            error = %e,
            admin_user_id = %admin_user_id,
            "support_data_viewed event: failed to persist (non-fatal)"
        );
    }

    Ok(Json(SupportDataResponse { data }))
}

// ==================== Onboarding Config Handler (Story 10B.6) ====================

/// Response for the onboarding config endpoint.
///
/// Returns all onboarding tour step definitions so that the platform admin
/// can review what tours and steps are configured across the platform.
#[derive(Debug, Serialize, ToSchema)]
pub struct OnboardingConfigResponse {
    /// All tour definitions (active and inactive).
    pub tours: Vec<OnboardingTour>,
    /// Total number of tours.
    pub total: usize,
}

/// Get onboarding tour configuration (platform admin view).
///
/// Returns all onboarding tour definitions including step definitions,
/// target roles, and active status. Useful for platform admins to audit
/// the onboarding experience and ensure tours are correctly configured.
///
/// User progress is tracked in the `user_onboarding_progress` table
/// which is persisted per-user per-tour via the onboarding repository.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/onboarding-config",
    responses(
        (status = 200, description = "Onboarding configuration retrieved", body = OnboardingConfigResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Onboarding"
)]
pub async fn get_onboarding_config(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<OnboardingConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let tours = state.onboarding_repo.list_all_tours().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to list onboarding tours");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to retrieve onboarding configuration",
            )),
        )
    })?;

    let total = tours.len();

    tracing::info!(
        total_tours = total,
        "Platform admin retrieved onboarding config"
    );

    Ok(Json(OnboardingConfigResponse { tours, total }))
}

//! Admin routes for user lifecycle management (Epic 1, Story 1.6).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create admin user-lifecycle router (Epic 1, Story 1.6).
///
/// Renamed in Phase 5 from `router()` → `lifecycle_router()` so the new
/// admin module hierarchy (`routes::admin::router`) can mount this alongside
/// Phase 5's capability-gated routers without name collision. The Phase 2
/// `memberships` sub-router is mounted by the parent `admin/mod.rs` router
/// directly (not nested inside this lifecycle router), so it is not merged
/// here.
pub fn lifecycle_router() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}", get(get_user))
        .route("/users/{id}/suspend", post(suspend_user))
        .route("/users/{id}/reactivate", post(reactivate_user))
        .route("/users/{id}/delete", post(delete_user))
}

// ==================== Types ====================

/// User info for admin view.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminUserInfo {
    /// User ID
    pub id: String,
    /// Email address
    pub email: String,
    /// Display name
    pub name: String,
    /// Phone number
    pub phone: Option<String>,
    /// Account status
    pub status: String,
    /// Preferred locale
    pub locale: String,
    /// When email was verified
    pub email_verified_at: Option<String>,
    /// When account was suspended
    pub suspended_at: Option<String>,
    /// Who suspended the account
    pub suspended_by: Option<String>,
    /// When account was created
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// List users query parameters.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListUsersQuery {
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Page size (max 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// Filter by status (pending, active, suspended, deleted)
    pub status: Option<String>,
    /// Search by email or name
    pub search: Option<String>,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

/// List users response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListUsersResponse {
    /// Users on this page
    pub users: Vec<AdminUserInfo>,
    /// Total number of users matching criteria
    pub total: i64,
    /// Current page number
    pub page: u32,
    /// Page size
    pub page_size: u32,
}

/// Suspend/reactivate request (optional reason).
#[derive(Debug, Deserialize, ToSchema)]
pub struct UserActionRequest {
    /// Reason for the action (optional)
    pub reason: Option<String>,
}

/// Admin action response.
#[derive(Debug, Serialize, ToSchema)]
pub struct AdminActionResponse {
    /// Success message
    pub message: String,
    /// Updated user info
    pub user: AdminUserInfo,
}

// ==================== Helper Functions ====================

/// Admin role names that grant access to admin endpoints.
/// Based on TenantRole enum from common crate.
const ADMIN_ROLES: &[&str] = &[
    "SuperAdministrator",
    "OrganizationAdmin",
    "super_admin",
    "org_admin",
    "admin",
];

/// Check if the user has any admin role.
fn has_admin_role(roles: &Option<Vec<String>>) -> bool {
    match roles {
        Some(user_roles) => user_roles.iter().any(|r| {
            ADMIN_ROLES
                .iter()
                .any(|admin| r.eq_ignore_ascii_case(admin))
        }),
        None => false,
    }
}

/// Extract and validate admin access token.
fn extract_admin_token(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<(Uuid, String), (StatusCode, Json<ErrorResponse>)> {
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

    // Check if user has admin role
    if !has_admin_role(&claims.roles) {
        tracing::warn!(
            user_id = %user_id,
            email = %claims.email,
            roles = ?claims.roles,
            "Unauthorized admin access attempt"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_PERMISSIONS",
                "Admin role required to access this endpoint",
            )),
        ));
    }

    Ok((user_id, claims.email))
}

/// Convert User model to AdminUserInfo.
fn user_to_admin_info(user: db::models::User) -> AdminUserInfo {
    AdminUserInfo {
        id: user.id.to_string(),
        email: user.email,
        name: user.name,
        phone: user.phone,
        status: user.status.clone(),
        locale: user.locale.clone(),
        email_verified_at: user.email_verified_at.map(|t| t.to_rfc3339()),
        suspended_at: user.suspended_at.map(|t| t.to_rfc3339()),
        suspended_by: user.suspended_by.map(|id| id.to_string()),
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    }
}

// ==================== Endpoints ====================

/// List users with pagination and filters.
#[utoipa::path(
    get,
    path = "/api/v1/admin/users",
    tag = "Admin",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<u32>, Query, description = "Page number (1-based)"),
        ("page_size" = Option<u32>, Query, description = "Page size (max 100)"),
        ("status" = Option<String>, Query, description = "Filter by status"),
        ("search" = Option<String>, Query, description = "Search by email or name")
    ),
    responses(
        (status = 200, description = "Users retrieved", body = ListUsersResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_admin_token(&headers, &state)?;

    // Validate page size
    let page_size = query.page_size.clamp(1, 100);
    let page = query.page.max(1);
    let offset = ((page - 1) * page_size) as i64;

    // Get users with pagination
    let (users, total) = match state
        .user_repo
        .list_users(
            offset,
            page_size as i64,
            query.status.as_deref(),
            query.search.as_deref(),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = %e, "Failed to list users");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to list users")),
            ));
        }
    };

    let user_infos: Vec<AdminUserInfo> = users.into_iter().map(user_to_admin_info).collect();

    Ok(Json(ListUsersResponse {
        users: user_infos,
        total,
        page,
        page_size,
    }))
}

/// Get user by ID.
#[utoipa::path(
    get,
    path = "/api/v1/admin/users/{id}",
    tag = "Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User retrieved", body = AdminUserInfo),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn get_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AdminUserInfo>, (StatusCode, Json<ErrorResponse>)> {
    let (_admin_id, _admin_email) = extract_admin_token(&headers, &state)?;

    let user_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_ID", "Invalid user ID format")),
        )
    })?;

    let user = match state.user_repo.find_by_id(user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("USER_NOT_FOUND", "User not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get user")),
            ));
        }
    };

    Ok(Json(user_to_admin_info(user)))
}

/// Suspend a user account.
#[utoipa::path(
    post,
    path = "/api/v1/admin/users/{id}/suspend",
    tag = "Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "User ID to suspend")
    ),
    request_body = UserActionRequest,
    responses(
        (status = 200, description = "User suspended", body = AdminActionResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 409, description = "User already suspended", body = ErrorResponse)
    )
)]
pub async fn suspend_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UserActionRequest>,
) -> Result<Json<AdminActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_admin_token(&headers, &state)?;

    let user_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_ID", "Invalid user ID format")),
        )
    })?;

    // Prevent self-suspension
    if user_id == admin_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "CANNOT_SELF_SUSPEND",
                "Cannot suspend your own account",
            )),
        ));
    }

    let user = match state.user_repo.suspend(user_id, admin_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User not found or already suspended",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to suspend user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to suspend user",
                )),
            ));
        }
    };

    // Revoke all user sessions
    if let Err(e) = state
        .session_repo
        .revoke_all_user_tokens(user_id, None)
        .await
    {
        tracing::error!(error = %e, "Failed to revoke user sessions");
    }

    tracing::info!(
        user_id = %user_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        reason = ?req.reason,
        "User suspended"
    );

    Ok(Json(AdminActionResponse {
        message: "User suspended successfully".to_string(),
        user: user_to_admin_info(user),
    }))
}

/// Reactivate a suspended user account.
#[utoipa::path(
    post,
    path = "/api/v1/admin/users/{id}/reactivate",
    tag = "Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "User ID to reactivate")
    ),
    request_body = UserActionRequest,
    responses(
        (status = 200, description = "User reactivated", body = AdminActionResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 409, description = "User not suspended", body = ErrorResponse)
    )
)]
pub async fn reactivate_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UserActionRequest>,
) -> Result<Json<AdminActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_admin_token(&headers, &state)?;

    let user_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_ID", "Invalid user ID format")),
        )
    })?;

    let user = match state.user_repo.reactivate(user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User not found or not suspended",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to reactivate user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to reactivate user",
                )),
            ));
        }
    };

    tracing::info!(
        user_id = %user_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        reason = ?req.reason,
        "User reactivated"
    );

    Ok(Json(AdminActionResponse {
        message: "User reactivated successfully".to_string(),
        user: user_to_admin_info(user),
    }))
}

/// Soft delete a user account.
#[utoipa::path(
    post,
    path = "/api/v1/admin/users/{id}/delete",
    tag = "Admin",
    security(("bearer_auth" = [])),
    params(
        ("id" = String, Path, description = "User ID to delete")
    ),
    request_body = UserActionRequest,
    responses(
        (status = 200, description = "User deleted", body = AdminActionResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UserActionRequest>,
) -> Result<Json<AdminActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_id, admin_email) = extract_admin_token(&headers, &state)?;

    let user_id: Uuid = id.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_ID", "Invalid user ID format")),
        )
    })?;

    // Prevent self-deletion
    if user_id == admin_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "CANNOT_SELF_DELETE",
                "Cannot delete your own account",
            )),
        ));
    }

    let user = match state.user_repo.soft_delete(user_id, admin_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "USER_NOT_FOUND",
                    "User not found or already deleted",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to delete user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete user",
                )),
            ));
        }
    };

    // Revoke all user sessions
    if let Err(e) = state
        .session_repo
        .revoke_all_user_tokens(user_id, None)
        .await
    {
        tracing::error!(error = %e, "Failed to revoke user sessions");
    }

    tracing::info!(
        user_id = %user_id,
        admin_id = %admin_id,
        admin_email = %admin_email,
        reason = ?req.reason,
        "User deleted"
    );

    Ok(Json(AdminActionResponse {
        message: "User deleted successfully".to_string(),
        user: user_to_admin_info(user),
    }))
}

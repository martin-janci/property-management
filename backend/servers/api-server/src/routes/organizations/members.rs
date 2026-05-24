//! Organization member routes (UC-27, Epic 2A).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;
use super::core::{
    extract_bearer_token, validate_access_token,
};

/// Create organizations members router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/members", get(list_organization_members))
        .route("/{id}/members", post(add_organization_member))
        .route("/{id}/members/{user_id}", put(update_organization_member))
        .route("/{id}/members/{user_id}", delete(remove_organization_member))
}

// ==================== Organization Members ====================

/// Member info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct MemberResponse {
    /// User ID
    pub user_id: Uuid,
    /// User name
    pub name: String,
    /// User email
    pub email: String,
    /// Role ID
    pub role_id: Option<Uuid>,
    /// Role name
    pub role_name: String,
    /// Role type
    pub role_type: String,
    /// Joined at
    pub joined_at: Option<String>,
}

/// List members response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListMembersResponse {
    /// Members
    pub members: Vec<MemberResponse>,
    /// Total count
    pub total: i64,
}

/// List organization members.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/members",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Members retrieved", body = ListMembersResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_organization_members(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ListMembersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if user is member of this organization
    match state
        .org_member_repo
        .find_by_org_and_user(id, user_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    }

    // Get all members with user details
    let (members_with_users, total) = match state
        .org_member_repo
        .list_org_members(id, 0, 100, None)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch members");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch members",
                )),
            ));
        }
    };

    // role_name is now populated by list_org_members via LEFT JOIN roles —
    // no per-member lookup required (previously N+1 SELECTs from roles).
    let member_responses: Vec<MemberResponse> = members_with_users
        .into_iter()
        .map(|m| {
            let role_name = match (m.role_id, m.role_name.as_deref()) {
                (Some(_), Some(name)) => name.to_string(),
                (Some(_), None) => "Unknown".to_string(),
                (None, _) => "No Role".to_string(),
            };

            MemberResponse {
                user_id: m.user_id,
                name: m.user_name,
                email: m.user_email,
                role_id: m.role_id,
                role_name,
                role_type: m.role_type,
                joined_at: m.joined_at.map(|dt| dt.to_rfc3339()),
            }
        })
        .collect();

    Ok(Json(ListMembersResponse {
        members: member_responses,
        total,
    }))
}

// ==================== Add Organization Member ====================

/// Add member request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddMemberRequest {
    /// User ID to add
    pub user_id: Uuid,
    /// Role ID to assign
    pub role_id: Uuid,
    /// Role type (e.g., "member", "admin", "owner")
    pub role_type: Option<String>,
}

/// Add member response.
#[derive(Debug, Serialize, ToSchema)]
pub struct AddMemberResponse {
    /// Success message
    pub message: String,
}

/// Add a member to an organization.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/members",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = AddMemberRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Member added", body = AddMemberResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization or user not found", body = ErrorResponse),
        (status = 409, description = "User is already a member", body = ErrorResponse)
    )
)]
pub async fn add_organization_member(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let admin_user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if requesting user has permission to add members
    let membership = match state
        .org_member_repo
        .find_by_org_and_user(id, admin_user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    let role = match membership.role_id {
        Some(role_id) => match state.role_repo.find_by_id(role_id).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    if !role.has_permission("users:create") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to add members",
            )),
        ));
    }

    // Verify user exists
    match state.user_repo.find_by_id(req.user_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("USER_NOT_FOUND", "User not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to find user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to find user")),
            ));
        }
    }

    // Verify role exists and belongs to this organization
    match state.role_repo.find_by_id(req.role_id).await {
        Ok(Some(r)) if r.organization_id == id => {}
        Ok(Some(_)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_ROLE",
                    "Role does not belong to this organization",
                )),
            ));
        }
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("ROLE_NOT_FOUND", "Role not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to find role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to find role")),
            ));
        }
    }

    // Check if user is already a member
    match state
        .org_member_repo
        .find_by_org_and_user(id, req.user_id)
        .await
    {
        Ok(Some(_)) => {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "ALREADY_MEMBER",
                    "User is already a member of this organization",
                )),
            ));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    }

    // Add member
    use db::models::CreateOrganizationMember;
    let create_member = CreateOrganizationMember {
        organization_id: id,
        user_id: req.user_id,
        role_id: Some(req.role_id),
        role_type: req.role_type.unwrap_or_else(|| "member".to_string()),
        invited_by: Some(admin_user_id),
    };

    if let Err(e) = state.org_member_repo.create(create_member).await {
        tracing::error!(error = %e, "Failed to add member");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to add member")),
        ));
    }

    tracing::info!(
        org_id = %id,
        user_id = %req.user_id,
        role_id = %req.role_id,
        added_by = %admin_user_id,
        "Member added to organization"
    );

    Ok((
        StatusCode::CREATED,
        Json(AddMemberResponse {
            message: "Member added successfully".to_string(),
        }),
    ))
}

// ==================== Update Organization Member ====================

/// Update member request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMemberRequest {
    /// New role ID to assign
    pub role_id: Uuid,
    /// Role type (optional, will be derived from role if not provided)
    pub role_type: Option<String>,
}

/// Update member response.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateMemberResponse {
    /// Success message
    pub message: String,
}

/// Update a member's role in an organization.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/members/{user_id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("user_id" = Uuid, Path, description = "User ID to update")
    ),
    request_body = UpdateMemberRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Member updated", body = UpdateMemberResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Member not found", body = ErrorResponse),
        (status = 400, description = "Invalid role", body = ErrorResponse)
    )
)]
pub async fn update_organization_member(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((org_id, target_user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateMemberRequest>,
) -> Result<Json<UpdateMemberResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let admin_user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if requesting user has permission
    let admin_membership = match state
        .org_member_repo
        .find_by_org_and_user(org_id, admin_user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    let role = match admin_membership.role_id {
        Some(role_id) => match state.role_repo.find_by_id(role_id).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    // Check permission to update members
    if !role.has_permission("users:update") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to update members",
            )),
        ));
    }

    // Verify the new role exists and belongs to the same organization
    let new_role = match state.role_repo.find_by_id(req.role_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_ROLE", "Role not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to verify role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify role",
                )),
            ));
        }
    };

    // Check role belongs to the same org
    if new_role.organization_id != org_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ROLE",
                "Role does not belong to this organization",
            )),
        ));
    }

    // Find the membership to update
    let target_membership = match state
        .org_member_repo
        .find_by_org_and_user(org_id, target_user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "MEMBER_NOT_FOUND",
                    "Member not found in organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to find membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to find membership",
                )),
            ));
        }
    };

    // Derive role_type from the role name if not provided
    let role_type = req
        .role_type
        .unwrap_or_else(|| new_role.name.to_lowercase().replace(' ', "_"));

    // Update the member
    use db::models::UpdateOrganizationMember;
    let update_data = UpdateOrganizationMember {
        role_id: Some(req.role_id),
        role_type: Some(role_type.clone()),
        status: None,
    };

    match state
        .org_member_repo
        .update(target_membership.id, update_data)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "MEMBER_NOT_FOUND",
                    "Member not found or already removed",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update member");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update member",
                )),
            ));
        }
    }

    tracing::info!(
        org_id = %org_id,
        user_id = %target_user_id,
        new_role_id = %req.role_id,
        updated_by = %admin_user_id,
        "Member role updated"
    );

    Ok(Json(UpdateMemberResponse {
        message: "Member role updated successfully".to_string(),
    }))
}

// ==================== Remove Organization Member ====================

/// Remove member response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RemoveMemberResponse {
    /// Success message
    pub message: String,
}

/// Remove a member from an organization.
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{id}/members/{user_id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("user_id" = Uuid, Path, description = "User ID to remove")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Member removed", body = RemoveMemberResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Member not found", body = ErrorResponse)
    )
)]
pub async fn remove_organization_member(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((org_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RemoveMemberResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let admin_user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if requesting user has permission
    let admin_membership = match state
        .org_member_repo
        .find_by_org_and_user(org_id, admin_user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_MEMBER",
                    "You are not a member of this organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to check membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    let role = match admin_membership.role_id {
        Some(role_id) => match state.role_repo.find_by_id(role_id).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    // Allow removing self, or need permission
    if target_user_id != admin_user_id && !role.has_permission("users:delete") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to remove members",
            )),
        ));
    }

    // Find the membership to remove
    let target_membership = match state
        .org_member_repo
        .find_by_org_and_user(org_id, target_user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "MEMBER_NOT_FOUND",
                    "Member not found in organization",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to find membership");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to find membership",
                )),
            ));
        }
    };

    // Remove member using the membership ID
    match state.org_member_repo.remove(target_membership.id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "MEMBER_NOT_FOUND",
                    "Member already removed",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to remove member");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to remove member",
                )),
            ));
        }
    }

    tracing::info!(
        org_id = %org_id,
        user_id = %target_user_id,
        removed_by = %admin_user_id,
        "Member removed from organization"
    );

    Ok(Json(RemoveMemberResponse {
        message: "Member removed successfully".to_string(),
    }))
}


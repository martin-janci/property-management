//! Organization role routes (UC-27, Epic 2A).

use axum::{
    extract::{Path, State},
    http::StatusCode,
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

/// Create organizations roles router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/roles", get(list_organization_roles))
        .route("/{id}/roles", post(create_organization_role))
        .route("/{id}/roles/{role_id}", get(get_organization_role))
        .route("/{id}/roles/{role_id}", put(update_organization_role))
        .route("/{id}/roles/{role_id}", delete(delete_organization_role))
}

// ==================== Organization Roles ====================

/// Role info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RoleResponse {
    /// Role ID
    pub id: Uuid,
    /// Role name
    pub name: String,
    /// Role description
    pub description: Option<String>,
    /// Permissions list
    pub permissions: Vec<String>,
    /// Whether this is a system role
    pub is_system: bool,
}

/// List roles response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListRolesResponse {
    /// Roles
    pub roles: Vec<RoleResponse>,
    /// Total count
    pub total: usize,
}

/// List organization roles.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/roles",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Roles retrieved", body = ListRolesResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_organization_roles(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ListRolesResponse>, (StatusCode, Json<ErrorResponse>)> {
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

    let roles = match state.role_repo.list_by_org(id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch roles");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch roles",
                )),
            ));
        }
    };

    let role_responses: Vec<RoleResponse> = roles
        .into_iter()
        .map(|r| {
            let permissions = r.permission_list();
            RoleResponse {
                id: r.id,
                name: r.name,
                description: r.description,
                permissions,
                is_system: r.is_system,
            }
        })
        .collect();

    let total = role_responses.len();

    Ok(Json(ListRolesResponse {
        roles: role_responses,
        total,
    }))
}

// ==================== Create Organization Role (Story 2A.6) ====================

/// Create role request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    /// Role name
    pub name: String,
    /// Role description
    pub description: Option<String>,
    /// Permissions list (format: "resource:action", e.g., "faults:create")
    pub permissions: Vec<String>,
}

/// Create role response.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateRoleResponse {
    /// Created role
    pub role: RoleResponse,
}

/// Create a custom role in an organization.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{id}/roles",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = CreateRoleRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Role created", body = CreateRoleResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 409, description = "Role name already exists", body = ErrorResponse)
    )
)]
pub async fn create_organization_role(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<(StatusCode, Json<CreateRoleResponse>), (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permissions
    let membership = match state
        .org_member_repo
        .find_by_org_and_user(org_id, user_id)
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

    // Check permission to manage roles
    if !role.has_permission("roles:create") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to create roles",
            )),
        ));
    }

    // Validate role name
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_NAME",
                "Role name cannot be empty",
            )),
        ));
    }

    // Check if role name already exists
    match state.role_repo.find_by_name(org_id, &req.name).await {
        Ok(Some(_)) => {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "ROLE_EXISTS",
                    "A role with this name already exists",
                )),
            ));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!(error = %e, "Failed to check role name");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check role name",
                )),
            ));
        }
    }

    // Create the role
    use db::models::CreateRole;
    let create_data = CreateRole {
        organization_id: org_id,
        name: req.name.trim().to_string(),
        description: req.description,
        permissions: req.permissions,
    };

    let new_role = match state.role_repo.create(create_data).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create role",
                )),
            ));
        }
    };

    let permissions = new_role.permission_list();
    let response = RoleResponse {
        id: new_role.id,
        name: new_role.name,
        description: new_role.description,
        permissions,
        is_system: new_role.is_system,
    };

    tracing::info!(org_id = %org_id, role_id = %response.id, "Custom role created");

    Ok((
        StatusCode::CREATED,
        Json(CreateRoleResponse { role: response }),
    ))
}

// ==================== Get Organization Role ====================

/// Get role response.
#[derive(Debug, Serialize, ToSchema)]
pub struct GetRoleResponse {
    /// Role details
    pub role: RoleResponse,
}

/// Get a specific role by ID.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/roles/{role_id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("role_id" = Uuid, Path, description = "Role ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Role retrieved", body = GetRoleResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse)
    )
)]
pub async fn get_organization_role(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((org_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<GetRoleResponse>, (StatusCode, Json<ErrorResponse>)> {
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
        .find_by_org_and_user(org_id, user_id)
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

    let role = match state.role_repo.find_by_id(role_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("ROLE_NOT_FOUND", "Role not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
            ));
        }
    };

    // Verify role belongs to this organization
    if role.organization_id != org_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "ROLE_NOT_FOUND",
                "Role not found in this organization",
            )),
        ));
    }

    let permissions = role.permission_list();
    let response = RoleResponse {
        id: role.id,
        name: role.name,
        description: role.description,
        permissions,
        is_system: role.is_system,
    };

    Ok(Json(GetRoleResponse { role: response }))
}

// ==================== Update Organization Role ====================

/// Update role request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    /// New role name (optional)
    pub name: Option<String>,
    /// New description (optional)
    pub description: Option<String>,
    /// New permissions list (optional)
    pub permissions: Option<Vec<String>>,
}

/// Update role response.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateRoleResponse {
    /// Updated role
    pub role: RoleResponse,
}

/// Update a custom role in an organization.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/roles/{role_id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("role_id" = Uuid, Path, description = "Role ID")
    ),
    request_body = UpdateRoleRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Role updated", body = UpdateRoleResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized or system role", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    )
)]
pub async fn update_organization_role(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((org_id, role_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<UpdateRoleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permissions
    let membership = match state
        .org_member_repo
        .find_by_org_and_user(org_id, user_id)
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

    let user_role = match membership.role_id {
        Some(rid) => match state.role_repo.find_by_id(rid).await {
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

    // Check permission to manage roles
    if !user_role.has_permission("roles:update") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to update roles",
            )),
        ));
    }

    // Get the role to update
    let existing_role = match state.role_repo.find_by_id(role_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("ROLE_NOT_FOUND", "Role not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
            ));
        }
    };

    // Verify role belongs to this organization
    if existing_role.organization_id != org_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "ROLE_NOT_FOUND",
                "Role not found in this organization",
            )),
        ));
    }

    // Cannot update system roles
    if existing_role.is_system {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "SYSTEM_ROLE",
                "System roles cannot be modified",
            )),
        ));
    }

    // If renaming, check for conflicts
    if let Some(ref new_name) = req.name {
        if new_name.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_NAME",
                    "Role name cannot be empty",
                )),
            ));
        }

        if new_name.to_lowercase() != existing_role.name.to_lowercase() {
            match state.role_repo.find_by_name(org_id, new_name).await {
                Ok(Some(_)) => {
                    return Err((
                        StatusCode::CONFLICT,
                        Json(ErrorResponse::new(
                            "ROLE_EXISTS",
                            "A role with this name already exists",
                        )),
                    ));
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::error!(error = %e, "Failed to check role name");
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "DATABASE_ERROR",
                            "Failed to check role name",
                        )),
                    ));
                }
            }
        }
    }

    // Update the role
    use db::models::UpdateRole;
    let update_data = UpdateRole {
        name: req.name.map(|n| n.trim().to_string()),
        description: req.description,
        permissions: req.permissions,
    };

    let updated_role = match state.role_repo.update(role_id, update_data).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ROLE_NOT_FOUND",
                    "Role not found or is a system role",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update role",
                )),
            ));
        }
    };

    let permissions = updated_role.permission_list();
    let response = RoleResponse {
        id: updated_role.id,
        name: updated_role.name,
        description: updated_role.description,
        permissions,
        is_system: updated_role.is_system,
    };

    tracing::info!(org_id = %org_id, role_id = %role_id, "Custom role updated");

    Ok(Json(UpdateRoleResponse { role: response }))
}

// ==================== Delete Organization Role ====================

/// Delete role response.
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteRoleResponse {
    /// Success message
    pub message: String,
}

/// Delete a custom role from an organization.
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{id}/roles/{role_id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("role_id" = Uuid, Path, description = "Role ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Role deleted", body = DeleteRoleResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized or system role", body = ErrorResponse),
        (status = 404, description = "Role not found", body = ErrorResponse)
    )
)]
pub async fn delete_organization_role(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path((org_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DeleteRoleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permissions
    let membership = match state
        .org_member_repo
        .find_by_org_and_user(org_id, user_id)
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

    let user_role = match membership.role_id {
        Some(rid) => match state.role_repo.find_by_id(rid).await {
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

    // Check permission to manage roles
    if !user_role.has_permission("roles:delete") {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to delete roles",
            )),
        ));
    }

    // Get the role to delete
    let existing_role = match state.role_repo.find_by_id(role_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("ROLE_NOT_FOUND", "Role not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
            ));
        }
    };

    // Verify role belongs to this organization
    if existing_role.organization_id != org_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "ROLE_NOT_FOUND",
                "Role not found in this organization",
            )),
        ));
    }

    // Cannot delete system roles
    if existing_role.is_system {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "SYSTEM_ROLE",
                "System roles cannot be deleted",
            )),
        ));
    }

    // Delete the role
    match state.role_repo.delete(role_id).await {
        Ok(true) => {}
        Ok(false) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ROLE_NOT_FOUND",
                    "Role not found or is a system role",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to delete role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete role",
                )),
            ));
        }
    }

    tracing::info!(org_id = %org_id, role_id = %role_id, "Custom role deleted");

    Ok(Json(DeleteRoleResponse {
        message: "Role deleted successfully".to_string(),
    }))
}


//! Organization core routes — CRUD, helpers.
//!
//! This module also exports shared helper functions (`extract_bearer_token`,
//! `validate_access_token`, `has_super_admin_role`) used by the sibling
//! member, role, and settings surface modules.

//! Organization routes (UC-27, Epic 2A) - Multi-tenancy.
//!
//! Implements organization management including CRUD operations,
//! membership, and tenant context.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{CreateOrganization, Organization, UpdateOrganization};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;



/// Default page size for organization listing (admin only)
pub const DEFAULT_ORG_LIST_LIMIT: i64 = 50;

/// Create organizations core router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_organization))
        .route("/", get(list_organizations))
        .route("/my", get(list_my_organizations))
        .route("/{id}", get(get_organization))
        .route("/{id}", put(update_organization))
        .route("/{id}", delete(delete_organization))
}

// ==================== Create Organization (Story 2A.1) ====================

/// Create organization request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateOrganizationRequest {
    /// Organization name
    pub name: String,
    /// URL-friendly slug (optional, auto-generated if not provided)
    pub slug: Option<String>,
    /// Contact email
    pub contact_email: String,
    /// Logo URL (optional)
    pub logo_url: Option<String>,
    /// Primary brand color (optional)
    pub primary_color: Option<String>,
}

/// Create organization response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationResponse {
    /// Organization ID
    pub id: Uuid,
    /// Organization name
    pub name: String,
    /// URL-friendly slug
    pub slug: String,
    /// Contact email
    pub contact_email: String,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary brand color
    pub primary_color: Option<String>,
    /// Organization status
    pub status: String,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

impl From<Organization> for OrganizationResponse {
    fn from(org: Organization) -> Self {
        Self {
            id: org.id,
            name: org.name,
            slug: org.slug,
            contact_email: org.contact_email,
            logo_url: org.logo_url,
            primary_color: org.primary_color,
            status: org.status,
            created_at: org.created_at.to_rfc3339(),
            updated_at: org.updated_at.to_rfc3339(),
        }
    }
}

/// Create a new organization.
#[utoipa::path(
    post,
    path = "/api/v1/organizations",
    tag = "Organizations",
    request_body = CreateOrganizationRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Organization created", body = OrganizationResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 409, description = "Organization with this slug already exists", body = ErrorResponse)
    )
)]
pub async fn create_organization(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Validate required fields
    if req.name.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_NAME",
                "Organization name cannot be empty",
            )),
        ));
    }

    if req.contact_email.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_EMAIL",
                "Contact email cannot be empty",
            )),
        ));
    }

    // Generate slug if not provided, or validate provided slug
    let slug = match &req.slug {
        Some(provided_slug) => {
            // Validate user-provided slug
            if let Err(error) = validate_slug(provided_slug) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("INVALID_SLUG", &error)),
                ));
            }
            provided_slug.clone()
        }
        None => {
            // Auto-generate from name
            let generated = generate_slug(&req.name);
            // Validate the generated slug as well
            if validate_slug(&generated).is_err() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "INVALID_NAME",
                        "Organization name cannot be converted to a valid slug. Please provide a custom slug.",
                    )),
                ));
            }
            generated
        }
    };

    // Check if slug is already taken
    match state
        .org_repo
        .find_by_slug_rls(&mut **rls.conn(), &slug)
        .await
    {
        Ok(Some(_)) => {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "SLUG_EXISTS",
                    "An organization with this slug already exists",
                )),
            ));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!(error = %e, "Database error checking slug");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check organization slug",
                )),
            ));
        }
    }

    // Create organization
    let create_org = CreateOrganization {
        name: req.name.clone(),
        slug,
        contact_email: req.contact_email.clone(),
        logo_url: req.logo_url.clone(),
        primary_color: req.primary_color.clone(),
    };

    let org = match state
        .org_repo
        .create_rls(&mut **rls.conn(), create_org)
        .await
    {
        Ok(org) => org,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create organization");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create organization",
                )),
            ));
        }
    };

    // Add creator as organization admin
    use db::models::CreateOrganizationMember;

    // First, get the Organization Admin role for this org
    let admin_role = match state
        .role_repo
        .find_by_name(org.id, "Organization Admin")
        .await
    {
        Ok(Some(role)) => role,
        Ok(None) => {
            tracing::error!(org_id = %org.id, "Organization Admin role not found");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "ROLE_NOT_FOUND",
                    "Organization Admin role not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to find admin role");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to find admin role",
                )),
            ));
        }
    };

    let create_member = CreateOrganizationMember {
        organization_id: org.id,
        user_id,
        role_id: Some(admin_role.id),
        role_type: "org_admin".to_string(),
        invited_by: None, // Self-join as creator
    };

    if let Err(e) = state.org_member_repo.create(create_member).await {
        tracing::error!(error = %e, org_id = %org.id, user_id = %user_id, "Failed to add creator as admin");
        // Continue anyway - organization was created
    }

    tracing::info!(
        org_id = %org.id,
        name = %org.name,
        creator_id = %user_id,
        "Organization created"
    );

    Ok((StatusCode::CREATED, Json(OrganizationResponse::from(org))))
}

// ==================== List Organizations ====================

/// List organizations response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListOrganizationsResponse {
    /// Organizations
    pub organizations: Vec<OrganizationResponse>,
    /// Total count
    pub total: i64,
}

/// List all organizations (admin only).
#[utoipa::path(
    get,
    path = "/api/v1/organizations",
    tag = "Organizations",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Organizations retrieved", body = ListOrganizationsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn list_organizations(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListOrganizationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Only super admins can list all organizations.
    // Super-admin status is carried by the JWT role claim (TenantRole::SuperAdmin
    // / PlatformAdmin), projected into the `app.is_super_admin` session GUC for
    // RLS. This mirrors the role gate used by platform_admin / subscriptions /
    // feature_packages handlers.
    if !has_super_admin_role(&claims.roles) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "This endpoint is restricted to super administrators. Use GET /api/v1/organizations/my to list your organizations.",
            )),
        ));
    }

    // Super admin can list all organizations
    let (orgs, total) = match state
        .org_repo
        .list_full(0, DEFAULT_ORG_LIST_LIMIT, None, None)
        .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = %e, user_id = %user_id, "Failed to list organizations");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list organizations",
                )),
            ));
        }
    };

    let organizations: Vec<OrganizationResponse> =
        orgs.into_iter().map(OrganizationResponse::from).collect();
    Ok(Json(ListOrganizationsResponse {
        organizations,
        total,
    }))
}

// ==================== List My Organizations ====================

/// List organizations the current user belongs to.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/my",
    tag = "Organizations",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User's organizations retrieved", body = ListOrganizationsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_my_organizations(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
) -> Result<Json<ListOrganizationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    let memberships = match state
        .org_member_repo
        .get_user_memberships_rls(&mut **rls.conn(), user_id)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch user organizations");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch organizations",
                )),
            ));
        }
    };

    // Batch-fetch full organization details for all memberships in one query
    // (avoids N+1: previously issued one SELECT per membership).
    let org_ids: Vec<Uuid> = memberships.iter().map(|m| m.organization_id).collect();

    let orgs = match state
        .org_repo
        .find_by_ids_rls(&mut **rls.conn(), &org_ids)
        .await
    {
        Ok(o) => o,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch organizations by ids");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch organizations",
                )),
            ));
        }
    };

    // Regroup into a HashMap so we can preserve membership ordering and
    // silently skip orgs that were deleted/inaccessible (matching prior behavior).
    let org_by_id: std::collections::HashMap<Uuid, Organization> =
        orgs.into_iter().map(|o| (o.id, o)).collect();

    let organizations: Vec<OrganizationResponse> = memberships
        .iter()
        .filter_map(|m| {
            org_by_id
                .get(&m.organization_id)
                .cloned()
                .map(OrganizationResponse::from)
        })
        .collect();

    let total = organizations.len() as i64;

    rls.release().await;
    Ok(Json(ListOrganizationsResponse {
        organizations,
        total,
    }))
}

// ==================== Get Organization ====================

/// Get a specific organization.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Organization retrieved", body = OrganizationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationResponse>, (StatusCode, Json<ErrorResponse>)> {
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
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
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
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    }

    let org = match state.org_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch organization");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch organization",
                )),
            ));
        }
    };

    rls.release().await;
    Ok(Json(OrganizationResponse::from(org)))
}

// ==================== Update Organization ====================

/// Update organization request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrganizationRequest {
    /// Organization name
    pub name: Option<String>,
    /// Contact email
    pub contact_email: Option<String>,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary brand color
    pub primary_color: Option<String>,
}

/// Update an organization.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = UpdateOrganizationRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Organization updated", body = OrganizationResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn update_organization(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if user is admin of this organization
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            rls.release().await;
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
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    // Get role and check permissions
    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => {
                rls.release().await;
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                rls.release().await;
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    // Check for organization:update permission
    if !role.has_permission("organization:update") {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to update this organization",
            )),
        ));
    }

    // Update organization
    let update = UpdateOrganization {
        name: req.name,
        contact_email: req.contact_email,
        logo_url: req.logo_url,
        primary_color: req.primary_color,
        settings: None,
    };

    let org = match state
        .org_repo
        .update_rls(&mut **rls.conn(), id, update)
        .await
    {
        Ok(Some(org)) => org,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update organization");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update organization",
                )),
            ));
        }
    };

    tracing::info!(org_id = %id, user_id = %user_id, "Organization updated");

    rls.release().await;
    Ok(Json(OrganizationResponse::from(org)))
}

// ==================== Delete Organization ====================

/// Delete organization response.
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteOrganizationResponse {
    /// Success message
    pub message: String,
}

/// Delete an organization (soft delete).
#[utoipa::path(
    delete,
    path = "/api/v1/organizations/{id}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Organization deleted", body = DeleteOrganizationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn delete_organization(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<DeleteOrganizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check if user is admin of this organization
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(Some(m)) => m,
        Ok(None) => {
            rls.release().await;
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
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to check membership",
                )),
            ));
        }
    };

    // Get role and check permissions
    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => {
                rls.release().await;
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to fetch role");
                rls.release().await;
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", "Failed to fetch role")),
                ));
            }
        },
        None => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new("NO_ROLE", "User has no role assigned")),
            ));
        }
    };

    // Check for organization:delete permission
    if !role.has_permission("organization:delete") {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to delete this organization",
            )),
        ));
    }

    // Soft delete using dedicated method
    match state.org_repo.archive_rls(&mut **rls.conn(), id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ORG_NOT_FOUND",
                    "Organization not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to delete organization");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete organization",
                )),
            ));
        }
    }

    tracing::info!(org_id = %id, user_id = %user_id, "Organization deleted");

    rls.release().await;
    Ok(Json(DeleteOrganizationResponse {
        message: "Organization deleted successfully".to_string(),
    }))
}


// ==================== Helper Functions ====================

/// Extract bearer token from Authorization header.
pub fn extract_bearer_token(
    headers: &axum::http::HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
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

    Ok(auth_header[7..].to_string())
}

/// Super admin role names accepted in the JWT `roles` claim.
///
/// Matches the canonical list used by the platform_admin / subscriptions /
/// feature_packages handlers.
pub const SUPER_ADMIN_ROLES: &[&str] = &[
    "SuperAdministrator",
    "super_admin",
    "superadmin",
    "platform_admin",
];

/// Check if the JWT `roles` claim grants super-admin access.
pub fn has_super_admin_role(roles: &Option<Vec<String>>) -> bool {
    match roles {
        Some(user_roles) => user_roles.iter().any(|r| {
            SUPER_ADMIN_ROLES
                .iter()
                .any(|admin| r.eq_ignore_ascii_case(admin))
        }),
        None => false,
    }
}

/// Validate access token and return claims.
pub fn validate_access_token(
    state: &AppState,
    token: &str,
) -> Result<crate::services::jwt::Claims, (StatusCode, Json<ErrorResponse>)> {
    state.jwt_service.validate_access_token(token).map_err(|e| {
        tracing::debug!(error = %e, "Invalid access token");
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_TOKEN",
                "Invalid or expired token",
            )),
        )
    })
}

/// Minimum slug length
pub const MIN_SLUG_LENGTH: usize = 3;

/// Maximum slug length
pub const MAX_SLUG_LENGTH: usize = 50;

/// Validate organization slug format.
///
/// Valid slugs:
/// - 3-50 characters
/// - Only lowercase alphanumeric and hyphens
/// - Cannot start or end with hyphen
/// - Cannot contain consecutive hyphens
pub fn validate_slug(slug: &str) -> Result<(), String> {
    // Check length
    if slug.len() < MIN_SLUG_LENGTH {
        return Err(format!(
            "Slug must be at least {} characters",
            MIN_SLUG_LENGTH
        ));
    }
    if slug.len() > MAX_SLUG_LENGTH {
        return Err(format!("Slug cannot exceed {} characters", MAX_SLUG_LENGTH));
    }

    // Check for valid characters only
    if !slug
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("Slug can only contain lowercase letters, numbers, and hyphens".to_string());
    }

    // Cannot start or end with hyphen
    if slug.starts_with('-') || slug.ends_with('-') {
        return Err("Slug cannot start or end with a hyphen".to_string());
    }

    // Cannot contain consecutive hyphens
    if slug.contains("--") {
        return Err("Slug cannot contain consecutive hyphens".to_string());
    }

    Ok(())
}

/// Generate URL-friendly slug from organization name.
pub fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                // Convert whitespace, hyphens, underscores, and any other chars to hyphen
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}


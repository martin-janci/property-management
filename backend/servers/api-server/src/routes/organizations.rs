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
const DEFAULT_ORG_LIST_LIMIT: i64 = 50;

/// Create organizations router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_organization))
        .route("/", get(list_organizations))
        .route("/my", get(list_my_organizations))
        .route("/{id}", get(get_organization))
        .route("/{id}", put(update_organization))
        .route("/{id}", delete(delete_organization))
        .route("/{id}/members", get(list_organization_members))
        .route("/{id}/members", post(add_organization_member))
        .route("/{id}/members/{user_id}", put(update_organization_member))
        .route("/{id}/members/{user_id}", delete(remove_organization_member))
        .route("/{id}/roles", get(list_organization_roles))
        .route("/{id}/roles", post(create_organization_role))
        .route("/{id}/roles/{role_id}", get(get_organization_role))
        .route("/{id}/roles/{role_id}", put(update_organization_role))
        .route("/{id}/roles/{role_id}", delete(delete_organization_role))
        // Settings and branding (Story 2A.4)
        .route("/{id}/settings", get(get_organization_settings))
        .route("/{id}/settings", put(update_organization_settings))
        .route("/{id}/branding", get(get_organization_branding))
        .route("/{id}/branding", put(update_organization_branding))
        // Data export (Story 2A.7)
        .route("/{id}/export", get(export_organization_data))
        // Feature preferences (Epic 110, Story 110.3)
        .route("/{id}/features", get(list_organization_features))
        .route("/{id}/features", put(bulk_update_organization_features))
        .route("/{id}/features/{key}", put(toggle_organization_feature))
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

    // Check if user is super admin by checking user record
    // Note: In production, this should check an is_super_admin field on the user
    let user = match state.user_repo.find_by_id(user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("USER_NOT_FOUND", "User not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify user",
                )),
            ));
        }
    };

    // Only super admins can list all organizations (Phase 1.2)
    if !user.is_super_administrator() {
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

    // Fetch full organization details for each membership
    let mut organizations = Vec::new();
    for membership in &memberships {
        if let Ok(Some(org)) = state
            .org_repo
            .find_by_id_rls(&mut **rls.conn(), membership.organization_id)
            .await
        {
            organizations.push(OrganizationResponse::from(org));
        }
    }

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

    let mut member_responses = Vec::new();
    for m in members_with_users {
        // Get role name
        let role_name = match m.role_id {
            Some(role_id) => match state.role_repo.find_by_id(role_id).await {
                Ok(Some(r)) => r.name,
                _ => "Unknown".to_string(),
            },
            None => "No Role".to_string(),
        };

        member_responses.push(MemberResponse {
            user_id: m.user_id,
            name: m.user_name,
            email: m.user_email,
            role_id: m.role_id,
            role_name,
            role_type: m.role_type,
            joined_at: m.joined_at.map(|dt| dt.to_rfc3339()),
        });
    }

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

// ==================== Helper Functions ====================

/// Extract bearer token from Authorization header.
fn extract_bearer_token(
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

/// Validate access token and return claims.
fn validate_access_token(
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
const MIN_SLUG_LENGTH: usize = 3;

/// Maximum slug length
const MAX_SLUG_LENGTH: usize = 50;

/// Validate organization slug format.
///
/// Valid slugs:
/// - 3-50 characters
/// - Only lowercase alphanumeric and hyphens
/// - Cannot start or end with hyphen
/// - Cannot contain consecutive hyphens
fn validate_slug(slug: &str) -> Result<(), String> {
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
fn generate_slug(name: &str) -> String {
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

// ==================== Organization Settings (Story 2A.4) ====================

/// Organization settings response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationSettingsResponse {
    /// Organization ID
    pub organization_id: Uuid,
    /// Settings JSON object
    pub settings: serde_json::Value,
}

/// Update organization settings request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrganizationSettingsRequest {
    /// Settings to update (will be merged with existing)
    pub settings: serde_json::Value,
}

/// Get organization settings.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/settings",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Settings retrieved", body = OrganizationSettingsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization_settings(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
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
    Ok(Json(OrganizationSettingsResponse {
        organization_id: id,
        settings: org.settings,
    }))
}

/// Update organization settings.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/settings",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = UpdateOrganizationSettingsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Settings updated", body = OrganizationSettingsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn update_organization_settings(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationSettingsRequest>,
) -> Result<Json<OrganizationSettingsResponse>, (StatusCode, Json<ErrorResponse>)> {
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

    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            _ => {
                rls.release().await;
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
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

    if !role.has_permission("organization:update") {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to update organization settings",
            )),
        ));
    }

    // Update settings
    let update = UpdateOrganization {
        name: None,
        contact_email: None,
        logo_url: None,
        primary_color: None,
        settings: Some(req.settings.clone()),
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
            tracing::error!(error = %e, "Failed to update organization settings");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update settings",
                )),
            ));
        }
    };

    tracing::info!(org_id = %id, user_id = %user_id, "Organization settings updated");

    rls.release().await;
    Ok(Json(OrganizationSettingsResponse {
        organization_id: id,
        settings: org.settings,
    }))
}

// ==================== Organization Branding (Story 2A.4) ====================

/// Organization branding response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationBrandingResponse {
    /// Organization ID
    pub organization_id: Uuid,
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary brand color (hex)
    pub primary_color: Option<String>,
    /// Organization name (for branding)
    pub name: String,
}

/// Update organization branding request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOrganizationBrandingRequest {
    /// Logo URL
    pub logo_url: Option<String>,
    /// Primary brand color (hex format, e.g., "#FF5733")
    pub primary_color: Option<String>,
}

/// Get organization branding.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/branding",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Branding retrieved", body = OrganizationBrandingResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn get_organization_branding(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OrganizationBrandingResponse>, (StatusCode, Json<ErrorResponse>)> {
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
    Ok(Json(OrganizationBrandingResponse {
        organization_id: id,
        logo_url: org.logo_url,
        primary_color: org.primary_color,
        name: org.name,
    }))
}

/// Update organization branding.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/branding",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = UpdateOrganizationBrandingRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Branding updated", body = OrganizationBrandingResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn update_organization_branding(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrganizationBrandingRequest>,
) -> Result<Json<OrganizationBrandingResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Validate color format if provided
    if let Some(ref color) = req.primary_color {
        if !is_valid_hex_color(color) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_COLOR",
                    "Primary color must be a valid hex color (e.g., #FF5733)",
                )),
            ));
        }
    }

    // Check membership and permissions
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

    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
            Ok(Some(r)) => r,
            _ => {
                rls.release().await;
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new("ROLE_NOT_FOUND", "User role not found")),
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

    if !role.has_permission("organization:update") {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to update organization branding",
            )),
        ));
    }

    // Update branding
    let update = UpdateOrganization {
        name: None,
        contact_email: None,
        logo_url: req.logo_url.clone(),
        primary_color: req.primary_color.clone(),
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
            tracing::error!(error = %e, "Failed to update organization branding");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update branding",
                )),
            ));
        }
    };

    tracing::info!(org_id = %id, user_id = %user_id, "Organization branding updated");

    rls.release().await;
    Ok(Json(OrganizationBrandingResponse {
        organization_id: id,
        logo_url: org.logo_url,
        primary_color: org.primary_color,
        name: org.name,
    }))
}

/// Validate hex color format.
fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }
    let hex = &color[1..];
    (hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Escape a field for CSV output to prevent injection and handle special characters.
/// - Fields containing commas, quotes, or newlines are wrapped in quotes
/// - Double quotes within fields are escaped by doubling them
/// - Fields starting with =, +, -, @ are prefixed with a single quote to prevent formula injection
fn escape_csv_field(field: &str) -> String {
    // Check for formula injection characters at start
    let needs_quote_prefix = field.starts_with('=')
        || field.starts_with('+')
        || field.starts_with('-')
        || field.starts_with('@');

    // Check if field needs quoting
    let needs_quotes = field.contains(',')
        || field.contains('"')
        || field.contains('\n')
        || field.contains('\r')
        || needs_quote_prefix;

    if needs_quotes {
        // Escape double quotes by doubling them
        let escaped = field.replace('"', "\"\"");
        if needs_quote_prefix {
            // Prefix with single quote to prevent formula execution
            format!("\"'{}\"", escaped)
        } else {
            format!("\"{}\"", escaped)
        }
    } else {
        field.to_string()
    }
}

/// Maximum number of members to export at once
const MAX_EXPORT_MEMBERS: i64 = 5000;

// ==================== Organization Data Export (Story 2A.7) ====================

/// Export member data.
#[derive(Debug, Serialize, ToSchema)]
pub struct ExportMember {
    /// User ID
    pub user_id: Uuid,
    /// User email
    pub email: Option<String>,
    /// User name
    pub name: Option<String>,
    /// Role name
    pub role_name: Option<String>,
    /// Role type
    pub role_type: String,
    /// Member status
    pub status: String,
    /// Join date
    pub joined_at: String,
}

/// Export role data.
#[derive(Debug, Serialize, ToSchema)]
pub struct ExportRole {
    /// Role ID
    pub id: Uuid,
    /// Role name
    pub name: String,
    /// Role description
    pub description: Option<String>,
    /// Permissions
    pub permissions: Vec<String>,
    /// Is system role
    pub is_system: bool,
}

/// Organization export query parameters.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExportQuery {
    /// Export format (json or csv)
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Organization data export response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationExportResponse {
    /// Export timestamp
    pub exported_at: String,
    /// Organization info
    pub organization: OrganizationResponse,
    /// Members list
    pub members: Vec<ExportMember>,
    /// Roles list
    pub roles: Vec<ExportRole>,
    /// Total members count
    pub total_members: usize,
    /// Total roles count
    pub total_roles: usize,
}

/// Export organization data.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/export",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("format" = Option<String>, Query, description = "Export format: json (default) or csv")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Data exported", body = OrganizationExportResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn export_organization_data(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permissions using RLS
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
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
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
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

    // Check permission to export data (org:export or org:read + users:read)
    // Note: has_permission() already checks for wildcard "*" internally
    let can_export = role.has_permission("organization:export")
        || (role.has_permission("organization:read") && role.has_permission("users:read"));

    if !can_export {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to export organization data",
            )),
        ));
    }

    // Get organization using RLS
    let org = match state.org_repo.find_by_id_rls(&mut **rls.conn(), id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
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
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch organization",
                )),
            ));
        }
    };

    // Get members with user info (paginated export to prevent memory issues) using RLS
    let (members, _total_member_count) = match state
        .org_member_repo
        .list_org_members_rls(&mut **rls.conn(), id, 0, MAX_EXPORT_MEMBERS, None)
        .await
    {
        Ok(m) => m,
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

    // Get roles using RLS
    let roles = match state.role_repo.list_by_org_rls(&mut **rls.conn(), id).await {
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

    // Build export members
    let export_members: Vec<ExportMember> = members
        .into_iter()
        .map(|m| {
            let role_name = roles
                .iter()
                .find(|r| Some(r.id) == m.role_id)
                .map(|r| r.name.clone());

            ExportMember {
                user_id: m.user_id,
                email: Some(m.user_email),
                name: Some(m.user_name),
                role_name,
                role_type: m.role_type,
                status: m.status,
                joined_at: m.joined_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
            }
        })
        .collect();

    // Build export roles
    let export_roles: Vec<ExportRole> = roles
        .into_iter()
        .map(|r| {
            let permissions = r.permission_list();
            ExportRole {
                id: r.id,
                name: r.name,
                description: r.description,
                permissions,
                is_system: r.is_system,
            }
        })
        .collect();

    let total_members = export_members.len();
    let total_roles = export_roles.len();

    let org_response = OrganizationResponse::from(org);

    // Handle CSV export
    if query.format.to_lowercase() == "csv" {
        let mut csv_data = String::new();
        csv_data.push_str("user_id,email,name,role_name,role_type,status,joined_at\n");

        for m in &export_members {
            csv_data.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                m.user_id,
                escape_csv_field(m.email.as_deref().unwrap_or("")),
                escape_csv_field(m.name.as_deref().unwrap_or("")),
                escape_csv_field(m.role_name.as_deref().unwrap_or("")),
                escape_csv_field(&m.role_type),
                escape_csv_field(&m.status),
                m.joined_at
            ));
        }

        tracing::info!(org_id = %id, format = "csv", members = total_members, "Organization data exported");

        return Ok((
            StatusCode::OK,
            [
                (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    "attachment; filename=\"organization-export.csv\"",
                ),
            ],
            csv_data,
        )
            .into_response());
    }

    // Default to JSON export
    let export = OrganizationExportResponse {
        exported_at: chrono::Utc::now().to_rfc3339(),
        organization: org_response,
        members: export_members,
        roles: export_roles,
        total_members,
        total_roles,
    };

    tracing::info!(org_id = %id, format = "json", "Organization data exported");

    Ok(Json(export).into_response())
}

// ==================== Organization Feature Preferences (Epic 110, Story 110.3) ====================

/// Organization feature preference response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OrganizationFeatureResponse {
    /// Feature flag key
    pub key: String,
    /// Feature flag name
    pub name: String,
    /// Feature flag description
    pub description: Option<String>,
    /// Whether globally enabled
    pub global_enabled: bool,
    /// Whether enabled for this organization (override)
    pub org_enabled: Option<bool>,
    /// Resolved enabled state for this organization
    pub effective_enabled: bool,
    /// Whether the org can toggle this feature
    pub can_toggle: bool,
}

/// List organization features response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListOrganizationFeaturesResponse {
    /// List of features with their states
    pub features: Vec<OrganizationFeatureResponse>,
}

/// Request to update a single feature preference.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFeaturePreferenceRequest {
    /// Whether to enable the feature
    pub is_enabled: bool,
}

/// Request to bulk update feature preferences.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkUpdateFeaturesRequest {
    /// Map of feature keys to enabled states
    pub features: std::collections::HashMap<String, bool>,
}

/// Response after updating feature preferences.
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateFeaturesResponse {
    /// Number of features updated
    pub updated: usize,
    /// Success message
    pub message: String,
}

/// List all features for an organization with their toggle states.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{id}/features",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of features", body = ListOrganizationFeaturesResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn list_organization_features(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ListOrganizationFeaturesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership using RLS
    let _membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
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

    // Get all feature flags
    let flags = match state.feature_flag_repo.list_all().await {
        Ok(flags) => flags,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch feature flags");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch feature flags",
                )),
            ));
        }
    };

    // Build response with org-level preferences
    let mut features = Vec::new();
    for flag in flags {
        // Get org-level override if exists
        let org_enabled = match state
            .feature_flag_repo
            .is_enabled_for_context(&flag.key, None, Some(id), None)
            .await
        {
            Ok(Some(enabled)) if enabled != flag.is_enabled => Some(enabled),
            Ok(_) => None,
            Err(_) => None,
        };

        // Resolve effective state
        let effective_enabled = org_enabled.unwrap_or(flag.is_enabled);

        features.push(OrganizationFeatureResponse {
            key: flag.key,
            name: flag.name,
            description: flag.description,
            global_enabled: flag.is_enabled,
            org_enabled,
            effective_enabled,
            can_toggle: true, // All features can be toggled by org admins
        });
    }

    Ok(Json(ListOrganizationFeaturesResponse { features }))
}

/// Bulk update feature preferences for an organization.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/features",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID")
    ),
    request_body = BulkUpdateFeaturesRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Features updated", body = UpdateFeaturesResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization not found", body = ErrorResponse)
    )
)]
pub async fn bulk_update_organization_features(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<BulkUpdateFeaturesRequest>,
) -> Result<Json<UpdateFeaturesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permission using RLS
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
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

    // Get role and check permissions using RLS
    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
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

    // Check for organization:manage_features or organization:update permission
    if !role.has_permission("organization:manage_features")
        && !role.has_permission("organization:update")
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to manage feature preferences",
            )),
        ));
    }

    let mut updated = 0;
    for (key, enabled) in req.features {
        // Get flag by key
        let flag = match state.feature_flag_repo.get_by_key(&key).await {
            Ok(Some(f)) => f,
            Ok(None) => {
                tracing::warn!(key = %key, "Feature flag not found, skipping");
                continue;
            }
            Err(e) => {
                tracing::error!(error = %e, key = %key, "Failed to fetch feature flag");
                continue;
            }
        };

        // Create or update override for this organization
        match state
            .feature_flag_repo
            .create_override(
                flag.id,
                db::models::platform_admin::FeatureFlagScope::Organization,
                id,
                enabled,
            )
            .await
        {
            Ok(_) => {
                updated += 1;
                tracing::info!(
                    org_id = %id,
                    flag_key = %key,
                    enabled = enabled,
                    "Feature preference updated"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, key = %key, "Failed to update feature preference");
            }
        }
    }

    Ok(Json(UpdateFeaturesResponse {
        updated,
        message: format!("Updated {} feature preferences", updated),
    }))
}

/// Toggle a single feature for an organization.
#[utoipa::path(
    put,
    path = "/api/v1/organizations/{id}/features/{key}",
    tag = "Organizations",
    params(
        ("id" = Uuid, Path, description = "Organization ID"),
        ("key" = String, Path, description = "Feature flag key")
    ),
    request_body = UpdateFeaturePreferenceRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Feature updated", body = OrganizationFeatureResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Organization or feature not found", body = ErrorResponse)
    )
)]
pub async fn toggle_organization_feature(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    mut rls: RlsConnection,
    Path((id, key)): Path<(Uuid, String)>,
    Json(req): Json<UpdateFeaturePreferenceRequest>,
) -> Result<Json<OrganizationFeatureResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // Check membership and permission using RLS
    let membership = match state
        .org_member_repo
        .find_by_org_and_user_rls(&mut **rls.conn(), id, user_id)
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

    // Get role and check permissions using RLS
    let role = match membership.role_id {
        Some(role_id) => match state
            .role_repo
            .find_by_id_rls(&mut **rls.conn(), role_id)
            .await
        {
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

    // Check for organization:manage_features or organization:update permission
    if !role.has_permission("organization:manage_features")
        && !role.has_permission("organization:update")
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "PERMISSION_DENIED",
                "You do not have permission to manage feature preferences",
            )),
        ));
    }

    // Get flag by key
    let flag = match state.feature_flag_repo.get_by_key(&key).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "FEATURE_NOT_FOUND",
                    "Feature flag not found",
                )),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch feature flag");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to fetch feature flag",
                )),
            ));
        }
    };

    // Create or update override for this organization
    match state
        .feature_flag_repo
        .create_override(
            flag.id,
            db::models::platform_admin::FeatureFlagScope::Organization,
            id,
            req.is_enabled,
        )
        .await
    {
        Ok(_) => {
            tracing::info!(
                org_id = %id,
                flag_key = %key,
                enabled = req.is_enabled,
                user_id = %user_id,
                "Feature preference updated"
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to update feature preference");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update feature preference",
                )),
            ));
        }
    }

    // Return updated feature state
    let feature_details = match state.feature_flag_repo.get_by_key(&key).await {
        Ok(Some(f)) => f,
        Ok(None) | Err(_) => flag.clone(),
    };

    Ok(Json(OrganizationFeatureResponse {
        key: feature_details.key,
        name: feature_details.name,
        description: feature_details.description,
        global_enabled: feature_details.is_enabled,
        org_enabled: Some(req.is_enabled),
        effective_enabled: req.is_enabled,
        can_toggle: true,
    }))
}

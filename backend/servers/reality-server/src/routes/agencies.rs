//! Agency routes (Epic 32: Agency Management).

use crate::extractors::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use db::{
    models::{
        CreateAgencyInvitation, CreateRealityAgency, RealityAgency, RealityAgencyInvitation,
        RealityAgencyMember, UpdateAgencyBranding, UpdateRealityAgency,
    },
    SqlxError,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create agencies router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_agency))
        .route("/", get(list_agencies))
        .route("/{id}", get(get_agency))
        .route("/{id}", put(update_agency))
        // /{id}/branding handled by routes::agency_branding (mounted at
        // /api/v1/agencies/{id}/branding) — registering it here too caused
        // axum's "Overlapping method route" panic at boot.
        .route("/{id}/members", get(list_members))
        .route("/{id}/invitations", post(create_invitation))
        .route("/by-slug/{slug}", get(get_agency_by_slug))
        .route("/invitations/{token}/accept", post(accept_invitation))
}

/// Agency response with summary info.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AgencyResponse {
    pub agency: RealityAgency,
}

/// Public directory list response. Returned by `GET /api/v1/agencies` and
/// consumed by the reality-web public agency directory plus the KMP
/// `AgencyRepository.listAgencies()` on Android/iOS.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AgencyListResponse {
    pub agencies: Vec<RealityAgency>,
    pub total: i64,
}

/// Pagination query for the public directory.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListAgenciesQuery {
    /// Page size (default 50, capped at 200).
    pub limit: Option<i64>,
    /// Offset into the result set (default 0).
    pub offset: Option<i64>,
}

/// Members list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MembersResponse {
    pub members: Vec<RealityAgencyMember>,
    pub total: i64,
}

/// Create a new agency.
#[utoipa::path(
    post,
    path = "/api/v1/agencies",
    tag = "Agencies",
    request_body = CreateRealityAgency,
    responses(
        (status = 201, description = "Agency created", body = AgencyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_agency(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(data): Json<CreateRealityAgency>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    let agency = state
        .reality_portal_repo
        .create_agency(auth.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("duplicate") || error_str.contains("already exists") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Agency with this name or slug already exists".to_string(),
                )
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create agency: {}", e),
                )
            }
        })?;

    Ok(Json(AgencyResponse { agency }))
}

/// List public (verified) agencies for the directory surface.
#[utoipa::path(
    get,
    path = "/api/v1/agencies",
    tag = "Agencies",
    params(ListAgenciesQuery),
    responses(
        (status = 200, description = "Paginated list of verified agencies", body = AgencyListResponse),
    )
)]
pub async fn list_agencies(
    State(state): State<AppState>,
    Query(query): Query<ListAgenciesQuery>,
) -> Result<Json<AgencyListResponse>, (axum::http::StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let (agencies, total) = state
        .reality_portal_repo
        .list_public_agencies(limit, offset)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list agencies: {}", e),
            )
        })?;

    Ok(Json(AgencyListResponse { agencies, total }))
}

/// Get agency by ID.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "Agency details", body = AgencyResponse),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn get_agency(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    let agency = state
        .reality_portal_repo
        .get_agency(id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get agency: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Agency not found".to_string(),
            )
        })?;

    Ok(Json(AgencyResponse { agency }))
}

/// Get agency by slug.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/by-slug/{slug}",
    tag = "Agencies",
    params(("slug" = String, Path, description = "Agency slug")),
    responses(
        (status = 200, description = "Agency details", body = AgencyResponse),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn get_agency_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    let agency = state
        .reality_portal_repo
        .get_agency_by_slug(&slug)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get agency: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Agency not found".to_string(),
            )
        })?;

    Ok(Json(AgencyResponse { agency }))
}

/// Update agency.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = UpdateRealityAgency,
    responses(
        (status = 200, description = "Agency updated", body = AgencyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn update_agency(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateRealityAgency>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    let agency = state
        .reality_portal_repo
        .update_agency(id, data)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "Agency not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update agency: {}", other),
            ),
        })?;

    Ok(Json(AgencyResponse { agency }))
}

/// Update agency branding.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}/branding",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = UpdateAgencyBranding,
    responses(
        (status = 200, description = "Branding updated", body = AgencyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn update_branding(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateAgencyBranding>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    let agency = state
        .reality_portal_repo
        .update_agency_branding(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update branding: {}", e),
            )
        })?;

    Ok(Json(AgencyResponse { agency }))
}

/// List agency members.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/members",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "List of members", body = MembersResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_members(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<MembersResponse>, (axum::http::StatusCode, String)> {
    let members = state
        .reality_portal_repo
        .get_agency_members(id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get members: {}", e),
            )
        })?;

    let total = members.len() as i64;

    Ok(Json(MembersResponse { members, total }))
}

/// Create agency invitation.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/{id}/invitations",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = CreateAgencyInvitation,
    responses(
        (status = 201, description = "Invitation created", body = RealityAgencyInvitation),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn create_invitation(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(agency_id): Path<Uuid>,
    Json(data): Json<CreateAgencyInvitation>,
) -> Result<Json<RealityAgencyInvitation>, (axum::http::StatusCode, String)> {
    let invitation = state
        .reality_portal_repo
        .create_invitation(agency_id, auth.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("not found") {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    "Agency not found".to_string(),
                )
            } else if error_str.contains("permission") || error_str.contains("unauthorized") {
                (
                    axum::http::StatusCode::FORBIDDEN,
                    "Not authorized to create invitations for this agency".to_string(),
                )
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create invitation: {}", e),
                )
            }
        })?;

    Ok(Json(invitation))
}

/// Accept invitation request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcceptInvitationRequest {
    pub user_id: Uuid, // Deprecated: now extracted from auth context
}

/// Accept agency invitation.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/invitations/{token}/accept",
    tag = "Agencies",
    params(("token" = String, Path, description = "Invitation token")),
    responses(
        (status = 200, description = "Invitation accepted", body = RealityAgencyMember),
        (status = 400, description = "Invalid or expired token"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn accept_invitation(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(token): Path<String>,
) -> Result<Json<RealityAgencyMember>, (axum::http::StatusCode, String)> {
    let member = state
        .reality_portal_repo
        .accept_invitation(&token, auth.user_id)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("expired") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Invitation has expired".to_string(),
                )
            } else if error_str.contains("not found") || error_str.contains("invalid") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Invalid invitation token".to_string(),
                )
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to accept invitation: {}", e),
                )
            }
        })?;

    Ok(Json(member))
}

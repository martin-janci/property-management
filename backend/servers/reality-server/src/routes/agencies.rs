//! Agency routes (Epic 32: Agency Management).
//!
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
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
        // `/me` must be registered before `/{id}` so the literal segment
        // isn't shadowed by the UUID path param (a `GET /agencies/me` used
        // to fall through to `get_agency` and fail UUID parsing).
        .route("/me", get(get_my_agency))
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
    principal: RequestPrincipal,
    Json(data): Json<CreateRealityAgency>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    let agency = state
        .reality_portal_repo
        .create_agency(principal.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("duplicate") || error_str.contains("already exists") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Agency with this name or slug already exists".to_string(),
                )
            } else {
                crate::util::errors::db_error("create agency", e)
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
        .map_err(|e| crate::util::errors::db_error("list agencies", e))?;

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
        .map_err(|e| crate::util::errors::db_error("get agency", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Agency not found".to_string(),
            )
        })?;

    Ok(Json(AgencyResponse { agency }))
}

/// Get the agency the authenticated portal user belongs to.
///
/// Returns the caller's agency as a bare `RealityAgency` (the reality-web
/// `useMyAgency()` hook reads the entity directly, not wrapped in
/// `AgencyResponse`). Answers `404` when the caller has no agency so the
/// frontend can show the "Create Agency" onboarding CTA rather than the
/// generic load-error/retry screen (Issue #2343).
#[utoipa::path(
    get,
    path = "/api/v1/agencies/me",
    tag = "Agencies",
    responses(
        (status = 200, description = "The caller's agency", body = RealityAgency),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Caller is not a member of any agency")
    ),
    security(("session_token" = []))
)]
pub async fn get_my_agency(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<RealityAgency>, (axum::http::StatusCode, String)> {
    let agency = state
        .reality_portal_repo
        .get_agency_for_user(principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get my agency", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "You are not a member of any agency".to_string(),
            )
        })?;

    Ok(Json(agency))
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
        .map_err(|e| crate::util::errors::db_error("get agency", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Agency not found".to_string(),
            )
        })?;

    Ok(Json(AgencyResponse { agency }))
}

/// Update agency.
///
/// SECURITY (H2, round-9 audit): requires an authenticated principal who
/// is an active member of the target agency. Previously this handler had
/// no auth extractor, allowing anyone on the internet to overwrite an
/// agency's name / slug / contact info via `PUT /api/v1/agencies/{id}`.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = UpdateRealityAgency,
    responses(
        (status = 200, description = "Agency updated", body = AgencyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency"),
        (status = 404, description = "Agency not found")
    ),
    security(("session_token" = []))
)]
pub async fn update_agency(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateRealityAgency>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    // SECURITY (H2): membership check via the shared helper in
    // `super::agency_imports` — single source of truth for "is this user
    // allowed to mutate this agency's data?".
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;
    super::agency_imports::check_agency_membership(&mut conn, id, principal.user_id).await?;
    drop(conn);

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
///
/// SECURITY (H2, round-9 audit): the *wired* `PUT /api/v1/agencies/{id}/branding`
/// route in `main.rs` is `routes::agency_branding::update_branding`, which
/// already authenticates and checks membership. This handler is currently
/// dead code (not mounted) but still surfaces in OpenAPI; we keep the auth
/// extractor here as defense-in-depth in case routing is ever changed.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}/branding",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = UpdateAgencyBranding,
    responses(
        (status = 200, description = "Branding updated", body = AgencyResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency"),
        (status = 404, description = "Agency not found")
    ),
    security(("session_token" = []))
)]
pub async fn update_branding(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateAgencyBranding>,
) -> Result<Json<AgencyResponse>, (axum::http::StatusCode, String)> {
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;
    super::agency_imports::check_agency_membership(&mut conn, id, principal.user_id).await?;
    drop(conn);

    let agency = state
        .reality_portal_repo
        .update_agency_branding(id, data)
        .await
        .map_err(|e| crate::util::errors::db_error("update branding", e))?;

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
        .map_err(|e| crate::util::errors::db_error("get members", e))?;

    let total = members.len() as i64;

    Ok(Json(MembersResponse { members, total }))
}

/// Create agency invitation.
///
/// SECURITY (invite-authz audit): requires an authenticated principal who is
/// an active member of the target agency. Previously this handler called the
/// repo directly with no membership gate, so ANY authenticated user could
/// invite themselves (or anyone) into ANY agency via
/// `POST /api/v1/agencies/{id}/invitations` — a self-service privilege
/// escalation. We now apply the same `check_agency_membership` gate the
/// sibling mutators (`update_agency`, `update_branding`) use: `404` when the
/// agency doesn't exist and `403` for a non-member (never leaks cross-agency
/// membership).
#[utoipa::path(
    post,
    path = "/api/v1/agencies/{id}/invitations",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = CreateAgencyInvitation,
    responses(
        (status = 201, description = "Invitation created", body = RealityAgencyInvitation),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency"),
        (status = 404, description = "Agency not found")
    ),
    security(("session_token" = []))
)]
pub async fn create_invitation(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(agency_id): Path<Uuid>,
    Json(data): Json<CreateAgencyInvitation>,
) -> Result<Json<RealityAgencyInvitation>, (axum::http::StatusCode, String)> {
    // SECURITY (invite-authz): gate on active membership of the target agency
    // via the shared helper — single source of truth for "is this user allowed
    // to mutate this agency's data?". Returns 404 (unknown agency) / 403
    // (non-member) before any invitation is created.
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;
    super::agency_imports::check_agency_membership(&mut conn, agency_id, principal.user_id).await?;
    drop(conn);

    let invitation = state
        .reality_portal_repo
        .create_invitation(agency_id, principal.user_id, data)
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
                crate::util::errors::db_error("create invitation", e)
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
    principal: RequestPrincipal,
    Path(token): Path<String>,
) -> Result<Json<RealityAgencyMember>, (axum::http::StatusCode, String)> {
    let member = state
        .reality_portal_repo
        .accept_invitation(&token, principal.user_id)
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
                crate::util::errors::db_error("accept invitation", e)
            }
        })?;

    Ok(Json(member))
}

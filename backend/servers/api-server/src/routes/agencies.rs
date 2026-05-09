//! Agency routes (Epic 17: Agency & Realtor Management).

use crate::state::AppState;
use api_core::extractors::TenantExtractor;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use db::models::{
    member_role, AcceptInvitation, Agency, AgencyBranding, AgencyMember, AgencyMembersResponse,
    CreateAgency, InviteMember, ListingEditHistory, ListingImportJob, Locale, UpdateAgency,
    UpdateMemberRole,
};
use uuid::Uuid;

/// Create agencies router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Agency CRUD
        .route("/", post(create_agency))
        .route("/{id}", get(get_agency))
        .route("/{id}", put(update_agency))
        .route("/{id}/branding", put(update_branding))
        // Members
        .route("/{id}/members", get(list_members))
        .route("/{id}/members/invite", post(invite_member))
        .route("/{id}/members/{user_id}/role", put(update_member_role))
        .route("/{id}/members/{user_id}", delete(remove_member))
        .route(
            "/{id}/members/{user_id}/reassign/{to_user_id}",
            post(reassign_listings),
        )
        // Invitations
        .route("/invitations/accept", post(accept_invitation))
        // Listings
        .route(
            "/{id}/listings/{listing_id}/visibility",
            put(update_visibility),
        )
        .route(
            "/{id}/listings/{listing_id}/history",
            get(get_listing_history),
        )
        // Import
        .route("/{id}/import", post(create_import_job))
        .route("/{id}/import/{job_id}", get(get_import_job))
        .route("/{id}/import", get(list_import_jobs))
}

/// Create a new agency.
#[utoipa::path(
    post,
    path = "/api/v1/agencies",
    tag = "Agencies",
    request_body = CreateAgency,
    responses(
        (status = 201, description = "Agency created", body = Agency),
        (status = 400, description = "Invalid data"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_agency(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<CreateAgency>,
) -> Result<Json<Agency>, (axum::http::StatusCode, String)> {
    let agency = state
        .agency_repo
        .create_agency(data, tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create agency: {}", e),
            )
        })?;

    Ok(Json(agency))
}

/// Get agency by ID.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "Agency details", body = Agency),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn get_agency(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Agency>, (axum::http::StatusCode, String)> {
    let agency = state.agency_repo.find_by_id(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get agency: {}", e),
        )
    })?;

    match agency {
        Some(a) => Ok(Json(a)),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Agency not found".to_string(),
        )),
    }
}

/// Update agency.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = UpdateAgency,
    responses(
        (status = 200, description = "Agency updated", body = Agency),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn update_agency(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateAgency>,
) -> Result<Json<Agency>, (axum::http::StatusCode, String)> {
    // Verify user is agency admin
    verify_agency_admin(&state, id, tenant.user_id).await?;

    let agency = state
        .agency_repo
        .update_agency(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update agency: {}", e),
            )
        })?;

    Ok(Json(agency))
}

/// Update agency branding.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}/branding",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = AgencyBranding,
    responses(
        (status = 200, description = "Branding updated", body = Agency),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn update_branding(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<AgencyBranding>,
) -> Result<Json<Agency>, (axum::http::StatusCode, String)> {
    // Verify user is agency admin
    verify_agency_admin(&state, id, tenant.user_id).await?;

    let agency = state
        .agency_repo
        .update_branding(id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update branding: {}", e),
            )
        })?;

    Ok(Json(agency))
}

/// List agency members.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/members",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "List of members", body = AgencyMembersResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_members(
    State(state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<AgencyMembersResponse>, (axum::http::StatusCode, String)> {
    let members = state.agency_repo.get_members(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get members: {}", e),
        )
    })?;

    let total = members.len() as i64;

    Ok(Json(AgencyMembersResponse { members, total }))
}

/// Invite a member to the agency.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/{id}/members/invite",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = InviteMember,
    responses(
        (status = 201, description = "Invitation sent"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn invite_member(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<InviteMember>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Verify user is agency admin
    verify_agency_admin(&state, id, tenant.user_id).await?;

    // Get the agency name for the email
    let agency = state
        .agency_repo
        .find_by_id(id)
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

    // Get inviter's name for the email
    let inviter = state
        .user_repo
        .find_by_id(tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get user: {}", e),
            )
        })?;

    let inviter_name = inviter
        .map(|u| u.name)
        .unwrap_or_else(|| "An administrator".to_string());

    let invitation = state
        .agency_repo
        .create_invitation(id, data.clone(), tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create invitation: {}", e),
            )
        })?;

    // Send invitation email
    let email_subject = format!("Invitation to join {} on Property Management", agency.name);
    let email_body = format!(
        "{} has invited you to join {} as a {}.\n\nClick the link below to accept the invitation:\n\n{}/accept-agency-invitation?token={}\n\nThis invitation expires in 7 days.",
        inviter_name,
        agency.name,
        data.role,
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
        invitation.token
    );

    if let Err(e) = state
        .email_service
        .send_notification_email(
            &data.email,
            &data.email,
            &email_subject,
            &email_body,
            &Locale::English,
        )
        .await
    {
        tracing::warn!(
            error = %e,
            email = %data.email,
            agency_id = %id,
            "Failed to send agency invitation email"
        );
        // Don't fail the request if email sending fails - the invitation is still created
    }

    Ok(axum::http::StatusCode::CREATED)
}

/// Accept an invitation.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/invitations/accept",
    tag = "Agencies",
    request_body = AcceptInvitation,
    responses(
        (status = 200, description = "Invitation accepted", body = AgencyMember),
        (status = 400, description = "Invalid or expired token"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn accept_invitation(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<AcceptInvitation>,
) -> Result<Json<AgencyMember>, (axum::http::StatusCode, String)> {
    let member = state
        .agency_repo
        .accept_invitation(&data.token, tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Failed to accept invitation: {}", e),
            )
        })?;

    Ok(Json(member))
}

/// Update member role.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}/members/{user_id}/role",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    request_body = UpdateMemberRole,
    responses(
        (status = 200, description = "Role updated", body = AgencyMember),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn update_member_role(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<UpdateMemberRole>,
) -> Result<Json<AgencyMember>, (axum::http::StatusCode, String)> {
    // Verify caller is agency admin
    verify_agency_admin(&state, id, tenant.user_id).await?;

    let member = state
        .agency_repo
        .update_member_role(id, user_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update role: {}", e),
            )
        })?;

    Ok(Json(member))
}

/// Remove member from agency.
#[utoipa::path(
    delete,
    path = "/api/v1/agencies/{id}/members/{user_id}",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("user_id" = Uuid, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn remove_member(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Verify caller is agency admin
    verify_agency_admin(&state, id, tenant.user_id).await?;

    let removed = state
        .agency_repo
        .remove_member(id, user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to remove member: {}", e),
            )
        })?;

    if !removed {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Member not found".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Reassign listings from one member to another.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/{id}/members/{user_id}/reassign/{to_user_id}",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("user_id" = Uuid, Path, description = "Source user ID"),
        ("to_user_id" = Uuid, Path, description = "Target user ID")
    ),
    responses(
        (status = 200, description = "Listings reassigned"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn reassign_listings(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path((id, user_id, to_user_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    // Verify caller is agency admin
    verify_agency_admin(&state, id, tenant.user_id).await?;

    let count = state
        .agency_repo
        .reassign_listings(id, user_id, to_user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to reassign listings: {}", e),
            )
        })?;

    Ok(Json(serde_json::json!({ "reassigned_count": count })))
}

/// Update listing visibility.
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}/listings/{listing_id}/visibility",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("listing_id" = Uuid, Path, description = "Listing ID")
    ),
    request_body = db::models::UpdateListingVisibility,
    responses(
        (status = 200, description = "Visibility updated"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn update_visibility(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path((id, listing_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<db::models::UpdateListingVisibility>,
) -> Result<Json<db::models::AgencyListing>, (axum::http::StatusCode, String)> {
    // Verify caller has access to listing
    verify_listing_access(&state, id, listing_id, tenant.user_id).await?;

    let listing = state
        .agency_repo
        .update_listing_visibility(listing_id, &data.visibility)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update visibility: {}", e),
            )
        })?;

    Ok(Json(listing))
}

/// Get listing edit history.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/listings/{listing_id}/history",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("listing_id" = Uuid, Path, description = "Listing ID")
    ),
    responses(
        (status = 200, description = "Edit history", body = Vec<ListingEditHistory>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_listing_history(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path((id, listing_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<ListingEditHistory>>, (axum::http::StatusCode, String)> {
    // Verify caller has access to listing
    verify_listing_access(&state, id, listing_id, tenant.user_id).await?;

    let history = state
        .agency_repo
        .get_listing_history(listing_id, 50)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get history: {}", e),
            )
        })?;

    Ok(Json(history))
}

/// Create import job.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/{id}/import",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = db::models::CreateImportJob,
    responses(
        (status = 201, description = "Import job created", body = ListingImportJob),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_import_job(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<db::models::CreateImportJob>,
) -> Result<Json<ListingImportJob>, (axum::http::StatusCode, String)> {
    let job = state
        .agency_repo
        .create_import_job(id, tenant.user_id, &data.source)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create import job: {}", e),
            )
        })?;

    Ok(Json(job))
}

/// Get import job.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/import/{job_id}",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("job_id" = Uuid, Path, description = "Import job ID")
    ),
    responses(
        (status = 200, description = "Import job details", body = ListingImportJob),
        (status = 404, description = "Job not found")
    )
)]
pub async fn get_import_job(
    State(state): State<AppState>,
    Path((_id, job_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ListingImportJob>, (axum::http::StatusCode, String)> {
    let job = state
        .agency_repo
        .get_import_job(job_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get import job: {}", e),
            )
        })?;

    match job {
        Some(j) => Ok(Json(j)),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Import job not found".to_string(),
        )),
    }
}

/// List import jobs.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/import",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "List of import jobs", body = Vec<ListingImportJob>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_import_jobs(
    State(state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ListingImportJob>>, (axum::http::StatusCode, String)> {
    let jobs = state
        .agency_repo
        .get_import_jobs(id, 20)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list import jobs: {}", e),
            )
        })?;

    Ok(Json(jobs))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Verify that the user is an admin of the specified agency.
/// Returns Ok(()) if authorized, Err with 403 status if not.
async fn verify_agency_admin(
    state: &AppState,
    agency_id: Uuid,
    user_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    let member = state
        .agency_repo
        .get_member(agency_id, user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check membership: {}", e),
            )
        })?;

    match member {
        Some(m) if m.is_active && m.role == member_role::ADMIN => Ok(()),
        Some(_) => Err((
            axum::http::StatusCode::FORBIDDEN,
            "Only agency admins can perform this action".to_string(),
        )),
        None => Err((
            axum::http::StatusCode::FORBIDDEN,
            "You are not a member of this agency".to_string(),
        )),
    }
}

/// Verify that the user has access to a listing (is a member of the agency or owns the listing).
async fn verify_listing_access(
    state: &AppState,
    agency_id: Uuid,
    listing_id: Uuid,
    user_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    // First check if user is an agency member
    let member = state
        .agency_repo
        .get_member(agency_id, user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check membership: {}", e),
            )
        })?;

    if member.is_none() || !member.as_ref().unwrap().is_active {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "You are not a member of this agency".to_string(),
        ));
    }

    // Check listing access based on visibility and role
    let can_access = state
        .agency_repo
        .can_access_listing(listing_id, user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check listing access: {}", e),
            )
        })?;

    if !can_access {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "You do not have access to this listing".to_string(),
        ));
    }

    Ok(())
}

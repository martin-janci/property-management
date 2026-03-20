//! Delegation routes (Epic 3, Story 3.4).
//!
//! Implements ownership delegation management - allowing unit owners to delegate
//! voting, document access, and other rights to other users.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::delegation::{CreateDelegation, Delegation, DelegationSummary};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Create delegations router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Delegation management
        .route("/", post(create_delegation))
        .route("/", get(list_my_delegations))
        .route("/received", get(list_received_delegations))
        .route("/{id}", get(get_delegation))
        .route("/{id}/accept", post(accept_delegation))
        .route("/{id}/decline", post(decline_delegation))
        .route("/{id}/revoke", delete(revoke_delegation))
        // Check delegation for a unit
        .route("/check/{unit_id}/{scope}", get(check_delegation))
}

// ==================== Request/Response Types ====================

/// Create delegation request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDelegationRequest {
    /// User ID of the delegate (user to receive delegation)
    pub delegate_user_id: Uuid,
    /// Unit ID to delegate rights for (optional - applies to all units if omitted)
    pub unit_id: Option<Uuid>,
    /// Delegation scopes (all, voting, documents, faults, financial)
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// When the delegation starts (defaults to today)
    pub start_date: Option<NaiveDate>,
    /// When the delegation expires (optional)
    pub end_date: Option<NaiveDate>,
}

fn default_scopes() -> Vec<String> {
    vec!["all".to_string()]
}

/// Delegation response.
#[derive(Debug, Serialize, ToSchema)]
pub struct DelegationResponse {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub delegate_user_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub status: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub accepted_at: Option<String>,
    pub revoked_at: Option<String>,
    pub revoked_reason: Option<String>,
    pub created_at: String,
}

impl From<Delegation> for DelegationResponse {
    fn from(d: Delegation) -> Self {
        Self {
            id: d.id,
            owner_user_id: d.owner_user_id,
            delegate_user_id: d.delegate_user_id,
            unit_id: d.unit_id,
            scopes: d.scopes,
            status: d.status,
            start_date: d.start_date.to_string(),
            end_date: d.end_date.map(|d| d.to_string()),
            accepted_at: d.accepted_at.map(|dt| dt.to_rfc3339()),
            revoked_at: d.revoked_at.map(|dt| dt.to_rfc3339()),
            revoked_reason: d.revoked_reason,
            created_at: d.created_at.to_rfc3339(),
        }
    }
}

/// Delegation summary response.
#[derive(Debug, Serialize, ToSchema)]
pub struct DelegationSummaryResponse {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub delegate_user_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub scopes: Vec<String>,
    pub status: String,
}

impl From<DelegationSummary> for DelegationSummaryResponse {
    fn from(d: DelegationSummary) -> Self {
        Self {
            id: d.id,
            owner_user_id: d.owner_user_id,
            delegate_user_id: d.delegate_user_id,
            unit_id: d.unit_id,
            scopes: d.scopes,
            status: d.status,
        }
    }
}

/// List delegations query parameters.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListDelegationsQuery {
    /// Filter by unit ID
    pub unit_id: Option<Uuid>,
    /// Filter by status
    pub status: Option<String>,
}

/// Check delegation response.
#[derive(Debug, Serialize, ToSchema)]
pub struct CheckDelegationResponse {
    pub has_delegation: bool,
}

// ==================== Handlers ====================

/// Create a new delegation (Story 3.4.1).
#[utoipa::path(
    post,
    path = "/api/v1/delegations",
    tag = "Delegations",
    request_body = CreateDelegationRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Delegation created", body = DelegationResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse)
    )
)]
pub async fn create_delegation(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<CreateDelegationRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate scopes
    let valid_scopes = ["all", "voting", "documents", "faults", "financial"];
    for scope in &req.scopes {
        if !valid_scopes.contains(&scope.as_str()) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_SCOPE",
                    format!("Invalid scope: {}. Must be one of: all, voting, documents, faults, financial", scope),
                )),
            ));
        }
    }

    if req.scopes.is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "MISSING_SCOPES",
                "At least one delegation scope is required",
            )),
        ));
    }

    // Verify delegate user exists
    let delegate_exists = match state.user_repo.find_by_id(req.delegate_user_id).await {
        Ok(user) => user.is_some(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to check delegate user");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            ));
        }
    };

    if !delegate_exists {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "USER_NOT_FOUND",
                "Delegate user not found",
            )),
        ));
    }

    // If unit_id is provided, verify user owns it
    if let Some(unit_id) = req.unit_id {
        let owners = match state
            .unit_repo
            .get_owners_rls(&mut **rls.conn(), unit_id)
            .await
        {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get unit owners");
                rls.release().await;
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Database error")),
                ));
            }
        };

        let is_owner = owners.iter().any(|o| o.user_id == auth.user_id);
        if !is_owner {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_OWNER",
                    "You must be an owner of the unit to create delegations",
                )),
            ));
        }
    }

    // Create delegation
    let create_data = CreateDelegation {
        delegate_user_id: req.delegate_user_id,
        unit_id: req.unit_id,
        scopes: req.scopes,
        start_date: req.start_date,
        end_date: req.end_date,
    };

    let delegation = match state
        .delegation_repo
        .create(auth.user_id, create_data)
        .await
    {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create delegation");
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to create delegation",
                )),
            ));
        }
    };

    rls.release().await;

    tracing::info!(
        delegation_id = %delegation.id,
        owner_id = %auth.user_id,
        delegate_id = %delegation.delegate_user_id,
        "Delegation created"
    );

    Ok((
        StatusCode::CREATED,
        Json(DelegationResponse::from(delegation)),
    ))
}

/// List delegations I created.
#[utoipa::path(
    get,
    path = "/api/v1/delegations",
    tag = "Delegations",
    params(ListDelegationsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of delegations", body = Vec<DelegationSummaryResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_my_delegations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListDelegationsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let delegations = state
        .delegation_repo
        .find_by_owner(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list delegations");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list delegations")),
            )
        })?;

    // Apply filters
    let filtered: Vec<DelegationSummaryResponse> = delegations
        .into_iter()
        .filter(|d| {
            if let Some(ref unit_id) = query.unit_id {
                if d.unit_id.as_ref() != Some(unit_id) {
                    return false;
                }
            }
            if let Some(ref status) = query.status {
                if &d.status != status {
                    return false;
                }
            }
            true
        })
        .map(DelegationSummaryResponse::from)
        .collect();

    Ok(Json(filtered))
}

/// List delegations I received.
#[utoipa::path(
    get,
    path = "/api/v1/delegations/received",
    tag = "Delegations",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of received delegations", body = Vec<DelegationSummaryResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_received_delegations(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let delegations = state
        .delegation_repo
        .find_by_delegate(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list received delegations");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list delegations")),
            )
        })?;

    let response: Vec<DelegationSummaryResponse> = delegations
        .into_iter()
        .map(DelegationSummaryResponse::from)
        .collect();

    Ok(Json(response))
}

/// Get delegation by ID.
#[utoipa::path(
    get,
    path = "/api/v1/delegations/{id}",
    tag = "Delegations",
    params(("id" = Uuid, Path, description = "Delegation ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Delegation found", body = DelegationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Delegation not found", body = ErrorResponse)
    )
)]
pub async fn get_delegation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let delegation = state
        .delegation_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Delegation not found")),
            )
        })?;

    // Check access - must be owner or delegate
    if delegation.owner_user_id != auth.user_id && delegation.delegate_user_id != auth.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "You are not authorized to view this delegation",
            )),
        ));
    }

    Ok(Json(DelegationResponse::from(delegation)))
}

/// Accept a delegation invitation (Story 3.4.2).
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/accept",
    tag = "Delegations",
    params(("id" = Uuid, Path, description = "Delegation ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Delegation accepted", body = DelegationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Delegation not found", body = ErrorResponse)
    )
)]
pub async fn accept_delegation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify the delegation exists and is for this user
    let existing = state
        .delegation_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Delegation not found")),
            )
        })?;

    if existing.delegate_user_id != auth.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "This delegation was not assigned to you",
            )),
        ));
    }

    let delegation = state
        .delegation_repo
        .accept(id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to accept delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to accept delegation",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Delegation not found or already processed",
                )),
            )
        })?;

    tracing::info!(
        delegation_id = %id,
        delegate_id = %auth.user_id,
        "Delegation accepted"
    );

    Ok(Json(DelegationResponse::from(delegation)))
}

/// Decline a delegation invitation.
#[utoipa::path(
    post,
    path = "/api/v1/delegations/{id}/decline",
    tag = "Delegations",
    params(("id" = Uuid, Path, description = "Delegation ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Delegation declined", body = DelegationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Delegation not found", body = ErrorResponse)
    )
)]
pub async fn decline_delegation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Verify the delegation exists and is for this user
    let existing = state
        .delegation_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Delegation not found")),
            )
        })?;

    if existing.delegate_user_id != auth.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "This delegation was not assigned to you",
            )),
        ));
    }

    let delegation = state
        .delegation_repo
        .decline(id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to decline delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to decline delegation",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Delegation not found or already processed",
                )),
            )
        })?;

    tracing::info!(
        delegation_id = %id,
        user_id = %auth.user_id,
        "Delegation declined"
    );

    Ok(Json(DelegationResponse::from(delegation)))
}

/// Revoke an active delegation (Story 3.4.3).
#[utoipa::path(
    delete,
    path = "/api/v1/delegations/{id}/revoke",
    tag = "Delegations",
    params(("id" = Uuid, Path, description = "Delegation ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Delegation revoked", body = DelegationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
        (status = 404, description = "Delegation not found", body = ErrorResponse)
    )
)]
pub async fn revoke_delegation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Check delegation exists and user is the owner
    let existing = state
        .delegation_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Delegation not found")),
            )
        })?;

    if existing.owner_user_id != auth.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_AUTHORIZED",
                "Only the owner can revoke a delegation",
            )),
        ));
    }

    let delegation = state
        .delegation_repo
        .revoke(id, auth.user_id, Some("Revoked by owner"))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to revoke delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to revoke delegation",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Delegation not found or already revoked",
                )),
            )
        })?;

    tracing::info!(
        delegation_id = %id,
        owner_id = %auth.user_id,
        "Delegation revoked"
    );

    Ok(Json(DelegationResponse::from(delegation)))
}

/// Check if user has delegation for a unit and scope (Story 3.4.4).
#[utoipa::path(
    get,
    path = "/api/v1/delegations/check/{unit_id}/{scope}",
    tag = "Delegations",
    params(
        ("unit_id" = Uuid, Path, description = "Unit ID"),
        ("scope" = String, Path, description = "Scope to check (all, voting, documents, faults, financial)")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Delegation check result", body = CheckDelegationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn check_delegation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((unit_id, scope)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let has_delegation = state
        .delegation_repo
        .has_delegation(auth.user_id, unit_id, &scope)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to check delegation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    Ok(Json(CheckDelegationResponse { has_delegation }))
}

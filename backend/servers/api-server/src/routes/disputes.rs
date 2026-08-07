//! Dispute resolution routes (Epic 77).
//!
//! Provides API endpoints for filing, mediating, tracking, and enforcing
//! dispute resolutions between parties.

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use db::models::{
    ActionItem, AddEvidence, CompleteActionItem, CreateActionItem, CreateEscalation, Dispute,
    DisputeActivity, DisputeEvidence, DisputeKpis, DisputeParty, DisputeQuery, DisputeResolution,
    DisputeStatistics, DisputeSummary, DisputeWithDetails, Escalation, FileDispute, MediationCase,
    MediationSession, PartyActionsDashboard, PartySubmission, ProposeResolution,
    RecordSessionNotes, ResolutionVote, ResolutionWithVotes, ResolveDispute, ResolveEscalation,
    ScheduleSession, SessionAttendance, SubmitResponse, UpdateDisputeStatus, UpdateMediationNotes,
    VoteOnResolution,
};
use db::repositories::{UpdateAttendanceData, UpdateSessionData};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

/// Create disputes router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Disputes (Story 77.1)
        .route("/", post(file_dispute))
        .route("/", get(list_disputes))
        .route("/statistics", get(get_statistics))
        .route("/kpis", get(get_kpis))
        .route("/{id}", get(get_dispute))
        .route("/{id}", patch(update_dispute_status))
        .route("/{id}", delete(withdraw_dispute))
        .route("/{id}/resolve", patch(resolve_dispute))
        .route("/{id}/mediation-notes", patch(update_mediation_notes))
        .route("/{id}/parties", get(list_parties))
        .route("/{id}/parties", post(add_party))
        .route("/{id}/evidence", get(list_evidence))
        .route("/{id}/evidence", post(add_evidence))
        .route("/{id}/evidence/{evidence_id}", delete(delete_evidence))
        .route("/{id}/activities", get(list_activities))
        // Mediation (Story 77.2)
        .route("/{id}/sessions", get(list_sessions))
        .route("/{id}/sessions", post(schedule_session))
        .route("/{id}/sessions/{session_id}", get(get_session))
        .route("/{id}/sessions/{session_id}", patch(update_session))
        .route("/{id}/sessions/{session_id}/cancel", post(cancel_session))
        .route(
            "/{id}/sessions/{session_id}/attendance",
            get(get_attendance),
        )
        .route(
            "/{id}/sessions/{session_id}/attendance/{party_id}",
            patch(update_attendance),
        )
        .route("/{id}/sessions/{session_id}/notes", post(record_notes))
        .route("/{id}/submissions", get(list_submissions))
        .route("/{id}/submissions", post(submit_response))
        .route("/{id}/mediation-case", get(get_mediation_case))
        // Resolution Tracking (Story 77.3)
        .route("/{id}/resolutions", get(list_resolutions))
        .route("/{id}/resolutions", post(propose_resolution))
        .route("/{id}/resolutions/{resolution_id}", get(get_resolution))
        .route(
            "/{id}/resolutions/{resolution_id}/vote",
            post(vote_on_resolution),
        )
        .route(
            "/{id}/resolutions/{resolution_id}/accept",
            post(accept_resolution),
        )
        .route(
            "/{id}/resolutions/{resolution_id}/implement",
            post(implement_resolution),
        )
        // Enforcement (Story 77.4)
        .route("/{id}/actions", get(list_action_items))
        .route("/{id}/actions", post(create_action_item))
        .route("/{id}/actions/{action_id}", get(get_action_item))
        .route("/{id}/actions/{action_id}", patch(update_action_item))
        .route(
            "/{id}/actions/{action_id}/complete",
            post(complete_action_item),
        )
        .route(
            "/{id}/actions/{action_id}/remind",
            post(send_action_reminder),
        )
        .route("/{id}/escalations", get(list_escalations))
        .route("/{id}/escalations", post(create_escalation))
        .route(
            "/{id}/escalations/{escalation_id}/resolve",
            post(resolve_escalation),
        )
        .route("/my-actions", get(get_my_actions))
        .route("/overdue-actions", get(list_overdue_actions))
}

// =============================================================================
// Shared helpers
// =============================================================================

/// Derive the caller's organization from the verified JWT, or 403 if absent.
///
/// Issue #2441: dispute sub-resource handlers must scope by the JWT tenant
/// (never a path- or body-supplied org) so they cannot cross tenants.
fn require_org(user: &AuthUser) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    user.tenant_id.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "No organization context")),
        )
    })
}

/// Map a repository error from an org-scoped sub-resource call to an HTTP
/// response. `NotFound` (dispute absent or owned by another org) becomes 404 so
/// the response is not a cross-tenant existence oracle; everything else is 500.
fn map_dispute_err(
    err: common::errors::AppError,
    log_ctx: &str,
) -> (StatusCode, Json<ErrorResponse>) {
    match err {
        common::errors::AppError::NotFound(_) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Dispute not found")),
        ),
        e => {
            tracing::error!("{}: {:?}", log_ctx, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", log_ctx)),
            )
        }
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Dispute KPI reporting window (`[window_start, window_end)`), keyed on filing
/// time. The organization is taken from the JWT tenant, never this query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct KpisQuery {
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
}

/// File dispute request.
#[derive(Debug, Deserialize)]
pub struct FileDisputeRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: FileDispute,
}

/// List disputes query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDisputesQuery {
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub filed_by: Option<Uuid>,
    pub assigned_to: Option<Uuid>,
    pub search: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListDisputesQuery> for DisputeQuery {
    fn from(q: &ListDisputesQuery) -> Self {
        DisputeQuery {
            building_id: q.building_id,
            category: q.category.clone(),
            status: q.status.clone(),
            priority: q.priority.clone(),
            filed_by: q.filed_by,
            assigned_to: q.assigned_to,
            search: q.search.clone(),
            from_date: q.from_date,
            to_date: q.to_date,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Add party request.
#[derive(Debug, Deserialize)]
pub struct AddPartyRequest {
    pub user_id: Uuid,
    pub role: String,
}

/// Add evidence request.
#[derive(Debug, Deserialize)]
pub struct AddEvidenceRequest {
    #[serde(flatten)]
    pub data: AddEvidence,
}

/// Update dispute status request.
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub reason: Option<String>,
}

/// Schedule session request.
#[derive(Debug, Deserialize)]
pub struct ScheduleSessionRequest {
    #[serde(flatten)]
    pub data: ScheduleSession,
}

/// Update session request.
#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub scheduled_at: Option<DateTime<Utc>>,
    pub duration_minutes: Option<i32>,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub status: Option<String>,
}

/// Update attendance request.
#[derive(Debug, Deserialize)]
pub struct UpdateAttendanceRequest {
    pub confirmed: Option<bool>,
    pub attended: Option<bool>,
    pub notes: Option<String>,
}

/// Record notes request.
#[derive(Debug, Deserialize)]
pub struct RecordNotesRequest {
    pub notes: String,
    pub outcome: Option<String>,
}

/// Submit response request.
#[derive(Debug, Deserialize)]
pub struct SubmitResponseRequest {
    pub submission_type: String,
    pub content: String,
    pub is_visible_to_all: bool,
}

/// Propose resolution request.
#[derive(Debug, Deserialize)]
pub struct ProposeResolutionRequest {
    #[serde(flatten)]
    pub data: ProposeResolution,
}

/// Vote on resolution request.
#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub accepted: bool,
    pub comments: Option<String>,
}

/// Create action item request.
#[derive(Debug, Deserialize)]
pub struct CreateActionRequest {
    #[serde(flatten)]
    pub data: CreateActionItem,
}

/// Update action item request.
#[derive(Debug, Deserialize)]
pub struct UpdateActionRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub status: Option<String>,
}

/// Complete action request.
#[derive(Debug, Deserialize)]
pub struct CompleteActionRequest {
    pub completion_notes: Option<String>,
}

/// Create escalation request.
#[derive(Debug, Deserialize)]
pub struct CreateEscalationRequest {
    pub action_item_id: Option<Uuid>,
    pub reason: String,
    pub severity: String,
    pub escalated_to: Option<Uuid>,
}

/// Resolve escalation request.
#[derive(Debug, Deserialize)]
pub struct ResolveEscalationRequest {
    pub resolution_notes: String,
}

/// Resolve dispute request.
#[derive(Debug, Deserialize)]
pub struct ResolveDisputeRequest {
    pub resolution_notes: String,
}

/// Update mediation notes request.
#[derive(Debug, Deserialize)]
pub struct UpdateMediationNotesRequest {
    pub notes: String,
}

/// Pagination query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// =============================================================================
// Story 77.1: Dispute Filing
// =============================================================================

/// File a new dispute.
async fn file_dispute(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<FileDisputeRequest>,
) -> Result<(StatusCode, Json<Dispute>), (StatusCode, Json<ErrorResponse>)> {
    let mut data = payload.data;
    data.filed_by = user.user_id;

    state
        .dispute_repo
        .file_dispute(payload.organization_id, data)
        .await
        .map(|d| (StatusCode::CREATED, Json(d)))
        .map_err(|e| {
            tracing::error!("Failed to file dispute: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to file dispute")),
            )
        })
}

/// List disputes for an organization.
async fn list_disputes(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListDisputesQuery>,
) -> Result<Json<Vec<DisputeSummary>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list disputes: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list disputes")),
            )
        })
}

/// Get dispute statistics.
async fn get_statistics(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<DisputeStatistics>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .get_statistics(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get statistics")),
            )
        })
}

/// Get dispute lifecycle KPIs (funnel + time-to-resolution) for a cohort.
///
/// Issue #2562 (follow-up to PR #2550 / #2533): exposes the SQL-backed
/// `DisputeRepository::get_dispute_kpis` — previously orphaned — through a thin
/// authenticated, org-scoped reporting endpoint. Like the other dispute
/// sub-resource handlers the organization is derived from the caller's JWT
/// tenant (never a query- or body-supplied org), so the KPIs cannot cross
/// tenants. This is a reporting query, not hot-path; callers should
/// cache/schedule it.
async fn get_kpis(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<KpisQuery>,
) -> Result<Json<DisputeKpis>, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = require_org(&user)?;

    // Issue #2575: validate the reporting window before touching the repo.
    // `get_dispute_kpis` filters the cohort on `window_start <= filed_at <
    // window_end`, so an inverted or empty window (`window_start >=
    // window_end`) silently yields a degenerate zero-count `DisputeKpis` with
    // a 200 — a caller cannot distinguish "no disputes in window" from
    // "I inverted my bounds". Reject it up front, mirroring how the sibling
    // mutating handlers surface `VALIDATION_ERROR` (see `resolve_dispute` /
    // `update_mediation_notes`). Cheap guard, clearer contract.
    if query.window_start >= query.window_end {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new(
                "VALIDATION_ERROR",
                "window_start must be strictly before window_end",
            )),
        ));
    }

    state
        .dispute_repo
        .get_dispute_kpis(organization_id, query.window_start, query.window_end)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get dispute KPIs: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get dispute KPIs")),
            )
        })
}

/// Get a specific dispute by ID.
///
/// Issue #760 / #834:
///   * The lookup is scoped to the caller's JWT tenant, so a dispute owned by
///     another org returns 404 (no cross-tenant IDOR).
///   * `mediation_notes` is confidential — it is only returned to a
///     manager/admin or the assigned mediator. For every other party it is
///     redacted to `None`, even though they may read the rest of the dispute.
async fn get_dispute(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DisputeWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    // Derive the org from the verified JWT so the read cannot cross tenants.
    let organization_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "No organization context")),
        )
    })?;

    let mut details = state
        .dispute_repo
        .find_by_id_with_details_for_org(id, organization_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get dispute: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get dispute")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Dispute not found")),
            )
        })?;

    // Confidentiality gate for mediation_notes: visible to managers/admins and
    // the assigned mediator only. Writes are already restricted in
    // `update_mediation_notes`; without this read gate the notes leaked to any
    // org member that could read the dispute.
    let is_manager = user
        .role
        .as_ref()
        .is_some_and(|r| r.is_manager() || r.is_admin());

    if !is_manager && details.dispute.mediation_notes.is_some() {
        let is_mediator = state
            .dispute_repo
            .find_party_by_user(id, user.user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to verify mediator access: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Failed to get dispute")),
                )
            })?
            .is_some_and(|p| p.role == db::models::disputes::party_role::MEDIATOR);

        if !is_mediator {
            details.dispute.mediation_notes = None;
        }
    }

    Ok(Json(details))
}

/// Update dispute status — enforces the state machine.
///
/// Returns 422 Unprocessable Entity when the requested transition is not
/// permitted from the current status (`filed` → `under_review` → `resolved`).
async fn update_dispute_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateStatusRequest>,
) -> Result<Json<Dispute>, (StatusCode, Json<ErrorResponse>)> {
    if !user.role.as_ref().is_some_and(|r| r.is_manager()) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "Insufficient role")),
        ));
    }
    // Issue #520: derive org from the authenticated JWT, not from the request
    // body, so a caller cannot bypass the tenancy guard by supplying a
    // different org_id in the payload.
    let organization_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "No organization context")),
        )
    })?;
    let result = state
        .dispute_repo
        .update_status(UpdateDisputeStatus {
            dispute_id: id,
            organization_id,
            status: data.status,
            reason: data.reason,
            updated_by: user.user_id,
        })
        .await;
    match result {
        Ok(d) => Ok(Json(d)),
        Err(common::errors::AppError::BadRequest(msg)) => {
            tracing::warn!("Invalid dispute status transition: {}", msg);
            Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse::new(
                    "INVALID_TRANSITION",
                    "Invalid status transition",
                )),
            ))
        }
        // Issue #520: a `NotFound` here can mean either "no such dispute"
        // or "dispute belongs to a different org" — surface both as 404
        // so the response shape is not an existence oracle for
        // cross-tenant probes.
        Err(common::errors::AppError::NotFound(_)) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Dispute not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to update dispute status: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update dispute status",
                )),
            ))
        }
    }
}

/// Resolve a dispute — sets status to `resolved`, records `resolved_at` and `resolution_notes`.
///
/// Only managers or admins may call this endpoint.
/// Returns 422 Unprocessable Entity when the current dispute status does not permit a transition
/// to `resolved` (e.g. `filed`, already `resolved`, `withdrawn`, or `closed`).
async fn resolve_dispute(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ResolveDisputeRequest>,
) -> Result<Json<Dispute>, (StatusCode, Json<ErrorResponse>)> {
    // RequireCapability is not used here: the mediator path in update_mediation_notes
    // already requires a DB lookup that cannot be expressed as a static capability,
    // and keeping both handlers consistent avoids a mixed RBAC pattern in the same
    // router.  The manual check below enforces the same policy.
    if !user
        .role
        .as_ref()
        .is_some_and(|r| r.is_manager() || r.is_admin())
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "Insufficient role")),
        ));
    }

    // Guard: resolution_notes must not be blank.
    if data.resolution_notes.trim().is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new(
                "VALIDATION_ERROR",
                "resolution_notes must not be empty",
            )),
        ));
    }

    // Require an organization context from the JWT so we can scope the UPDATE.
    let organization_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "No organization context")),
        )
    })?;

    let result = state
        .dispute_repo
        .resolve_dispute(ResolveDispute {
            dispute_id: id,
            resolution_notes: data.resolution_notes,
            resolved_by: user.user_id,
            organization_id,
        })
        .await;

    match result {
        Ok(dispute) => Ok(Json(dispute)),
        Err(common::errors::AppError::BadRequest(msg)) => {
            tracing::warn!("Invalid dispute resolve attempt: {}", msg);
            Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ErrorResponse::new(
                    "INVALID_TRANSITION",
                    "Dispute cannot be resolved from its current status",
                )),
            ))
        }
        Err(common::errors::AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", msg.as_str())),
        )),
        Err(e) => {
            tracing::error!("Failed to resolve dispute: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to resolve dispute")),
            ))
        }
    }
}

/// Update mediation notes on a dispute.
///
/// Accessible by the assigned mediator or any manager/admin.
/// Does not change the dispute status.
async fn update_mediation_notes(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateMediationNotesRequest>,
) -> Result<Json<Dispute>, (StatusCode, Json<ErrorResponse>)> {
    // Guard: mediation notes must not be blank.
    if data.notes.trim().is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new(
                "VALIDATION_ERROR",
                "notes must not be empty",
            )),
        ));
    }

    // RequireCapability is not used here because access involves two distinct
    // paths: static role check (manager/admin) AND a dynamic DB lookup to
    // verify mediator-party membership.  Neither path can be expressed as a
    // single static capability constant, so the check is performed inline.
    let is_manager = user
        .role
        .as_ref()
        .is_some_and(|r| r.is_manager() || r.is_admin());

    if !is_manager {
        // Check whether the caller is the assigned mediator for this dispute.
        let party = state
            .dispute_repo
            .find_party_by_user(id, user.user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find party: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Failed to verify access")),
                )
            })?;

        let is_mediator = party
            .as_ref()
            .is_some_and(|p| p.role == db::models::disputes::party_role::MEDIATOR);

        if !is_mediator {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "Only the assigned mediator or a manager may update mediation notes",
                )),
            ));
        }
    }

    // Require an organization context from the JWT so we can scope the UPDATE.
    let organization_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "No organization context")),
        )
    })?;

    let result = state
        .dispute_repo
        .update_mediation_notes(UpdateMediationNotes {
            dispute_id: id,
            notes: data.notes,
            updated_by: user.user_id,
            organization_id,
        })
        .await;

    match result {
        Ok(dispute) => Ok(Json(dispute)),
        Err(common::errors::AppError::NotFound(msg)) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", msg.as_str())),
        )),
        Err(e) => {
            tracing::error!("Failed to update mediation notes: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update mediation notes",
                )),
            ))
        }
    }
}

/// Withdraw a dispute.
///
/// Issue #760 / #834: the withdraw is scoped to the caller's JWT tenant, so a
/// dispute owned by another org returns 404 and is never mutated.
async fn withdraw_dispute(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "No organization context")),
        )
    })?;

    match state
        .dispute_repo
        .withdraw(id, organization_id, user.user_id)
        .await
    {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(common::errors::AppError::NotFound(_)) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Dispute not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to withdraw dispute: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to withdraw dispute")),
            ))
        }
    }
}

/// List parties for a dispute.
///
/// Issue #2441: the party roster (`user_id` + `role` PII) is scoped to the
/// caller's JWT tenant. A `dispute_id` owned by another org returns 404 so a
/// caller cannot enumerate a foreign dispute's parties.
async fn list_parties(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DisputeParty>>, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = require_org(&user)?;
    match state.dispute_repo.list_parties(id, organization_id).await {
        Ok(parties) => Ok(Json(parties)),
        Err(e) => Err(map_dispute_err(e, "Failed to list parties")),
    }
}

/// Add a party to a dispute.
///
/// Issue #2441: the upsert is scoped to the caller's JWT tenant. A `dispute_id`
/// owned by another org returns 404 and is never mutated, so a caller cannot
/// inject or overwrite a party (e.g. a MEDIATOR role) on a foreign dispute.
async fn add_party(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AddPartyRequest>,
) -> Result<(StatusCode, Json<DisputeParty>), (StatusCode, Json<ErrorResponse>)> {
    let organization_id = require_org(&user)?;
    match state
        .dispute_repo
        .add_party(id, data.user_id, &data.role, organization_id)
        .await
    {
        Ok(party) => Ok((StatusCode::CREATED, Json(party))),
        Err(e) => Err(map_dispute_err(e, "Failed to add party")),
    }
}

/// List evidence for a dispute.
///
/// Issue #2441: evidence metadata (including the S3 `storage_url`) is scoped to
/// the caller's JWT tenant. A `dispute_id` owned by another org returns 404.
async fn list_evidence(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DisputeEvidence>>, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = require_org(&user)?;
    match state.dispute_repo.list_evidence(id, organization_id).await {
        Ok(evidence) => Ok(Json(evidence)),
        Err(e) => Err(map_dispute_err(e, "Failed to list evidence")),
    }
}

/// Add evidence to a dispute.
///
/// Issue #2483 (follow-up to #2441 / PR #2450, which org-scoped the other five
/// dispute sub-resource handlers but missed this one): the INSERT is scoped to
/// the caller's JWT tenant. A `dispute_id` owned by another org returns 404 and
/// is never mutated, so a caller cannot attach evidence to a foreign dispute by
/// guessing the `dispute_id`.
async fn add_evidence(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AddEvidenceRequest>,
) -> Result<(StatusCode, Json<DisputeEvidence>), (StatusCode, Json<ErrorResponse>)> {
    let organization_id = require_org(&user)?;
    let mut evidence = data.data;
    evidence.dispute_id = id;
    evidence.uploaded_by = user.user_id;

    state
        .dispute_repo
        .add_evidence(evidence, organization_id)
        .await
        .map(|e| (StatusCode::CREATED, Json(e)))
        .map_err(|e| map_dispute_err(e, "Failed to add evidence"))
}

/// Delete evidence from a dispute.
///
/// Issue #2441: the delete is scoped to the caller's JWT tenant via the parent
/// dispute. A `dispute_id`/`evidence_id` owned by another org matches nothing
/// and returns 404, so a caller cannot destroy a foreign org's evidence.
async fn delete_evidence(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, evidence_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = require_org(&user)?;
    let deleted = state
        .dispute_repo
        .delete_evidence(id, evidence_id, organization_id)
        .await
        .map_err(|e| map_dispute_err(e, "Failed to delete evidence"))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Evidence not found")),
        ))
    }
}

/// List activities for a dispute.
///
/// Issue #2441: the activity log is scoped to the caller's JWT tenant. A
/// `dispute_id` owned by another org returns 404.
async fn list_activities(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<DisputeActivity>>, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = require_org(&user)?;
    match state
        .dispute_repo
        .list_activities(
            id,
            organization_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(activities) => Ok(Json(activities)),
        Err(e) => Err(map_dispute_err(e, "Failed to list activities")),
    }
}

// =============================================================================
// Story 77.2: Mediation Process
// =============================================================================

/// List mediation sessions for a dispute.
async fn list_sessions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<MediationSession>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_sessions(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list sessions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list sessions")),
            )
        })
}

/// Schedule a new mediation session.
async fn schedule_session(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ScheduleSessionRequest>,
) -> Result<(StatusCode, Json<MediationSession>), (StatusCode, Json<ErrorResponse>)> {
    let mut session_data = data.data;
    session_data.dispute_id = id;
    session_data.mediator_id = user.user_id;

    state
        .dispute_repo
        .schedule_session(session_data)
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(|e| {
            tracing::error!("Failed to schedule session: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to schedule session")),
            )
        })
}

/// Get a specific mediation session.
async fn get_session(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MediationSession>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .find_session_by_id(id, session_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get session: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get session")),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Session not found")),
            )
        })
}

/// Update a mediation session.
async fn update_session(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, session_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<UpdateSessionRequest>,
) -> Result<Json<MediationSession>, (StatusCode, Json<ErrorResponse>)> {
    let update_data = UpdateSessionData {
        scheduled_at: data.scheduled_at,
        duration_minutes: data.duration_minutes,
        location: data.location,
        meeting_url: data.meeting_url,
        status: data.status,
    };
    state
        .dispute_repo
        .update_session(session_id, update_data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update session: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update session")),
            )
        })
}

/// Cancel a mediation session.
async fn cancel_session(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MediationSession>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .cancel_session(session_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to cancel session: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to cancel session")),
            )
        })
}

/// Get attendance for a session.
async fn get_attendance(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, session_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<SessionAttendance>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_attendance(session_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get attendance: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get attendance")),
            )
        })
}

/// Update attendance for a party.
async fn update_attendance(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, session_id, party_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(data): Json<UpdateAttendanceRequest>,
) -> Result<Json<SessionAttendance>, (StatusCode, Json<ErrorResponse>)> {
    let update_data = UpdateAttendanceData {
        confirmed: data.confirmed,
        attended: data.attended,
        notes: data.notes,
    };
    state
        .dispute_repo
        .update_attendance(session_id, party_id, update_data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update attendance: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update attendance",
                )),
            )
        })
}

/// Record notes for a session.
async fn record_notes(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, session_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<RecordNotesRequest>,
) -> Result<Json<MediationSession>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .record_session_notes(RecordSessionNotes {
            session_id,
            notes: data.notes,
            outcome: data.outcome,
        })
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to record notes: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to record notes")),
            )
        })
}

/// List submissions for a dispute.
async fn list_submissions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<PartySubmission>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_submissions(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list submissions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list submissions")),
            )
        })
}

/// Submit a response.
async fn submit_response(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<SubmitResponseRequest>,
) -> Result<(StatusCode, Json<PartySubmission>), (StatusCode, Json<ErrorResponse>)> {
    // Get user's party ID for this dispute
    let party = state
        .dispute_repo
        .find_party_by_user(id, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find party: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to find party")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "User is not a party to this dispute",
                )),
            )
        })?;

    state
        .dispute_repo
        .submit_response(SubmitResponse {
            dispute_id: id,
            party_id: party.id,
            submission_type: data.submission_type,
            content: data.content,
            is_visible_to_all: data.is_visible_to_all,
        })
        .await
        .map(|s| (StatusCode::CREATED, Json(s)))
        .map_err(|e| {
            tracing::error!("Failed to submit response: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to submit response")),
            )
        })
}

/// Get mediation case with all sessions and submissions.
async fn get_mediation_case(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<MediationCase>, (StatusCode, Json<ErrorResponse>)> {
    match state.dispute_repo.get_mediation_case(id).await {
        Ok(Some(case)) => Ok(Json(case)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Mediation case not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get mediation case: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get mediation case",
                )),
            ))
        }
    }
}

// =============================================================================
// Story 77.3: Resolution Tracking
// =============================================================================

/// List resolutions for a dispute.
async fn list_resolutions(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DisputeResolution>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_resolutions(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list resolutions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list resolutions")),
            )
        })
}

/// Propose a resolution.
async fn propose_resolution(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ProposeResolutionRequest>,
) -> Result<(StatusCode, Json<DisputeResolution>), (StatusCode, Json<ErrorResponse>)> {
    let mut proposal = data.data;
    proposal.dispute_id = id;
    proposal.proposed_by = user.user_id;

    state
        .dispute_repo
        .propose_resolution(proposal)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| {
            tracing::error!("Failed to propose resolution: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to propose resolution",
                )),
            )
        })
}

/// Get a resolution with votes.
async fn get_resolution(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, resolution_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ResolutionWithVotes>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .dispute_repo
        .get_resolution_with_votes(resolution_id)
        .await
    {
        Ok(Some(resolution)) => Ok(Json(resolution)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Resolution not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get resolution: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get resolution")),
            ))
        }
    }
}

/// Vote on a resolution.
async fn vote_on_resolution(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, resolution_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<VoteRequest>,
) -> Result<Json<ResolutionVote>, (StatusCode, Json<ErrorResponse>)> {
    // Get user's party ID for this dispute
    let party = state
        .dispute_repo
        .find_party_by_user(id, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find party: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to find party")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "User is not a party to this dispute",
                )),
            )
        })?;

    state
        .dispute_repo
        .vote_on_resolution(VoteOnResolution {
            resolution_id,
            party_id: party.id,
            accepted: data.accepted,
            comments: data.comments,
        })
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to vote on resolution: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to vote on resolution",
                )),
            )
        })
}

/// Accept a resolution (all parties agreed).
async fn accept_resolution(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_id, resolution_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DisputeResolution>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .accept_resolution(resolution_id, user.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to accept resolution: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to accept resolution",
                )),
            )
        })
}

/// Mark a resolution as implemented.
async fn implement_resolution(
    State(state): State<AppState>,
    user: AuthUser,
    Path((_id, resolution_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DisputeResolution>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .implement_resolution(resolution_id, user.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to implement resolution: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to implement resolution",
                )),
            )
        })
}

// =============================================================================
// Story 77.4: Resolution Enforcement
// =============================================================================

/// List action items for a dispute.
async fn list_action_items(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ActionItem>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_action_items(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list action items: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list action items",
                )),
            )
        })
}

/// Create an action item.
async fn create_action_item(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateActionRequest>,
) -> Result<(StatusCode, Json<ActionItem>), (StatusCode, Json<ErrorResponse>)> {
    let mut action = data.data;
    action.dispute_id = id;

    state
        .dispute_repo
        .create_action_item(action)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| {
            tracing::error!("Failed to create action item: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to create action item",
                )),
            )
        })
}

/// Get an action item.
async fn get_action_item(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ActionItem>, (StatusCode, Json<ErrorResponse>)> {
    match state.dispute_repo.find_action_item(action_id).await {
        Ok(Some(item)) => Ok(Json(item)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Action item not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get action item: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get action item")),
            ))
        }
    }
}

/// Update an action item.
async fn update_action_item(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, action_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<UpdateActionRequest>,
) -> Result<Json<ActionItem>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .update_action_item(
            action_id,
            data.title,
            data.description,
            data.due_date,
            data.status,
        )
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update action item: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update action item",
                )),
            )
        })
}

/// Complete an action item.
async fn complete_action_item(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, action_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<CompleteActionRequest>,
) -> Result<Json<ActionItem>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .complete_action_item(CompleteActionItem {
            action_item_id: action_id,
            completion_notes: data.completion_notes,
        })
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to complete action item: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to complete action item",
                )),
            )
        })
}

/// Send a reminder for an action item.
async fn send_action_reminder(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, action_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .send_action_reminder(action_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::error!("Failed to send reminder: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to send reminder")),
            )
        })
}

/// List escalations for a dispute.
async fn list_escalations(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Escalation>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_escalations(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list escalations: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list escalations")),
            )
        })
}

/// Create an escalation.
async fn create_escalation(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateEscalationRequest>,
) -> Result<(StatusCode, Json<Escalation>), (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .create_escalation(CreateEscalation {
            dispute_id: id,
            action_item_id: data.action_item_id,
            escalated_by: user.user_id,
            escalated_to: data.escalated_to,
            reason: data.reason,
            severity: data.severity,
        })
        .await
        .map(|e| (StatusCode::CREATED, Json(e)))
        .map_err(|e| {
            tracing::error!("Failed to create escalation: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to create escalation",
                )),
            )
        })
}

/// Resolve an escalation.
async fn resolve_escalation(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((_id, escalation_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<ResolveEscalationRequest>,
) -> Result<Json<Escalation>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .resolve_escalation(ResolveEscalation {
            escalation_id,
            resolution_notes: data.resolution_notes,
        })
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to resolve escalation: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to resolve escalation",
                )),
            )
        })
}

/// Get action items for the current user.
async fn get_my_actions(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<PartyActionsDashboard>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .get_party_actions(query.organization_id, user.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get my actions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get my actions")),
            )
        })
}

/// List overdue action items.
async fn list_overdue_actions(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<ActionItem>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_overdue_actions(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list overdue actions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list overdue actions",
                )),
            )
        })
}

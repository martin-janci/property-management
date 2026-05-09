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
    DisputeActivity, DisputeEvidence, DisputeParty, DisputeQuery, DisputeResolution,
    DisputeStatistics, DisputeSummary, DisputeWithDetails, Escalation, FileDispute, MediationCase,
    MediationSession, PartyActionsDashboard, PartySubmission, ProposeResolution,
    RecordSessionNotes, ResolutionVote, ResolutionWithVotes, ResolveEscalation, ScheduleSession,
    SessionAttendance, SubmitResponse, UpdateDisputeStatus, VoteOnResolution,
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
        .route("/{id}", get(get_dispute))
        .route("/{id}", patch(update_dispute_status))
        .route("/{id}", delete(withdraw_dispute))
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
// Request/Response Types
// =============================================================================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
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

/// Get a specific dispute by ID.
async fn get_dispute(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<DisputeWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .find_by_id_with_details(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get dispute: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get dispute")),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Dispute not found")),
            )
        })
}

/// Update dispute status.
async fn update_dispute_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateStatusRequest>,
) -> Result<Json<Dispute>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .update_status(UpdateDisputeStatus {
            dispute_id: id,
            status: data.status,
            reason: data.reason,
            updated_by: user.user_id,
        })
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update dispute status: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to update dispute status",
                )),
            )
        })
}

/// Withdraw a dispute.
async fn withdraw_dispute(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .withdraw(id, user.user_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::error!("Failed to withdraw dispute: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to withdraw dispute")),
            )
        })
}

/// List parties for a dispute.
async fn list_parties(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DisputeParty>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_parties(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list parties: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list parties")),
            )
        })
}

/// Add a party to a dispute.
async fn add_party(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AddPartyRequest>,
) -> Result<(StatusCode, Json<DisputeParty>), (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .add_party(id, data.user_id, &data.role)
        .await
        .map(|p| (StatusCode::CREATED, Json(p)))
        .map_err(|e| {
            tracing::error!("Failed to add party: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to add party")),
            )
        })
}

/// List evidence for a dispute.
async fn list_evidence(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DisputeEvidence>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_evidence(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list evidence: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list evidence")),
            )
        })
}

/// Add evidence to a dispute.
async fn add_evidence(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AddEvidenceRequest>,
) -> Result<(StatusCode, Json<DisputeEvidence>), (StatusCode, Json<ErrorResponse>)> {
    let mut evidence = data.data;
    evidence.dispute_id = id;
    evidence.uploaded_by = user.user_id;

    state
        .dispute_repo
        .add_evidence(evidence)
        .await
        .map(|e| (StatusCode::CREATED, Json(e)))
        .map_err(|e| {
            tracing::error!("Failed to add evidence: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to add evidence")),
            )
        })
}

/// Delete evidence from a dispute.
async fn delete_evidence(
    State(state): State<AppState>,
    _user: AuthUser,
    Path((id, evidence_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .dispute_repo
        .delete_evidence(id, evidence_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete evidence: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to delete evidence")),
            )
        })?;

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
async fn list_activities(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<DisputeActivity>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .dispute_repo
        .list_activities(id, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list activities: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list activities")),
            )
        })
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

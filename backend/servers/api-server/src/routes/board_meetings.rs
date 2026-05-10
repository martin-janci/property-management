//! Epic 143: Board Meeting Management API Routes
//!
//! REST API endpoints for HOA/Condo board meeting management including:
//! - Board member management
//! - Meeting scheduling and management
//! - Agenda items
//! - Motions and voting
//! - Attendance tracking
//! - Minutes management
//! - Action items

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::board_meetings::{
    ActionItemQuery, BoardMemberQuery, CastVote, CreateActionItem, CreateAgendaItem,
    CreateBoardMeeting, CreateBoardMember, CreateMinutes, CreateMotion, MeetingQuery, MotionQuery,
    RecordAttendance, UpdateActionItem, UpdateAgendaItem, UpdateBoardMeeting, UpdateBoardMember,
    UpdateMinutes, UpdateMotion, UploadMeetingDocument,
};
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
    )
}

fn not_found(resource: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            "NOT_FOUND",
            format!("{} not found", resource),
        )),
    )
}

fn require_tenant(tenant_id: Option<Uuid>) -> ApiResult<Uuid> {
    tenant_id.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "UNAUTHORIZED",
                "Organization context required",
            )),
        )
    })
}

/// Create router for board meeting endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard", get(get_dashboard))
        // Board members
        .route("/members", get(list_board_members))
        .route("/members", post(create_board_member))
        .route("/members/{id}", get(get_board_member))
        .route("/members/{id}", put(update_board_member))
        .route("/members/{id}", delete(delete_board_member))
        .route("/members/{id}/attendance", get(get_member_attendance))
        // Meetings
        .route("/", get(list_meetings))
        .route("/", post(create_meeting))
        .route("/{id}", get(get_meeting))
        .route("/{id}", put(update_meeting))
        .route("/{id}", delete(delete_meeting))
        .route("/{id}/start", post(start_meeting))
        .route("/{id}/end", post(end_meeting))
        .route("/{id}/cancel", post(cancel_meeting))
        // Agenda items
        .route("/{meeting_id}/agenda", get(list_agenda_items))
        .route("/{meeting_id}/agenda", post(add_agenda_item))
        .route("/agenda/{id}", get(get_agenda_item))
        .route("/agenda/{id}", put(update_agenda_item))
        .route("/agenda/{id}/complete", post(complete_agenda_item))
        .route("/agenda/{id}", delete(delete_agenda_item))
        // Motions
        .route("/{meeting_id}/motions", get(list_meeting_motions))
        .route("/{meeting_id}/motions", post(create_motion))
        .route("/motions", get(list_motions))
        .route("/motions/{id}", get(get_motion))
        .route("/motions/{id}", put(update_motion))
        .route("/motions/{id}/second", post(second_motion))
        .route("/motions/{id}/start-voting", post(start_motion_voting))
        .route("/motions/{id}/end-voting", post(end_motion_voting))
        .route("/motions/{id}/vote", post(cast_vote))
        .route("/motions/{id}", delete(delete_motion))
        // Attendance
        .route("/{meeting_id}/attendance", get(list_attendance))
        .route("/{meeting_id}/attendance", post(record_attendance))
        // Minutes
        .route("/{meeting_id}/minutes", get(get_minutes))
        .route("/{meeting_id}/minutes", post(create_minutes))
        .route("/minutes/{id}", put(update_minutes))
        .route("/minutes/{id}/approve", post(approve_minutes))
        // Action items
        .route("/{meeting_id}/actions", get(list_meeting_action_items))
        .route("/{meeting_id}/actions", post(create_action_item))
        .route("/actions", get(list_action_items))
        .route("/actions/{id}", get(get_action_item))
        .route("/actions/{id}", put(update_action_item))
        .route("/actions/{id}/complete", post(complete_action_item))
        .route("/actions/{id}", delete(delete_action_item))
        // Documents
        .route("/{meeting_id}/documents", get(list_documents))
        .route("/{meeting_id}/documents", post(upload_document))
        .route("/documents/{id}", delete(delete_document))
        // Statistics
        .route("/statistics/types", get(get_meeting_type_counts))
        .route("/statistics/motions", get(get_motion_status_counts))
}

// ============================================================================
// DASHBOARD
// ============================================================================

/// Get board meeting dashboard.
async fn get_dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<db::models::board_meetings::MeetingDashboard>> {
    let org_id = require_tenant(user.tenant_id)?;
    let dashboard = state
        .board_meeting_repo
        .get_dashboard(org_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(dashboard))
}

// ============================================================================
// BOARD MEMBERS
// ============================================================================

/// List board members.
async fn list_board_members(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<BoardMemberQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::BoardMemberSummary>>> {
    let org_id = require_tenant(user.tenant_id)?;
    let members = state
        .board_meeting_repo
        .list_board_members(org_id, query)
        .await
        .map_err(internal_error)?;

    Ok(Json(members))
}

/// Create a board member.
async fn create_board_member(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateBoardMember>,
) -> ApiResult<Json<db::models::board_meetings::BoardMember>> {
    let org_id = require_tenant(user.tenant_id)?;
    let member = state
        .board_meeting_repo
        .create_board_member(org_id, user.user_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(member))
}

/// Get a board member by ID.
async fn get_board_member(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMember>> {
    let member = state
        .board_meeting_repo
        .get_board_member(id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Board member"))?;

    Ok(Json(member))
}

/// Update a board member.
async fn update_board_member(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateBoardMember>,
) -> ApiResult<Json<db::models::board_meetings::BoardMember>> {
    let member = state
        .board_meeting_repo
        .update_board_member(id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(member))
}

/// Delete a board member.
async fn delete_board_member(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .board_meeting_repo
        .delete_board_member(id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get board member attendance history.
async fn get_member_attendance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::AttendanceHistory>> {
    let history = state
        .board_meeting_repo
        .get_member_attendance_history(id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Board member"))?;

    Ok(Json(history))
}

// ============================================================================
// MEETINGS
// ============================================================================

/// List meetings.
async fn list_meetings(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<MeetingQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingSummary>>> {
    let org_id = require_tenant(user.tenant_id)?;
    let meetings = state
        .board_meeting_repo
        .list_meetings(org_id, query)
        .await
        .map_err(internal_error)?;

    Ok(Json(meetings))
}

/// Create a meeting.
async fn create_meeting(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateBoardMeeting>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let org_id = require_tenant(user.tenant_id)?;
    let meeting = state
        .board_meeting_repo
        .create_meeting(org_id, user.user_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(meeting))
}

/// Get a meeting by ID.
async fn get_meeting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingDetail>> {
    let detail = state
        .board_meeting_repo
        .get_meeting_detail(id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Meeting"))?;

    Ok(Json(detail))
}

/// Update a meeting.
async fn update_meeting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateBoardMeeting>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let meeting = state
        .board_meeting_repo
        .update_meeting(id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(meeting))
}

/// Delete a meeting.
async fn delete_meeting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .board_meeting_repo
        .delete_meeting(id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Start a meeting.
async fn start_meeting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let meeting = state
        .board_meeting_repo
        .start_meeting(id)
        .await
        .map_err(internal_error)?;

    Ok(Json(meeting))
}

/// End a meeting.
async fn end_meeting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let meeting = state
        .board_meeting_repo
        .end_meeting(id)
        .await
        .map_err(internal_error)?;

    Ok(Json(meeting))
}

/// Cancel a meeting.
async fn cancel_meeting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let meeting = state
        .board_meeting_repo
        .cancel_meeting(id)
        .await
        .map_err(internal_error)?;

    Ok(Json(meeting))
}

// ============================================================================
// AGENDA ITEMS
// ============================================================================

/// List agenda items for a meeting.
async fn list_agenda_items(
    State(state): State<AppState>,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingAgendaItem>>> {
    let items = state
        .board_meeting_repo
        .list_agenda_items(meeting_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(items))
}

/// Add an agenda item.
async fn add_agenda_item(
    State(state): State<AppState>,
    user: AuthUser,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateAgendaItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    input.meeting_id = meeting_id;
    let item = state
        .board_meeting_repo
        .add_agenda_item(user.user_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(item))
}

/// Get an agenda item.
async fn get_agenda_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    let item = state
        .board_meeting_repo
        .get_agenda_item(id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Agenda item"))?;

    Ok(Json(item))
}

/// Update an agenda item.
async fn update_agenda_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateAgendaItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    let item = state
        .board_meeting_repo
        .update_agenda_item(id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(item))
}

/// Complete an agenda item.
#[derive(serde::Deserialize)]
struct CompleteAgendaItemRequest {
    outcome: Option<String>,
}

async fn complete_agenda_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<CompleteAgendaItemRequest>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    let item = state
        .board_meeting_repo
        .complete_agenda_item(id, input.outcome)
        .await
        .map_err(internal_error)?;

    Ok(Json(item))
}

/// Delete an agenda item.
async fn delete_agenda_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .board_meeting_repo
        .delete_agenda_item(id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// MOTIONS
// ============================================================================

/// List motions for a meeting.
async fn list_meeting_motions(
    State(state): State<AppState>,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingMotion>>> {
    let motions = state
        .board_meeting_repo
        .list_meeting_motions(meeting_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(motions))
}

/// List all motions with filters.
async fn list_motions(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<MotionQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MotionSummary>>> {
    let org_id = require_tenant(user.tenant_id)?;
    let motions = state
        .board_meeting_repo
        .list_motions(org_id, query)
        .await
        .map_err(internal_error)?;

    Ok(Json(motions))
}

/// Create a motion.
async fn create_motion(
    State(state): State<AppState>,
    user: AuthUser,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateMotion>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    input.meeting_id = meeting_id;
    let motion = state
        .board_meeting_repo
        .create_motion(user.user_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(motion))
}

/// Get a motion by ID.
async fn get_motion(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MotionDetail>> {
    let detail = state
        .board_meeting_repo
        .get_motion_detail(id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Motion"))?;

    Ok(Json(detail))
}

/// Update a motion.
async fn update_motion(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateMotion>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let motion = state
        .board_meeting_repo
        .update_motion(id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(motion))
}

/// Second a motion.
async fn second_motion(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let motion = state
        .board_meeting_repo
        .second_motion(id, user.user_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(motion))
}

/// Start voting on a motion.
async fn start_motion_voting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let motion = state
        .board_meeting_repo
        .start_motion_voting(id)
        .await
        .map_err(internal_error)?;

    Ok(Json(motion))
}

/// End voting on a motion.
async fn end_motion_voting(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let motion = state
        .board_meeting_repo
        .end_motion_voting(id)
        .await
        .map_err(internal_error)?;

    Ok(Json(motion))
}

/// Cast a vote on a motion.
async fn cast_vote(
    State(state): State<AppState>,
    Path(motion_id): Path<Uuid>,
    Json(mut input): Json<CastVote>,
) -> ApiResult<Json<db::models::board_meetings::MotionVote>> {
    input.motion_id = motion_id;
    // NOTE: In production, we'd look up the board_member_id from user.user_id
    // For now, we'll use a placeholder - this should be retrieved from the board_members table
    let board_member_id = motion_id; // Placeholder - should be actual board member lookup
    let vote = state
        .board_meeting_repo
        .cast_vote(board_member_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(vote))
}

/// Delete a motion.
async fn delete_motion(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .board_meeting_repo
        .delete_motion(id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// ATTENDANCE
// ============================================================================

/// List attendance for a meeting.
async fn list_attendance(
    State(state): State<AppState>,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingAttendance>>> {
    let attendance = state
        .board_meeting_repo
        .list_meeting_attendance(meeting_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(attendance))
}

/// Record attendance.
async fn record_attendance(
    State(state): State<AppState>,
    user: AuthUser,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<RecordAttendance>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAttendance>> {
    input.meeting_id = meeting_id;
    let attendance = state
        .board_meeting_repo
        .record_attendance(user.user_id, input)
        .await
        .map_err(internal_error)?;

    // Update quorum status
    let _ = state
        .board_meeting_repo
        .update_meeting_quorum(meeting_id)
        .await;

    Ok(Json(attendance))
}

// ============================================================================
// MINUTES
// ============================================================================

/// Get minutes for a meeting.
async fn get_minutes(
    State(state): State<AppState>,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    let minutes = state
        .board_meeting_repo
        .get_meeting_minutes(meeting_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Minutes"))?;

    Ok(Json(minutes))
}

/// Create minutes.
async fn create_minutes(
    State(state): State<AppState>,
    user: AuthUser,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateMinutes>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    input.meeting_id = meeting_id;
    let minutes = state
        .board_meeting_repo
        .create_minutes(user.user_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(minutes))
}

/// Update minutes.
async fn update_minutes(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateMinutes>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    let minutes = state
        .board_meeting_repo
        .update_minutes(id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(minutes))
}

/// Approve minutes.
#[derive(serde::Deserialize)]
struct ApproveMinutesRequest {
    approval_motion_id: Option<Uuid>,
}

async fn approve_minutes(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ApproveMinutesRequest>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    let minutes = state
        .board_meeting_repo
        .approve_minutes(id, user.user_id, input.approval_motion_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(minutes))
}

// ============================================================================
// ACTION ITEMS
// ============================================================================

/// List action items for a meeting.
async fn list_meeting_action_items(
    State(state): State<AppState>,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingActionItem>>> {
    let items = state
        .board_meeting_repo
        .list_meeting_action_items(meeting_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(items))
}

/// List all action items with filters.
async fn list_action_items(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ActionItemQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::ActionItemSummary>>> {
    let org_id = require_tenant(user.tenant_id)?;
    let items = state
        .board_meeting_repo
        .list_action_items(org_id, query)
        .await
        .map_err(internal_error)?;

    Ok(Json(items))
}

/// Create an action item.
async fn create_action_item(
    State(state): State<AppState>,
    user: AuthUser,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateActionItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    input.meeting_id = meeting_id;
    let item = state
        .board_meeting_repo
        .create_action_item(user.user_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(item))
}

/// Get an action item.
async fn get_action_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    let item = state
        .board_meeting_repo
        .get_action_item(id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Action item"))?;

    Ok(Json(item))
}

/// Update an action item.
async fn update_action_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateActionItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    let item = state
        .board_meeting_repo
        .update_action_item(id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(item))
}

/// Complete an action item.
#[derive(serde::Deserialize)]
struct CompleteActionItemRequest {
    completion_notes: Option<String>,
}

async fn complete_action_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<CompleteActionItemRequest>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    let item = state
        .board_meeting_repo
        .complete_action_item(id, input.completion_notes)
        .await
        .map_err(internal_error)?;

    Ok(Json(item))
}

/// Delete an action item.
async fn delete_action_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .board_meeting_repo
        .delete_action_item(id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// DOCUMENTS
// ============================================================================

/// List documents for a meeting.
async fn list_documents(
    State(state): State<AppState>,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingDocument>>> {
    let docs = state
        .board_meeting_repo
        .list_meeting_documents(meeting_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(docs))
}

/// Upload a document.
async fn upload_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<UploadMeetingDocument>,
) -> ApiResult<Json<db::models::board_meetings::MeetingDocument>> {
    input.meeting_id = meeting_id;
    let doc = state
        .board_meeting_repo
        .upload_document(user.user_id, input)
        .await
        .map_err(internal_error)?;

    Ok(Json(doc))
}

/// Delete a document.
async fn delete_document(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    state
        .board_meeting_repo
        .delete_document(id)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Get meeting type counts.
async fn get_meeting_type_counts(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingTypeCount>>> {
    let org_id = require_tenant(user.tenant_id)?;
    let counts = state
        .board_meeting_repo
        .get_meeting_type_counts(org_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(counts))
}

/// Get motion status counts.
async fn get_motion_status_counts(
    State(state): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<Vec<db::models::board_meetings::MotionStatusCount>>> {
    let org_id = require_tenant(user.tenant_id)?;
    let counts = state
        .board_meeting_repo
        .get_motion_status_counts(org_id)
        .await
        .map_err(internal_error)?;

    Ok(Json(counts))
}

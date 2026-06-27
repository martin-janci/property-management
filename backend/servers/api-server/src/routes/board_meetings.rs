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
//!
//! # RLS (PAP-137)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the board-meeting
//! tables, so every query MUST run on a connection that has
//! `app.current_org_id` set or it collapses to deny-all. Each handler
//! therefore acquires an [`RlsConnection`] (which validates tenant membership
//! and sets the org/user GUCs on a dedicated connection) and passes
//! `&mut **rls.conn()` to the repository. The authoritative organization is
//! `rls.tenant_id()` — the tenant the caller was validated against — never a
//! client-supplied value, so the SQL org filter and the RLS context can never
//! disagree. By-id repo queries stay keyed on `(id, organization_id)` (child
//! tables via the root meeting) as defense-in-depth, so a cross-tenant probe
//! resolves to no row → `404` even on an RLS-exempt pool. Child-table inserts
//! (`agenda`, `motions`, `votes`, `attendance`, `minutes`, `actions`,
//! `documents`) validate the root meeting/motion org first.
//! `rls.release()` clears the context before the connection returns to the
//! pool.

use crate::state::AppState;
use api_core::extractors::RlsConnection;
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

fn forbidden(resource: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::new(
            "FORBIDDEN",
            format!("Access forbidden: {}", resource),
        )),
    )
}

/// Validate that `meeting_id` belongs to the caller's organization.
///
/// Root-meeting org check for child-table operations (agenda items, motions,
/// attendance, minutes, action items, documents). A meeting owned by another
/// organization surfaces as `404 NOT_FOUND` — indistinguishable from a
/// missing meeting.
async fn require_meeting_in_org(
    state: &AppState,
    rls: &mut RlsConnection,
    meeting_id: Uuid,
) -> ApiResult<()> {
    let org_id = rls.tenant_id();
    state
        .board_meeting_repo
        .get_meeting(&mut **rls.conn(), meeting_id, org_id)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found("Meeting"))?;
    Ok(())
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
    mut rls: RlsConnection,
) -> ApiResult<Json<db::models::board_meetings::MeetingDashboard>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_dashboard(rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

// ============================================================================
// BOARD MEMBERS
// ============================================================================

/// List board members.
async fn list_board_members(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<BoardMemberQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::BoardMemberSummary>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_board_members(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a board member.
async fn create_board_member(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateBoardMember>,
) -> ApiResult<Json<db::models::board_meetings::BoardMember>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .board_meeting_repo
        .create_board_member(&mut **rls.conn(), org_id, user_id, input)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Get a board member by ID.
async fn get_board_member(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMember>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_board_member(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Board member")))
        .map(Json);
    rls.release().await;
    out
}

/// Update a board member.
async fn update_board_member(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateBoardMember>,
) -> ApiResult<Json<db::models::board_meetings::BoardMember>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .update_board_member(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Board member")))
        .map(Json);
    rls.release().await;
    out
}

/// Delete a board member.
async fn delete_board_member(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .delete_board_member(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Board member"))
            }
        });
    rls.release().await;
    out
}

/// Get board member attendance history.
async fn get_member_attendance(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::AttendanceHistory>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_member_attendance_history(rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|h| h.ok_or_else(|| not_found("Board member")))
        .map(Json);
    rls.release().await;
    out
}

// ============================================================================
// MEETINGS
// ============================================================================

/// List meetings.
async fn list_meetings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<MeetingQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingSummary>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_meetings(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a meeting.
async fn create_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateBoardMeeting>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .board_meeting_repo
        .create_meeting(&mut **rls.conn(), org_id, user_id, input)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Get a meeting by ID.
async fn get_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingDetail>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_meeting_detail(rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|d| d.ok_or_else(|| not_found("Meeting")))
        .map(Json);
    rls.release().await;
    out
}

/// Update a meeting.
async fn update_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateBoardMeeting>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .update_meeting(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Meeting")))
        .map(Json);
    rls.release().await;
    out
}

/// Delete a meeting.
async fn delete_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .delete_meeting(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Meeting"))
            }
        });
    rls.release().await;
    out
}

/// Start a meeting.
async fn start_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .start_meeting(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Meeting")))
        .map(Json);
    rls.release().await;
    out
}

/// End a meeting.
async fn end_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .end_meeting(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Meeting")))
        .map(Json);
    rls.release().await;
    out
}

/// Cancel a meeting.
async fn cancel_meeting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::BoardMeeting>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .cancel_meeting(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Meeting")))
        .map(Json);
    rls.release().await;
    out
}

// ============================================================================
// AGENDA ITEMS
// ============================================================================

/// List agenda items for a meeting.
async fn list_agenda_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingAgendaItem>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_agenda_items(&mut **rls.conn(), meeting_id, org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Add an agenda item.
async fn add_agenda_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateAgendaItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    input.meeting_id = meeting_id;
    let user_id = rls.user_id();
    let out = async {
        require_meeting_in_org(&state, &mut rls, meeting_id).await?;
        state
            .board_meeting_repo
            .add_agenda_item(&mut **rls.conn(), user_id, input)
            .await
            .map(Json)
            .map_err(internal_error)
    }
    .await;
    rls.release().await;
    out
}

/// Get an agenda item.
async fn get_agenda_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_agenda_item(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|i| i.ok_or_else(|| not_found("Agenda item")))
        .map(Json);
    rls.release().await;
    out
}

/// Update an agenda item.
async fn update_agenda_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateAgendaItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .update_agenda_item(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(internal_error)
        .and_then(|i| i.ok_or_else(|| not_found("Agenda item")))
        .map(Json);
    rls.release().await;
    out
}

/// Complete an agenda item.
#[derive(serde::Deserialize)]
struct CompleteAgendaItemRequest {
    outcome: Option<String>,
}

async fn complete_agenda_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<CompleteAgendaItemRequest>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAgendaItem>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .complete_agenda_item(&mut **rls.conn(), id, org_id, input.outcome)
        .await
        .map_err(internal_error)
        .and_then(|i| i.ok_or_else(|| not_found("Agenda item")))
        .map(Json);
    rls.release().await;
    out
}

/// Delete an agenda item.
async fn delete_agenda_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .delete_agenda_item(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Agenda item"))
            }
        });
    rls.release().await;
    out
}

// ============================================================================
// MOTIONS
// ============================================================================

/// List motions for a meeting.
async fn list_meeting_motions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingMotion>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_meeting_motions(&mut **rls.conn(), meeting_id, org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// List all motions with filters.
async fn list_motions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<MotionQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MotionSummary>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_motions(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a motion.
async fn create_motion(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateMotion>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    input.meeting_id = meeting_id;
    let user_id = rls.user_id();
    let out = async {
        require_meeting_in_org(&state, &mut rls, meeting_id).await?;
        state
            .board_meeting_repo
            .create_motion(&mut **rls.conn(), user_id, input)
            .await
            .map(Json)
            .map_err(internal_error)
    }
    .await;
    rls.release().await;
    out
}

/// Get a motion by ID.
async fn get_motion(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MotionDetail>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_motion_detail(rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|d| d.ok_or_else(|| not_found("Motion")))
        .map(Json);
    rls.release().await;
    out
}

/// Update a motion.
async fn update_motion(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateMotion>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .update_motion(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Motion")))
        .map(Json);
    rls.release().await;
    out
}

/// Second a motion.
async fn second_motion(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .board_meeting_repo
        .second_motion(&mut **rls.conn(), id, org_id, user_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Motion")))
        .map(Json);
    rls.release().await;
    out
}

/// Start voting on a motion.
async fn start_motion_voting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .start_motion_voting(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Motion")))
        .map(Json);
    rls.release().await;
    out
}

/// End voting on a motion.
async fn end_motion_voting(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMotion>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .end_motion_voting(rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Motion")))
        .map(Json);
    rls.release().await;
    out
}

/// Cast a vote on a motion.
async fn cast_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(motion_id): Path<Uuid>,
    Json(mut input): Json<CastVote>,
) -> ApiResult<Json<db::models::board_meetings::MotionVote>> {
    input.motion_id = motion_id;
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = async {
        // Root-motion org check before writing a vote row.
        state
            .board_meeting_repo
            .get_motion(&mut **rls.conn(), motion_id, org_id)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| not_found("Motion"))?;

        // Resolve the caller's board_member_id from the board_members table by user_id (+ org/meeting scope)
        let board_member = state
            .board_meeting_repo
            .get_board_member_for_motion(&mut **rls.conn(), motion_id, user_id, org_id)
            .await
            .map_err(internal_error)?
            .ok_or_else(|| forbidden("Caller is not a board member of the meeting"))?;

        state
            .board_meeting_repo
            .cast_vote(&mut **rls.conn(), board_member.id, input)
            .await
            .map(Json)
            .map_err(internal_error)
    }
    .await;
    rls.release().await;
    out
}

/// Delete a motion.
async fn delete_motion(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .delete_motion(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Motion"))
            }
        });
    rls.release().await;
    out
}

// ============================================================================
// ATTENDANCE
// ============================================================================

/// List attendance for a meeting.
async fn list_attendance(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingAttendance>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_meeting_attendance(&mut **rls.conn(), meeting_id, org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Record attendance.
async fn record_attendance(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<RecordAttendance>,
) -> ApiResult<Json<db::models::board_meetings::MeetingAttendance>> {
    input.meeting_id = meeting_id;
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = async {
        require_meeting_in_org(&state, &mut rls, meeting_id).await?;
        let attendance = state
            .board_meeting_repo
            .record_attendance(&mut **rls.conn(), user_id, input)
            .await
            .map_err(internal_error)?;

        // Update quorum status
        let _ = state
            .board_meeting_repo
            .update_meeting_quorum(rls.conn(), meeting_id, org_id)
            .await;

        Ok(Json(attendance))
    }
    .await;
    rls.release().await;
    out
}

// ============================================================================
// MINUTES
// ============================================================================

/// Get minutes for a meeting.
async fn get_minutes(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_meeting_minutes(&mut **rls.conn(), meeting_id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Minutes")))
        .map(Json);
    rls.release().await;
    out
}

/// Create minutes.
async fn create_minutes(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateMinutes>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    input.meeting_id = meeting_id;
    let user_id = rls.user_id();
    let out = async {
        require_meeting_in_org(&state, &mut rls, meeting_id).await?;
        state
            .board_meeting_repo
            .create_minutes(&mut **rls.conn(), user_id, input)
            .await
            .map(Json)
            .map_err(internal_error)
    }
    .await;
    rls.release().await;
    out
}

/// Update minutes.
async fn update_minutes(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateMinutes>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .update_minutes(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Minutes")))
        .map(Json);
    rls.release().await;
    out
}

/// Approve minutes.
#[derive(serde::Deserialize)]
struct ApproveMinutesRequest {
    approval_motion_id: Option<Uuid>,
}

async fn approve_minutes(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<ApproveMinutesRequest>,
) -> ApiResult<Json<db::models::board_meetings::MeetingMinutes>> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .board_meeting_repo
        .approve_minutes(
            &mut **rls.conn(),
            id,
            org_id,
            user_id,
            input.approval_motion_id,
        )
        .await
        .map_err(internal_error)
        .and_then(|m| m.ok_or_else(|| not_found("Minutes")))
        .map(Json);
    rls.release().await;
    out
}

// ============================================================================
// ACTION ITEMS
// ============================================================================

/// List action items for a meeting.
async fn list_meeting_action_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingActionItem>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_meeting_action_items(&mut **rls.conn(), meeting_id, org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// List all action items with filters.
async fn list_action_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ActionItemQuery>,
) -> ApiResult<Json<Vec<db::models::board_meetings::ActionItemSummary>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_action_items(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create an action item.
async fn create_action_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<CreateActionItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    input.meeting_id = meeting_id;
    let user_id = rls.user_id();
    let out = async {
        require_meeting_in_org(&state, &mut rls, meeting_id).await?;
        state
            .board_meeting_repo
            .create_action_item(&mut **rls.conn(), user_id, input)
            .await
            .map(Json)
            .map_err(internal_error)
    }
    .await;
    rls.release().await;
    out
}

/// Get an action item.
async fn get_action_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_action_item(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|i| i.ok_or_else(|| not_found("Action item")))
        .map(Json);
    rls.release().await;
    out
}

/// Update an action item.
async fn update_action_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateActionItem>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .update_action_item(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(internal_error)
        .and_then(|i| i.ok_or_else(|| not_found("Action item")))
        .map(Json);
    rls.release().await;
    out
}

/// Complete an action item.
#[derive(serde::Deserialize)]
struct CompleteActionItemRequest {
    completion_notes: Option<String>,
}

async fn complete_action_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<CompleteActionItemRequest>,
) -> ApiResult<Json<db::models::board_meetings::MeetingActionItem>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .complete_action_item(&mut **rls.conn(), id, org_id, input.completion_notes)
        .await
        .map_err(internal_error)
        .and_then(|i| i.ok_or_else(|| not_found("Action item")))
        .map(Json);
    rls.release().await;
    out
}

/// Delete an action item.
async fn delete_action_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .delete_action_item(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Action item"))
            }
        });
    rls.release().await;
    out
}

// ============================================================================
// DOCUMENTS
// ============================================================================

/// List documents for a meeting.
async fn list_documents(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingDocument>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .list_meeting_documents(&mut **rls.conn(), meeting_id, org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Upload a document.
async fn upload_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(meeting_id): Path<Uuid>,
    Json(mut input): Json<UploadMeetingDocument>,
) -> ApiResult<Json<db::models::board_meetings::MeetingDocument>> {
    input.meeting_id = meeting_id;
    let user_id = rls.user_id();
    let out = async {
        require_meeting_in_org(&state, &mut rls, meeting_id).await?;
        state
            .board_meeting_repo
            .upload_document(&mut **rls.conn(), user_id, input)
            .await
            .map(Json)
            .map_err(internal_error)
    }
    .await;
    rls.release().await;
    out
}

/// Delete a document.
async fn delete_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .delete_document(&mut **rls.conn(), id, org_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Document"))
            }
        });
    rls.release().await;
    out
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Get meeting type counts.
async fn get_meeting_type_counts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> ApiResult<Json<Vec<db::models::board_meetings::MeetingTypeCount>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_meeting_type_counts(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Get motion status counts.
async fn get_motion_status_counts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> ApiResult<Json<Vec<db::models::board_meetings::MotionStatusCount>>> {
    let org_id = rls.tenant_id();
    let out = state
        .board_meeting_repo
        .get_motion_status_counts(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

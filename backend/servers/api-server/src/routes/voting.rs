//! Voting routes (Epic 5: Building Voting & Decisions).

use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use common::errors::ErrorResponse;
use common::notifications::{Notification, NotificationCategory};
use db::models::{
    CancelVote, CastVote, CreateVote, CreateVoteComment, CreateVoteQuestion, HideVoteComment,
    PublishVote, QuestionOption, UpdateVote, UpdateVoteQuestion, Vote, VoteAuditLog,
    VoteCommentWithUser, VoteEligibility, VoteListQuery, VoteQuestion, VoteReceipt, VoteReportData,
    VoteResults, VoteSummary, VoteWithDetails,
};
use genpdf::Element;
use serde::Deserialize;
use sqlx::Error as SqlxError;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Create voting router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Vote CRUD
        .route("/", post(create_vote))
        .route("/", get(list_votes))
        .route("/{id}", get(get_vote))
        .route("/{id}", put(update_vote))
        .route("/{id}", delete(delete_vote))
        // Workflow
        .route("/{id}/publish", post(publish_vote))
        .route("/{id}/cancel", post(cancel_vote))
        .route("/{id}/close", post(close_vote))
        // Questions
        .route("/{id}/questions", post(add_question))
        .route("/{id}/questions", get(list_questions))
        .route("/{id}/questions/{question_id}", put(update_question))
        .route("/{id}/questions/{question_id}", delete(delete_question))
        // Voting
        .route("/{id}/eligibility", get(check_eligibility))
        .route("/{id}/cast", post(cast_vote))
        .route("/{id}/my-response", get(get_my_response))
        // Comments
        .route("/{id}/comments", post(add_comment))
        .route("/{id}/comments", get(list_comments))
        .route("/{id}/comments/{comment_id}/replies", get(list_replies))
        .route("/{id}/comments/{comment_id}/hide", post(hide_comment))
        // Results
        .route("/{id}/results", get(get_results))
        // Reports
        .route("/{id}/report", get(get_report_data))
        .route("/{id}/report.pdf", get(get_report_pdf))
        // Audit
        .route("/{id}/audit", get(get_audit_log))
        // Building votes
        .route(
            "/building/{building_id}/active",
            get(list_active_by_building),
        )
}

// ============================================================================
// Request/Response types
// ============================================================================

/// Request to create a vote.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateVoteRequest {
    pub building_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: DateTime<Utc>,
    pub quorum_type: String,
    pub quorum_percentage: Option<i32>,
    pub allow_delegation: Option<bool>,
    pub anonymous_voting: Option<bool>,
}

/// Request to update a vote.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateVoteRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub quorum_type: Option<String>,
    pub quorum_percentage: Option<i32>,
    pub allow_delegation: Option<bool>,
    pub anonymous_voting: Option<bool>,
}

/// Request to publish a vote.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishVoteRequest {
    pub start_at: Option<DateTime<Utc>>,
}

/// Request to cancel a vote.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelVoteRequest {
    pub reason: String,
}

/// Request to add a question.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddQuestionRequest {
    pub question_text: String,
    pub description: Option<String>,
    pub question_type: String,
    pub options: Vec<QuestionOption>,
    pub display_order: Option<i32>,
    pub is_required: Option<bool>,
}

/// Request to update a question.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateQuestionRequest {
    pub question_text: Option<String>,
    pub description: Option<String>,
    pub options: Option<Vec<QuestionOption>>,
    pub display_order: Option<i32>,
    pub is_required: Option<bool>,
}

/// Request to cast a vote.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CastVoteRequest {
    pub unit_id: Uuid,
    pub delegation_id: Option<Uuid>,
    pub answers: serde_json::Value,
}

/// Request to add a comment.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddCommentRequest {
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub ai_consent: bool,
}

/// Request to hide a comment.
#[derive(Debug, Deserialize, ToSchema)]
pub struct HideCommentRequest {
    pub reason: String,
}

/// Query params for listing votes.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListVotesQuery {
    pub building_id: Option<Uuid>,
    pub status: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query params for my-response.
#[derive(Debug, Deserialize, IntoParams)]
pub struct MyResponseQuery {
    pub unit_id: Uuid,
}

// ============================================================================
// Vote CRUD Handlers
// ============================================================================

/// Create a new vote.
#[utoipa::path(
    post,
    path = "/api/v1/voting",
    request_body = CreateVoteRequest,
    responses(
        (status = 201, description = "Vote created", body = Vote),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn create_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateVoteRequest>,
) -> Result<(StatusCode, Json<Vote>), (StatusCode, Json<ErrorResponse>)> {
    let data = CreateVote {
        organization_id: rls.tenant_id(),
        building_id: req.building_id,
        title: req.title,
        description: req.description,
        start_at: req.start_at,
        end_at: req.end_at,
        quorum_type: req.quorum_type,
        quorum_percentage: req.quorum_percentage,
        allow_delegation: req.allow_delegation,
        anonymous_voting: req.anonymous_voting,
        created_by: rls.user_id(),
    };

    let vote = state
        .vote_repo
        .create_poll_rls(&mut **rls.conn(), data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create vote: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to create vote: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(vote)))
}

/// List votes.
#[utoipa::path(
    get,
    path = "/api/v1/voting",
    params(ListVotesQuery),
    responses(
        (status = 200, description = "List of votes", body = Vec<VoteSummary>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn list_votes(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListVotesQuery>,
) -> Result<Json<Vec<VoteSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let list_query = VoteListQuery {
        building_id: query.building_id,
        status: query.status.map(|s| vec![s]),
        created_by: None,
        from_date: query.from_date,
        to_date: query.to_date,
        limit: query.limit,
        offset: query.offset,
    };

    // Note: list() doesn't have an RLS version yet, use legacy method
    // The RLS context is still set on the connection for any sub-queries
    let votes = state
        .vote_repo
        .list(rls.tenant_id(), list_query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list votes: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to list votes: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(votes))
}

/// Get vote by ID.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "Vote details", body = VoteWithDetails),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn get_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VoteWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    // Scope the lookup to the caller's tenant so a vote belonging to another
    // organization resolves to `None` (mapped to 404 below) instead of leaking
    // cross-tenant data.
    let vote = state
        .vote_repo
        .find_by_id_with_details_for_org(id, rls.tenant_id())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    rls.release().await;
    Ok(Json(vote))
}

/// Update vote.
#[utoipa::path(
    put,
    path = "/api/v1/voting/{id}",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    request_body = UpdateVoteRequest,
    responses(
        (status = 200, description = "Vote updated", body = Vote),
        (status = 400, description = "Vote cannot be modified"),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn update_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateVoteRequest>,
) -> Result<Json<Vote>, (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists and is in draft status
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.can_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Vote can only be edited in draft status",
            )),
        ));
    }

    let data = UpdateVote {
        title: req.title,
        description: req.description,
        start_at: req.start_at,
        end_at: req.end_at,
        quorum_type: req.quorum_type,
        quorum_percentage: req.quorum_percentage,
        allow_delegation: req.allow_delegation,
        anonymous_voting: req.anonymous_voting,
    };

    // Note: update() doesn't have an RLS version yet
    let vote = state.vote_repo.update(id, data).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to update vote: {}", e),
            )),
        )
    })?;

    rls.release().await;
    Ok(Json(vote))
}

/// Delete vote.
#[utoipa::path(
    delete,
    path = "/api/v1/voting/{id}",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 204, description = "Vote deleted"),
        (status = 400, description = "Vote cannot be deleted"),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn delete_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists and is in draft status
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.can_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Vote can only be deleted in draft status",
            )),
        ));
    }

    // Note: delete() doesn't have an RLS version yet
    state.vote_repo.delete(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to delete vote: {}", e),
            )),
        )
    })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Workflow Handlers
// ============================================================================

/// Publish a vote.
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/publish",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    request_body = PublishVoteRequest,
    responses(
        (status = 200, description = "Vote published", body = Vote),
        (status = 400, description = "Vote cannot be published"),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn publish_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<PublishVoteRequest>,
) -> Result<Json<Vote>, (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists and is in draft status
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.can_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Vote can only be published from draft status",
            )),
        ));
    }

    let data = PublishVote {
        start_at: req.start_at,
    };

    // Note: publish() doesn't have an RLS version yet
    let vote = state
        .vote_repo
        .publish(id, rls.user_id(), data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to publish vote: {}", e),
                )),
            )
        })?;

    // Story 5.1: notify eligible owners when a vote is published. Only dispatch
    // on immediate publishes (status == ACTIVE); SCHEDULED votes are handled by
    // the scheduler's activation path at start time, so gating avoids a
    // double-notification. Best-effort: log on error, never fail the publish.
    if vote.status == db::models::vote_status::ACTIVE {
        match state
            .vote_repo
            .get_eligible_owner_user_ids(vote.building_id)
            .await
        {
            Ok(eligible_ids) if !eligible_ids.is_empty() => {
                let notification = Notification::new(
                    Uuid::nil(),
                    NotificationCategory::Votes,
                    format!("New Vote: {}", vote.title),
                    format!(
                        "A new vote has started: \"{}\". Deadline: {}",
                        vote.title,
                        vote.end_at.format("%Y-%m-%d %H:%M")
                    ),
                )
                .with_action_url(format!("/voting/{}", vote.id))
                .with_data(serde_json::json!({
                    "vote_id": vote.id,
                    "organization_id": vote.organization_id,
                    "building_id": vote.building_id,
                    "end_at": vote.end_at.to_rfc3339(),
                }));

                let results = state
                    .notification_pipeline
                    .dispatch_to_users(&eligible_ids, &notification, Some(vote.id), None)
                    .await;

                tracing::info!(
                    vote_id = %vote.id,
                    recipients = results.len(),
                    "Vote published — VoteStarted notifications dispatched"
                );
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(
                    vote_id = %vote.id,
                    error = %e,
                    "Failed to load eligible owners for vote-published notification"
                );
            }
        }
    }

    rls.release().await;
    Ok(Json(vote))
}

/// Cancel a vote.
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/cancel",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    request_body = CancelVoteRequest,
    responses(
        (status = 200, description = "Vote cancelled", body = Vote),
        (status = 400, description = "Vote cannot be cancelled"),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn cancel_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CancelVoteRequest>,
) -> Result<Json<Vote>, (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if existing.status == "closed" || existing.status == "cancelled" {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Vote cannot be cancelled in current status",
            )),
        ));
    }

    let data = CancelVote { reason: req.reason };

    // Note: cancel() doesn't have an RLS version yet
    let vote = state
        .vote_repo
        .cancel(id, rls.user_id(), data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to cancel vote: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(vote))
}

/// Close a vote and calculate results.
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/close",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "Vote closed", body = Vote),
        (status = 400, description = "Vote cannot be closed"),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn close_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vote>, (StatusCode, Json<ErrorResponse>)> {
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.is_active() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Only active votes can be closed",
            )),
        ));
    }

    // Note: close() doesn't have an RLS version yet
    let vote = state.vote_repo.close(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to close vote: {}", e),
            )),
        )
    })?;

    rls.release().await;
    Ok(Json(vote))
}

// ============================================================================
// Question Handlers
// ============================================================================

/// Add a question to a vote.
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/questions",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    request_body = AddQuestionRequest,
    responses(
        (status = 201, description = "Question added", body = VoteQuestion),
        (status = 400, description = "Cannot add questions"),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn add_question(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<AddQuestionRequest>,
) -> Result<(StatusCode, Json<VoteQuestion>), (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists and is in draft status
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.can_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Questions can only be added to draft votes",
            )),
        ));
    }

    let data = CreateVoteQuestion {
        vote_id: id,
        question_text: req.question_text,
        description: req.description,
        question_type: req.question_type,
        options: req.options,
        display_order: req.display_order,
        is_required: req.is_required,
    };

    // Note: add_question() doesn't have an RLS version yet
    let question = state.vote_repo.add_question(data).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to add question: {}", e),
            )),
        )
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(question)))
}

/// List questions for a vote.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/questions",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "List of questions", body = Vec<VoteQuestion>),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn list_questions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VoteQuestion>>, (StatusCode, Json<ErrorResponse>)> {
    // Note: get_questions() doesn't have an RLS version yet
    let questions = state.vote_repo.get_questions(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to list questions: {}", e),
            )),
        )
    })?;

    rls.release().await;
    Ok(Json(questions))
}

/// Update a question.
#[utoipa::path(
    put,
    path = "/api/v1/voting/{id}/questions/{question_id}",
    params(
        ("id" = Uuid, Path, description = "Vote ID"),
        ("question_id" = Uuid, Path, description = "Question ID")
    ),
    request_body = UpdateQuestionRequest,
    responses(
        (status = 200, description = "Question updated", body = VoteQuestion),
        (status = 400, description = "Cannot update question"),
        (status = 404, description = "Vote or question not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn update_question(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((id, question_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateQuestionRequest>,
) -> Result<Json<VoteQuestion>, (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists and is in draft status
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.can_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Questions can only be updated in draft votes",
            )),
        ));
    }

    let data = UpdateVoteQuestion {
        question_text: req.question_text,
        description: req.description,
        options: req.options,
        display_order: req.display_order,
        is_required: req.is_required,
    };

    // Note: update_question() doesn't have an RLS version yet
    let question = state
        .vote_repo
        .update_question(question_id, data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to update question: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(question))
}

/// Delete a question.
#[utoipa::path(
    delete,
    path = "/api/v1/voting/{id}/questions/{question_id}",
    params(
        ("id" = Uuid, Path, description = "Vote ID"),
        ("question_id" = Uuid, Path, description = "Question ID")
    ),
    responses(
        (status = 204, description = "Question deleted"),
        (status = 400, description = "Cannot delete question"),
        (status = 404, description = "Vote or question not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn delete_question(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((id, question_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists and is in draft status
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.can_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Questions can only be deleted from draft votes",
            )),
        ));
    }

    // Note: delete_question() doesn't have an RLS version yet
    state
        .vote_repo
        .delete_question(question_id, id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to delete question: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Voting Handlers
// ============================================================================

/// Check vote eligibility for current user.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/eligibility",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "Eligibility status", body = VoteEligibility),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn check_eligibility(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VoteEligibility>, (StatusCode, Json<ErrorResponse>)> {
    // Note: check_eligibility() doesn't have an RLS version yet
    let eligibility = state
        .vote_repo
        .check_eligibility(id, rls.user_id())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to check eligibility: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(eligibility))
}

/// Cast a vote.
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/cast",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    request_body = CastVoteRequest,
    responses(
        (status = 200, description = "Vote cast", body = VoteReceipt),
        (status = 400, description = "Cannot cast vote"),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn cast_vote(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CastVoteRequest>,
) -> Result<Json<VoteReceipt>, (StatusCode, Json<ErrorResponse>)> {
    // Check vote exists and is active
    let existing = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get vote: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    if !existing.is_active() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Vote is not currently active",
            )),
        ));
    }

    // Re-validate the delegation at cast time (Story 5.4): a delegation that was
    // active when eligibility was loaded may have since been revoked or expired.
    // Run the check on the same RLS connection so it sees the same tenant context.
    if let Some(delegation_id) = req.delegation_id {
        let user_id = rls.user_id();
        let is_valid: bool = match sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM delegations
                WHERE id = $1
                  AND delegate_user_id = $2
                  AND unit_id = $3
                  AND status = 'active'
                  AND (scopes && ARRAY['voting','all']::delegation_scope[])
                  AND (end_date IS NULL OR end_date >= CURRENT_DATE)
            )
            "#,
        )
        .bind(delegation_id)
        .bind(user_id)
        .bind(req.unit_id)
        .fetch_one(&mut **rls.conn())
        .await
        {
            Ok(exists) => exists,
            Err(e) => {
                rls.release().await;
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        format!("Failed to validate delegation: {}", e),
                    )),
                ));
            }
        };

        if !is_valid {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "DELEGATION_REVOKED",
                    "Delegation revoked",
                )),
            ));
        }
    }

    let data = CastVote {
        vote_id: id,
        user_id: rls.user_id(),
        unit_id: req.unit_id,
        delegation_id: req.delegation_id,
        answers: req.answers,
    };

    let receipt = state
        .vote_repo
        .cast_vote_rls(rls.conn(), data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to cast vote: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(receipt))
}

/// Get current user's response for a vote.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/my-response",
    params(
        ("id" = Uuid, Path, description = "Vote ID"),
        MyResponseQuery
    ),
    responses(
        (status = 200, description = "User's response"),
        (status = 404, description = "Vote or response not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn get_my_response(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(params): Query<MyResponseQuery>,
) -> Result<Json<Option<db::models::VoteResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Note: get_user_response() doesn't have an RLS version yet
    let response = state
        .vote_repo
        .get_user_response(id, params.unit_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get response: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(response))
}

// ============================================================================
// Comment Handlers
// ============================================================================

/// Add a comment to a vote.
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    request_body = AddCommentRequest,
    responses(
        (status = 201, description = "Comment added", body = db::models::VoteComment),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn add_comment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<AddCommentRequest>,
) -> Result<(StatusCode, Json<db::models::VoteComment>), (StatusCode, Json<ErrorResponse>)> {
    let data = CreateVoteComment {
        vote_id: id,
        user_id: rls.user_id(),
        parent_id: req.parent_id,
        content: req.content,
        ai_consent: req.ai_consent,
    };

    // Note: add_comment() doesn't have an RLS version yet
    let comment = state.vote_repo.add_comment(data).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to add comment: {}", e),
            )),
        )
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(comment)))
}

/// List comments for a vote.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "List of comments", body = Vec<VoteCommentWithUser>),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn list_comments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VoteCommentWithUser>>, (StatusCode, Json<ErrorResponse>)> {
    // Note: list_comments() doesn't have an RLS version yet
    let comments = state
        .vote_repo
        .list_comments(id, false)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to list comments: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(comments))
}

/// List replies to a comment.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/comments/{comment_id}/replies",
    params(
        ("id" = Uuid, Path, description = "Vote ID"),
        ("comment_id" = Uuid, Path, description = "Comment ID")
    ),
    responses(
        (status = 200, description = "List of replies", body = Vec<VoteCommentWithUser>),
        (status = 404, description = "Vote or comment not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn list_replies(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<VoteCommentWithUser>>, (StatusCode, Json<ErrorResponse>)> {
    // Note: list_replies() doesn't have an RLS version yet
    let replies = state
        .vote_repo
        .list_replies(comment_id, false)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to list replies: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(replies))
}

/// Hide a comment.
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/comments/{comment_id}/hide",
    params(
        ("id" = Uuid, Path, description = "Vote ID"),
        ("comment_id" = Uuid, Path, description = "Comment ID")
    ),
    request_body = HideCommentRequest,
    responses(
        (status = 200, description = "Comment hidden", body = db::models::VoteComment),
        (status = 404, description = "Vote or comment not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn hide_comment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<HideCommentRequest>,
) -> Result<Json<db::models::VoteComment>, (StatusCode, Json<ErrorResponse>)> {
    let data = HideVoteComment { reason: req.reason };

    // Note: hide_comment() doesn't have an RLS version yet
    let comment = state
        .vote_repo
        .hide_comment(comment_id, rls.user_id(), data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to hide comment: {}", e),
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(comment))
}

// ============================================================================
// Results Handlers
// ============================================================================

/// Get vote results.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/results",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "Vote results", body = VoteResults),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn get_results(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VoteResults>, (StatusCode, Json<ErrorResponse>)> {
    let results = state
        .vote_repo
        .get_poll_results_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to get results: {}", e),
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    rls.release().await;
    Ok(Json(results))
}

// ============================================================================
// Report Handlers
// ============================================================================

/// Request query for report formatting.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportQuery {
    pub format: Option<String>,
}

/// Get vote report PDF.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/report.pdf",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "Report PDF", body = Vec<u8>),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn get_report_pdf(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let pdf_bytes = generate_and_archive_pdf(&state, &mut rls, id).await?;
    rls.release().await;
    
    let headers = [
        (axum::http::header::CONTENT_TYPE, "application/pdf"),
        (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"report.pdf\""),
    ];
    Ok((StatusCode::OK, headers, pdf_bytes).into_response())
}

/// Get vote report data (stub - returns data for PDF generation).
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/report",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "Report data", body = VoteReportData),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn get_report_data(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ReportQuery>,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    // Note: generate_report_data() doesn't have an RLS version yet
    let report = state
        .vote_repo
        .generate_report_data(id)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to generate report: {}", e),
                )),
            ),
        })?;

    // Secure by default: check organization matches RLS tenant context
    if report.vote.organization_id != rls.tenant_id() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
        ));
    }

    // Check if PDF format is requested via query param or Accept header
    let accept_header = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let is_pdf = query.format.as_deref() == Some("pdf") || accept_header == "application/pdf";

    if is_pdf {
        let pdf_bytes = generate_and_archive_pdf_internal(&state, &mut rls, &report).await?;
        rls.release().await;
        let pdf_headers = [
            (axum::http::header::CONTENT_TYPE, "application/pdf"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"report.pdf\""),
        ];
        Ok((StatusCode::OK, pdf_headers, pdf_bytes).into_response())
    } else {
        rls.release().await;
        Ok(Json(report).into_response())
    }
}

async fn generate_and_archive_pdf(
    state: &AppState,
    rls: &mut RlsConnection,
    id: Uuid,
) -> Result<Vec<u8>, (StatusCode, Json<ErrorResponse>)> {
    let report = state
        .vote_repo
        .generate_report_data(id)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to generate report: {}", e),
                )),
            ),
        })?;

    // Secure by default: check organization matches RLS tenant context
    if report.vote.organization_id != rls.tenant_id() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
        ));
    }

    generate_and_archive_pdf_internal(state, rls, &report).await
}

async fn generate_and_archive_pdf_internal(
    state: &AppState,
    rls: &mut RlsConnection,
    report: &VoteReportData,
) -> Result<Vec<u8>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Render the report to PDF
    let pdf_bytes = render_pdf_report(report).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "PDF_GENERATION_ERROR",
                format!("Failed to render PDF: {}", e),
            )),
        )
    })?;

    // Archive the PDF in the documents system
    let file_name = format!("vote_report_{}.pdf", report.vote.id);
    let file_key = integrations::generate_storage_key(org_id, &file_name);

    if let Some(ref storage_service) = state.storage_service {
        if storage_service.has_s3_client() {
            storage_service
                .upload(&file_key, pdf_bytes.clone(), "application/pdf")
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "STORAGE_ERROR",
                            format!("Failed to upload report to S3: {}", e),
                        )),
                    )
                })?;
        }
    }

    let create_doc = db::models::CreateDocument {
        organization_id: org_id,
        folder_id: None,
        title: format!("Vote Report: {}", report.vote.title),
        description: Some(format!("Auto-generated PDF report for vote: {}", report.vote.title)),
        category: "reports".to_string(), // db::models::document_category::REPORTS
        file_key: file_key.clone(),
        file_name,
        mime_type: "application/pdf".to_string(),
        size_bytes: pdf_bytes.len() as i64,
        access_scope: Some("organization".to_string()),
        access_target_ids: None,
        access_roles: None,
        created_by: user_id,
    };

    state
        .document_repo
        .create_rls(&mut **rls.conn(), create_doc)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to create document record for report: {}", e),
                )),
            )
        })?;

    Ok(pdf_bytes)
}

fn render_pdf_report(report: &VoteReportData) -> Result<Vec<u8>, String> {
    let font_family = genpdf::fonts::from_files("/usr/share/fonts/truetype/liberation", "LiberationSans", None)
        .map_err(|e| format!("Failed to load font LiberationSans: {}", e))?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(format!("Vote Report: {}", report.vote.title));

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    // Document Title
    doc.push(
        genpdf::elements::Paragraph::new(&report.vote.title)
            .styled(genpdf::style::Style::new().bold().with_font_size(18)),
    );

    // Spacing
    doc.push(genpdf::elements::Break::new(1));

    // Metadata
    let mut metadata_layout = genpdf::elements::LinearLayout::vertical();
    
    let generated_at_str = report.generated_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    
    metadata_layout.push(
        genpdf::elements::Paragraph::default()
            .string("Vote ID: ")
            .styled_string(report.vote.id.to_string(), genpdf::style::Style::new().bold())
    );
    metadata_layout.push(
        genpdf::elements::Paragraph::default()
            .string("Building ID: ")
            .styled_string(report.vote.building_id.to_string(), genpdf::style::Style::new().bold())
    );
    metadata_layout.push(
        genpdf::elements::Paragraph::default()
            .string("Quorum Type: ")
            .styled_string(report.vote.quorum_type.clone(), genpdf::style::Style::new().bold())
    );
    if let Some(qp) = report.vote.quorum_percentage {
        metadata_layout.push(
            genpdf::elements::Paragraph::default()
                .string("Quorum Percentage: ")
                .styled_string(format!("{}%", qp), genpdf::style::Style::new().bold())
        );
    }
    
    // Status
    metadata_layout.push(
        genpdf::elements::Paragraph::default()
            .string("Status: ")
            .styled_string(report.vote.status.clone(), genpdf::style::Style::new().bold())
    );
    
    // Generated At Timestamp
    metadata_layout.push(
        genpdf::elements::Paragraph::default()
            .string("Report Generated At: ")
            .styled_string(generated_at_str, genpdf::style::Style::new().bold())
    );
    
    doc.push(metadata_layout);
    doc.push(genpdf::elements::Break::new(2));

    // Description
    if let Some(desc) = &report.vote.description {
        doc.push(
            genpdf::elements::Paragraph::new("Description")
                .styled(genpdf::style::Style::new().bold().with_font_size(14)),
        );
        doc.push(genpdf::elements::Paragraph::new(desc));
        doc.push(genpdf::elements::Break::new(2));
    }

    // Results section
    doc.push(
        genpdf::elements::Paragraph::new("Voting Results Summary")
            .styled(genpdf::style::Style::new().bold().with_font_size(14)),
    );
    
    if let Some(results) = &report.results {
        let quorum_met_str = if results.quorum_met { "YES" } else { "NO" };
        let part_rate = format!("{:.2}%", results.participation_rate * 100.0);
        
        let mut results_summary = genpdf::elements::LinearLayout::vertical();
        results_summary.push(
            genpdf::elements::Paragraph::default()
                .string("Quorum Met: ")
                .styled_string(quorum_met_str, genpdf::style::Style::new().bold())
        );
        results_summary.push(
            genpdf::elements::Paragraph::default()
                .string("Participation: ")
                .styled_string(
                    format!("{} of {} units (Rate: {})", results.participation_count, results.eligible_count, part_rate),
                    genpdf::style::Style::new().bold()
                )
        );
        doc.push(results_summary);
        doc.push(genpdf::elements::Break::new(1));
        
        // Questions Results Detail
        for (idx, q_res) in results.questions.iter().enumerate() {
            doc.push(
                genpdf::elements::Paragraph::new(format!("Question {}: {}", idx + 1, q_res.question_text))
                    .styled(genpdf::style::Style::new().bold().with_font_size(12)),
            );
            doc.push(genpdf::elements::Paragraph::new(format!("Type: {}  |  Total Ballots: {}  |  Weighted Total: {}", q_res.question_type, q_res.total_votes, q_res.weighted_total)));
            
            doc.push(genpdf::elements::Break::new(1));
            
            // Results table
            // Table header: Option, Votes, Weighted Count, Percentage
            let mut table = genpdf::elements::TableLayout::new(vec![3, 1, 2, 2]);
            let decorator = genpdf::elements::FrameCellDecorator::new(true, true, false);
            table.set_cell_decorator(decorator);
            
            table.row()
                .element(genpdf::elements::Paragraph::new("Option").styled(genpdf::style::Style::new().bold()))
                .element(genpdf::elements::Paragraph::new("Count").styled(genpdf::style::Style::new().bold()))
                .element(genpdf::elements::Paragraph::new("Weighted").styled(genpdf::style::Style::new().bold()))
                .element(genpdf::elements::Paragraph::new("Percentage").styled(genpdf::style::Style::new().bold()))
                .push()
                .map_err(|e| format!("Failed to add table header: {}", e))?;
            
            for opt in &q_res.results {
                table.row()
                    .element(genpdf::elements::Paragraph::new(&opt.option_text))
                    .element(genpdf::elements::Paragraph::new(opt.count.to_string()))
                    .element(genpdf::elements::Paragraph::new(format!("{:.4}", opt.weighted_count)))
                    .element(genpdf::elements::Paragraph::new(format!("{:.2}%", opt.percentage)))
                    .push()
                    .map_err(|e| format!("Failed to add table row: {}", e))?;
            }
            
            doc.push(table);
            doc.push(genpdf::elements::Break::new(2));
        }
    } else {
        doc.push(genpdf::elements::Paragraph::new("No results calculated yet."));
        doc.push(genpdf::elements::Break::new(2));
    }

    // Participation Details section
    doc.push(
        genpdf::elements::Paragraph::new("Participation Details")
            .styled(genpdf::style::Style::new().bold().with_font_size(14)),
    );
    doc.push(genpdf::elements::Break::new(1));
    
    // Table header: Unit, Voted, Weight
    let mut part_table = genpdf::elements::TableLayout::new(vec![4, 2, 2]);
    let decorator = genpdf::elements::FrameCellDecorator::new(true, true, false);
    part_table.set_cell_decorator(decorator);
    
    part_table.row()
        .element(genpdf::elements::Paragraph::new("Unit Designation").styled(genpdf::style::Style::new().bold()))
        .element(genpdf::elements::Paragraph::new("Voted").styled(genpdf::style::Style::new().bold()))
        .element(genpdf::elements::Paragraph::new("Vote Weight").styled(genpdf::style::Style::new().bold()))
        .push()
        .map_err(|e| format!("Failed to add participation header: {}", e))?;
    
    for detail in &report.participation_details {
        part_table.row()
            .element(genpdf::elements::Paragraph::new(&detail.unit_designation))
            .element(genpdf::elements::Paragraph::new(if detail.voted { "Yes" } else { "No" }))
            .element(genpdf::elements::Paragraph::new(detail.vote_weight.to_string()))
            .push()
            .map_err(|e| format!("Failed to add participation row: {}", e))?;
    }
    
    doc.push(part_table);

    // Render to bytes
    let mut bytes = Vec::new();
    doc.render(&mut bytes).map_err(|e| format!("Failed to render PDF: {}", e))?;

    Ok(bytes)
}

// ============================================================================
// Audit Handlers
// ============================================================================

/// Get vote audit log.
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}/audit",
    params(
        ("id" = Uuid, Path, description = "Vote ID")
    ),
    responses(
        (status = 200, description = "Audit log entries", body = Vec<VoteAuditLog>),
        (status = 404, description = "Vote not found"),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn get_audit_log(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VoteAuditLog>>, (StatusCode, Json<ErrorResponse>)> {
    // Note: get_audit_log() doesn't have an RLS version yet
    let entries = state.vote_repo.get_audit_log(id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                format!("Failed to get audit log: {}", e),
            )),
        )
    })?;

    rls.release().await;
    Ok(Json(entries))
}

// ============================================================================
// Building-specific Handlers
// ============================================================================

/// List active votes for a building.
#[utoipa::path(
    get,
    path = "/api/v1/voting/building/{building_id}/active",
    params(
        ("building_id" = Uuid, Path, description = "Building ID")
    ),
    responses(
        (status = 200, description = "List of active votes", body = Vec<VoteSummary>),
        (status = 401, description = "Unauthorized"),
    ),
    tag = "Voting"
)]
async fn list_active_by_building(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(building_id): Path<Uuid>,
) -> Result<Json<Vec<VoteSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let votes = state
        .vote_repo
        .list_polls_by_building_rls(&mut **rls.conn(), building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    format!("Failed to list votes: {}", e),
                )),
            )
        })?;

    // Filter to active only (RLS version returns all statuses)
    let active_votes: Vec<_> = votes.into_iter().filter(|v| v.status == "active").collect();

    rls.release().await;
    Ok(Json(active_votes))
}

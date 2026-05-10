//! Voting routes (Epic 5: Building Voting & Decisions).

use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use common::errors::ErrorResponse;
use db::models::{
    CancelVote, CastVote, CreateVote, CreateVoteComment, CreateVoteQuestion, HideVoteComment,
    PublishVote, QuestionOption, UpdateVote, UpdateVoteQuestion, Vote, VoteAuditLog,
    VoteCommentWithUser, VoteEligibility, VoteListQuery, VoteQuestion, VoteReceipt, VoteReportData,
    VoteResults, VoteSummary, VoteWithDetails,
};
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
    // Note: find_by_id_with_details doesn't have an RLS version yet
    let vote = state
        .vote_repo
        .find_by_id_with_details(id)
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

    let data = CastVote {
        vote_id: id,
        user_id: rls.user_id(),
        unit_id: req.unit_id,
        delegation_id: req.delegation_id,
        answers: req.answers,
    };

    let receipt = state
        .vote_repo
        .cast_vote_rls(&mut **rls.conn(), data)
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
) -> Result<Json<VoteReportData>, (StatusCode, Json<ErrorResponse>)> {
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

    rls.release().await;
    Ok(Json(report))
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

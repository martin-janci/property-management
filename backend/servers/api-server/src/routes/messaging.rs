//! Messaging routes (Epic 6, Story 6.5: Direct Messaging).

use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    BlockWithUserInfo, CreateBlock, CreateMessage, CreateThread, Message, MessageThread,
    MessageWithSender, ParticipantInfo, ThreadWithPreview,
};
use db::repositories::MessagingRepository;
use serde::{Deserialize, Serialize};
use sqlx::Error as SqlxError;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed message content length (characters).
const MAX_MESSAGE_LENGTH: usize = 10_000;

/// Maximum length of the message preview embedded in the realtime WebSocket
/// event (characters). Keeps the pub/sub payload small; full content is
/// fetched via the thread endpoint.
const MESSAGE_PREVIEW_LEN: usize = 120;

// ============================================================================
// Response Types
// ============================================================================

/// Response for thread list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ThreadListResponse {
    pub threads: Vec<ThreadWithPreview>,
    pub count: usize,
    pub total: i64,
}

/// Response for thread detail with messages.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ThreadDetailResponse {
    pub thread: MessageThread,
    pub other_participant: ParticipantInfo,
    pub messages: Vec<MessageWithSender>,
    pub message_count: i64,
}

/// Response for message creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SendMessageResponse {
    pub message: String,
    pub sent_message: Message,
}

/// Response for unread count.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnreadMessagesResponse {
    pub unread_count: i64,
}

/// Response for blocked users list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BlockedUsersResponse {
    pub blocked_users: Vec<BlockWithUserInfo>,
    pub count: usize,
}

/// Generic success response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageSuccessResponse {
    pub message: String,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for starting a new thread.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StartThreadRequest {
    /// The user ID to start a conversation with.
    pub recipient_id: Uuid,
    /// Optional initial message.
    pub initial_message: Option<String>,
}

/// Request for sending a message.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
}

/// Query for listing threads.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListThreadsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Optional case-insensitive filter on the other participant's name.
    pub search: Option<String>,
}

/// Query for listing messages.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListMessagesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Router
// ============================================================================

/// Create the messaging router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Thread endpoints
        .route("/threads", get(list_threads))
        .route("/threads", post(start_thread))
        .route("/threads/{id}", get(get_thread))
        .route("/threads/{id}/messages", post(send_message))
        .route(
            "/threads/{id}/messages/{message_id}",
            delete(delete_message),
        )
        .route("/threads/{id}/read", post(mark_thread_read))
        // Block endpoints
        .route("/users/blocked", get(list_blocked_users))
        .route("/users/{id}/block", post(block_user))
        .route("/users/{id}/block", delete(unblock_user))
        // Stats
        .route("/unread-count", get(get_unread_count))
}

// ============================================================================
// Thread Handlers
// ============================================================================

/// List message threads for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/messages/threads",
    params(ListThreadsQuery),
    responses(
        (status = 200, description = "Threads retrieved successfully", body = ThreadListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn list_threads(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListThreadsQuery>,
) -> Result<Json<ThreadListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = MessagingRepository::new(state.db.clone());
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    // Treat an empty/whitespace-only search string as no filter.
    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let threads = repo
        .list_threads_rls(
            &mut **rls.conn(),
            user_id,
            tenant_id,
            query.limit,
            query.offset,
            search,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to list threads: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let total = repo
        .count_threads_rls(&mut **rls.conn(), user_id, tenant_id, search)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count threads: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(ThreadListResponse {
        count: threads.len(),
        threads,
        total,
    }))
}

/// Start a new thread or get existing thread with another user.
#[utoipa::path(
    post,
    path = "/api/v1/messages/threads",
    request_body = StartThreadRequest,
    responses(
        (status = 200, description = "Thread created or retrieved", body = ThreadDetailResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "User is blocked", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn start_thread(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(body): Json<StartThreadRequest>,
) -> Result<Json<ThreadDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    // Can't message yourself
    if body.recipient_id == user_id {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_RECIPIENT",
                "Cannot start a conversation with yourself",
            )),
        ));
    }

    // Security: Verify recipient is in same organization (Critical 1.1 / 2.3 fix)
    let recipient_org: Option<(Uuid,)> =
        sqlx::query_as("SELECT organization_id FROM users WHERE id = $1")
            .bind(body.recipient_id)
            .fetch_optional(&mut **rls.conn())
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", e.to_string())),
                )
            })?;

    match recipient_org {
        None => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("USER_NOT_FOUND", "Recipient not found")),
            ));
        }
        Some((org_id,)) if org_id != tenant_id => {
            rls.release().await;
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "CROSS_ORG_DENIED",
                    "Cannot message users from different organizations",
                )),
            ));
        }
        _ => {} // Same org, continue
    }

    let repo = MessagingRepository::new(state.db.clone());

    // Check if either user has blocked the other
    let is_blocked = repo
        .is_blocked_rls(&mut **rls.conn(), user_id, body.recipient_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check block status: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    if is_blocked {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "USER_BLOCKED",
                "Cannot message this user",
            )),
        ));
    }

    // Get or create thread
    let thread = repo
        .get_or_create_thread_rls(
            &mut **rls.conn(),
            CreateThread {
                organization_id: tenant_id,
                participant_ids: vec![user_id, body.recipient_id],
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create thread: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    // Send initial message if provided
    if let Some(content) = body.initial_message {
        if content.len() > MAX_MESSAGE_LENGTH {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "MESSAGE_TOO_LONG",
                    format!("Message cannot exceed {} characters", MAX_MESSAGE_LENGTH),
                )),
            ));
        }

        let initial = repo
            .create_message_rls(
                &mut **rls.conn(),
                CreateMessage {
                    thread_id: thread.id,
                    sender_id: user_id,
                    content,
                },
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to send initial message: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", e.to_string())),
                )
            })?;

        // Realtime fanout: notify the recipient's WebSocket channel (Epic 2B / 8A.3).
        dispatch_new_message_event(
            state.pubsub_service.as_ref(),
            body.recipient_id,
            &initial,
            user_id,
        )
        .await;
    }

    // Get messages and other participant info
    let messages = repo
        .get_thread_messages_rls(&mut **rls.conn(), thread.id, Some(50), None)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let message_count = repo
        .count_thread_messages_rls(&mut **rls.conn(), thread.id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    // Get other participant info
    let other_participant = get_other_participant(&mut rls, &thread, user_id).await?;

    rls.release().await;

    Ok(Json(ThreadDetailResponse {
        thread,
        other_participant,
        messages,
        message_count,
    }))
}

/// Get thread details with messages.
#[utoipa::path(
    get,
    path = "/api/v1/messages/threads/{id}",
    params(
        ("id" = Uuid, Path, description = "Thread ID"),
        ListMessagesQuery,
    ),
    responses(
        (status = 200, description = "Thread retrieved successfully", body = ThreadDetailResponse),
        (status = 403, description = "Not a participant", body = ErrorResponse),
        (status = 404, description = "Thread not found", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn get_thread(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<ThreadDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = MessagingRepository::new(state.db.clone());
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    // Get thread
    let thread = repo
        .get_thread_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let thread = thread.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Thread not found")),
        )
    })?;

    // Security: Verify thread belongs to current tenant (Critical 1.1 fix)
    if thread.organization_id != tenant_id {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Access denied to this thread",
            )),
        ));
    }

    // Check if user is a participant
    if !thread.participant_ids.contains(&user_id) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_PARTICIPANT",
                "You are not a participant in this thread",
            )),
        ));
    }

    // Get messages
    let messages = repo
        .get_thread_messages_rls(&mut **rls.conn(), id, query.limit, query.offset)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let message_count = repo
        .count_thread_messages_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    // Get other participant info
    let other_participant = get_other_participant(&mut rls, &thread, user_id).await?;

    // Mark thread as read
    let _ = repo
        .mark_thread_read_rls(&mut **rls.conn(), id, user_id)
        .await;

    rls.release().await;

    Ok(Json(ThreadDetailResponse {
        thread,
        other_participant,
        messages,
        message_count,
    }))
}

/// Send a message in a thread.
#[utoipa::path(
    post,
    path = "/api/v1/messages/threads/{id}/messages",
    params(
        ("id" = Uuid, Path, description = "Thread ID"),
    ),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent successfully", body = SendMessageResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not a participant or blocked", body = ErrorResponse),
        (status = 404, description = "Thread not found", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn send_message(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    // Validate content
    if body.content.trim().is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "EMPTY_MESSAGE",
                "Message cannot be empty",
            )),
        ));
    }

    if body.content.len() > MAX_MESSAGE_LENGTH {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "MESSAGE_TOO_LONG",
                format!("Message cannot exceed {} characters", MAX_MESSAGE_LENGTH),
            )),
        ));
    }

    let repo = MessagingRepository::new(state.db.clone());

    // Get thread and verify participation
    let thread = repo
        .get_thread_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let thread = thread.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Thread not found")),
        )
    })?;

    // Security: Verify thread belongs to current tenant (Critical 1.1 fix)
    if thread.organization_id != tenant_id {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Access denied to this thread",
            )),
        ));
    }

    if !thread.participant_ids.contains(&user_id) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_PARTICIPANT",
                "You are not a participant in this thread",
            )),
        ));
    }

    // Check if blocked
    let other_user_id = thread
        .participant_ids
        .iter()
        .find(|&&uid| uid != user_id)
        .copied()
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INVALID_THREAD",
                    "Thread has invalid participants",
                )),
            )
        })?;

    let is_blocked = repo
        .is_blocked_rls(&mut **rls.conn(), user_id, other_user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    if is_blocked {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "USER_BLOCKED",
                "Cannot message this user",
            )),
        ));
    }

    // Send message
    let message = repo
        .create_message_rls(
            &mut **rls.conn(),
            CreateMessage {
                thread_id: id,
                sender_id: user_id,
                content: body.content,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to send message: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    // Realtime fanout: notify the recipient's WebSocket channel (Epic 2B / 8A.3).
    dispatch_new_message_event(
        state.pubsub_service.as_ref(),
        other_user_id,
        &message,
        user_id,
    )
    .await;

    Ok(Json(SendMessageResponse {
        message: "Message sent successfully".to_string(),
        sent_message: message,
    }))
}

/// Delete a message in a thread (soft delete).
///
/// Only the sender of a message may delete it. The message must belong to the
/// given thread, the thread must belong to the caller's tenant, and the caller
/// must be a participant.
#[utoipa::path(
    delete,
    path = "/api/v1/messages/threads/{id}/messages/{message_id}",
    params(
        ("id" = Uuid, Path, description = "Thread ID"),
        ("message_id" = Uuid, Path, description = "Message ID"),
    ),
    responses(
        (status = 200, description = "Message deleted successfully", body = MessageSuccessResponse),
        (status = 403, description = "Not a participant or not the sender", body = ErrorResponse),
        (status = 404, description = "Thread or message not found", body = ErrorResponse),
    ),
    tag = "messaging"
)]
pub async fn delete_message(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MessageSuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = MessagingRepository::new(state.db.clone());
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    // Get thread and verify it exists
    let thread = repo
        .get_thread_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let thread = thread.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Thread not found")),
        )
    })?;

    // Security: Verify thread belongs to current tenant (Critical 1.1 fix)
    if thread.organization_id != tenant_id {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Access denied to this thread",
            )),
        ));
    }

    if !thread.participant_ids.contains(&user_id) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_PARTICIPANT",
                "You are not a participant in this thread",
            )),
        ));
    }

    // Fetch the target message and verify it belongs to this thread and that
    // the caller is its sender.
    let target: Option<(Uuid, Uuid)> = sqlx::query_as(
        r#"
        SELECT thread_id, sender_id FROM messages WHERE id = $1
        "#,
    )
    .bind(message_id)
    .fetch_optional(&mut **rls.conn())
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", e.to_string())),
        )
    })?;

    let (msg_thread_id, msg_sender_id) = target.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Message not found")),
        )
    })?;

    if msg_thread_id != id || msg_sender_id != user_id {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You can only delete your own messages",
            )),
        ));
    }

    repo.delete_message_rls(&mut **rls.conn(), message_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete message: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(MessageSuccessResponse {
        message: "Message deleted successfully".to_string(),
    }))
}

/// Mark all messages in a thread as read.
#[utoipa::path(
    post,
    path = "/api/v1/messages/threads/{id}/read",
    params(
        ("id" = Uuid, Path, description = "Thread ID"),
    ),
    responses(
        (status = 200, description = "Thread marked as read", body = MessageSuccessResponse),
        (status = 403, description = "Not a participant", body = ErrorResponse),
        (status = 404, description = "Thread not found", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn mark_thread_read(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<MessageSuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = MessagingRepository::new(state.db.clone());
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    // Get thread to verify tenant (Critical 1.1 fix)
    let thread = repo
        .get_thread_rls(&mut **rls.conn(), id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    let thread = thread.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Thread not found")),
        )
    })?;

    // Security: Verify thread belongs to current tenant
    if thread.organization_id != tenant_id {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Access denied to this thread",
            )),
        ));
    }

    // Check if user is participant
    if !thread.participant_ids.contains(&user_id) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NOT_PARTICIPANT",
                "You are not a participant in this thread",
            )),
        ));
    }

    let marked = repo
        .mark_thread_read_rls(&mut **rls.conn(), id, user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(MessageSuccessResponse {
        message: format!("{} messages marked as read", marked),
    }))
}

// ============================================================================
// Block Handlers
// ============================================================================

/// List blocked users.
#[utoipa::path(
    get,
    path = "/api/v1/messages/users/blocked",
    responses(
        (status = 200, description = "Blocked users retrieved", body = BlockedUsersResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn list_blocked_users(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<BlockedUsersResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = MessagingRepository::new(state.db.clone());
    let user_id = rls.user_id();

    let blocked_users = repo
        .list_blocked_users_rls(&mut **rls.conn(), user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(BlockedUsersResponse {
        count: blocked_users.len(),
        blocked_users,
    }))
}

/// Block a user.
#[utoipa::path(
    post,
    path = "/api/v1/messages/users/{id}/block",
    params(
        ("id" = Uuid, Path, description = "User ID to block"),
    ),
    responses(
        (status = 200, description = "User blocked", body = MessageSuccessResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn block_user(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(user_id_to_block): Path<Uuid>,
) -> Result<Json<MessageSuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    // Can't block yourself
    if user_id_to_block == user_id {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_BLOCK", "Cannot block yourself")),
        ));
    }

    let repo = MessagingRepository::new(state.db.clone());

    repo.block_user_rls(
        &mut **rls.conn(),
        CreateBlock {
            blocker_id: user_id,
            blocked_id: user_id_to_block,
            organization_id: tenant_id,
        },
    )
    .await
    .map_err(|e| match e {
        SqlxError::Protocol(msg) if msg.contains("already blocked") => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("ALREADY_BLOCKED", msg)),
        ),
        _ => {
            tracing::error!("Failed to block user: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        }
    })?;

    rls.release().await;

    Ok(Json(MessageSuccessResponse {
        message: "User blocked successfully".to_string(),
    }))
}

/// Unblock a user.
#[utoipa::path(
    delete,
    path = "/api/v1/messages/users/{id}/block",
    params(
        ("id" = Uuid, Path, description = "User ID to unblock"),
    ),
    responses(
        (status = 200, description = "User unblocked", body = MessageSuccessResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn unblock_user(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(user_id_to_unblock): Path<Uuid>,
) -> Result<Json<MessageSuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = MessagingRepository::new(state.db.clone());
    let user_id = rls.user_id();

    repo.unblock_user_rls(&mut **rls.conn(), user_id, user_id_to_unblock)
        .await
        .map_err(|e| {
            tracing::error!("Failed to unblock user: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(MessageSuccessResponse {
        message: "User unblocked successfully".to_string(),
    }))
}

// ============================================================================
// Stats Handlers
// ============================================================================

/// Get unread message count.
#[utoipa::path(
    get,
    path = "/api/v1/messages/unread-count",
    responses(
        (status = 200, description = "Unread count retrieved", body = UnreadMessagesResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "messaging"
)]
async fn get_unread_count(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<UnreadMessagesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = MessagingRepository::new(state.db.clone());
    let user_id = rls.user_id();
    let tenant_id = rls.tenant_id();

    let unread_count = repo
        .count_unread_rls(&mut **rls.conn(), user_id, tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;

    Ok(Json(UnreadMessagesResponse { unread_count }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the other participant's info from a thread.
async fn get_other_participant(
    rls: &mut RlsConnection,
    thread: &MessageThread,
    current_user_id: Uuid,
) -> Result<ParticipantInfo, (StatusCode, Json<ErrorResponse>)> {
    let other_user_id = thread
        .participant_ids
        .iter()
        .find(|&&uid| uid != current_user_id)
        .copied()
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INVALID_THREAD",
                    "Thread has invalid participants",
                )),
            )
        })?;

    // Get user info. Issue #1008: `users` has a single `name` column — map it
    // into first_name and leave last_name empty to keep the ParticipantInfo shape.
    let user = sqlx::query_as::<_, (Uuid, String, String, String)>(
        r#"
        SELECT id, name AS first_name, '' AS last_name, email FROM users WHERE id = $1
        "#,
    )
    .bind(other_user_id)
    .fetch_optional(&mut **rls.conn())
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", e.to_string())),
        )
    })?;

    let (id, first_name, last_name, email) = user.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "USER_NOT_FOUND",
                "Participant not found",
            )),
        )
    })?;

    Ok(ParticipantInfo {
        id,
        first_name,
        last_name,
        email,
    })
}

/// Build the realtime WebSocket event payload for a newly-sent direct message.
///
/// The payload mirrors the shape consumed by the `notifications:{user_id}`
/// WebSocket channel (see `routes/ws_notifications.rs`). It carries enough
/// for a client to badge/refresh the conversation without leaking the full
/// message body — the preview is truncated to `MESSAGE_PREVIEW_LEN` chars.
fn new_message_event_payload(message: &Message, sender_id: Uuid) -> serde_json::Value {
    let preview: String = message.content.chars().take(MESSAGE_PREVIEW_LEN).collect();
    serde_json::json!({
        "message_id": message.id,
        "thread_id": message.thread_id,
        "sender_id": sender_id,
        "preview": preview,
    })
}

/// Publish a `message.created` realtime event to the recipient's WebSocket
/// notification channel (Epic 2B / Story 8A.3 fanout pattern).
///
/// Best-effort: the message has already been persisted, so a pub/sub failure
/// is logged and swallowed rather than failing the request. No-op when Redis
/// pub/sub is not configured (local dev without Redis).
async fn dispatch_new_message_event(
    pubsub: Option<&integrations::PubSubService>,
    recipient_id: Uuid,
    message: &Message,
    sender_id: Uuid,
) {
    let Some(pubsub) = pubsub else {
        return;
    };
    let channel = format!("notifications:{recipient_id}");
    let msg = integrations::PubSubMessage::new(
        &channel,
        "message.created",
        new_message_event_payload(message, sender_id),
    );
    if let Err(e) = pubsub.publish(&channel, msg).await {
        // Non-fatal: the message is already persisted; realtime fanout is
        // best-effort and the recipient will still see it on next fetch.
        tracing::warn!(
            recipient_id = %recipient_id,
            channel = %channel,
            error = %e,
            "[messaging] Failed to publish message.created to WebSocket channel (non-fatal)"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_message(content: &str) -> Message {
        let now = Utc::now();
        Message {
            id: Uuid::new_v4(),
            thread_id: Uuid::new_v4(),
            sender_id: Uuid::new_v4(),
            content: content.to_string(),
            read_at: None,
            deleted_at: None,
            deleted_by: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn event_payload_carries_ids_and_sender() {
        let msg = sample_message("hello there");
        let sender = Uuid::new_v4();
        let payload = new_message_event_payload(&msg, sender);

        assert_eq!(payload["message_id"], serde_json::json!(msg.id));
        assert_eq!(payload["thread_id"], serde_json::json!(msg.thread_id));
        assert_eq!(payload["sender_id"], serde_json::json!(sender));
        assert_eq!(payload["preview"], serde_json::json!("hello there"));
    }

    #[test]
    fn event_payload_truncates_long_content_to_preview_len() {
        let long = "x".repeat(MESSAGE_PREVIEW_LEN + 500);
        let msg = sample_message(&long);
        let payload = new_message_event_payload(&msg, Uuid::new_v4());

        let preview = payload["preview"].as_str().expect("preview is a string");
        assert_eq!(preview.chars().count(), MESSAGE_PREVIEW_LEN);
        // Must not leak the full body over the wire.
        assert!(preview.len() < long.len());
    }
}

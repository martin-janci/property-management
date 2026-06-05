//! Announcement comment routes (Story 6.3) - list, create, delete.

use super::shared::*;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use common::notifications::{Notification, NotificationCategory};
use db::models::{AnnouncementComment, CreateComment, DeleteComment};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Comments sub-router (list, create, delete).
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/comments", get(list_comments))
        .route("/{id}/comments", post(create_comment))
        .route("/{id}/comments/{comment_id}", delete(delete_comment))
}

/// List comments for an announcement.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Announcement ID"),
        ListCommentsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Comment list", body = CommentsResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn list_comments(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<ListCommentsQuery>,
) -> Result<Json<CommentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check announcement exists
    match state
        .announcement_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Announcement not found")),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to find announcement: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find announcement",
                )),
            ));
        }
        Ok(Some(_)) => {}
    }

    // Get total count
    let total = match state
        .announcement_repo
        .get_comment_count_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to get comment count: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get comment count",
                )),
            ));
        }
    };

    // Fetch top-level comments through the RLS-scoped connection so the DB
    // policy blocks cross-tenant access. get_threaded_comments uses self.pool
    // (no RLS) and is intentionally not used here.
    let top_level = match state
        .announcement_repo
        .get_comments_rls(&mut **rls.conn(), id, query.limit, query.offset)
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to list comments: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list comments",
                )),
            ));
        }
    };

    // Fetch replies for each top-level comment through the same RLS connection.
    let mut comments = Vec::with_capacity(top_level.len());
    for row in top_level {
        let replies = match state
            .announcement_repo
            .get_comment_replies_rls(&mut **rls.conn(), row.id)
            .await
        {
            Ok(reply_rows) if !reply_rows.is_empty() => Some(
                reply_rows
                    .into_iter()
                    .map(|r| r.into_comment_with_author(None))
                    .collect(),
            ),
            Ok(_) => None,
            Err(e) => {
                rls.release().await;
                tracing::error!("Failed to get comment replies: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to get comment replies",
                    )),
                ));
            }
        };
        comments.push(row.into_comment_with_author(replies));
    }

    rls.release().await;

    Ok(Json(CommentsResponse {
        count: comments.len(),
        comments,
        total,
    }))
}

/// Create a comment on an announcement.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/comments",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    request_body = CreateCommentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Comment created", body = AnnouncementComment),
        (status = 400, description = "Invalid request or comments disabled", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn create_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<AnnouncementComment>), (StatusCode, Json<ErrorResponse>)> {
    // Validate content length
    if req.content.is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Comment content is required",
            )),
        ));
    }
    if req.content.len() > MAX_COMMENT_LENGTH {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Comment exceeds maximum length of {} characters",
                    MAX_COMMENT_LENGTH
                ),
            )),
        ));
    }

    // Check announcement exists and has comments enabled
    let announcement = match state
        .announcement_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(a)) => a,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Announcement not found")),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to find announcement: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find announcement",
                )),
            ));
        }
    };

    // Check comments are enabled
    if !announcement.comments_enabled {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "COMMENTS_DISABLED",
                "Comments are disabled for this announcement",
            )),
        ));
    }

    // Check announcement is published
    if !announcement.is_published() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Cannot comment on unpublished announcements",
            )),
        ));
    }

    // If parent_id is provided, verify it exists and belongs to same announcement.
    // Capture the parent comment's author so we can notify them about the reply
    // before `parent` drops at the end of this block (Story 6.3 AC).
    let mut parent_author: Option<Uuid> = None;
    if let Some(parent_id) = req.parent_id {
        match state
            .announcement_repo
            .get_comment_rls(&mut **rls.conn(), parent_id)
            .await
        {
            Ok(Some(parent)) => {
                if parent.announcement_id != id {
                    rls.release().await;
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "BAD_REQUEST",
                            "Parent comment belongs to different announcement",
                        )),
                    ));
                }
                // Only allow one level of nesting
                if parent.parent_id.is_some() {
                    rls.release().await;
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "BAD_REQUEST",
                            "Cannot reply to a reply - maximum nesting depth is 2",
                        )),
                    ));
                }
                parent_author = Some(parent.user_id);
            }
            Ok(None) => {
                rls.release().await;
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Parent comment not found")),
                ));
            }
            Err(e) => {
                rls.release().await;
                tracing::error!("Failed to find parent comment: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to find parent comment",
                    )),
                ));
            }
        }
    }

    // Sanitize content
    let sanitized_content = sanitize_markdown(&req.content);

    let data = CreateComment {
        announcement_id: id,
        user_id: auth.user_id,
        parent_id: req.parent_id,
        content: sanitized_content,
        ai_training_consent: req.ai_training_consent,
    };

    match state
        .announcement_repo
        .create_comment_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(comment) => {
            rls.release().await;
            tracing::info!(
                comment_id = %comment.id,
                announcement_id = %id,
                user_id = %auth.user_id,
                "Comment created"
            );

            // Story 6.3 AC: notify the announcement author and, for replies,
            // the parent comment's author. Exclude the actor (commenter). The
            // RLS connection is released above; the pipeline does not need it
            // (matches the publish_announcement ordering).
            let mut recipients: Vec<Uuid> = Vec::new();
            for candidate in [Some(announcement.author_id), parent_author]
                .into_iter()
                .flatten()
            {
                if candidate != auth.user_id && !recipients.contains(&candidate) {
                    recipients.push(candidate);
                }
            }

            if !recipients.is_empty() {
                // Short snippet of the comment body for the notification preview.
                let snippet: String = comment.content.chars().take(140).collect();
                let notification = Notification::new(
                    Uuid::nil(),
                    NotificationCategory::Announcements,
                    format!("New comment on {}", announcement.title),
                    snippet,
                )
                .with_action_url(format!("/announcements/{}", id))
                .with_data(serde_json::json!({
                    "announcement_id": id,
                    "comment_id": comment.id,
                }));

                let (sent, skipped, failed) = state
                    .notification_pipeline
                    .broadcast(&recipients, &notification, Some(id))
                    .await;

                tracing::info!(
                    comment_id = %comment.id,
                    announcement_id = %id,
                    recipients = recipients.len(),
                    sent,
                    skipped,
                    failed,
                    "Dispatched comment notifications"
                );
            }

            Ok((StatusCode::CREATED, Json(comment)))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to create comment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create comment",
                )),
            ))
        }
    }
}

/// Delete a comment (author or manager moderation).
#[utoipa::path(
    delete,
    path = "/api/v1/announcements/{id}/comments/{comment_id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID"),
        ("comment_id" = Uuid, Path, description = "Comment ID")
    ),
    request_body = Option<DeleteCommentRequest>,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Comment deleted", body = AnnouncementComment),
        (status = 403, description = "Not authorized to delete", body = ErrorResponse),
        (status = 404, description = "Comment not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn delete_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path((id, comment_id)): Path<(Uuid, Uuid)>,
    body: Option<Json<DeleteCommentRequest>>,
) -> Result<Json<AnnouncementComment>, (StatusCode, Json<ErrorResponse>)> {
    // Get the comment
    let comment = match state
        .announcement_repo
        .get_comment_rls(&mut **rls.conn(), comment_id)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Comment not found")),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to find comment: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to find comment")),
            ));
        }
    };

    // Verify comment belongs to the announcement
    if comment.announcement_id != id {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Comment not found")),
        ));
    }

    // Check if already deleted
    if comment.is_deleted() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Comment is already deleted",
            )),
        ));
    }

    // Authorization: author can delete their own, managers can delete any
    let is_author = comment.user_id == auth.user_id;
    let is_manager = tenant.role.is_manager();

    if !is_author && !is_manager {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You can only delete your own comments",
            )),
        ));
    }

    // Get deletion reason (only for manager moderation)
    let deletion_reason = if is_manager && !is_author {
        body.and_then(|b| b.reason.clone())
    } else {
        None
    };

    let data = DeleteComment {
        comment_id,
        deleted_by: auth.user_id,
        deletion_reason,
    };

    match state
        .announcement_repo
        .delete_comment_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(deleted) => {
            rls.release().await;
            tracing::info!(
                comment_id = %comment_id,
                deleted_by = %auth.user_id,
                is_moderation = %(!is_author && is_manager),
                "Comment deleted"
            );
            Ok(Json(deleted))
        }
        Err(SqlxError::RowNotFound) => {
            rls.release().await;
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Comment not found")),
            ))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to delete comment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete comment",
                )),
            ))
        }
    }
}

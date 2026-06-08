//! Announcement engagement routes - attachments, read/acknowledge (Story 6.2).

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
use db::models::{AnnouncementAttachment, CreateAnnouncementAttachment};
use uuid::Uuid;

/// Engagement sub-router (attachments + read/acknowledge).
pub(super) fn router() -> Router<AppState> {
    Router::new()
        // Attachments
        .route("/{id}/attachments", get(list_attachments))
        .route("/{id}/attachments", post(add_attachment))
        .route(
            "/{id}/attachments/{attachment_id}",
            delete(delete_attachment),
        )
        // Read/Acknowledge (Story 6.2)
        .route("/{id}/read", post(mark_read))
        .route("/{id}/acknowledge", post(acknowledge))
        .route("/{id}/acknowledgments", get(get_acknowledgments))
}

/// List attachments for an announcement.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/{id}/attachments",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Attachments list", body = AttachmentsResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn list_attachments(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<AttachmentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .announcement_repo
        .get_attachments_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(attachments) => {
            rls.release().await;
            Ok(Json(AttachmentsResponse { attachments }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to list attachments: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list attachments",
                )),
            ))
        }
    }
}

/// Add an attachment to an announcement.
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/attachments",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    request_body = AddAttachmentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Attachment added", body = AnnouncementAttachment),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn add_attachment(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<AddAttachmentRequest>,
) -> Result<(StatusCode, Json<AnnouncementAttachment>), (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can add attachments",
            )),
        ));
    }
    let data = CreateAnnouncementAttachment {
        announcement_id: id,
        file_key: req.file_key,
        file_name: req.file_name,
        file_type: req.file_type,
        file_size: req.file_size,
    };

    match state
        .announcement_repo
        .add_attachment_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(attachment) => {
            rls.release().await;
            Ok((StatusCode::CREATED, Json(attachment)))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to add attachment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to add attachment",
                )),
            ))
        }
    }
}

/// Delete an attachment.
///
/// Requires manager-level role.
#[utoipa::path(
    delete,
    path = "/api/v1/announcements/{id}/attachments/{attachment_id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID"),
        ("attachment_id" = Uuid, Path, description = "Attachment ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Attachment deleted"),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Attachment not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn delete_attachment(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path((_id, attachment_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete attachments",
            )),
        ));
    }
    match state
        .announcement_repo
        .delete_attachment_rls(&mut **rls.conn(), attachment_id)
        .await
    {
        Ok(_) => {
            rls.release().await;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to delete attachment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete attachment",
                )),
            ))
        }
    }
}

/// Mark an announcement as read (Story 6.2 foundation).
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/read",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Marked as read"),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn mark_read(
    State(state): State<AppState>,
    auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    match state
        .announcement_repo
        .mark_read_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(_) => {
            rls.release().await;
            Ok(StatusCode::OK)
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to mark as read: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to mark as read",
                )),
            ))
        }
    }
}

/// Acknowledge an announcement (Story 6.2 foundation).
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/acknowledge",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Acknowledged"),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn acknowledge(
    State(state): State<AppState>,
    auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    match state
        .announcement_repo
        .acknowledge_rls(&mut **rls.conn(), id, user_id)
        .await
    {
        Ok(_) => {
            rls.release().await;
            Ok(StatusCode::OK)
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to acknowledge: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to acknowledge",
                )),
            ))
        }
    }
}

/// Get acknowledgment statistics and list for an announcement (Story 6.2).
///
/// Requires manager-level role.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/{id}/acknowledgments",
    params(
        ("id" = Uuid, Path, description = "Announcement ID"),
        AcknowledgmentListQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Acknowledgment stats and list", body = AcknowledgmentStatsResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn get_acknowledgments(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(_query): Query<AcknowledgmentListQuery>,
) -> Result<Json<AcknowledgmentStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can view acknowledgment statistics",
            )),
        ));
    }

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

    match state
        .announcement_repo
        .get_acknowledgment_stats_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(stats) => {
            rls.release().await;
            Ok(Json(AcknowledgmentStatsResponse { stats }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to get acknowledgment stats: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get acknowledgment statistics",
                )),
            ))
        }
    }
}

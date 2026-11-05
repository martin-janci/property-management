//! Announcement engagement routes - attachments, read/acknowledge (Story 6.2).

use super::shared::*;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, ValidatedTenantExtractor};
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
    tenant: ValidatedTenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<AttachmentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Org scope: 404 unless the announcement belongs to the caller's org.
    load_announcement_in_org(&state, &mut rls, tenant.tenant_id, id).await?;

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
        (status = 400, description = "file_key outside the organization's storage namespace", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found, or file_key in another organization's namespace", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn add_attachment(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: ValidatedTenantExtractor,
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

    // Org scope: 404 unless the announcement belongs to the caller's org.
    load_announcement_in_org(&state, &mut rls, tenant.tenant_id, id).await?;
    if let Err(e) = validate_attachment_file_key(&req.file_key, tenant.tenant_id) {
        rls.release().await;
        return Err(e);
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

/// SECURITY: an attachment's `file_key` must lie in the caller's org storage
/// namespace — `{org_id}/…`, the shape every upload / presign producer mints
/// via `integrations::generate_storage_key` (same contract as
/// `documents::core::validate_file_key_org_scope`). Otherwise a manager could
/// attach any bucket object (another org's file) to an announcement and have
/// residents fetch it.
///
/// A key inside another org's namespace (`{other_org_uuid}/…`) answers 404 —
/// the object does not exist for this caller; any other key that lacks the
/// org prefix (or contains `..`) is a malformed request, 400.
pub(super) fn validate_attachment_file_key(
    file_key: &str,
    org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if crate::routes::documents::core::validate_file_key_org_scope(file_key, org_id).is_ok() {
        return Ok(());
    }
    let foreign_org_namespace = !file_key.contains("..")
        && file_key
            .split_once('/')
            .is_some_and(|(head, _)| Uuid::parse_str(head).is_ok_and(|other| other != org_id));
    if foreign_org_namespace {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "File not found")),
        ))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_FILE_KEY",
                "file_key must reference an object uploaded for this organization",
            )),
        ))
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
    tenant: ValidatedTenantExtractor,
    mut rls: RlsConnection,
    Path((id, attachment_id)): Path<(Uuid, Uuid)>,
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

    // Org scope: 404 unless the announcement belongs to the caller's org.
    load_announcement_in_org(&state, &mut rls, tenant.tenant_id, id).await?;
    // Nested id: the attachment must belong to this announcement.
    match state
        .announcement_repo
        .delete_attachment_for_announcement_rls(&mut **rls.conn(), id, attachment_id)
        .await
    {
        Ok(0) => {
            rls.release().await;
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Attachment not found")),
            ))
        }
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
    tenant: ValidatedTenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Org scope: 404 unless the announcement belongs to the caller's org.
    load_announcement_in_org(&state, &mut rls, tenant.tenant_id, id).await?;
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
    tenant: ValidatedTenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Org scope: 404 unless the announcement belongs to the caller's org.
    load_announcement_in_org(&state, &mut rls, tenant.tenant_id, id).await?;
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
    tenant: ValidatedTenantExtractor,
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

    // Org scope: 404 unless the announcement belongs to the caller's org.
    load_announcement_in_org(&state, &mut rls, tenant.tenant_id, id).await?;

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

#[cfg(test)]
mod tests {
    use super::validate_attachment_file_key;
    use axum::http::StatusCode;
    use uuid::Uuid;

    #[test]
    fn attachment_file_key_must_be_in_org_namespace() {
        let org = Uuid::new_v4();
        let other = Uuid::new_v4();
        assert!(validate_attachment_file_key(&format!("{org}/2026/11/x_a.pdf"), org).is_ok());

        let status = |k: &str| validate_attachment_file_key(k, org).unwrap_err().0;
        assert_eq!(
            status(&format!("{other}/2026/11/x_a.pdf")),
            StatusCode::NOT_FOUND
        );
        assert_eq!(status("org/files/doc.pdf"), StatusCode::BAD_REQUEST);
        assert_eq!(status("messages/t/a.pdf"), StatusCode::BAD_REQUEST);
        assert_eq!(status(&format!("{org}")), StatusCode::BAD_REQUEST);
        assert_eq!(
            status(&format!("{org}/../{other}/a.pdf")),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status(&format!("{other}/../a.pdf")),
            StatusCode::BAD_REQUEST
        );
    }
}

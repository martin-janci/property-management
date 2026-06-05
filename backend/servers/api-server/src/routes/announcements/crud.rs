//! Announcement CRUD routes (Story 6.1) - create, list, get, update, delete.

use super::shared::*;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{target_type, AnnouncementListQuery, CreateAnnouncement, UpdateAnnouncement};
use uuid::Uuid;

/// CRUD sub-router (create, list, published list, get, update, delete).
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_announcement))
        .route("/", get(list_announcements))
        .route("/published", get(list_published_announcements))
        .route("/{id}", get(get_announcement))
        .route("/{id}", put(update_announcement))
        .route("/{id}", delete(delete_announcement))
}

/// Create a new announcement (Story 6.1).
///
/// Requires manager-level role (Manager, TechnicalManager, OrgAdmin, or SuperAdmin).
#[utoipa::path(
    post,
    path = "/api/v1/announcements",
    request_body = CreateAnnouncementRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Announcement created", body = CreateAnnouncementResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn create_announcement(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<CreateAnnouncementRequest>,
) -> Result<(StatusCode, Json<CreateAnnouncementResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    let role = tenant.role;
    if !role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can create announcements",
            )),
        ));
    }

    let author_id = auth.user_id;
    let org_id = tenant.tenant_id;

    // Validate content length (H-3: Content validation)
    if req.title.len() > MAX_TITLE_LENGTH {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Title exceeds maximum length of {} characters",
                    MAX_TITLE_LENGTH
                ),
            )),
        ));
    }
    if req.content.len() > MAX_CONTENT_LENGTH {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Content exceeds maximum length of {} characters",
                    MAX_CONTENT_LENGTH
                ),
            )),
        ));
    }

    // Validate target_type
    if !target_type::ALL_TYPES.contains(&req.target_type.as_str()) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Invalid target_type")),
        ));
    }

    // Validate target_ids based on target_type
    if req.target_type != target_type::ALL && req.target_ids.as_ref().is_none_or(|v| v.is_empty()) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "target_ids required for non-'all' target_type",
            )),
        ));
    }

    // Security: Validate target_ids exist in the organization (Critical 1.3 fix)
    if req.target_type != target_type::ALL {
        if let Some(ref target_ids) = req.target_ids {
            let validation_result =
                validate_target_ids(rls.conn(), org_id, &req.target_type, target_ids).await;

            if let Err(err_msg) = validation_result {
                rls.release().await;
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("INVALID_TARGET_IDS", err_msg)),
                ));
            }
        }
    }

    // Sanitize content (M-2: Basic markdown sanitization)
    let sanitized_content = sanitize_markdown(&req.content);

    let data = CreateAnnouncement {
        organization_id: org_id,
        author_id,
        title: req.title,
        content: sanitized_content,
        target_type: req.target_type,
        target_ids: req.target_ids.unwrap_or_default(),
        scheduled_at: req.scheduled_at,
        comments_enabled: req.comments_enabled,
        acknowledgment_required: req.acknowledgment_required,
    };

    match state
        .announcement_repo
        .create_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(announcement) => {
            rls.release().await;
            Ok((
                StatusCode::CREATED,
                Json(CreateAnnouncementResponse {
                    id: announcement.id,
                    message: "Announcement created successfully".to_string(),
                }),
            ))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to create announcement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create announcement",
                )),
            ))
        }
    }
}

/// List announcements with filters (for managers).
///
/// Requires manager-level role.
#[utoipa::path(
    get,
    path = "/api/v1/announcements",
    params(ListAnnouncementsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement list", body = AnnouncementListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn list_announcements(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<ListAnnouncementsQuery>,
) -> Result<Json<AnnouncementListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can list all announcements",
            )),
        ));
    }

    let org_id = tenant.tenant_id;

    let list_query = AnnouncementListQuery {
        status: query
            .status
            .map(|s| s.split(',').map(String::from).collect()),
        target_type: query.target_type.clone(),
        author_id: query.author_id,
        pinned: query.pinned,
        from_date: query.from_date,
        to_date: query.to_date,
        limit: query.limit,
        offset: query.offset,
    };

    // Get total count for pagination (H-4)
    let count_query = AnnouncementListQuery {
        status: list_query.status.clone(),
        target_type: query.target_type,
        author_id: query.author_id,
        pinned: query.pinned,
        from_date: query.from_date,
        to_date: query.to_date,
        limit: None,
        offset: None,
    };
    let total = state
        .announcement_repo
        .count_rls(&mut **rls.conn(), org_id, count_query)
        .await
        .unwrap_or(0);

    match state
        .announcement_repo
        .list_rls(&mut **rls.conn(), org_id, list_query)
        .await
    {
        Ok(announcements) => {
            let count = announcements.len();
            rls.release().await;
            Ok(Json(AnnouncementListResponse {
                announcements,
                count,
                total,
            }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to list announcements: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list announcements",
                )),
            ))
        }
    }
}

/// List published announcements (for all authenticated users).
#[utoipa::path(
    get,
    path = "/api/v1/announcements/published",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Published announcement list", body = AnnouncementListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn list_published_announcements(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<ListAnnouncementsQuery>,
) -> Result<Json<AnnouncementListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;

    // Get total count for pagination (H-4)
    let total = state
        .announcement_repo
        .count_published_rls(&mut **rls.conn(), org_id)
        .await
        .unwrap_or(0);

    match state
        .announcement_repo
        .list_published_rls(&mut **rls.conn(), org_id, query.limit, query.offset)
        .await
    {
        Ok(announcements) => {
            let count = announcements.len();
            rls.release().await;
            Ok(Json(AnnouncementListResponse {
                announcements,
                count,
                total,
            }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to list published announcements: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list announcements",
                )),
            ))
        }
    }
}

/// Get announcement details.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement details", body = AnnouncementDetailResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn get_announcement(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<AnnouncementDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let announcement = match state
        .announcement_repo
        .find_by_id_with_details_rls(&mut **rls.conn(), id)
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
            tracing::error!("Failed to get announcement: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get announcement",
                )),
            ));
        }
    };

    let attachments = state
        .announcement_repo
        .get_attachments_rls(&mut **rls.conn(), id)
        .await
        .unwrap_or_default();

    rls.release().await;
    Ok(Json(AnnouncementDetailResponse {
        announcement,
        attachments,
    }))
}

/// Update announcement details.
///
/// Requires manager-level role.
#[utoipa::path(
    put,
    path = "/api/v1/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    request_body = UpdateAnnouncementRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement updated", body = AnnouncementActionResponse),
        (status = 400, description = "Cannot update", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn update_announcement(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAnnouncementRequest>,
) -> Result<Json<AnnouncementActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update announcements",
            )),
        ));
    }

    // Validate content length if provided (H-3)
    if let Some(ref title) = req.title {
        if title.len() > MAX_TITLE_LENGTH {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Title exceeds maximum length of {} characters",
                        MAX_TITLE_LENGTH
                    ),
                )),
            ));
        }
    }
    if let Some(ref content) = req.content {
        if content.len() > MAX_CONTENT_LENGTH {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Content exceeds maximum length of {} characters",
                        MAX_CONTENT_LENGTH
                    ),
                )),
            ));
        }
    }
    // Check announcement exists and can be edited
    let existing = match state
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

    if !existing.can_edit() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Published announcements cannot be edited",
            )),
        ));
    }

    // Validate target_type if provided
    if let Some(ref tt) = req.target_type {
        if !target_type::ALL_TYPES.contains(&tt.as_str()) {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", "Invalid target_type")),
            ));
        }
    }

    // Sanitize content if provided (M-2)
    let sanitized_content = req.content.map(|c| sanitize_markdown(&c));

    let data = UpdateAnnouncement {
        title: req.title,
        content: sanitized_content,
        target_type: req.target_type,
        target_ids: req.target_ids,
        scheduled_at: req.scheduled_at,
        comments_enabled: req.comments_enabled,
        acknowledgment_required: req.acknowledgment_required,
    };

    match state
        .announcement_repo
        .update_rls(&mut **rls.conn(), id, data)
        .await
    {
        Ok(announcement) => {
            rls.release().await;
            Ok(Json(AnnouncementActionResponse {
                message: "Announcement updated".to_string(),
                announcement,
            }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to update announcement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update announcement",
                )),
            ))
        }
    }
}

/// Delete an announcement (draft only).
///
/// Requires manager-level role.
#[utoipa::path(
    delete,
    path = "/api/v1/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Announcement deleted"),
        (status = 400, description = "Cannot delete", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
pub(super) async fn delete_announcement(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete announcements",
            )),
        ));
    }
    // Check if draft
    let existing = match state
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

    if !existing.is_draft() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Only draft announcements can be deleted",
            )),
        ));
    }

    match state
        .announcement_repo
        .delete_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(_) => {
            rls.release().await;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to delete announcement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete announcement",
                )),
            ))
        }
    }
}

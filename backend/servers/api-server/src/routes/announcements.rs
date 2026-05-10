//! Announcement routes (Epic 6: Announcements & Communication, Epic 92: Intelligent Document Generation).

use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use db::models::{
    target_type, AcknowledgmentStats, Announcement, AnnouncementAttachment, AnnouncementComment,
    AnnouncementListQuery, AnnouncementStatistics, AnnouncementSummary, AnnouncementWithDetails,
    CommentWithAuthor, CreateAnnouncement, CreateAnnouncementAttachment, CreateComment,
    DeleteComment, UpdateAnnouncement, UserAcknowledgmentStatus,
};
use serde::{Deserialize, Serialize};
use sqlx::Error as SqlxError;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed title length (characters).
const MAX_TITLE_LENGTH: usize = 200;

/// Maximum allowed content length (characters).
const MAX_CONTENT_LENGTH: usize = 50_000;

/// Maximum allowed comment length (characters).
const MAX_COMMENT_LENGTH: usize = 2000;

// ============================================================================
// Response Types
// ============================================================================

/// Response for announcement creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAnnouncementResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for announcement list with pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementListResponse {
    pub announcements: Vec<AnnouncementSummary>,
    /// Number of items in this response.
    pub count: usize,
    /// Total number of items matching the query (for pagination).
    pub total: i64,
}

/// Response for announcement details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementDetailResponse {
    pub announcement: AnnouncementWithDetails,
    pub attachments: Vec<AnnouncementAttachment>,
}

/// Response for generic announcement action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementActionResponse {
    pub message: String,
    pub announcement: Announcement,
}

/// Response for attachments list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AttachmentsResponse {
    pub attachments: Vec<AnnouncementAttachment>,
}

/// Response for statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatisticsResponse {
    pub statistics: AnnouncementStatistics,
}

/// Response for unread count.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnreadCountResponse {
    pub unread_count: i64,
}

/// Response for acknowledgment statistics (Story 6.2).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgmentStatsResponse {
    pub stats: AcknowledgmentStats,
}

/// Response for acknowledgment list (Story 6.2).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgmentListResponse {
    pub users: Vec<UserAcknowledgmentStatus>,
    pub count: usize,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAnnouncementRequest {
    pub title: String,
    pub content: String,
    pub target_type: String,
    pub target_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub comments_enabled: Option<bool>,
    pub acknowledgment_required: Option<bool>,
}

/// Request for updating an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub target_type: Option<String>,
    pub target_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub comments_enabled: Option<bool>,
    pub acknowledgment_required: Option<bool>,
}

/// Request for scheduling an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScheduleAnnouncementRequest {
    pub scheduled_at: DateTime<Utc>,
}

/// Request for pinning/unpinning an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PinAnnouncementRequest {
    pub pinned: bool,
}

/// Request for adding an attachment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddAttachmentRequest {
    pub file_key: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
}

/// Query for listing announcements.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListAnnouncementsQuery {
    pub status: Option<String>,
    pub target_type: Option<String>,
    pub author_id: Option<Uuid>,
    pub pinned: Option<bool>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Comment Types (Story 6.3)
// ============================================================================

/// Request for creating a comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCommentRequest {
    pub content: String,
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub ai_training_consent: bool,
}

/// Request for deleting a comment (moderation).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteCommentRequest {
    pub reason: Option<String>,
}

/// Response for comment list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommentsResponse {
    pub comments: Vec<CommentWithAuthor>,
    pub count: usize,
    pub total: i64,
}

/// Response for single comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommentResponse {
    pub comment: CommentWithAuthor,
}

/// Query for listing comments.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListCommentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Epic 92.4: AI Announcement Draft Types
// ============================================================================

/// Request for AI-generated announcement draft.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenerateAiDraftRequest {
    /// Topic or subject of the announcement
    pub topic: String,
    /// Key points to include in the announcement
    pub key_points: Option<Vec<String>>,
    /// Urgency level (low, medium, high, critical)
    pub urgency: Option<String>,
    /// Target audience description
    pub audience: Option<String>,
    /// Tone (formal, friendly, urgent, informative)
    pub tone: Option<String>,
    /// Language (sk, cs, de, en)
    #[serde(default = "default_language")]
    pub language: String,
    /// Number of draft variants to generate (1-3)
    pub num_drafts: Option<i32>,
    /// Building ID for context
    pub building_id: Option<Uuid>,
}

fn default_language() -> String {
    "en".to_string()
}

/// Single announcement draft.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementDraft {
    pub title: String,
    pub content: String,
    pub suggested_target: String,
    pub tone_analysis: String,
}

/// Response for AI-generated announcement drafts.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenerateAiDraftResponse {
    pub drafts: Vec<AnnouncementDraft>,
    pub tokens_used: i32,
    pub generation_time_ms: u64,
    pub provider: String,
    pub model: String,
}

// ============================================================================
// Router
// ============================================================================

/// Create announcements router.
pub fn router() -> Router<AppState> {
    Router::new()
        // CRUD
        .route("/", post(create_announcement))
        .route("/", get(list_announcements))
        .route("/published", get(list_published_announcements))
        .route("/{id}", get(get_announcement))
        .route("/{id}", put(update_announcement))
        .route("/{id}", delete(delete_announcement))
        // Publishing
        .route("/{id}/publish", post(publish_announcement))
        .route("/{id}/schedule", post(schedule_announcement))
        .route("/{id}/archive", post(archive_announcement))
        // Pinning
        .route("/{id}/pin", post(pin_announcement))
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
        // Comments (Story 6.3)
        .route("/{id}/comments", get(list_comments))
        .route("/{id}/comments", post(create_comment))
        .route("/{id}/comments/{comment_id}", delete(delete_comment))
        // Statistics
        .route("/statistics", get(get_statistics))
        .route("/unread-count", get(get_unread_count))
        // Epic 92.4: AI Draft Generation
        .route("/ai-draft", post(generate_ai_draft))
}

// ============================================================================
// Handlers
// ============================================================================

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
async fn create_announcement(
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
async fn list_announcements(
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
async fn list_published_announcements(
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
async fn get_announcement(
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
async fn update_announcement(
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
async fn delete_announcement(
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

/// Publish an announcement immediately (Story 6.1).
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/publish",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement published", body = AnnouncementActionResponse),
        (status = 400, description = "Cannot publish", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
async fn publish_announcement(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<AnnouncementActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can publish announcements",
            )),
        ));
    }

    match state
        .announcement_repo
        .publish_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(announcement) => {
            rls.release().await;
            // Story 84.4: Trigger notification event for announcement publish
            let target_type = parse_target_type(&announcement.target_type);

            // Parse target_ids from JSON value
            let target_ids: Vec<Uuid> = announcement
                .target_ids
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                        .collect()
                })
                .unwrap_or_default();

            let notification_event = common::NotificationEvent::AnnouncementPublished {
                announcement_id: announcement.id,
                organization_id: announcement.organization_id,
                target_type,
                target_ids: target_ids.clone(),
                title: announcement.title.clone(),
            };

            // Log the notification event for observability and downstream processing
            // The notification service (Epic 2B) can subscribe to these log events via log aggregation
            // or implement direct Redis pub/sub integration when available
            tracing::info!(
                announcement_id = %announcement.id,
                organization_id = %announcement.organization_id,
                target_type = %announcement.target_type,
                target_ids_count = %target_ids.len(),
                notification_title = %notification_event.title(),
                notification_category = %notification_event.category(),
                notification_priority = ?notification_event.priority(),
                notification_event = ?serde_json::to_string(&notification_event).ok(),
                "Announcement published - notification event dispatched"
            );

            Ok(Json(AnnouncementActionResponse {
                message: "Announcement published".to_string(),
                announcement,
            }))
        }
        Err(SqlxError::RowNotFound) => {
            rls.release().await;
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Announcement not found")),
            ))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to publish announcement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to publish announcement",
                )),
            ))
        }
    }
}

/// Schedule an announcement for future publishing (Story 6.1).
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/schedule",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    request_body = ScheduleAnnouncementRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement scheduled", body = AnnouncementActionResponse),
        (status = 400, description = "Invalid schedule", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
async fn schedule_announcement(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<ScheduleAnnouncementRequest>,
) -> Result<Json<AnnouncementActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can schedule announcements",
            )),
        ));
    }
    // Validate scheduled_at is in the future
    if req.scheduled_at <= Utc::now() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Scheduled time must be in the future",
            )),
        ));
    }

    match state
        .announcement_repo
        .schedule_rls(&mut **rls.conn(), id, req.scheduled_at)
        .await
    {
        Ok(announcement) => {
            rls.release().await;
            Ok(Json(AnnouncementActionResponse {
                message: "Announcement scheduled".to_string(),
                announcement,
            }))
        }
        Err(SqlxError::RowNotFound) => {
            rls.release().await;
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Announcement not found")),
            ))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to schedule announcement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to schedule announcement",
                )),
            ))
        }
    }
}

/// Archive an announcement.
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/archive",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement archived", body = AnnouncementActionResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
async fn archive_announcement(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<AnnouncementActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can archive announcements",
            )),
        ));
    }
    match state
        .announcement_repo
        .archive_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(announcement) => {
            rls.release().await;
            Ok(Json(AnnouncementActionResponse {
                message: "Announcement archived".to_string(),
                announcement,
            }))
        }
        Err(SqlxError::RowNotFound) => {
            rls.release().await;
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Announcement not found")),
            ))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to archive announcement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to archive announcement",
                )),
            ))
        }
    }
}

/// Pin/unpin an announcement (Story 6.4).
///
/// Requires manager-level role. Maximum 3 pinned announcements per organization.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/pin",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    request_body = PinAnnouncementRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Pin status updated", body = AnnouncementActionResponse),
        (status = 400, description = "Maximum pinned limit reached", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Announcement not found", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
async fn pin_announcement(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<PinAnnouncementRequest>,
) -> Result<Json<AnnouncementActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can pin announcements",
            )),
        ));
    }

    let user_id = auth.user_id;

    let result = if req.pinned {
        // Get announcement to check org_id and if already pinned
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

        // Check if it's published
        if announcement.status != "published" {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Announcement not found")),
            ));
        }

        // If already pinned, just return it
        if announcement.pinned {
            rls.release().await;
            return Ok(Json(AnnouncementActionResponse {
                message: "Announcement pinned".to_string(),
                announcement,
            }));
        }

        // Check pinned count (max 3 per org)
        let pinned_count = state
            .announcement_repo
            .count_pinned_rls(&mut **rls.conn(), announcement.organization_id)
            .await
            .unwrap_or(0);

        if pinned_count >= 3 {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "PINNED_LIMIT_REACHED",
                    "Maximum of 3 pinned announcements reached",
                )),
            ));
        }

        // Pin the announcement
        state
            .announcement_repo
            .pin_rls(&mut **rls.conn(), id, user_id)
            .await
    } else {
        state
            .announcement_repo
            .unpin_rls(&mut **rls.conn(), id)
            .await
    };

    match result {
        Ok(announcement) => {
            rls.release().await;
            Ok(Json(AnnouncementActionResponse {
                message: if req.pinned {
                    "Announcement pinned"
                } else {
                    "Announcement unpinned"
                }
                .to_string(),
                announcement,
            }))
        }
        Err(SqlxError::RowNotFound) => {
            rls.release().await;
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Announcement not found")),
            ))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to update pin status: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update pin status",
                )),
            ))
        }
    }
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
async fn list_attachments(
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
async fn add_attachment(
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
async fn delete_attachment(
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
async fn mark_read(
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
async fn acknowledge(
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

/// Get announcement statistics.
///
/// Requires manager-level role.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/statistics",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Announcement statistics", body = StatisticsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
async fn get_statistics(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<StatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role (Task 3.7)
    if !tenant.role.is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can view statistics",
            )),
        ));
    }

    let org_id = tenant.tenant_id;

    match state
        .announcement_repo
        .get_statistics_rls(&mut **rls.conn(), org_id)
        .await
    {
        Ok(statistics) => {
            rls.release().await;
            Ok(Json(StatisticsResponse { statistics }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to get statistics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get statistics",
                )),
            ))
        }
    }
}

/// Get unread announcement count.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/unread-count",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unread count", body = UnreadCountResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
async fn get_unread_count(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<UnreadCountResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;
    let user_id = auth.user_id;

    match state
        .announcement_repo
        .count_unread_rls(&mut **rls.conn(), org_id, user_id)
        .await
    {
        Ok(unread_count) => {
            rls.release().await;
            Ok(Json(UnreadCountResponse { unread_count }))
        }
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to get unread count: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get unread count",
                )),
            ))
        }
    }
}

/// Query for acknowledgment list pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct AcknowledgmentListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
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
async fn get_acknowledgments(
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate that target_ids exist within the organization.
///
/// Security fix (Critical 1.3): Ensures managers can only target buildings/units
/// that belong to their organization.
async fn validate_target_ids(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    target_type: &str,
    target_ids: &[Uuid],
) -> Result<(), String> {
    if target_ids.is_empty() {
        return Ok(());
    }

    match target_type {
        target_type::BUILDING => {
            // Validate buildings exist in the organization
            let (count,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*) FROM buildings
                WHERE id = ANY($1) AND organization_id = $2
                "#,
            )
            .bind(target_ids)
            .bind(org_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

            if (count as usize) != target_ids.len() {
                return Err(format!(
                    "One or more target buildings not found in organization (found {}/{})",
                    count,
                    target_ids.len()
                ));
            }
        }
        target_type::UNITS => {
            // Validate units exist in the organization
            let (count,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*) FROM units u
                JOIN buildings b ON b.id = u.building_id
                WHERE u.id = ANY($1) AND b.organization_id = $2
                "#,
            )
            .bind(target_ids)
            .bind(org_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

            if (count as usize) != target_ids.len() {
                return Err(format!(
                    "One or more target units not found in organization (found {}/{})",
                    count,
                    target_ids.len()
                ));
            }
        }
        target_type::ROLES => {
            // For roles, we validate against the TenantRole enum
            // Role IDs are typically UUIDs mapped to role names, but since our system
            // uses enum roles, we skip validation here (roles are enforced at enum level)
            // If you have a role_mappings table, validate against that
        }
        _ => {
            // Unknown target type - already validated earlier, but be safe
        }
    }

    Ok(())
}

/// Parse target type string to notification TargetType enum.
///
/// Used for converting database target_type values to notification event types.
fn parse_target_type(target_type: &str) -> common::notifications::TargetType {
    match target_type {
        target_type::ALL => common::notifications::TargetType::All,
        target_type::BUILDING => common::notifications::TargetType::Building,
        target_type::UNITS => common::notifications::TargetType::Units,
        target_type::ROLES => common::notifications::TargetType::Roles,
        _ => common::notifications::TargetType::All, // Default fallback
    }
}

/// Sanitize markdown/HTML content using ammonia.
///
/// Security fix (Critical 1.5): Uses ammonia library for robust XSS protection.
/// Allows safe markdown/HTML elements while removing all dangerous content
/// including script tags, event handlers, and javascript: URLs.
fn sanitize_markdown(content: &str) -> String {
    use ammonia::Builder;
    use std::collections::HashSet;

    // Define allowed tags for markdown content
    let allowed_tags: HashSet<&str> = [
        // Text formatting
        "p",
        "br",
        "strong",
        "b",
        "em",
        "i",
        "u",
        "s",
        "del",
        "ins",
        "mark",
        // Headings
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        // Lists
        "ul",
        "ol",
        "li",
        // Quotes and code
        "blockquote",
        "code",
        "pre",
        // Links and images
        "a",
        "img",
        // Tables
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        // Misc
        "hr",
        "div",
        "span",
        "sup",
        "sub",
    ]
    .into_iter()
    .collect();

    // Define allowed attributes for specific tags
    let mut tag_attributes = std::collections::HashMap::new();
    tag_attributes.insert(
        "a",
        ["href", "title", "rel", "target"]
            .into_iter()
            .collect::<HashSet<_>>(),
    );
    tag_attributes.insert(
        "img",
        ["src", "alt", "title", "width", "height"]
            .into_iter()
            .collect::<HashSet<_>>(),
    );
    tag_attributes.insert(
        "td",
        ["colspan", "rowspan"].into_iter().collect::<HashSet<_>>(),
    );
    tag_attributes.insert(
        "th",
        ["colspan", "rowspan", "scope"]
            .into_iter()
            .collect::<HashSet<_>>(),
    );

    // Define allowed URL schemes (prevent javascript:, data:, vbscript: etc.)
    let allowed_schemes: HashSet<&str> = ["http", "https", "mailto"].into_iter().collect();

    Builder::default()
        .tags(allowed_tags)
        .tag_attributes(tag_attributes)
        .link_rel(Some("noopener noreferrer"))
        .url_schemes(allowed_schemes)
        .strip_comments(true)
        .clean(content)
        .to_string()
}

// ============================================================================
// Comment Handlers (Story 6.3)
// ============================================================================

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
async fn list_comments(
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

    // Release RLS connection before calling get_threaded_comments
    // (it uses pool directly for multiple queries)
    rls.release().await;

    // Get threaded comments
    match state
        .announcement_repo
        .get_threaded_comments(id, query.limit, query.offset)
        .await
    {
        Ok(comments) => Ok(Json(CommentsResponse {
            count: comments.len(),
            comments,
            total,
        })),
        Err(e) => {
            tracing::error!("Failed to list comments: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list comments",
                )),
            ))
        }
    }
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
async fn create_comment(
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

    // If parent_id is provided, verify it exists and belongs to same announcement
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
async fn delete_comment(
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
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find comment",
                )),
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

// ============================================================================
// Epic 92.4: AI Announcement Draft Generation Handler
// ============================================================================

/// Generate AI-powered announcement drafts (Story 92.4).
///
/// Uses LLM to generate well-structured announcement drafts with appropriate tone.
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/ai-draft",
    request_body = GenerateAiDraftRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Drafts generated", body = GenerateAiDraftResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Announcements"
)]
async fn generate_ai_draft(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Json(req): Json<GenerateAiDraftRequest>,
) -> Result<Json<GenerateAiDraftResponse>, (StatusCode, Json<ErrorResponse>)> {
    use std::time::Instant;

    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can generate announcement drafts",
            )),
        ));
    }

    let start_time = Instant::now();

    // Validate topic
    if req.topic.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Topic is required")),
        ));
    }

    if req.topic.len() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Topic exceeds maximum length of 500 characters",
            )),
        ));
    }

    // Determine provider and model
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o".to_string(),
        "azure_openai" => "gpt-4o".to_string(),
        _ => "claude-3-5-sonnet-20241022".to_string(),
    });

    let num_drafts = req.num_drafts.unwrap_or(1).clamp(1, 3);
    let urgency = req.urgency.as_deref().unwrap_or("medium");
    let tone = req.tone.as_deref().unwrap_or("professional");

    // Build system prompt
    let system_prompt = build_announcement_system_prompt(urgency, tone, &req.language);

    // Build user prompt
    let key_points_text = req
        .key_points
        .as_ref()
        .filter(|k| !k.is_empty())
        .map(|points| {
            format!(
                "\n\nKey points to include:\n{}",
                points
                    .iter()
                    .map(|p| format!("- {}", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
        .unwrap_or_default();

    let audience_text = req
        .audience
        .as_ref()
        .map(|a| format!("\n\nTarget audience: {}", a))
        .unwrap_or_default();

    let user_prompt = format!(
        r#"Generate {} announcement draft(s) for the following topic:

Topic: {}{}{}

Please provide {} complete draft(s), each with:
1. A clear, attention-grabbing title
2. Well-structured content (200-400 words)
3. Appropriate call to action if needed
4. Suggested target audience type (all, building, unit, role)

Format each draft as:
DRAFT 1:
TITLE: [Title here]
CONTENT:
[Content here]
TARGET: [Suggested target type]
TONE: [Brief analysis of the tone used]

[Repeat for additional drafts if requested]"#,
        num_drafts, req.topic, key_points_text, audience_text, num_drafts
    );

    // Call LLM
    let llm_request = integrations::ChatCompletionRequest {
        model: model.clone(),
        messages: vec![
            integrations::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            integrations::ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ],
        temperature: Some(0.7), // Higher temperature for creative variety
        max_tokens: Some(4000),
    };

    let response = state
        .llm_client
        .chat(&provider, &llm_request)
        .await
        .map_err(|e| {
            tracing::error!("LLM announcement generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LLM_ERROR",
                    format!("Failed to generate drafts: {}", e),
                )),
            )
        })?;

    let response_content = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    // Parse the drafts
    let drafts = parse_announcement_drafts(&response_content);

    let generation_time_ms = start_time.elapsed().as_millis() as u64;

    tracing::info!(
        "Generated {} announcement drafts for topic '{}' (tokens: {}, latency: {}ms)",
        drafts.len(),
        req.topic,
        response.usage.total_tokens,
        generation_time_ms
    );

    Ok(Json(GenerateAiDraftResponse {
        drafts,
        tokens_used: response.usage.total_tokens,
        generation_time_ms,
        provider,
        model,
    }))
}

/// Build the system prompt for announcement generation.
fn build_announcement_system_prompt(urgency: &str, tone: &str, language: &str) -> String {
    let urgency_instruction = match urgency {
        "critical" => "This is a CRITICAL announcement. Use urgent language, emphasize importance, and include clear action items with deadlines.",
        "high" => "This is a high-priority announcement. Use clear, direct language and emphasize the importance of timely action.",
        "low" => "This is a routine announcement. Use a relaxed, informative tone.",
        _ => "This is a standard announcement. Use clear, professional language.",
    };

    let tone_instruction = match tone {
        "formal" => "Use formal, official language appropriate for legal or regulatory matters.",
        "friendly" => "Use warm, approachable language that builds community connection.",
        "urgent" => "Use direct, action-oriented language with clear calls to action.",
        _ => "Use professional, clear language that is respectful and informative.",
    };

    let language_name = match language {
        "sk" => "Slovak",
        "cs" => "Czech",
        "de" => "German",
        _ => "English",
    };

    format!(
        r#"You are an expert property management communications specialist.

Your task is to create professional announcements for building residents and owners.

Language: Write in {} language
Urgency Level: {}
Tone: {}

Guidelines:
1. Write clear, well-structured announcements
2. Use appropriate formatting (paragraphs, bullet points where helpful)
3. Include relevant dates, times, and locations when applicable
4. Provide clear contact information or next steps
5. Be culturally appropriate for Central European readers
6. Avoid jargon and use accessible language

The announcements should be ready for immediate publication after minimal editing."#,
        language_name, urgency_instruction, tone_instruction
    )
}

/// Parse announcement drafts from LLM response.
fn parse_announcement_drafts(content: &str) -> Vec<AnnouncementDraft> {
    let mut drafts = Vec::new();
    let mut current_draft: Option<AnnouncementDraft> = None;
    let mut current_section = "";
    let mut current_content = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for new draft
        if trimmed.to_uppercase().starts_with("DRAFT ") {
            // Save previous draft if exists
            if let Some(mut draft) = current_draft.take() {
                if current_section == "content" {
                    draft.content = current_content.trim().to_string();
                }
                if !draft.title.is_empty() {
                    drafts.push(draft);
                }
            }
            current_draft = Some(AnnouncementDraft {
                title: String::new(),
                content: String::new(),
                suggested_target: "all".to_string(),
                tone_analysis: String::new(),
            });
            current_section = "";
            current_content.clear();
            continue;
        }

        if let Some(ref mut draft) = current_draft {
            if trimmed.to_uppercase().starts_with("TITLE:") {
                if current_section == "content" {
                    draft.content = current_content.trim().to_string();
                }
                draft.title = trimmed
                    .strip_prefix("TITLE:")
                    .or_else(|| trimmed.strip_prefix("Title:"))
                    .unwrap_or(trimmed)
                    .trim()
                    .to_string();
                current_section = "title";
                current_content.clear();
            } else if trimmed.to_uppercase().starts_with("CONTENT:") {
                current_section = "content";
                current_content.clear();
            } else if trimmed.to_uppercase().starts_with("TARGET:") {
                if current_section == "content" {
                    draft.content = current_content.trim().to_string();
                }
                draft.suggested_target = trimmed
                    .strip_prefix("TARGET:")
                    .or_else(|| trimmed.strip_prefix("Target:"))
                    .unwrap_or(trimmed)
                    .trim()
                    .to_lowercase();
                current_section = "target";
                current_content.clear();
            } else if trimmed.to_uppercase().starts_with("TONE:") {
                if current_section == "content" {
                    draft.content = current_content.trim().to_string();
                }
                draft.tone_analysis = trimmed
                    .strip_prefix("TONE:")
                    .or_else(|| trimmed.strip_prefix("Tone:"))
                    .unwrap_or(trimmed)
                    .trim()
                    .to_string();
                current_section = "tone";
                current_content.clear();
            } else if current_section == "content" {
                current_content.push_str(trimmed);
                current_content.push('\n');
            }
        }
    }

    // Don't forget the last draft
    if let Some(mut draft) = current_draft {
        if current_section == "content" {
            draft.content = current_content.trim().to_string();
        }
        if !draft.title.is_empty() {
            drafts.push(draft);
        }
    }

    // If no drafts were parsed, create one from the entire content
    if drafts.is_empty() && !content.trim().is_empty() {
        drafts.push(AnnouncementDraft {
            title: "Generated Announcement".to_string(),
            content: content.to_string(),
            suggested_target: "all".to_string(),
            tone_analysis: "Unable to parse structured response".to_string(),
        });
    }

    drafts
}

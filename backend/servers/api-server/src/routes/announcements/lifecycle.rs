//! Announcement lifecycle routes (Story 6.1, 6.4) - publish, schedule, archive, pin.

use super::shared::*;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use chrono::Utc;
use common::errors::ErrorResponse;
use common::notifications::{Notification, NotificationCategory};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Lifecycle sub-router (publish, schedule, archive, pin).
pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/publish", post(publish_announcement))
        .route("/{id}/schedule", post(schedule_announcement))
        .route("/{id}/archive", post(archive_announcement))
        // Pinning - PATCH is the canonical verb; POST kept for back-compat
        .route("/{id}/pin", post(pin_announcement).patch(pin_announcement))
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
pub(super) async fn publish_announcement(
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
            // Parse target_ids from JSON value.
            let target_ids: Vec<Uuid> = announcement
                .target_ids
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                        .collect()
                })
                .unwrap_or_default();

            // Resolve the recipient user ids WHILE the manager's RLS context is
            // still set (they can read their org's memberships / units /
            // residents). Resolution failure must not fail the publish - it has
            // already succeeded - so we log and fall back to no recipients.
            let recipients = match resolve_announcement_recipients(
                rls.conn(),
                announcement.organization_id,
                &announcement.target_type,
                &target_ids,
            )
            .await
            {
                Ok(ids) => ids,
                Err(e) => {
                    tracing::error!(
                        announcement_id = %announcement.id,
                        error = %e,
                        "Failed to resolve announcement recipients; published without fan-out"
                    );
                    Vec::new()
                }
            };
            rls.release().await;

            // Story 84.4 / Epic 2B: build the notification event + payload.
            let target_type = parse_target_type(&announcement.target_type);
            let notification_event = common::NotificationEvent::AnnouncementPublished {
                announcement_id: announcement.id,
                organization_id: announcement.organization_id,
                target_type,
                target_ids: target_ids.clone(),
                title: announcement.title.clone(),
            };

            // Epic 2B: real fan-out through the notification pipeline. The
            // pipeline applies PreferenceRouter per recipient and respects
            // urgency (Urgent announcements bypass preferences); the in-app DB
            // record is written for every recipient regardless of preference.
            let notification = Notification::new(
                Uuid::nil(),
                NotificationCategory::Announcements,
                notification_event.title(),
                announcement.content.clone(),
            )
            .with_priority(notification_event.priority())
            .with_action_url(format!("/announcements/{}", announcement.id))
            .with_data(serde_json::json!({
                "announcement_id": announcement.id,
                "organization_id": announcement.organization_id,
                "target_type": announcement.target_type,
            }));

            let (sent, skipped, failed) = state
                .notification_pipeline
                .broadcast(&recipients, &notification, Some(announcement.id))
                .await;

            tracing::info!(
                announcement_id = %announcement.id,
                organization_id = %announcement.organization_id,
                target_type = %announcement.target_type,
                recipients = recipients.len(),
                priority = ?notification_event.priority(),
                channels_sent = sent,
                channels_skipped = skipped,
                channels_failed = failed,
                "Announcement published - notifications dispatched via pipeline"
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
pub(super) async fn schedule_announcement(
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
pub(super) async fn archive_announcement(
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
/// Pinned announcements are returned first in all list endpoints
/// (`ORDER BY pinned DESC, published_at DESC`).
///
/// Accepts both PATCH (preferred) and POST (backward-compat).
#[utoipa::path(
    patch,
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
pub(super) async fn pin_announcement(
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

        // Atomically check the max-3 cap and pin in a single tx with
        // SELECT ... FOR UPDATE on the org's pinned rows. Defends issue
        // #518 (TOCTOU race that let two concurrent PATCHes both pass the
        // guard). Also surfaces DB errors instead of masking them with
        // `.unwrap_or(0)`.
        match state
            .announcement_repo
            .pin_with_cap_rls(
                rls.conn(),
                id,
                announcement.organization_id,
                user_id,
                db::repositories::AnnouncementRepository::MAX_PINNED_PER_ORG,
            )
            .await
        {
            Ok(Ok(pinned)) => Ok(pinned),
            Ok(Err(_actual_count)) => {
                rls.release().await;
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "PINNED_LIMIT_REACHED",
                        "Maximum of 3 pinned announcements reached",
                    )),
                ));
            }
            Err(e) => Err(e),
        }
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

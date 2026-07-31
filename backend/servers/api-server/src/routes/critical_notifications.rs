//! Critical Notifications routes (Epic 8A, Story 8A.2).

use api_core::extractors::principal::RequestPrincipal;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    AcknowledgeCriticalNotificationResponse, CreateCriticalNotificationRequest,
    CreateCriticalNotificationResponse, CriticalNotificationResponse, CriticalNotificationStats,
    UnacknowledgedNotificationsResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

/// Create critical notifications router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_notification))
        .route("/", get(list_notifications))
        .route("/unacknowledged", get(get_unacknowledged))
        .route("/{notification_id}/acknowledge", post(acknowledge))
        .route("/{notification_id}/stats", get(get_stats))
}

// ==================== Create Notification (Story 8A.2, AC-1) ====================

/// Create a critical notification (admin only).
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{org_id}/critical-notifications",
    tag = "Critical Notifications",
    security(("bearer_auth" = [])),
    request_body = CreateCriticalNotificationRequest,
    responses(
        (status = 201, description = "Notification created", body = CreateCriticalNotificationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized (admin only)", body = ErrorResponse)
    )
)]
pub async fn create_notification(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(req): Json<CreateCriticalNotificationRequest>,
) -> Result<(StatusCode, Json<CreateCriticalNotificationResponse>), (StatusCode, Json<ErrorResponse>)>
{
    // Extract tenant context
    let tenant_id = require_tenant_id(&principal)?;

    // P0-07: real admin role lookup. Platform principals always pass;
    // tenant principals must have a manager-tier membership in the
    // effective org. Previously gated only on is_platform() which made
    // the endpoint unreachable for org-admins.
    let is_admin = principal.is_platform()
        || db::repositories::MembershipRepository::new(state.db.clone())
            .is_manager_in_org(principal.user_id, tenant_id)
            .await
            .unwrap_or(false);
    if !is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only administrators can perform this action",
            )),
        ));
    }

    // Create the notification
    let notification = match state
        .critical_notification_repo
        .create(tenant_id, &req.title, &req.message, principal.user_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, org_id = %tenant_id, "Failed to create critical notification");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create notification",
                )),
            ));
        }
    };

    tracing::info!(
        notification_id = %notification.id,
        org_id = %tenant_id,
        created_by = %principal.user_id,
        "Critical notification created"
    );

    // Epic 2B: actually dispatch the critical notification to every active org
    // member. Critical notifications are `Urgent`, so the pipeline bypasses
    // per-user preferences (these alerts must reach everyone) while still
    // writing the mandatory in-app DB record per recipient. Dispatch failures
    // are logged but never fail the create — the acknowledgement-tracking row
    // (the source of truth for the dashboard) has already been persisted.
    match db::repositories::MembershipRepository::new(state.db.clone())
        .list_active_member_ids(tenant_id)
        .await
    {
        Ok(recipients) if !recipients.is_empty() => {
            let payload = common::notifications::Notification::new(
                uuid::Uuid::nil(),
                common::notifications::NotificationCategory::Announcements,
                notification.title.clone(),
                notification.message.clone(),
            )
            .with_priority(common::notifications::NotificationPriority::Urgent)
            .with_data(serde_json::json!({
                "critical_notification_id": notification.id,
                "organization_id": tenant_id,
            }));

            let (sent, skipped, failed) = state
                .notification_pipeline
                .broadcast(&recipients, &payload, Some(notification.id))
                .await;
            tracing::info!(
                notification_id = %notification.id,
                recipients = recipients.len(),
                channels_sent = sent,
                channels_skipped = skipped,
                channels_failed = failed,
                "Critical notification dispatched via pipeline (urgency bypass)"
            );
        }
        Ok(_) => {
            tracing::warn!(
                notification_id = %notification.id,
                org_id = %tenant_id,
                "Critical notification created but org has no active members to notify"
            );
        }
        Err(e) => {
            tracing::error!(
                notification_id = %notification.id,
                error = %e,
                "Failed to resolve recipients for critical notification dispatch"
            );
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(CreateCriticalNotificationResponse {
            id: notification.id,
            title: notification.title,
            message: notification.message,
            created_at: notification.created_at,
        }),
    ))
}

// ==================== List Notifications (Story 8A.2, AC-2) ====================

/// List all critical notifications for the organization.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{org_id}/critical-notifications",
    tag = "Critical Notifications",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Notifications list with acknowledgment status", body = Vec<CriticalNotificationResponse>),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_notifications(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<Vec<CriticalNotificationResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant_id = require_tenant_id(&principal)?;

    // Get notifications with acknowledgment status
    let notifications_with_status = match state
        .critical_notification_repo
        .get_for_org_with_status(principal.user_id, tenant_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, org_id = %tenant_id, "Failed to get critical notifications");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve notifications",
                )),
            ));
        }
    };

    let responses: Vec<CriticalNotificationResponse> = notifications_with_status
        .into_iter()
        .map(|(n, ack_at)| CriticalNotificationResponse {
            id: n.id,
            title: n.title,
            message: n.message,
            created_by: n.created_by,
            created_at: n.created_at,
            is_acknowledged: ack_at.is_some(),
            acknowledged_at: ack_at,
        })
        .collect();

    Ok(Json(responses))
}

// ==================== Get Unacknowledged (Story 8A.2, AC-2) ====================

/// Get unacknowledged critical notifications for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{org_id}/critical-notifications/unacknowledged",
    tag = "Critical Notifications",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Unacknowledged notifications", body = UnacknowledgedNotificationsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_unacknowledged(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<UnacknowledgedNotificationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant_id = require_tenant_id(&principal)?;

    // Get unacknowledged notifications
    let notifications = match state
        .critical_notification_repo
        .get_unacknowledged(principal.user_id, tenant_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, user_id = %principal.user_id, "Failed to get unacknowledged notifications");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve notifications",
                )),
            ));
        }
    };

    let count = notifications.len() as i64;
    let responses: Vec<CriticalNotificationResponse> = notifications
        .into_iter()
        .map(|n| CriticalNotificationResponse {
            id: n.id,
            title: n.title,
            message: n.message,
            created_by: n.created_by,
            created_at: n.created_at,
            is_acknowledged: false,
            acknowledged_at: None,
        })
        .collect();

    Ok(Json(UnacknowledgedNotificationsResponse {
        notifications: responses,
        count,
    }))
}

// ==================== Acknowledge (Story 8A.2, AC-3) ====================

/// Notification ID path parameter.
#[derive(Debug, Deserialize)]
pub struct NotificationPath {
    notification_id: Uuid,
}

/// Acknowledge a critical notification.
#[utoipa::path(
    post,
    path = "/api/v1/organizations/{org_id}/critical-notifications/{notification_id}/acknowledge",
    tag = "Critical Notifications",
    security(("bearer_auth" = [])),
    params(
        ("notification_id" = Uuid, Path, description = "Notification ID to acknowledge")
    ),
    responses(
        (status = 200, description = "Notification acknowledged", body = AcknowledgeCriticalNotificationResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Notification not found", body = ErrorResponse)
    )
)]
pub async fn acknowledge(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<NotificationPath>,
) -> Result<Json<AcknowledgeCriticalNotificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant_id = require_tenant_id(&principal)?;

    // Verify notification exists and belongs to the org
    let notification = match state
        .critical_notification_repo
        .get_by_id(path.notification_id)
        .await
    {
        Ok(Some(n)) => n,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Notification not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, notification_id = %path.notification_id, "Failed to get notification");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify notification",
                )),
            ));
        }
    };

    // Verify notification belongs to user's org
    if notification.organization_id != tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Notification not found")),
        ));
    }

    // Create acknowledgment
    let ack = match state
        .critical_notification_repo
        .acknowledge(path.notification_id, principal.user_id)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, notification_id = %path.notification_id, user_id = %principal.user_id, "Failed to acknowledge notification");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to acknowledge notification",
                )),
            ));
        }
    };

    tracing::info!(
        notification_id = %path.notification_id,
        user_id = %principal.user_id,
        "Critical notification acknowledged"
    );

    Ok(Json(AcknowledgeCriticalNotificationResponse {
        notification_id: ack.notification_id,
        acknowledged_at: ack.acknowledged_at,
    }))
}

// ==================== Get Stats (Story 8A.2, AC-4) ====================

/// Get acknowledgment statistics for a notification (admin only).
#[utoipa::path(
    get,
    path = "/api/v1/organizations/{org_id}/critical-notifications/{notification_id}/stats",
    tag = "Critical Notifications",
    security(("bearer_auth" = [])),
    params(
        ("notification_id" = Uuid, Path, description = "Notification ID")
    ),
    responses(
        (status = 200, description = "Notification statistics", body = CriticalNotificationStats),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 403, description = "Not authorized (admin only)", body = ErrorResponse),
        (status = 404, description = "Notification not found", body = ErrorResponse)
    )
)]
pub async fn get_stats(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<NotificationPath>,
) -> Result<Json<CriticalNotificationStats>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant_id = require_tenant_id(&principal)?;

    // P0-07: real admin role lookup (see create_notification above).
    let is_admin = principal.is_platform()
        || db::repositories::MembershipRepository::new(state.db.clone())
            .is_manager_in_org(principal.user_id, tenant_id)
            .await
            .unwrap_or(false);
    if !is_admin {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only administrators can perform this action",
            )),
        ));
    }

    // Verify notification exists and belongs to the org
    let notification = match state
        .critical_notification_repo
        .get_by_id(path.notification_id)
        .await
    {
        Ok(Some(n)) => n,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Notification not found")),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, notification_id = %path.notification_id, "Failed to get notification");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify notification",
                )),
            ));
        }
    };

    // Verify notification belongs to user's org
    if notification.organization_id != tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Notification not found")),
        ));
    }

    // Get stats
    let stats = match state
        .critical_notification_repo
        .get_stats(path.notification_id, tenant_id)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, notification_id = %path.notification_id, "Failed to get notification stats");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve statistics",
                )),
            ));
        }
    };

    Ok(Json(stats))
}

// ==================== Helper Functions ====================
//
// SECURITY: The previous `extract_tenant_context` helper deserialized the
// client-supplied `X-Tenant-Context` JSON header directly into a
// `TenantContext`. No JWT verification — any unauthenticated caller could
// forge tenancy AND claim arbitrary admin role. That helper has been
// deleted; every handler now goes through `RequestPrincipal` (verified
// bearer JWT + host-resolved tenant).
//
// The `is_admin` branches gating `create_notification` / `get_stats` now
// perform a real admin role lookup — platform principals always pass, others
// go through `MembershipRepository::is_manager_in_org` (P0-07); no
// client-supplied role is trusted.

/// Resolve the effective tenant id from a verified [`RequestPrincipal`].
fn require_tenant_id(
    principal: &RequestPrincipal,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    principal.effective_org.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "TENANT_REQUIRED",
                "Critical-notifications endpoints require a tenant-resolved request",
            )),
        )
    })
}

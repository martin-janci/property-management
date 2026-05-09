//! Critical Notifications routes (Epic 8A, Story 8A.2).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::{errors::ErrorResponse, TenantContext};
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
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateCriticalNotificationRequest>,
) -> Result<(StatusCode, Json<CreateCriticalNotificationResponse>), (StatusCode, Json<ErrorResponse>)>
{
    // Extract tenant context
    let tenant = extract_tenant_context(&headers)?;

    // Verify user is admin
    if !tenant.role.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only administrators can create critical notifications",
            )),
        ));
    }

    // Create the notification
    let notification = match state
        .critical_notification_repo
        .create(tenant.tenant_id, &req.title, &req.message, tenant.user_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, org_id = %tenant.tenant_id, "Failed to create critical notification");
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
        org_id = %tenant.tenant_id,
        created_by = %tenant.user_id,
        "Critical notification created"
    );

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
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<CriticalNotificationResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant = extract_tenant_context(&headers)?;

    // Get notifications with acknowledgment status
    let notifications_with_status = match state
        .critical_notification_repo
        .get_for_org_with_status(tenant.user_id, tenant.tenant_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, org_id = %tenant.tenant_id, "Failed to get critical notifications");
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
    headers: axum::http::HeaderMap,
) -> Result<Json<UnacknowledgedNotificationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant = extract_tenant_context(&headers)?;

    // Get unacknowledged notifications
    let notifications = match state
        .critical_notification_repo
        .get_unacknowledged(tenant.user_id, tenant.tenant_id)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, user_id = %tenant.user_id, "Failed to get unacknowledged notifications");
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
    headers: axum::http::HeaderMap,
    Path(path): Path<NotificationPath>,
) -> Result<Json<AcknowledgeCriticalNotificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant = extract_tenant_context(&headers)?;

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
    if notification.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Notification not found")),
        ));
    }

    // Create acknowledgment
    let ack = match state
        .critical_notification_repo
        .acknowledge(path.notification_id, tenant.user_id)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, notification_id = %path.notification_id, user_id = %tenant.user_id, "Failed to acknowledge notification");
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
        user_id = %tenant.user_id,
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
    headers: axum::http::HeaderMap,
    Path(path): Path<NotificationPath>,
) -> Result<Json<CriticalNotificationStats>, (StatusCode, Json<ErrorResponse>)> {
    // Extract tenant context
    let tenant = extract_tenant_context(&headers)?;

    // Verify user is admin
    if !tenant.role.is_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only administrators can view notification statistics",
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
    if notification.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Notification not found")),
        ));
    }

    // Get stats
    let stats = match state
        .critical_notification_repo
        .get_stats(path.notification_id, tenant.tenant_id)
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

/// Extract tenant context from request headers.
fn extract_tenant_context(
    headers: &axum::http::HeaderMap,
) -> Result<TenantContext, (StatusCode, Json<ErrorResponse>)> {
    // Get the X-Tenant-Context header
    let tenant_header = headers
        .get("X-Tenant-Context")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_CONTEXT",
                    "Tenant context required",
                )),
            )
        })?;

    // Parse the tenant context
    serde_json::from_str(tenant_header).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CONTEXT",
                "Invalid tenant context format",
            )),
        )
    })
}

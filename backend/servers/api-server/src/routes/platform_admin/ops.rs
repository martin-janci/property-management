//! Platform-admin operations endpoints: system health monitoring,
//! announcements, and scheduled maintenance.
//!
//! Stories 10B.3 (health) and 10B.4 (announcements + maintenance), plus the
//! public-facing announcement/maintenance endpoints. Behaviour is identical to
//! the original `platform_admin.rs`; this is a pure structural move.

use admin_core::RequireCapability;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{
    CreateMaintenanceRequest, CreateSystemAnnouncementRequest, MetricThreshold,
    ScheduledMaintenance, SystemAnnouncement, SystemAnnouncementAcknowledgment,
};
use db::repositories::{ActiveAnnouncement, HealthDashboard, MetricHistory};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::extract_super_admin_token;
use super::tenants::{default_page, default_page_size};
use crate::state::AppState;

// ==================== Health Monitoring Handlers (Story 10B.3) ====================

/// Query parameters for metric history.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct MetricHistoryQuery {
    /// Time range preset: 1h, 6h, 24h, 7d, 30d
    #[serde(default = "default_time_range")]
    pub range: String,
}

fn default_time_range() -> String {
    "24h".to_string()
}

/// Query parameters for alerts.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct AlertsQuery {
    /// Only show unacknowledged alerts
    #[serde(default)]
    pub active_only: bool,
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Page size (max 100)
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

/// Request to update a threshold.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateThresholdRequest {
    pub warning_threshold: Option<f64>,
    pub critical_threshold: Option<f64>,
}

/// Response for alert acknowledgment.
#[derive(Debug, Serialize, ToSchema)]
pub struct AlertAcknowledgeResponse {
    pub message: String,
    pub alert: db::models::MetricAlert,
}

/// Get health dashboard with current metrics and alerts.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/dashboard",
    responses(
        (status = 200, description = "Health dashboard retrieved", body = HealthDashboard),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_health_dashboard(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<HealthDashboard>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let dashboard = state
        .health_monitoring_repo
        .get_dashboard()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get health dashboard");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get health dashboard",
                )),
            )
        })?;

    Ok(Json(dashboard))
}

/// Get historical metrics for a specific metric.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/metrics/{name}/history",
    params(
        ("name" = String, Path, description = "Metric name"),
        MetricHistoryQuery
    ),
    responses(
        (status = 200, description = "Metric history retrieved", body = MetricHistory),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_metric_history(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(name): Path<String>,
    Query(query): Query<MetricHistoryQuery>,
) -> Result<Json<MetricHistory>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let now = chrono::Utc::now();
    let start_time = match query.range.as_str() {
        "1h" => now - chrono::Duration::hours(1),
        "6h" => now - chrono::Duration::hours(6),
        "24h" => now - chrono::Duration::hours(24),
        "7d" => now - chrono::Duration::days(7),
        "30d" => now - chrono::Duration::days(30),
        _ => now - chrono::Duration::hours(24), // default to 24h
    };

    let history = state
        .health_monitoring_repo
        .get_metric_history(&name, start_time, now)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, metric_name = %name, "Failed to get metric history");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get metric history",
                )),
            )
        })?;

    Ok(Json(history))
}

/// Get health alerts.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/alerts",
    params(AlertsQuery),
    responses(
        (status = 200, description = "Alerts retrieved", body = Vec<db::models::MetricAlert>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_health_alerts(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<AlertsQuery>,
) -> Result<Json<Vec<db::models::MetricAlert>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let alerts = if query.active_only {
        state.health_monitoring_repo.get_active_alerts().await
    } else {
        let offset = ((query.page.saturating_sub(1)) as i64) * (query.page_size as i64);
        state
            .health_monitoring_repo
            .get_alerts(query.page_size as i64, offset)
            .await
    }
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get alerts");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get alerts")),
        )
    })?;

    Ok(Json(alerts))
}

/// Acknowledge a health alert.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/health/alerts/{id}/acknowledge",
    params(
        ("id" = Uuid, Path, description = "Alert ID")
    ),
    responses(
        (status = 200, description = "Alert acknowledged", body = AlertAcknowledgeResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Alert not found or already acknowledged"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn acknowledge_alert(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertAcknowledgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _) = extract_super_admin_token(&headers, &state)?;

    let alert = state
        .health_monitoring_repo
        .acknowledge_alert(id, admin_user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, alert_id = %id, "Failed to acknowledge alert");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to acknowledge alert",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ALERT_NOT_FOUND",
                    "Alert not found or already acknowledged",
                )),
            )
        })?;

    Ok(Json(AlertAcknowledgeResponse {
        message: "Alert acknowledged".to_string(),
        alert,
    }))
}

/// Get all metric thresholds.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/health/thresholds",
    responses(
        (status = 200, description = "Thresholds retrieved", body = Vec<MetricThreshold>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn get_thresholds(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<MetricThreshold>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let thresholds = state
        .health_monitoring_repo
        .get_thresholds()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get thresholds");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get thresholds",
                )),
            )
        })?;

    Ok(Json(thresholds))
}

/// Update a metric threshold.
#[utoipa::path(
    put,
    path = "/api/v1/platform-admin/health/thresholds/{name}",
    params(
        ("name" = String, Path, description = "Metric name")
    ),
    request_body = UpdateThresholdRequest,
    responses(
        (status = 200, description = "Threshold updated", body = MetricThreshold),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Threshold not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Health"
)]
pub async fn update_threshold(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(name): Path<String>,
    Json(request): Json<UpdateThresholdRequest>,
) -> Result<Json<MetricThreshold>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let threshold = state
        .health_monitoring_repo
        .update_threshold(&name, request.warning_threshold, request.critical_threshold)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, metric_name = %name, "Failed to update threshold");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update threshold",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "THRESHOLD_NOT_FOUND",
                    "Metric threshold not found",
                )),
            )
        })?;

    Ok(Json(threshold))
}

// ==================== System Announcement Handlers (Story 10B.4) ====================

/// Query parameters for listing announcements.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct ListAnnouncementsAdminQuery {
    /// Include deleted announcements
    #[serde(default)]
    pub include_deleted: bool,
}

/// Request to update an announcement.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub message: Option<String>,
    pub severity: Option<String>,
    pub start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub end_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub is_dismissible: Option<bool>,
    pub requires_acknowledgment: Option<bool>,
}

/// Response for announcement creation.
#[derive(Debug, Serialize, ToSchema)]
pub struct AnnouncementResponse {
    pub message: String,
    pub announcement: SystemAnnouncement,
}

/// Response for maintenance scheduling.
#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenanceResponse {
    pub message: String,
    pub maintenance: ScheduledMaintenance,
    pub announcement: Option<SystemAnnouncement>,
}

/// Response for acknowledgment.
#[derive(Debug, Serialize, ToSchema)]
pub struct AcknowledgmentResponse {
    pub message: String,
    pub acknowledgment: SystemAnnouncementAcknowledgment,
}

/// List all system announcements (admin view).
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/announcements",
    params(ListAnnouncementsAdminQuery),
    responses(
        (status = 200, description = "Announcements retrieved", body = Vec<SystemAnnouncement>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn list_system_announcements(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ListAnnouncementsAdminQuery>,
) -> Result<Json<Vec<SystemAnnouncement>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let announcements = state
        .system_announcement_repo
        .list_all(query.include_deleted)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list announcements");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list announcements",
                )),
            )
        })?;

    Ok(Json(announcements))
}

/// Create a system announcement.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/announcements",
    request_body = CreateSystemAnnouncementRequest,
    responses(
        (status = 201, description = "Announcement created", body = AnnouncementResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn create_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CreateSystemAnnouncementRequest>,
) -> Result<(StatusCode, Json<AnnouncementResponse>), (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _) = extract_super_admin_token(&headers, &state)?;

    let announcement = state
        .system_announcement_repo
        .create_announcement(
            &request.title,
            &request.message,
            request.severity.as_str(),
            request.start_at,
            request.end_at,
            request.is_dismissible,
            request.requires_acknowledgment,
            admin_user_id,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create announcement",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(AnnouncementResponse {
            message: "Announcement created".to_string(),
            announcement,
        }),
    ))
}

/// Get a specific announcement.
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    responses(
        (status = 200, description = "Announcement retrieved", body = SystemAnnouncement),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Announcement not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn get_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<SystemAnnouncement>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let announcement = state
        .system_announcement_repo
        .get_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to get announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get announcement",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ANNOUNCEMENT_NOT_FOUND",
                    "Announcement not found",
                )),
            )
        })?;

    Ok(Json(announcement))
}

/// Update a system announcement.
#[utoipa::path(
    put,
    path = "/api/v1/platform-admin/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    request_body = UpdateAnnouncementRequest,
    responses(
        (status = 200, description = "Announcement updated", body = SystemAnnouncement),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Announcement not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn update_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateAnnouncementRequest>,
) -> Result<Json<SystemAnnouncement>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let announcement = state
        .system_announcement_repo
        .update_announcement(
            id,
            request.title.as_deref(),
            request.message.as_deref(),
            request.severity.as_deref(),
            request.start_at,
            request.end_at,
            request.is_dismissible,
            request.requires_acknowledgment,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to update announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update announcement",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ANNOUNCEMENT_NOT_FOUND",
                    "Announcement not found",
                )),
            )
        })?;

    Ok(Json(announcement))
}

/// Delete a system announcement (soft delete).
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/announcements/{id}",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    responses(
        (status = 204, description = "Announcement deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Announcement not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Announcements"
)]
pub async fn delete_system_announcement(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let deleted = state
        .system_announcement_repo
        .delete_announcement(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to delete announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete announcement",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "ANNOUNCEMENT_NOT_FOUND",
                "Announcement not found",
            )),
        ))
    }
}

/// Schedule a maintenance window.
#[utoipa::path(
    post,
    path = "/api/v1/platform-admin/maintenance",
    request_body = CreateMaintenanceRequest,
    responses(
        (status = 201, description = "Maintenance scheduled", body = MaintenanceResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Maintenance"
)]
pub async fn schedule_maintenance(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CreateMaintenanceRequest>,
) -> Result<(StatusCode, Json<MaintenanceResponse>), (StatusCode, Json<ErrorResponse>)> {
    let (admin_user_id, _) = extract_super_admin_token(&headers, &state)?;

    // Create announcement if requested
    let announcement = if request.create_announcement {
        let ann = state
            .system_announcement_repo
            .create_announcement(
                &format!("Scheduled Maintenance: {}", request.title),
                &format!(
                    "Maintenance is scheduled from {} to {}. {}",
                    request.start_at,
                    request.end_at,
                    request.description.as_deref().unwrap_or("")
                ),
                "warning",
                request.start_at - chrono::Duration::hours(24), // Announce 24h before
                Some(request.end_at),
                true,
                false,
                admin_user_id,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create maintenance announcement");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to create maintenance announcement",
                    )),
                )
            })?;
        Some(ann)
    } else {
        None
    };

    let maintenance = state
        .system_announcement_repo
        .schedule_maintenance(
            &request.title,
            request.description.as_deref(),
            request.start_at,
            request.end_at,
            request.is_read_only_mode,
            announcement.as_ref().map(|a| a.id),
            admin_user_id,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to schedule maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to schedule maintenance",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(MaintenanceResponse {
            message: "Maintenance scheduled".to_string(),
            maintenance,
            announcement,
        }),
    ))
}

/// Get upcoming maintenance windows (admin view).
#[utoipa::path(
    get,
    path = "/api/v1/platform-admin/maintenance",
    responses(
        (status = 200, description = "Maintenance windows retrieved", body = Vec<ScheduledMaintenance>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Maintenance"
)]
pub async fn get_upcoming_maintenance_admin(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<ScheduledMaintenance>>, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let maintenance = state
        .system_announcement_repo
        .get_upcoming_maintenance()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get upcoming maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get upcoming maintenance",
                )),
            )
        })?;

    Ok(Json(maintenance))
}

/// Delete a scheduled maintenance.
#[utoipa::path(
    delete,
    path = "/api/v1/platform-admin/maintenance/{id}",
    params(
        ("id" = Uuid, Path, description = "Maintenance ID")
    ),
    responses(
        (status = 204, description = "Maintenance deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires SuperAdmin role"),
        (status = 404, description = "Maintenance not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Platform Admin - Maintenance"
)]
pub async fn delete_scheduled_maintenance(
    _cap: RequireCapability,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    extract_super_admin_token(&headers, &state)?;

    let deleted = state
        .system_announcement_repo
        .delete_maintenance(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, maintenance_id = %id, "Failed to delete maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete maintenance",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "MAINTENANCE_NOT_FOUND",
                "Maintenance not found",
            )),
        ))
    }
}

// ==================== Public Announcement Handlers ====================

/// Get active announcements for current user.
#[utoipa::path(
    get,
    path = "/api/v1/announcements/active",
    responses(
        (status = 200, description = "Active announcements retrieved", body = Vec<ActiveAnnouncement>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Announcements"
)]
pub async fn get_active_announcements(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<ActiveAnnouncement>>, (StatusCode, Json<ErrorResponse>)> {
    // Extract user from token
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    let token = &auth_header[7..];
    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|e| {
            tracing::debug!(error = %e, "Invalid access token");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            )
        })?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    let announcements = state
        .system_announcement_repo
        .get_active_for_user(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get active announcements");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get announcements",
                )),
            )
        })?;

    Ok(Json(announcements))
}

/// Acknowledge an announcement.
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/acknowledge",
    params(
        ("id" = Uuid, Path, description = "Announcement ID")
    ),
    responses(
        (status = 200, description = "Announcement acknowledged", body = AcknowledgmentResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Announcements"
)]
pub async fn acknowledge_announcement(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<AcknowledgmentResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract user from token
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    let token = &auth_header[7..];
    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|e| {
            tracing::debug!(error = %e, "Invalid access token");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            )
        })?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    let acknowledgment = state
        .system_announcement_repo
        .record_acknowledgment(id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, announcement_id = %id, "Failed to acknowledge announcement");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to acknowledge announcement")),
            )
        })?;

    Ok(Json(AcknowledgmentResponse {
        message: "Announcement acknowledged".to_string(),
        acknowledgment,
    }))
}

/// Get upcoming maintenance (public).
#[utoipa::path(
    get,
    path = "/api/v1/maintenance/upcoming",
    responses(
        (status = 200, description = "Upcoming maintenance retrieved", body = Vec<ScheduledMaintenance>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Maintenance"
)]
pub async fn get_upcoming_maintenance(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScheduledMaintenance>>, (StatusCode, Json<ErrorResponse>)> {
    let maintenance = state
        .system_announcement_repo
        .get_upcoming_maintenance()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get upcoming maintenance");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get upcoming maintenance",
                )),
            )
        })?;

    Ok(Json(maintenance))
}

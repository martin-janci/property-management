//! Granular notification preference API routes (Epic 8B).
//!
//! Stories covered:
//! - 8B.1: Per-Event Type Preferences
//! - 8B.2: Per-Channel Delivery Preferences
//! - 8B.3: Notification Schedule (Do Not Disturb)
//! - 8B.4: Role-Based Default Preferences

use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use chrono::NaiveTime;
use common::errors::ErrorResponse;
use db::models::{
    CategorySummary, EventPreferenceWithDetails, EventPreferencesResponse,
    GroupedNotificationsResponse, NotificationDigest, NotificationEventCategory,
    NotificationGroupWithNotifications, NotificationSchedule, NotificationScheduleResponse,
    RoleDefaultsListResponse, RoleNotificationDefaults, UpdateEventPreferenceRequest,
    UpdateNotificationScheduleRequest, UpdateRoleDefaultsRequest,
};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::state::AppState;

/// Create router for granular notification preference endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        // Event type preferences (Stories 8B.1 & 8B.2)
        .route("/events", get(list_event_preferences))
        .route("/events/{event_type}", put(update_event_preference))
        .route("/events/reset", post(reset_event_preferences))
        .route(
            "/events/category/{category}",
            put(update_category_preferences),
        )
        // Schedule / quiet hours (Story 8B.3)
        .route("/schedule", get(get_schedule).put(update_schedule))
        // Role defaults (Story 8B.4) - admin endpoints
        .route("/roles", get(list_role_defaults))
        .route(
            "/roles/{role}",
            get(get_role_defaults)
                .put(update_role_defaults)
                .delete(delete_role_defaults),
        )
        .route("/roles/{role}/apply", post(apply_role_defaults))
        // Notification Grouping (Epic 29, Story 29.4)
        .route("/groups", get(list_notification_groups))
        .route(
            "/groups/{group_id}",
            get(get_notification_group).delete(delete_notification_group),
        )
        .route("/groups/{group_id}/read", post(mark_group_read))
        .route("/groups/read-all", post(mark_all_groups_read))
        // Notification Digests (Epic 29, Story 29.3)
        .route("/digests", get(list_digests))
}

// ============================================================================
// Event Type Preferences (Stories 8B.1 & 8B.2)
// ============================================================================

/// List all event preferences for the current user.
pub async fn list_event_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<EventPreferencesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let preferences = state
        .granular_notification_repo
        .get_user_event_preferences(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    // Group by category for summary
    let mut category_map: std::collections::HashMap<NotificationEventCategory, (i32, i32)> =
        std::collections::HashMap::new();

    for pref in &preferences {
        let entry = category_map.entry(pref.category).or_insert((0, 0));
        entry.0 += 1; // total
        if pref.push_enabled || pref.email_enabled || pref.in_app_enabled {
            entry.1 += 1; // enabled (at least one channel)
        }
    }

    let categories: Vec<CategorySummary> = NotificationEventCategory::all()
        .into_iter()
        .filter_map(|cat| {
            category_map
                .get(&cat)
                .map(|(total, enabled)| CategorySummary {
                    category: cat,
                    display_name: cat.to_string(),
                    total_events: *total,
                    enabled_events: *enabled,
                })
        })
        .collect();

    Ok(Json(EventPreferencesResponse {
        preferences,
        categories,
    }))
}

/// Update preference for a specific event type.
pub async fn update_event_preference(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(event_type): Path<String>,
    Json(request): Json<UpdateEventPreferenceRequest>,
) -> Result<Json<EventPreferenceWithDetails>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let _pref = state
        .granular_notification_repo
        .upsert_event_preference(
            user_id,
            &event_type,
            request.push_enabled,
            request.email_enabled,
            request.in_app_enabled,
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("RowNotFound") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        "EVENT_TYPE_NOT_FOUND",
                        "Unknown event type",
                    )),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
                )
            }
        })?;

    // Re-fetch with details
    let preferences = state
        .granular_notification_repo
        .get_user_event_preferences(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let updated = preferences
        .into_iter()
        .find(|p| p.event_type == event_type)
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "PREFERENCE_NOT_FOUND",
                    "Failed to update",
                )),
            )
        })?;

    info!(
        user_id = %user_id,
        event_type = %event_type,
        "Updated event notification preference"
    );

    Ok(Json(updated))
}

/// Reset all event preferences to defaults.
pub async fn reset_event_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<EventPreferencesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let deleted = state
        .granular_notification_repo
        .reset_event_preferences(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %user_id,
        deleted = deleted,
        "Reset event notification preferences to defaults"
    );

    // Return fresh preferences (all defaults)
    list_event_preferences(State(state), auth).await
}

/// Update all preferences for a category.
pub async fn update_category_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
    Json(request): Json<UpdateEventPreferenceRequest>,
) -> Result<Json<EventPreferencesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    // Parse category
    let category = match category.to_lowercase().as_str() {
        "fault" => NotificationEventCategory::Fault,
        "vote" => NotificationEventCategory::Vote,
        "announcement" => NotificationEventCategory::Announcement,
        "document" => NotificationEventCategory::Document,
        "message" => NotificationEventCategory::Message,
        "critical" => NotificationEventCategory::Critical,
        "finance" => NotificationEventCategory::Finance,
        "facility" => NotificationEventCategory::Facility,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_CATEGORY", "Unknown category")),
            ))
        }
    };

    let updated = state
        .granular_notification_repo
        .update_category_preferences(
            user_id,
            category,
            request.push_enabled,
            request.email_enabled,
            request.in_app_enabled,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %user_id,
        category = %category,
        updated = updated,
        "Updated category notification preferences"
    );

    list_event_preferences(State(state), auth).await
}

// ============================================================================
// Notification Schedule (Story 8B.3)
// ============================================================================

/// Get user's notification schedule.
pub async fn get_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<NotificationScheduleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let schedule = state
        .granular_notification_repo
        .get_user_schedule(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    match schedule {
        Some(s) => {
            // Check if currently in quiet hours
            let is_currently_quiet = check_quiet_hours(&s);
            Ok(Json(NotificationScheduleResponse {
                schedule: s,
                is_currently_quiet,
            }))
        }
        None => {
            // Return default schedule
            let default_schedule = NotificationSchedule {
                id: Uuid::nil(),
                user_id,
                quiet_hours_enabled: false,
                quiet_hours_start: None,
                quiet_hours_end: None,
                timezone: "UTC".to_string(),
                weekend_quiet_hours_enabled: false,
                weekend_quiet_hours_start: None,
                weekend_quiet_hours_end: None,
                digest_enabled: false,
                digest_frequency: None,
                digest_time: None,
                digest_day_of_week: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            Ok(Json(NotificationScheduleResponse {
                schedule: default_schedule,
                is_currently_quiet: false,
            }))
        }
    }
}

/// Update user's notification schedule.
pub async fn update_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(request): Json<UpdateNotificationScheduleRequest>,
) -> Result<Json<NotificationScheduleResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    // Parse time strings
    let quiet_start = request
        .quiet_hours_start
        .as_ref()
        .map(|s| parse_time(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_TIME", &e)),
            )
        })?;

    let quiet_end = request
        .quiet_hours_end
        .as_ref()
        .map(|s| parse_time(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_TIME", &e)),
            )
        })?;

    let weekend_start = request
        .weekend_quiet_hours_start
        .as_ref()
        .map(|s| parse_time(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_TIME", &e)),
            )
        })?;

    let weekend_end = request
        .weekend_quiet_hours_end
        .as_ref()
        .map(|s| parse_time(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_TIME", &e)),
            )
        })?;

    let digest_time = request
        .digest_time
        .as_ref()
        .map(|s| parse_time(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_TIME", &e)),
            )
        })?;

    let schedule = state
        .granular_notification_repo
        .upsert_schedule(
            user_id,
            request.quiet_hours_enabled,
            quiet_start,
            quiet_end,
            request.timezone.as_deref(),
            request.weekend_quiet_hours_enabled,
            weekend_start,
            weekend_end,
            request.digest_enabled,
            request.digest_frequency.as_deref(),
            digest_time,
            request.digest_day_of_week,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let is_currently_quiet = check_quiet_hours(&schedule);

    info!(
        user_id = %user_id,
        quiet_hours_enabled = schedule.quiet_hours_enabled,
        "Updated notification schedule"
    );

    Ok(Json(NotificationScheduleResponse {
        schedule,
        is_currently_quiet,
    }))
}

fn parse_time(s: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(s, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|_| format!("Invalid time format: {}. Use HH:MM or HH:MM:SS", s))
}

fn check_quiet_hours(schedule: &NotificationSchedule) -> bool {
    // Issue #980: evaluate in the user's stored timezone (and honour the
    // separate weekend window) via the shared, unit-tested helper, rather than
    // the previous UTC-only/weekend-blind comparison.
    crate::services::quiet_hours::evaluate(schedule, chrono::Utc::now()).active
}

// ============================================================================
// Role-Based Defaults (Story 8B.4)
// ============================================================================

/// List role notification defaults for an organization.
pub async fn list_role_defaults(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
) -> Result<Json<RoleDefaultsListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = tenant.tenant_id;

    let role_defaults = state
        .granular_notification_repo
        .get_role_defaults(organization_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(RoleDefaultsListResponse { role_defaults }))
}

/// Get role defaults for a specific role.
pub async fn get_role_defaults(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Path(role): Path<String>,
) -> Result<Json<RoleNotificationDefaults>, (StatusCode, Json<ErrorResponse>)> {
    let organization_id = tenant.tenant_id;

    let defaults = state
        .granular_notification_repo
        .get_role_default(organization_id, &role)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "ROLE_DEFAULTS_NOT_FOUND",
                    "No defaults configured for this role",
                )),
            )
        })?;

    Ok(Json(defaults))
}

/// Update role notification defaults.
pub async fn update_role_defaults(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(role): Path<String>,
    Json(request): Json<UpdateRoleDefaultsRequest>,
) -> Result<Json<RoleNotificationDefaults>, (StatusCode, Json<ErrorResponse>)> {
    // Changing org-wide role notification defaults is a manager action
    // (closes #808). Without this any member could rewrite org defaults.
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_ROLE",
                "Manager role required to change role notification defaults",
            )),
        ));
    }

    let organization_id = tenant.tenant_id;
    let created_by = auth.user_id;

    let quiet_start = request
        .default_quiet_hours_start
        .as_ref()
        .map(|s| parse_time(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_TIME", &e)),
            )
        })?;

    let quiet_end = request
        .default_quiet_hours_end
        .as_ref()
        .map(|s| parse_time(s))
        .transpose()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_TIME", &e)),
            )
        })?;

    let defaults = state
        .granular_notification_repo
        .upsert_role_defaults(
            organization_id,
            &role,
            request.event_preferences,
            request.default_quiet_hours_enabled,
            quiet_start,
            quiet_end,
            created_by,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        organization_id = %organization_id,
        role = %role,
        "Updated role notification defaults"
    );

    Ok(Json(defaults))
}

/// Delete role notification defaults.
pub async fn delete_role_defaults(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Path(role): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Deleting org-wide role notification defaults is a manager action
    // (closes #808).
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_ROLE",
                "Manager role required to delete role notification defaults",
            )),
        ));
    }

    let organization_id = tenant.tenant_id;

    state
        .granular_notification_repo
        .delete_role_defaults(organization_id, &role)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        organization_id = %organization_id,
        role = %role,
        "Deleted role notification defaults"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Apply role defaults to the current user.
pub async fn apply_role_defaults(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(role): Path<String>,
) -> Result<Json<EventPreferencesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;
    let organization_id = tenant.tenant_id;

    state
        .granular_notification_repo
        .apply_role_defaults_to_user(user_id, organization_id, &role)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %user_id,
        role = %role,
        "Applied role defaults to user"
    );

    list_event_preferences(State(state), auth).await
}

// ============================================================================
// Notification Grouping (Epic 29, Story 29.4)
// ============================================================================

/// Query parameters for listing notification groups.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGroupsQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
    #[serde(default)]
    pub include_read: bool,
}

fn default_limit() -> i32 {
    50
}

/// List notification groups for the current user.
pub async fn list_notification_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Query(query): axum::extract::Query<ListGroupsQuery>,
) -> Result<Json<GroupedNotificationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let groups = state
        .granular_notification_repo
        .get_grouped_notifications(user_id, query.limit, query.offset, query.include_read)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let total_unread = state
        .granular_notification_repo
        .get_unread_group_count(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(GroupedNotificationsResponse {
        groups,
        total_unread,
    }))
}

/// Get a specific notification group with its notifications.
pub async fn get_notification_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> Result<Json<NotificationGroupWithNotifications>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let groups = state
        .granular_notification_repo
        .get_grouped_notifications(user_id, 100, 0, true)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let group = groups
        .into_iter()
        .find(|g| g.group.id == group_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "GROUP_NOT_FOUND",
                    "Notification group not found",
                )),
            )
        })?;

    Ok(Json(group))
}

/// Delete a notification group.
pub async fn delete_notification_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let deleted = state
        .granular_notification_repo
        .delete_group(user_id, group_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    if !deleted {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "GROUP_NOT_FOUND",
                "Notification group not found",
            )),
        ));
    }

    info!(
        user_id = %user_id,
        group_id = %group_id,
        "Deleted notification group"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Mark a notification group as read.
pub async fn mark_group_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(group_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    state
        .granular_notification_repo
        .mark_group_read(user_id, group_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %user_id,
        group_id = %group_id,
        "Marked notification group as read"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Response for mark all groups read.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkAllReadResponse {
    pub marked_count: i32,
}

/// Mark all notification groups as read.
pub async fn mark_all_groups_read(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<MarkAllReadResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let marked_count = state
        .granular_notification_repo
        .mark_all_groups_read(user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    info!(
        user_id = %user_id,
        marked_count = marked_count,
        "Marked all notification groups as read"
    );

    Ok(Json(MarkAllReadResponse { marked_count }))
}

// ============================================================================
// Notification Digests (Epic 29, Story 29.3)
// ============================================================================

/// Query parameters for listing digests.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDigestsQuery {
    #[serde(default = "default_digest_limit")]
    pub limit: i32,
}

fn default_digest_limit() -> i32 {
    10
}

/// List recent notification digests for the current user.
pub async fn list_digests(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Query(query): axum::extract::Query<ListDigestsQuery>,
) -> Result<Json<Vec<NotificationDigest>>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

    let digests = state
        .granular_notification_repo
        .get_user_digests(user_id, query.limit)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    Ok(Json(digests))
}

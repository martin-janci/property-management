//! Notification Preferences routes (Epic 8A, Story 8A.1).

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    DisableAllWarningResponse, NotificationChannel, NotificationPreferenceResponse,
    NotificationPreferencesResponse, UpdateNotificationPreferenceRequest,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::state::AppState;

/// Create notification preferences router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_preferences))
        .route("/{channel}", patch(update_preference))
}

// ==================== Get Preferences (Story 8A.1, AC-1) ====================

/// Get notification preferences response.
#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GetPreferencesResponse {
    pub preferences: Vec<NotificationPreferenceResponse>,
    pub all_disabled_warning: Option<String>,
}

/// Get all notification preferences for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/users/me/notification-preferences",
    tag = "Notification Preferences",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Preferences retrieved", body = NotificationPreferencesResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_preferences(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<NotificationPreferencesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // Get all preferences for the user (RLS-scoped to the authenticated user).
    let preferences = match state
        .notification_pref_repo
        .get_by_user_rls(&mut **rls.conn(), user_id)
        .await
    {
        Ok(prefs) => prefs,
        Err(e) => {
            tracing::error!(error = %e, user_id = %user_id, "Failed to get notification preferences");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to retrieve preferences",
                )),
            ));
        }
    };

    // RLS lookup complete — clear context and return the connection to the pool.
    rls.release().await;

    // Check if all channels are disabled
    let all_disabled = preferences.iter().all(|p| !p.enabled);
    let all_disabled_warning = if all_disabled {
        Some(
            "All notification channels are disabled. You may miss important updates and alerts."
                .to_string(),
        )
    } else {
        None
    };

    let preference_responses: Vec<NotificationPreferenceResponse> =
        preferences.into_iter().map(|p| p.into()).collect();

    Ok(Json(NotificationPreferencesResponse {
        preferences: preference_responses,
        all_disabled_warning,
    }))
}

// ==================== Update Preference (Story 8A.1, AC-2, AC-3) ====================

/// Channel path parameter.
#[derive(Debug, Deserialize)]
pub struct ChannelPath {
    channel: String,
}

/// Update preference response.
#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferenceResponse {
    pub preference: NotificationPreferenceResponse,
    pub all_disabled_warning: Option<String>,
}

/// Update a notification preference for a specific channel.
#[utoipa::path(
    patch,
    path = "/api/v1/users/me/notification-preferences/{channel}",
    tag = "Notification Preferences",
    security(("bearer_auth" = [])),
    params(
        ("channel" = String, Path, description = "Notification channel (push, email, in_app)")
    ),
    request_body = UpdateNotificationPreferenceRequest,
    responses(
        (status = 200, description = "Preference updated", body = UpdatePreferenceResponse),
        (status = 400, description = "Invalid channel or confirmation required", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 409, description = "All channels would be disabled - confirmation required", body = DisableAllWarningResponse)
    )
)]
pub async fn update_preference(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<ChannelPath>,
    Json(req): Json<UpdateNotificationPreferenceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // Parse channel from path
    let channel = match path.channel.as_str() {
        "push" => NotificationChannel::Push,
        "email" => NotificationChannel::Email,
        "in_app" => NotificationChannel::InApp,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_CHANNEL",
                    "Channel must be one of: push, email, in_app",
                )),
            ));
        }
    };

    // If disabling, check if this would disable all channels.
    // (Replaces the deprecated would_disable_all() helper with inline RLS-scoped
    // queries: it would disable all iff exactly one channel is currently enabled
    // and it is this channel.)
    if !req.enabled {
        let enabled_count = match state
            .notification_pref_repo
            .count_enabled_rls(&mut **rls.conn(), user_id)
            .await
        {
            Ok(count) => count,
            Err(e) => {
                tracing::error!(error = %e, user_id = %user_id, "Failed to count enabled channels");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to update preference",
                    )),
                ));
            }
        };

        let current = match state
            .notification_pref_repo
            .get_by_user_and_channel_rls(&mut **rls.conn(), user_id, channel)
            .await
        {
            Ok(pref) => pref,
            Err(e) => {
                tracing::error!(error = %e, user_id = %user_id, "Failed to load channel preference");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to update preference",
                    )),
                ));
            }
        };

        let would_disable_all = enabled_count == 1 && current.map(|p| p.enabled).unwrap_or(false);

        if would_disable_all && !req.confirm_disable_all {
            // Return warning response requiring confirmation
            rls.release().await;
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "CONFIRMATION_REQUIRED",
                    "Disabling this channel would disable all notifications. You may miss important updates. Set confirmDisableAll to true to confirm.",
                )),
            ));
        }
    }

    // Update the preference (RLS-scoped to the authenticated user).
    let updated = match state
        .notification_pref_repo
        .update_channel_rls(&mut **rls.conn(), user_id, channel, req.enabled)
        .await
    {
        Ok(pref) => pref,
        Err(e) => {
            tracing::error!(error = %e, user_id = %user_id, channel = %channel, "Failed to update notification preference");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update preference",
                )),
            ));
        }
    };

    // Check if all channels are now disabled (RLS-scoped to the authenticated user).
    let has_any_enabled = match state
        .notification_pref_repo
        .count_enabled_rls(&mut **rls.conn(), user_id)
        .await
    {
        Ok(count) => count > 0,
        Err(e) => {
            tracing::error!(error = %e, "Failed to check enabled channels");
            true // Assume not all disabled if check fails
        }
    };

    // RLS operations complete — clear context and return the connection to the pool.
    rls.release().await;

    let all_disabled_warning = if !has_any_enabled {
        Some(
            "All notification channels are now disabled. You may miss important updates and alerts."
                .to_string(),
        )
    } else {
        None
    };

    tracing::info!(
        user_id = %user_id,
        channel = %channel,
        enabled = req.enabled,
        "Notification preference updated"
    );

    Ok(Json(UpdatePreferenceResponse {
        preference: updated.into(),
        all_disabled_warning,
    }))
}

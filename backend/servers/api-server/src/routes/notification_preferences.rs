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
    headers: axum::http::HeaderMap,
) -> Result<Json<NotificationPreferencesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    // P0-08: RLS-aware fetch. user_id from the verified JWT pins the
    // query to the caller; RlsConnection enforces tenant isolation on
    // the underlying connection.
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
    headers: axum::http::HeaderMap,
    Path(path): Path<ChannelPath>,
    Json(req): Json<UpdateNotificationPreferenceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Extract and validate access token
    let token = extract_bearer_token(&headers)?;
    let claims = validate_access_token(&state, &token)?;

    let user_id: uuid::Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

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

    // P0-08: RLS-aware "would this disable everything" check, built from
    // the same two primitives the deprecated `would_disable_all` used.
    if !req.enabled {
        let enabled_count = state
            .notification_pref_repo
            .count_enabled_rls(&mut **rls.conn(), user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %user_id, "Failed to count enabled channels");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to update preference",
                    )),
                )
            })?;
        let current = state
            .notification_pref_repo
            .get_by_user_and_channel_rls(&mut **rls.conn(), user_id, channel)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, user_id = %user_id, "Failed to load current preference");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "DATABASE_ERROR",
                        "Failed to update preference",
                    )),
                )
            })?;
        let would_disable_all = matches!(current, Some(p) if p.enabled) && enabled_count == 1;

        if would_disable_all && !req.confirm_disable_all {
            // Return warning response requiring confirmation
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "CONFIRMATION_REQUIRED",
                    "Disabling this channel would disable all notifications. You may miss important updates. Set confirmDisableAll to true to confirm.",
                )),
            ));
        }
    }

    // Update the preference (RLS-aware)
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

    // Check if all channels are now disabled (RLS-aware)
    let has_any_enabled = match state
        .notification_pref_repo
        .count_enabled_rls(&mut **rls.conn(), user_id)
        .await
        .map(|c| c > 0)
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(error = %e, "Failed to check enabled channels");
            true // Assume not all disabled if check fails
        }
    };

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

// ==================== Helper Functions ====================

/// Extract bearer token from Authorization header.
fn extract_bearer_token(
    headers: &axum::http::HeaderMap,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
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

    Ok(auth_header[7..].to_string())
}

/// Validate access token and return claims.
fn validate_access_token(
    state: &AppState,
    token: &str,
) -> Result<crate::services::jwt::Claims, (StatusCode, Json<ErrorResponse>)> {
    state.jwt_service.validate_access_token(token).map_err(|e| {
        tracing::debug!(error = %e, "Invalid access token");
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_TOKEN",
                "Invalid or expired token",
            )),
        )
    })
}

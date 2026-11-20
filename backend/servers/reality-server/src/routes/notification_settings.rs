//! Portal user notification settings: device push tokens and per-topic
//! preferences. Mounted inside the users router (`/api/v1/users/me/...`).
//!
//! GH #1300: `PortalPrincipal`; every query is keyed on `principal.user_id`.
//! `device_push_tokens` is under FORCE RLS keyed on `app.current_user_id`, so
//! the token handlers run on a connection whose request context is set to the
//! caller and cleared again before the connection goes back to the pool. The
//! saved-search alert drainer (`services::search_alert_drainer`) fans alerts
//! out to these tokens.

use crate::state::AppState;
use api_core::extractors::PortalPrincipal;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use db::models::PortalNotificationPreferences;
use db::repositories::DevicePushTokenRepository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// FCM tokens are ~152 chars, APNs 64 hex bytes; 512 matches api-server.
const MAX_TOKEN_LEN: usize = 512;
const MAX_DEVICE_ID_LEN: usize = 200;

/// Routes relative to the users router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me/push-tokens", post(register_push_token))
        .route("/me/push-tokens/{token}", delete(unregister_push_token))
        .route(
            "/me/notification-preferences",
            get(get_preferences).put(update_preferences),
        )
}

/// Register-token body. `platform` accepts the app names (`android`, `ios`)
/// as well as the transport names (`fcm`, `apns`).
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterPortalPushTokenRequest {
    pub token: String,
    pub platform: String,
    /// Optional device identifier, stored as the device name for diagnostics.
    pub device_id: Option<String>,
}

/// Register-token response (matches the mobile client's expectation).
#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterPortalPushTokenResponse {
    pub success: bool,
}

fn transport_platform(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "android" | "fcm" => Some("fcm"),
        "ios" | "apns" => Some("apns"),
        _ => None,
    }
}

fn bad_request(msg: &str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

fn internal(context: &str, e: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::error!(error = %e, "{context}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}

/// Register (or refresh) a device push token for the signed-in portal user.
#[utoipa::path(
    post,
    path = "/api/v1/users/me/push-tokens",
    tag = "Users",
    request_body = RegisterPortalPushTokenRequest,
    responses(
        (status = 200, description = "Token registered", body = RegisterPortalPushTokenResponse),
        (status = 400, description = "Invalid token or platform"),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn register_push_token(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Json(req): Json<RegisterPortalPushTokenRequest>,
) -> Result<Json<RegisterPortalPushTokenResponse>, (StatusCode, String)> {
    let token = req.token.trim();
    if token.is_empty() {
        return Err(bad_request("token must not be empty"));
    }
    if token.len() > MAX_TOKEN_LEN {
        return Err(bad_request("token exceeds maximum length"));
    }
    let platform = transport_platform(&req.platform)
        .ok_or_else(|| bad_request("platform must be one of: android, ios"))?;
    let device_name = req
        .device_id
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .map(|d| d.chars().take(MAX_DEVICE_ID_LEN).collect::<String>());

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| internal("acquire connection", e))?;
    db::set_request_context(&mut *conn, None, Some(principal.user_id), false)
        .await
        .map_err(|e| internal("set request context", e))?;

    // SECURITY: never log the raw token value.
    let result = DevicePushTokenRepository::new(state.db.clone())
        .upsert_rls(
            &mut *conn,
            principal.user_id,
            token,
            platform,
            Some("three.two.bit.ppt.reality"),
            device_name.as_deref(),
        )
        .await;
    let cleared = db::clear_request_context(&mut *conn).await;
    if cleared.is_err() {
        // Never return a connection that may still carry this user's context.
        conn.detach();
    }

    result.map_err(|e| internal("register push token", e))?;
    Ok(Json(RegisterPortalPushTokenResponse { success: true }))
}

/// Remove a device push token (e.g. on sign-out). Idempotent: 204 either way.
#[utoipa::path(
    delete,
    path = "/api/v1/users/me/push-tokens/{token}",
    tag = "Users",
    params(("token" = String, Path, description = "URL-encoded device push token")),
    responses(
        (status = 204, description = "Token removed"),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn unregister_push_token(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(token): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| internal("acquire connection", e))?;
    db::set_request_context(&mut *conn, None, Some(principal.user_id), false)
        .await
        .map_err(|e| internal("set request context", e))?;

    let result = DevicePushTokenRepository::new(state.db.clone())
        .delete_token_rls(&mut *conn, principal.user_id, &token)
        .await;
    if db::clear_request_context(&mut *conn).await.is_err() {
        conn.detach();
    }

    result.map_err(|e| internal("unregister push token", e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Read the caller's notification preferences (defaults when never saved).
#[utoipa::path(
    get,
    path = "/api/v1/users/me/notification-preferences",
    tag = "Users",
    responses(
        (status = 200, description = "Notification preferences", body = PortalNotificationPreferences),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn get_preferences(
    State(state): State<AppState>,
    principal: PortalPrincipal,
) -> Result<Json<PortalNotificationPreferences>, (StatusCode, String)> {
    state
        .reality_portal_repo
        .get_notification_preferences(principal.user_id)
        .await
        .map(Json)
        .map_err(|e| internal("get notification preferences", e))
}

/// Replace the caller's notification preferences.
#[utoipa::path(
    put,
    path = "/api/v1/users/me/notification-preferences",
    tag = "Users",
    request_body = PortalNotificationPreferences,
    responses(
        (status = 200, description = "Saved preferences", body = PortalNotificationPreferences),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn update_preferences(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Json(prefs): Json<PortalNotificationPreferences>,
) -> Result<Json<PortalNotificationPreferences>, (StatusCode, String)> {
    state
        .reality_portal_repo
        .upsert_notification_preferences(principal.user_id, prefs)
        .await
        .map(Json)
        .map_err(|e| internal("update notification preferences", e))
}

#[cfg(test)]
mod tests {
    use super::transport_platform;

    #[test]
    fn platform_names_map_to_transports() {
        assert_eq!(transport_platform("android"), Some("fcm"));
        assert_eq!(transport_platform(" iOS "), Some("apns"));
        assert_eq!(transport_platform("fcm"), Some("fcm"));
        assert_eq!(transport_platform("apns"), Some("apns"));
        assert_eq!(transport_platform("web"), None);
    }
}

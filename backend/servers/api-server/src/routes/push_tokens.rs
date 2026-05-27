// Epic 8A-3: Mobile OS push notification registration
//
// POST /api/v1/users/me/push-tokens  -- upsert a device FCM/APNs token
// DELETE /api/v1/users/me/push-tokens/{token} -- remove a token (e.g. on logout)

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{PushTokenResponse, RegisterPushTokenRequest};

use crate::state::AppState;

/// Create the push-tokens router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(register_push_token))
        .route("/{token}", delete(unregister_push_token))
}

// -- POST / -- register or refresh a device push token ------------------------

/// Register (or refresh) a device push token for the current user.
///
/// The endpoint is idempotent: sending the same token a second time simply
/// refreshes `last_seen_at`, so clients can call it on every app open.
#[utoipa::path(
    post,
    path = "/api/v1/users/me/push-tokens",
    tag = "Push Notifications",
    security(("bearer_auth" = [])),
    request_body = RegisterPushTokenRequest,
    responses(
        (status = 200, description = "Token registered", body = PushTokenResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    )
)]
pub async fn register_push_token(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<RegisterPushTokenRequest>,
) -> Result<Json<PushTokenResponse>, (StatusCode, Json<ErrorResponse>)> {
    // FCM tokens are ~152 chars; APNs device tokens are 64 hex bytes (128 chars).
    // 512 matches the DB CHECK constraint and is a safe ceiling for both platforms.
    const MAX_TOKEN_LEN: usize = 512;

    if req.token.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "token must not be empty")),
        ));
    }

    if req.token.len() > MAX_TOKEN_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "token exceeds maximum length",
            )),
        ));
    }

    let user_id = rls.user_id();

    // SECURITY: never log the raw token value -- it grants push-send authority for this device.
    let row = state
        .device_push_token_repo
        .upsert_rls(
            &mut **rls.conn(),
            user_id,
            &req.token,
            req.platform.as_str(),
            req.app_id.as_deref(),
            req.device_name.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("failed to upsert push token: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "failed to register token",
                )),
            )
        })?;

    rls.release().await;

    Ok(Json(PushTokenResponse::from(row)))
}

// -- DELETE /{token} -- unregister a device push token -----------------------

/// Remove a device push token (e.g. on user logout or app uninstall).
///
/// The endpoint is idempotent: if the token is not found the response is still 204.
/// Axum percent-decodes the path segment before passing it to the handler, so
/// callers must percent-encode any `/` or `%` characters in the raw token value.
#[utoipa::path(
    delete,
    path = "/api/v1/users/me/push-tokens/{token}",
    tag = "Push Notifications",
    security(("bearer_auth" = [])),
    params(("token" = String, Path, description = "URL-encoded device push token")),
    responses(
        (status = 204, description = "Token removed"),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
    )
)]
pub async fn unregister_push_token(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    // Axum automatically percent-decodes path segments.
    Path(token): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // SECURITY: never log the raw token value.
    state
        .device_push_token_repo
        .delete_token_rls(&mut **rls.conn(), user_id, &token)
        .await
        .map_err(|e| {
            tracing::error!("failed to delete push token: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "failed to remove token",
                )),
            )
        })?;

    rls.release().await;

    Ok(StatusCode::NO_CONTENT)
}

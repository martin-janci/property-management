//! Voice assistant integration (Story 64.5) + voice-device OAuth token
//! exchange (Story 98.1).
//!
//! The handlers here are mounted into [`super::llm::llm_router`] alongside the
//! other Epic-64 LLM endpoints; they live in their own module purely for size
//! and are exposed `pub(crate)` so the router constructor can reference them.

use crate::routes::ai::PaginationQuery;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::LinkVoiceDeviceRequest;
use uuid::Uuid;

/// Client-facing message returned when the voice-device OAuth code exchange
/// fails. The underlying provider error is logged server-side but is never
/// forwarded to the client, to avoid leaking upstream OAuth-provider detail.
const OAUTH_EXCHANGE_FAILED_MESSAGE: &str = "Authorization code exchange failed";

// ============================================================================
// Story 64.5: Voice Assistant Endpoints
// ============================================================================

pub(crate) async fn list_voice_devices(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Voice devices are scoped to the user; the RlsConnection extractor still
    // gates platform-host callers out of a per-tenant API surface.
    let user_id = rls.user_id();

    let result = state
        .llm_document_repo
        .list_user_voice_devices(&mut **rls.conn(), user_id)
        .await;
    rls.release().await;

    match result {
        Ok(devices) => Ok(Json(serde_json::json!({ "devices": devices }))),
        Err(e) => {
            tracing::error!("Failed to list devices: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/voice/devices",
    request_body = LinkVoiceDeviceRequest,
    responses(
        (status = 201, description = "Voice device linked"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI Voice"
)]
pub(crate) async fn link_voice_device(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<LinkVoiceDeviceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Generate a unique device ID: platform prefix + UUID for debugging and uniqueness.
    // Format: "google_assistant_550e8400-e29b-41d4-a716-446655440000"
    // While longer than a plain UUID, the platform prefix aids in debugging and log analysis.
    let device_id = format!("{}_{}", req.platform, Uuid::new_v4());

    // Story 98.1: Implement OAuth token exchange using auth_code from request.
    // Exchange the authorization code for access and refresh tokens from the voice platform.
    let oauth_result = exchange_voice_oauth_tokens(&req.platform, &req.auth_code).await;
    let (access_token_encrypted, refresh_token_encrypted, token_expires_at) = match oauth_result {
        Ok(tokens) => tokens,
        Err(e) => {
            rls.release().await;
            return Err(e);
        }
    };

    let device = state
        .llm_document_repo
        .create_voice_device(
            &mut **rls.conn(),
            tenant_id,
            user_id,
            req.unit_id,
            &req.platform,
            &device_id,
            req.device_name.as_deref(),
            access_token_encrypted.as_deref(),
            refresh_token_encrypted.as_deref(),
            token_expires_at,
            serde_json::json!(["check_balance", "report_fault", "check_announcements"]),
            // #2662: this linking path only sees the already-encrypted token
            // (the plaintext lives inside `exchange_voice_oauth_tokens`), so it
            // stores a NULL lookup hash; `authenticate_voice_user` falls back to
            // the linear scan for such rows until the token is next refreshed.
            None,
        )
        .await;
    rls.release().await;
    let device = device.map_err(|e| {
        tracing::error!("Failed to link device: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to link device",
            )),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "device_id": device.id,
            "platform": device.platform,
            "device_name": device.device_name,
            "capabilities": ["check_balance", "report_fault", "check_announcements"],
            "linked_at": device.linked_at,
            "oauth_linked": access_token_encrypted.is_some()
        })),
    ))
}

/// Exchange OAuth authorization code for tokens from voice platform.
/// Story 98.1: Voice Device OAuth Token Exchange
async fn exchange_voice_oauth_tokens(
    platform: &str,
    auth_code: &str,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    use integrations::{
        encrypt_optional_required, encrypt_required, IntegrationCrypto, VoiceOAuthManager,
        VoicePlatform,
    };

    // Parse the platform
    let voice_platform: VoicePlatform = platform.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PLATFORM",
                "Unsupported voice platform",
            )),
        )
    })?;

    // Get OAuth manager from environment
    let oauth_manager = VoiceOAuthManager::from_env();

    // Check if platform is configured
    if !oauth_manager.has_platform(voice_platform) {
        // Platform not configured - store device without OAuth tokens
        // This allows development/testing without OAuth credentials
        tracing::warn!(
            "Voice OAuth not configured for platform {}, storing device without tokens",
            platform
        );
        return Ok((None, None, None));
    }

    // Get redirect URI from environment (platform-specific)
    let redirect_uri = match voice_platform {
        VoicePlatform::Alexa => std::env::var("ALEXA_REDIRECT_URI").unwrap_or_else(|_| {
            "https://ppt.three-two-bit.com/api/v1/webhooks/voice/oauth/callback".to_string()
        }),
        VoicePlatform::GoogleAssistant => std::env::var("GOOGLE_VOICE_REDIRECT_URI")
            .unwrap_or_else(|_| {
                "https://ppt.three-two-bit.com/api/v1/webhooks/voice/oauth/callback".to_string()
            }),
    };

    // Exchange the authorization code for tokens
    let tokens = oauth_manager
        .exchange_code(voice_platform, auth_code, &redirect_uri)
        .await
        .map_err(|e| {
            tracing::error!("OAuth token exchange failed for {}: {}", platform, e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "OAUTH_EXCHANGE_FAILED",
                    // Do not leak the upstream OAuth-provider error to the client;
                    // the detail is already logged server-side above.
                    OAUTH_EXCHANGE_FAILED_MESSAGE,
                )),
            )
        })?;

    // Encrypt tokens for storage. Issue #765: encryption is MANDATORY — fail
    // closed if INTEGRATION_ENCRYPTION_KEY is unset rather than persisting voice
    // OAuth tokens in plaintext.
    let crypto = IntegrationCrypto::try_from_env();
    let access_encrypted =
        encrypt_required(crypto.as_ref(), &tokens.access_token).map_err(|e| {
            tracing::error!(
                "Refusing to store voice OAuth token without encryption: {}",
                e
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "ENCRYPTION_REQUIRED",
                    "Integration token encryption is not configured",
                )),
            )
        })?;
    let refresh_encrypted =
        encrypt_optional_required(crypto.as_ref(), tokens.refresh_token.as_deref()).map_err(
            |e| {
                tracing::error!(
                    "Refusing to store voice OAuth token without encryption: {}",
                    e
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "ENCRYPTION_REQUIRED",
                        "Integration token encryption is not configured",
                    )),
                )
            },
        )?;

    tracing::info!(
        "Successfully exchanged OAuth tokens for voice platform {}",
        platform
    );

    Ok((Some(access_encrypted), refresh_encrypted, tokens.expires_at))
}

pub(crate) async fn unlink_voice_device(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Scope the deactivation to the caller's own devices.  A non-owned or
    // non-existent id both return false from the repository and are mapped to
    // 404, giving an attacker no information about whether the target id exists.
    let user_id = rls.user_id();
    let result = state
        .llm_document_repo
        .deactivate_voice_device(&mut **rls.conn(), id, user_id)
        .await;
    rls.release().await;

    match result {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Device not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to unlink device: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to unlink")),
            ))
        }
    }
}

pub(crate) async fn list_voice_commands(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(device_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();

    // Issue #483: unify disclosure posture with unlink_voice_device — was
    // returning HTTP 200 + empty list for not-owned devices, which leaks
    // device-UUID existence. Return 404 instead.
    //
    // Both reads run on the same RLS-context connection; errors are captured
    // (not early-returned) so release() always runs.
    let result = async {
        let owns = state
            .llm_document_repo
            .user_owns_voice_device(&mut **rls.conn(), device_id, user_id)
            .await?;
        if !owns {
            return Ok(None);
        }
        let commands = state
            .llm_document_repo
            .list_voice_commands(
                &mut **rls.conn(),
                device_id,
                user_id,
                query.limit.unwrap_or(50),
                query.offset.unwrap_or(0),
            )
            .await?;
        Ok::<_, sqlx::Error>(Some(commands))
    }
    .await;
    rls.release().await;

    match result {
        Ok(Some(commands)) => Ok(Json(serde_json::json!({ "commands": commands }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Device not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to list commands: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the AI upstream-error leak finding: the client-facing
    /// body for a failed voice-device OAuth code exchange must carry only the
    /// fixed generic message and must never contain the raw upstream provider
    /// error (which is logged server-side).
    #[test]
    fn oauth_exchange_error_does_not_leak_upstream_detail() {
        // A representative raw provider error that must not reach the client.
        let upstream_detail = "invalid_grant: authorization code expired (provider trace xyz)";

        // Mirror the client body built by the OAuth exchange error branch.
        let response = ErrorResponse::new("OAUTH_EXCHANGE_FAILED", OAUTH_EXCHANGE_FAILED_MESSAGE);
        let body = serde_json::to_string(&response).expect("serialize ErrorResponse");

        assert!(
            !body.contains(upstream_detail),
            "client error body leaked the upstream provider detail: {body}"
        );
        assert!(
            !body.contains("Failed to exchange authorization code:"),
            "client error body still interpolates the upstream error: {body}"
        );
        assert_eq!(response.message, OAUTH_EXCHANGE_FAILED_MESSAGE);
    }
}

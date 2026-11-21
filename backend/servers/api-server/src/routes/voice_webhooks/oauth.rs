//! Voice OAuth token linking: code exchange + token refresh (Stories 93.1/98.1).
//!
//! Split verbatim from `voice_webhooks.rs`. Both endpoints are invoked by the
//! authenticated Property-Management user from the voice account-linking flow
//! (never anonymously by the voice platform), require a valid PM access token,
//! and carry over the security posture of the original module unchanged:
//! fail-closed simulated tokens gated behind `debug_assertions` (#890),
//! fail-closed token encryption (#765), device ownership enforcement on refresh
//! (#2769), and the atomic active-device upsert (#2794). The two per-user
//! sliding-window rate limiters (#2769) that throttle these handlers live here
//! too — they are two **independent** maps (exchange / refresh) by design. The
//! router that mounts both handlers lives in [`super`].

use crate::routes::rate_limit::{rate_limit_allowed, InProcessRateLimiter};
use crate::services::voice_oauth::{
    classify_voice_oauth_error, encrypt_voice_token_pair, voice_oauth_error_response,
    voice_simulated_oauth_allowed, OAUTH_EXCHANGE_FAILED_MESSAGE, OAUTH_REFRESH_FAILED_MESSAGE,
};
use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use common::errors::ErrorResponse;
use db::models::{
    voice_platform, OAuthError, VoiceOAuthExchangeRequest, VoiceOAuthExchangeResponse,
    VoiceTokenRefreshRequest, VoiceTokenRefreshResult,
};
use db::RlsPool;
use integrations::IntegrationCrypto;
use uuid::Uuid;

// ============================================================================
// Handler error channel
// ============================================================================
//
// The voice OAuth account-linking handlers emit two body shapes. Most failures
// use the PPT-standard `ErrorResponse` envelope (`code`/`message`), but the
// sanitized **upstream exchange/refresh failure** path returns an RFC 6749 §5.2
// OAuth error body (`{error, error_description}`) — the account-linking flow is
// an OAuth 2.0 token exchange, so its grant failures must speak the spec's wire
// shape and status codes (`invalid_grant` 400, `server_error` 500, …). This
// enum lets both shapes flow through a single handler `Result` while each
// serialises to its own wire form; the `From` impls make `?` carry either an
// `ErrorResponse` tuple or an `OAuthError` tuple unchanged.
enum VoiceOAuthHandlerError {
    /// PPT-standard `{code, message, …}` error envelope.
    Standard(StatusCode, Json<ErrorResponse>),
    /// RFC 6749 §5.2 `{error, error_description?}` OAuth error.
    OAuth(StatusCode, Json<OAuthError>),
}

impl IntoResponse for VoiceOAuthHandlerError {
    fn into_response(self) -> Response {
        match self {
            Self::Standard(status, body) => (status, body).into_response(),
            Self::OAuth(status, body) => (status, body).into_response(),
        }
    }
}

impl From<(StatusCode, Json<ErrorResponse>)> for VoiceOAuthHandlerError {
    fn from((status, body): (StatusCode, Json<ErrorResponse>)) -> Self {
        Self::Standard(status, body)
    }
}

impl From<(StatusCode, Json<OAuthError>)> for VoiceOAuthHandlerError {
    fn from((status, body): (StatusCode, Json<OAuthError>)) -> Self {
        Self::OAuth(status, body)
    }
}

// ============================================================================
// Rate limiting (#2769)
// ============================================================================
//
// `oauth_token_refresh` rotates the linked user's upstream OAuth token and
// makes a live call to Amazon/Google. Even for an authenticated owner, an
// unbounded loop is an amplification vector, so throttle per PM user with the
// shared in-process sliding-window limiter (same primitive as `routes::mfa`).
// `std::time::Duration` is fully qualified here to avoid clashing with the
// `chrono::Duration` already imported in this module.
const VOICE_REFRESH_MAX_ATTEMPTS: u32 = 10;
const VOICE_REFRESH_WINDOW: std::time::Duration = std::time::Duration::from_secs(900);
const VOICE_REFRESH_SWEEP_THRESHOLD: usize = 1024;

static VOICE_REFRESH_RATE_LIMITER: std::sync::LazyLock<InProcessRateLimiter<Uuid>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Record a refresh attempt for `user_id`; `false` once more than
/// `VOICE_REFRESH_MAX_ATTEMPTS` occur in a rolling `VOICE_REFRESH_WINDOW`.
fn voice_refresh_attempt_allowed(user_id: Uuid) -> bool {
    rate_limit_allowed(
        &VOICE_REFRESH_RATE_LIMITER,
        user_id,
        VOICE_REFRESH_MAX_ATTEMPTS,
        VOICE_REFRESH_WINDOW,
        VOICE_REFRESH_SWEEP_THRESHOLD,
    )
}

// `oauth_token_exchange` completes account linking by making a live call to
// Amazon/Google (`exchange_code`). Like `oauth_token_refresh`, an authenticated
// caller must not be able to loop it and amplify against the upstream OAuth
// provider, so throttle per PM user with the same in-process sliding-window
// limiter idiom (#2769).
const VOICE_EXCHANGE_MAX_ATTEMPTS: u32 = 10;
const VOICE_EXCHANGE_WINDOW: std::time::Duration = std::time::Duration::from_secs(900);
const VOICE_EXCHANGE_SWEEP_THRESHOLD: usize = 1024;

static VOICE_EXCHANGE_RATE_LIMITER: std::sync::LazyLock<InProcessRateLimiter<Uuid>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Record an exchange attempt for `user_id`; `false` once more than
/// `VOICE_EXCHANGE_MAX_ATTEMPTS` occur in a rolling `VOICE_EXCHANGE_WINDOW`.
fn voice_exchange_attempt_allowed(user_id: Uuid) -> bool {
    rate_limit_allowed(
        &VOICE_EXCHANGE_RATE_LIMITER,
        user_id,
        VOICE_EXCHANGE_MAX_ATTEMPTS,
        VOICE_EXCHANGE_WINDOW,
        VOICE_EXCHANGE_SWEEP_THRESHOLD,
    )
}

// ============================================================================
// OAuth Token Exchange (Story 93.1)
// ============================================================================

/// Exchange OAuth authorization code for tokens.
///
/// This endpoint completes account linking: it is invoked by the
/// authenticated Property Management user (from the voice account-linking
/// flow), **not** by the voice platform itself. It therefore requires a
/// valid PM access token (`AuthUser`) and binds the resulting voice device
/// to that user's identity and organization — never to random UUIDs
/// (security fix #890).
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/oauth/exchange",
    request_body = VoiceOAuthExchangeRequest,
    responses(
        (status = 200, description = "Token exchange successful", body = VoiceOAuthExchangeResponse),
        (status = 400, description = "Invalid authorization code"),
        (status = 401, description = "Unauthorized - missing or invalid PM access token"),
        (status = 403, description = "Forbidden - no active organization context"),
        (status = 429, description = "Too many exchange attempts"),
        (status = 500, description = "Token exchange failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "Voice OAuth"
)]
pub(crate) async fn oauth_token_exchange(
    State(state): State<AppState>,
    api_core::TenantAuthUser(auth): api_core::TenantAuthUser,
    Json(request): Json<VoiceOAuthExchangeRequest>,
) -> Result<Json<VoiceOAuthExchangeResponse>, VoiceOAuthHandlerError> {
    use integrations::{VoiceOAuthManager, VoicePlatform};

    // Throttle authenticated abuse: a valid principal must not be able to loop
    // this endpoint and amplify against the upstream OAuth provider (#2769).
    if !voice_exchange_attempt_allowed(auth.user_id) {
        tracing::warn!(
            user_id = %auth.user_id,
            platform = %request.platform,
            "Voice OAuth exchange rate limit exceeded"
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse::new(
                "RATE_LIMITED",
                "Too many token exchange attempts; try again later",
            )),
        )
            .into());
    }

    // Bind the linking to the authenticated PM user + their active org.
    // Without an org context we cannot create a tenant-scoped voice device.
    let user_id = auth.user_id;
    let org_id = auth.tenant_id.ok_or_else(|| {
        tracing::warn!(
            user_id = %auth.user_id,
            "Voice OAuth exchange rejected: no active organization context"
        );
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "NO_ORGANIZATION_CONTEXT",
                "An active organization context is required to link a voice device",
            )),
        )
    })?;

    tracing::info!(
        user_id = %user_id,
        org_id = %org_id,
        platform = %request.platform,
        "OAuth token exchange for voice device linking"
    );

    // Validate platform
    if request.platform != voice_platform::ALEXA
        && request.platform != voice_platform::GOOGLE_ASSISTANT
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PLATFORM",
                "Unsupported voice platform",
            )),
        )
            .into());
    }

    // Story 98.1: Implement actual OAuth token exchange
    let voice_platform: VoicePlatform = request.platform.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PLATFORM",
                "Unsupported voice platform",
            )),
        )
    })?;

    // Get OAuth manager and check if platform is configured
    let oauth_manager = VoiceOAuthManager::from_env();
    let crypto = IntegrationCrypto::try_from_env();

    let (access_encrypted, refresh_encrypted, expires_at, access_hash) = if oauth_manager
        .has_platform(voice_platform)
    {
        // Get redirect URI from environment
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
            .exchange_code(voice_platform, &request.auth_code, &redirect_uri)
            .await
            .map_err(|e| {
                // RFC 6749 §5.2: a provider grant rejection becomes
                // `invalid_grant` (400); transport/parse faults become
                // `server_error` (500). Upstream detail is logged, never sent.
                voice_oauth_error_response(
                    classify_voice_oauth_error(&e, OAUTH_EXCHANGE_FAILED_MESSAGE),
                    e,
                )
            })?;

        // #765 (fail-closed encryption) + #2662 (indexed hash in lock-step),
        // centralized in `encrypt_voice_token_pair`.
        let (access_encrypted, refresh_encrypted, access_hash) = encrypt_voice_token_pair(
            crypto.as_ref(),
            &tokens.access_token,
            tokens.refresh_token.as_deref(),
        )?;

        (
            access_encrypted,
            refresh_encrypted,
            tokens.expires_at,
            access_hash,
        )
    } else if cfg!(debug_assertions) {
        // Platform not configured - use simulated tokens for development only.
        // Gated behind `debug_assertions` (security fix #890): release builds
        // must never mint fake voice tokens.
        tracing::warn!(
            "Voice OAuth not configured for platform {}, using simulated tokens (debug build)",
            request.platform
        );
        let simulated_access = format!("voice_access_{}_{}", request.platform, Uuid::new_v4());
        let simulated_refresh = format!("voice_refresh_{}_{}", request.platform, Uuid::new_v4());
        let (access_encrypted, refresh_encrypted, access_hash) =
            encrypt_voice_token_pair(crypto.as_ref(), &simulated_access, Some(&simulated_refresh))?;
        (
            access_encrypted,
            refresh_encrypted,
            Some(Utc::now() + Duration::hours(1)),
            access_hash,
        )
    } else {
        tracing::error!(
            "Voice OAuth not configured for platform {} in a release build",
            request.platform
        );
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "OAUTH_NOT_CONFIGURED",
                "Voice OAuth is not configured for this platform",
            )),
        )
            .into());
    };

    // Device is bound to the authenticated PM user and their organization
    // (derived above from the verified access token — never random UUIDs).
    //
    // PAP-170 (PAP-150 P2): bind the verified PM user's org/user RLS context for
    // the device write instead of touching the raw `state.db` pool.
    // `voice_assistant_devices` is not RLS-bound today, but binding context is
    // defense in depth; org/user come from the verified PM access token above.
    let mut guard = RlsPool::new(state.db.clone())
        .acquire_with_rls(org_id, user_id, false)
        .await
        .map_err(|e| {
            tracing::error!("Failed to acquire RLS connection: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DEVICE_CREATION_FAILED",
                    "Failed to link voice device",
                )),
            )
        })?;

    // Upsert on (org, user, platform). A re-link must ROTATE the tokens on the
    // existing device row for this tuple, not insert an independent new row:
    // the old code minted a fresh random `device_id` and created a row every
    // call, so re-linking accumulated multiple active devices whose previously
    // stored tokens stayed independently usable (stale-token accumulation). By
    // reusing the existing row we overwrite its `access_token_encrypted` /
    // `access_token_hash`, which invalidates the superseded token.
    //
    // #2794: the "at most one active device per (org, user, platform)" invariant
    // is enforced by the DATABASE — the `uniq_voice_devices_active_org_user_platform`
    // partial UNIQUE index (WHERE is_active = TRUE, migration 00231) is the single
    // writer. A single atomic `INSERT ... ON CONFLICT ... DO UPDATE` (upsert) makes
    // the DB the arbiter: first link inserts, re-link conflicts on the index and
    // rotates the tokens on the existing row in place. This closes the
    // concurrent-re-link race the previous sequential find -> branch ->
    // insert/update path allowed (two racing requests could each observe "no
    // existing row" and both INSERT). On conflict the `device_id` is preserved, so
    // the returned id is stable across re-links.
    let device_id = format!("{}_{}", request.platform, Uuid::new_v4());
    let device = state
        .llm_document_repo
        .upsert_active_voice_device(
            guard.conn(),
            org_id,
            user_id,
            None,
            &request.platform,
            &device_id,
            Some("Voice Assistant"),
            Some(&access_encrypted),
            refresh_encrypted.as_deref(),
            expires_at,
            serde_json::json!(["check_balance", "report_fault", "check_announcements"]),
            access_hash.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to upsert voice device: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DEVICE_CREATION_FAILED",
                    "Failed to link voice device",
                )),
            )
        })?;

    tracing::info!(
        "Voice device linked successfully: {} (platform: {})",
        device.id,
        request.platform
    );

    Ok(Json(VoiceOAuthExchangeResponse {
        device_id: device.id,
        success: true,
        message: "Voice assistant linked successfully".to_string(),
        capabilities: vec![
            "check_balance".to_string(),
            "report_fault".to_string(),
            "check_announcements".to_string(),
        ],
    }))
}

/// Refresh OAuth tokens for a voice device.
///
/// SECURITY (#2769): this endpoint rotates the linked user's upstream
/// Amazon/Google OAuth token, so it is invoked by the **authenticated
/// Property Management user** (from the voice account-management flow), not
/// anonymously by the voice platform. It requires a valid PM access token
/// (`AuthUser`) and only refreshes a device the caller **owns**
/// (`auth.user_id == device.user_id`) or that a platform admin manages.
///
/// Before this fix the handler took only `State` + `Json<{device_id}>` with no
/// auth extractor, so any caller who learned a `device_id` could rotate that
/// user's token and amplify unauthenticated traffic against the upstream OAuth
/// endpoints. Authenticated callers are additionally throttled per-user.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/oauth/refresh",
    request_body = VoiceTokenRefreshRequest,
    responses(
        (status = 200, description = "Token refresh successful", body = VoiceTokenRefreshResult),
        (status = 401, description = "Unauthorized - missing or invalid PM access token"),
        (status = 403, description = "Forbidden - caller does not own the device"),
        (status = 404, description = "Device not found"),
        (status = 429, description = "Too many refresh attempts"),
        (status = 500, description = "Token refresh failed"),
    ),
    security(("bearer_auth" = [])),
    tag = "Voice OAuth"
)]
pub(crate) async fn oauth_token_refresh(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Json(request): Json<VoiceTokenRefreshRequest>,
) -> Result<Json<VoiceTokenRefreshResult>, VoiceOAuthHandlerError> {
    use integrations::{decrypt_if_available, VoiceOAuthManager, VoicePlatform};

    // Throttle authenticated abuse: a valid principal must not be able to loop
    // this endpoint and amplify against the upstream OAuth provider (#2769).
    if !voice_refresh_attempt_allowed(auth.user_id) {
        tracing::warn!(
            user_id = %auth.user_id,
            device_id = %request.device_id,
            "Voice OAuth refresh rate limit exceeded"
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse::new(
                "RATE_LIMITED",
                "Too many token refresh attempts; try again later",
            )),
        )
            .into());
    }

    // Find the device.
    // `voice_assistant_devices` is not RLS-bound (see migration 00226), so the
    // lookup runs on a context-cleared connection; the ownership check below is
    // what authorizes the rotation.
    let rls_pool = RlsPool::new(state.db.clone());
    let mut lookup = rls_pool.acquire_public().await.map_err(|e| {
        tracing::error!("Database error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
        )
    })?;
    let device = state
        .llm_document_repo
        .find_voice_device(&mut **lookup.conn(), request.device_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "DEVICE_NOT_FOUND",
                    "Voice device not found",
                )),
            )
        })?;
    drop(lookup);

    // Authorize: only the device's owner (or a platform admin) may rotate its
    // upstream OAuth token (#2769). Reject anyone else before touching the
    // provider or the stored token.
    if auth.user_id != device.user_id && !auth.is_platform_admin() {
        tracing::warn!(
            caller_user_id = %auth.user_id,
            device_id = %device.id,
            device_owner = %device.user_id,
            "Voice OAuth refresh rejected: caller does not own the device"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You do not have permission to refresh this device's tokens",
            )),
        )
            .into());
    }

    // Check if device has refresh token
    let refresh_token_encrypted = match &device.refresh_token_encrypted {
        Some(token) => token,
        None => {
            return Ok(Json(VoiceTokenRefreshResult {
                success: false,
                expires_at: None,
                error: Some("No refresh token available".to_string()),
            }));
        }
    };

    // Story 98.1: Use actual OAuth client to refresh tokens
    let crypto = IntegrationCrypto::try_from_env();
    let oauth_manager = VoiceOAuthManager::from_env();

    // Parse platform
    let voice_platform: VoicePlatform = device.platform.parse().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INVALID_PLATFORM",
                "Device has invalid platform",
            )),
        )
    })?;

    let (new_access_encrypted, new_refresh_encrypted, new_expires_at, new_access_hash) =
        if oauth_manager.has_platform(voice_platform) {
            // Decrypt the refresh token
            let refresh_token = decrypt_if_available(crypto.as_ref(), refresh_token_encrypted);

            // Refresh the tokens using OAuth client
            let tokens = oauth_manager
                .refresh_token(voice_platform, &refresh_token)
                .await
                .map_err(|e| {
                    // RFC 6749 §5.2: a rejected refresh token is an
                    // `invalid_grant` (400) — not a 500. Transport/parse faults
                    // map to `server_error` (500). Upstream detail is logged only.
                    voice_oauth_error_response(
                        classify_voice_oauth_error(&e, OAUTH_REFRESH_FAILED_MESSAGE),
                        e,
                    )
                })?;

            // #765 (fail-closed encryption) + #2662 (indexed hash in lock-step),
            // centralized in `encrypt_voice_token_pair`.
            let (access_encrypted, refresh_encrypted, access_hash) = encrypt_voice_token_pair(
                crypto.as_ref(),
                &tokens.access_token,
                tokens.refresh_token.as_deref(),
            )?;

            (
                access_encrypted,
                refresh_encrypted,
                tokens.expires_at,
                access_hash,
            )
        } else if voice_simulated_oauth_allowed(cfg!(debug_assertions)) {
            // Platform not configured - use simulated tokens for development only.
            // Gated (security fix #890): release builds must never mint fake voice
            // tokens or overwrite the device's real stored token with one.
            tracing::warn!(
                "Voice OAuth not configured for platform {}, using simulated refresh (debug build)",
                device.platform
            );
            let new_access = format!("voice_access_refreshed_{}", Uuid::new_v4());
            let (access_encrypted, refresh_encrypted, access_hash) =
                encrypt_voice_token_pair(crypto.as_ref(), &new_access, None)?;
            (
                access_encrypted,
                refresh_encrypted,
                Some(Utc::now() + Duration::hours(1)),
                access_hash,
            )
        } else {
            // Fail closed (#890): an unconfigured platform in a release build must
            // not mint a simulated token — returning here also leaves the device's
            // real stored token untouched (no `update_voice_device_tokens` below).
            tracing::error!(
                "Voice OAuth not configured for platform {} in a release build; refusing to mint a simulated refresh token",
                device.platform
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "OAUTH_NOT_CONFIGURED",
                    "Voice OAuth is not configured for this platform",
                )),
            )
                .into());
        };

    // Update the device tokens under the device's own org/user RLS context.
    let mut guard = rls_pool
        .acquire_with_rls(device.organization_id, device.user_id, false)
        .await
        .map_err(|e| {
            tracing::error!("Failed to acquire RLS connection: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_UPDATE_FAILED",
                    "Failed to update tokens",
                )),
            )
        })?;
    state
        .llm_document_repo
        .update_voice_device_tokens(
            &mut **guard.conn(),
            device.id,
            &new_access_encrypted,
            new_refresh_encrypted.as_deref(),
            new_expires_at,
            new_access_hash.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update tokens: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "TOKEN_UPDATE_FAILED",
                    "Failed to update tokens",
                )),
            )
        })?;

    tracing::info!(
        "Successfully refreshed OAuth tokens for voice device {}",
        device.id
    );

    Ok(Json(VoiceTokenRefreshResult {
        success: true,
        expires_at: new_expires_at,
        error: None,
    }))
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    // --- oauth exchange rate limiting (#2769) ---------------------------

    #[test]
    fn exchange_rate_limiter_throttles_after_max_attempts() {
        // Fresh principal => isolated bucket in the process-global limiter.
        let user_id = Uuid::new_v4();
        for i in 0..VOICE_EXCHANGE_MAX_ATTEMPTS {
            assert!(
                voice_exchange_attempt_allowed(user_id),
                "attempt {i} within the window should be allowed"
            );
        }
        // One past the limit within the same window is throttled, mirroring the
        // /oauth/refresh guard.
        assert!(
            !voice_exchange_attempt_allowed(user_id),
            "attempt beyond VOICE_EXCHANGE_MAX_ATTEMPTS should be throttled"
        );
    }

    #[test]
    fn exchange_rate_limiter_isolates_per_user() {
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        for _ in 0..VOICE_EXCHANGE_MAX_ATTEMPTS {
            assert!(voice_exchange_attempt_allowed(user_a));
        }
        // user_a is now throttled, but a distinct principal is unaffected.
        assert!(!voice_exchange_attempt_allowed(user_a));
        assert!(voice_exchange_attempt_allowed(user_b));
    }
}

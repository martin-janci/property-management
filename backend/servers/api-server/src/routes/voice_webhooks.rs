//! Voice assistant webhook routes (Epic 93: Voice Assistant & OAuth Completion).
//!
//! Story 93.3: Voice Platform Webhooks
//! - Alexa Skills Kit webhook handler
//! - Google Actions webhook handler
//! - Request signature verification
//! - User authentication via OAuth token

use crate::routes::rate_limit::{rate_limit_allowed, InProcessRateLimiter};
use crate::services::VoiceCommandProcessor;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Duration, Utc};
use common::errors::ErrorResponse;
use db::models::{
    voice_platform, AlexaCard, AlexaIntent, AlexaOutputSpeech, AlexaRequestBody, AlexaResponseBody,
    AlexaSkillRequest, AlexaSkillResponse, GoogleActionsRequest, GoogleActionsResponse,
    GoogleContent, GooglePrompt, GoogleSceneResponse, GoogleSessionResponse, GoogleSimpleResponse,
    VoiceActionResult, VoiceOAuthExchangeRequest, VoiceOAuthExchangeResponse,
    VoiceTokenRefreshRequest, VoiceTokenRefreshResult, WebhookVerificationResult,
};
use db::RlsPool;
use hmac::{Hmac, KeyInit, Mac};
use integrations::{encrypt_optional_required, encrypt_required, CryptoError, IntegrationCrypto};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::PgConnection;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Map a fail-closed encryption error to an HTTP 500.
///
/// Issue #765: persisted voice OAuth tokens must always be encrypted; when
/// `INTEGRATION_ENCRYPTION_KEY` is unset we refuse to store them in plaintext.
fn voice_encryption_required(e: CryptoError) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!(error = %e, "Refusing to store voice OAuth token without encryption");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new(
            "ENCRYPTION_REQUIRED",
            "Integration token encryption is not configured",
        )),
    )
}

/// Whether an unconfigured voice OAuth platform may fall back to minting a
/// locally-simulated token instead of failing the request.
///
/// Simulated tokens are a development-only aid. Fail-open fix (security #890):
/// in RELEASE builds an unconfigured platform must fail closed — never mint or
/// store a fake token, which would otherwise clobber the device's real stored
/// token with a dead simulated one. The build flag is threaded through as an
/// explicit argument so the release (fail-closed) path stays unit-testable —
/// test binaries always compile with `debug_assertions` on.
fn voice_simulated_oauth_allowed(debug_build: bool) -> bool {
    debug_build
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
// Router
// ============================================================================

/// Voice webhook router for external platform integrations.
pub fn voice_webhook_router() -> Router<AppState> {
    Router::new()
        // Alexa Skills Kit
        .route("/alexa", post(alexa_webhook))
        .route("/alexa/health", post(alexa_health_check))
        // Google Actions
        .route("/google", post(google_actions_webhook))
        // OAuth token exchange endpoints
        .route("/oauth/exchange", post(oauth_token_exchange))
        .route("/oauth/refresh", post(oauth_token_refresh))
        // Verification endpoint
        .route("/verify", post(verify_webhook_signature))
}

// ============================================================================
// Alexa Skills Kit Webhook
// ============================================================================

/// Alexa Skills Kit webhook endpoint.
///
/// Handles all Alexa skill requests including:
/// - LaunchRequest: Skill opened
/// - IntentRequest: User spoke a command
/// - SessionEndedRequest: Session terminated
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/alexa",
    request_body = AlexaSkillRequest,
    responses(
        (status = 200, description = "Alexa skill response", body = AlexaSkillResponse),
        (status = 401, description = "Unauthorized - invalid signature or token"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Voice Webhooks"
)]
async fn alexa_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut rls: RlsConnection,
    body: Bytes,
) -> Result<Json<AlexaSkillResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request signature (Story 93.3)
    if let Err(e) = verify_alexa_signature(&headers, &body).await {
        tracing::warn!("Alexa signature verification failed: {}", e);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
        ));
    }

    // Parse the request body
    let request: AlexaSkillRequest = serde_json::from_slice(&body).map_err(|e| {
        tracing::error!("Failed to parse Alexa request: {}", e);
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_REQUEST",
                "Invalid request format",
            )),
        )
    })?;

    // Extract access token for user authentication
    let access_token = request.session.user.access_token.as_ref();

    // Authenticate user via OAuth token
    let device = if let Some(token) = access_token {
        authenticate_voice_user(rls.conn(), token, voice_platform::ALEXA).await?
    } else {
        // Account linking not complete - return link card
        rls.release().await;
        return Ok(Json(build_alexa_link_account_response()));
    };

    // Process the request based on type
    let locale = match &request.request {
        AlexaRequestBody::LaunchRequest { locale, .. } => locale.clone(),
        AlexaRequestBody::IntentRequest { locale, .. } => locale.clone(),
        AlexaRequestBody::SessionEndedRequest { locale, .. } => locale.clone(),
    };

    let response = match &request.request {
        AlexaRequestBody::LaunchRequest { .. } => {
            // Welcome message
            let processor = VoiceCommandProcessor::new(
                state.llm_document_repo.clone(),
                state.fault_repo.clone(),
                state.unit_repo.clone(),
                state.announcement_repo.clone(),
                state.meter_repo.clone(),
            );
            let (result, _) = processor
                .process_command(rls.conn(), device.id, "help", &locale)
                .await
                .map_err(|e| {
                    tracing::error!("Voice command processing failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("PROCESSING_ERROR", e.to_string())),
                    )
                })?;
            build_alexa_response(&result)
        }
        AlexaRequestBody::IntentRequest { intent, .. } => {
            // Process the intent
            let command_text = extract_alexa_command_text(intent);
            let processor = VoiceCommandProcessor::new(
                state.llm_document_repo.clone(),
                state.fault_repo.clone(),
                state.unit_repo.clone(),
                state.announcement_repo.clone(),
                state.meter_repo.clone(),
            );
            let (result, _) = processor
                .process_command(rls.conn(), device.id, &command_text, &locale)
                .await
                .map_err(|e| {
                    tracing::error!("Voice command processing failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("PROCESSING_ERROR", e.to_string())),
                    )
                })?;
            build_alexa_response(&result)
        }
        AlexaRequestBody::SessionEndedRequest { .. } => {
            // Session ended - no response needed
            AlexaSkillResponse {
                version: "1.0".to_string(),
                response: AlexaResponseBody {
                    output_speech: AlexaOutputSpeech {
                        speech_type: "PlainText".to_string(),
                        text: None,
                        ssml: None,
                    },
                    card: None,
                    should_end_session: true,
                },
            }
        }
    };

    rls.release().await;
    Ok(Json(response))
}

/// Alexa health check endpoint for skill validation.
async fn alexa_health_check() -> StatusCode {
    StatusCode::OK
}

// ============================================================================
// Google Actions Webhook
// ============================================================================

/// Google Actions webhook endpoint.
///
/// Handles Google Assistant requests via the Actions SDK. The request is
/// authenticated by a Google-issued ID token supplied as
/// `Authorization: Bearer <jwt>`: its RS256 signature is verified against
/// Google's JWKS and its `aud` claim must equal `GOOGLE_ACTIONS_PROJECT_ID`
/// (issuer + expiry are enforced too). Once verified, the linked account is
/// resolved from the OAuth access token in `user.params.bearerToken`.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/google",
    request_body = GoogleActionsRequest,
    responses(
        (status = 200, description = "Google Actions response", body = GoogleActionsResponse),
        (status = 401, description = "Unauthorized - missing or invalid Google ID token"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Voice Webhooks"
)]
async fn google_actions_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut rls: RlsConnection,
    Json(request): Json<GoogleActionsRequest>,
) -> Result<Json<GoogleActionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify request: Google-issued ID token (Bearer) with RS256 signature +
    // aud/iss/exp validation. Fails closed on any verification error.
    if let Err(e) = verify_google_request(&headers).await {
        tracing::warn!("Google Actions verification failed: {}", e);
        rls.release().await;
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_REQUEST", &e)),
        ));
    }

    // Extract access token from user params
    let access_token = request
        .user
        .params
        .as_ref()
        .and_then(|p| p.get("bearerToken"))
        .and_then(|v| v.as_str());

    // Authenticate user via OAuth token
    let device = if let Some(token) = access_token {
        authenticate_voice_user(rls.conn(), token, voice_platform::GOOGLE_ASSISTANT).await?
    } else {
        // Account linking not complete
        rls.release().await;
        return Ok(Json(build_google_link_account_response(
            &request.session.id,
        )));
    };

    // Get locale from session
    let locale = request.session.language_code.as_deref().unwrap_or("en-US");

    // Extract command text from intent
    let command_text = request
        .intent
        .query
        .as_deref()
        .unwrap_or(&request.handler.name);

    // Process the command
    let processor = VoiceCommandProcessor::new(
        state.llm_document_repo.clone(),
        state.fault_repo.clone(),
        state.unit_repo.clone(),
        state.announcement_repo.clone(),
        state.meter_repo.clone(),
    );
    let (result, _) = processor
        .process_command(rls.conn(), device.id, command_text, locale)
        .await
        .map_err(|e| {
            tracing::error!("Voice command processing failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("PROCESSING_ERROR", e.to_string())),
            )
        })?;

    rls.release().await;
    Ok(Json(build_google_response(&request.session.id, &result)))
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
async fn oauth_token_exchange(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Json(request): Json<VoiceOAuthExchangeRequest>,
) -> Result<Json<VoiceOAuthExchangeResponse>, (StatusCode, Json<ErrorResponse>)> {
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
        ));
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
        ));
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
                tracing::error!("OAuth token exchange failed: {}", e);
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "OAUTH_EXCHANGE_FAILED",
                        format!("Failed to exchange authorization code: {}", e),
                    )),
                )
            })?;

        // Issue #765: encryption is MANDATORY — fail closed if no key is set.
        let access_encrypted = encrypt_required(crypto.as_ref(), &tokens.access_token)
            .map_err(voice_encryption_required)?;
        let refresh_encrypted =
            encrypt_optional_required(crypto.as_ref(), tokens.refresh_token.as_deref())
                .map_err(voice_encryption_required)?;
        // #2662: derive the indexed lookup hash from the plaintext token.
        let access_hash = voice_access_token_hash(&tokens.access_token);

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
        let access_hash = voice_access_token_hash(&simulated_access);
        (
            encrypt_required(crypto.as_ref(), &simulated_access)
                .map_err(voice_encryption_required)?,
            Some(
                encrypt_required(crypto.as_ref(), &simulated_refresh)
                    .map_err(voice_encryption_required)?,
            ),
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
        ));
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
            &mut **guard.conn(),
            org_id,
            user_id,
            None,
            &request.platform,
            &device_id,
            Some("Voice Assistant"),
            &access_encrypted,
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
async fn oauth_token_refresh(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Json(request): Json<VoiceTokenRefreshRequest>,
) -> Result<Json<VoiceTokenRefreshResult>, (StatusCode, Json<ErrorResponse>)> {
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
        ));
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
        ));
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
                    tracing::error!("Token refresh failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "TOKEN_REFRESH_FAILED",
                            format!("Failed to refresh token: {}", e),
                        )),
                    )
                })?;

            // Issue #765: encryption is MANDATORY — fail closed if no key is set.
            let access_encrypted = encrypt_required(crypto.as_ref(), &tokens.access_token)
                .map_err(voice_encryption_required)?;
            let refresh_encrypted =
                encrypt_optional_required(crypto.as_ref(), tokens.refresh_token.as_deref())
                    .map_err(voice_encryption_required)?;
            // #2662: keep the indexed lookup hash in lock-step with the new token.
            let access_hash = voice_access_token_hash(&tokens.access_token);

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
            let access_hash = voice_access_token_hash(&new_access);
            (
                encrypt_required(crypto.as_ref(), &new_access)
                    .map_err(voice_encryption_required)?,
                None,
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
            ));
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
// Signature Verification (Story 93.3)
// ============================================================================

/// Verify webhook request signature.
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyWebhookRequest {
    pub platform: String,
    pub signature: String,
    pub body: String,
    pub timestamp: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/verify",
    request_body = VerifyWebhookRequest,
    responses(
        (status = 200, description = "Verification result", body = WebhookVerificationResult),
    ),
    tag = "Voice Webhooks"
)]
async fn verify_webhook_signature(
    Json(request): Json<VerifyWebhookRequest>,
) -> Json<WebhookVerificationResult> {
    let result = match request.platform.as_str() {
        "alexa" => {
            // SECURITY: Alexa request signing is certificate-chain based — the
            // caller must present a `SignatureCertChainUrl` header, a base64
            // `Signature` header, and the *raw* request body, which are then
            // verified as a PKCS#1 v1.5 RSA-SHA1 signature under an Amazon-issued
            // leaf certificate (see `verify_alexa_signature` /
            // `verify_alexa_cert_and_signature`, wired into the mounted
            // `POST /api/v1/webhooks/voice/alexa` receiver).
            //
            // This JSON introspection endpoint receives none of that material
            // (no cert-chain URL, and `body` is a lossy string rather than the
            // raw signed bytes), so it CANNOT cryptographically verify an Alexa
            // signature and MUST NOT claim it did. The previous implementation
            // returned `valid: !signature.is_empty()`, accepting any non-empty
            // string as a valid Alexa signature — a forgery vector that let an
            // attacker have an arbitrary payload reported as "valid". Fail
            // CLOSED: never report `valid: true` from this endpoint for Alexa.
            WebhookVerificationResult {
                valid: false,
                platform: "alexa".to_string(),
                error: Some(if request.signature.is_empty() {
                    "Missing signature".to_string()
                } else {
                    "Alexa signature verification is certificate-chain based and \
                     is performed by POST /api/v1/webhooks/voice/alexa; this \
                     endpoint cannot verify Alexa signatures"
                        .to_string()
                }),
            }
        }
        "google" => {
            // Google uses HMAC-SHA256
            match verify_hmac_signature(&request.signature, &request.body) {
                Ok(valid) => WebhookVerificationResult {
                    valid,
                    platform: "google".to_string(),
                    error: if valid {
                        None
                    } else {
                        Some("Invalid signature".to_string())
                    },
                },
                Err(e) => WebhookVerificationResult {
                    valid: false,
                    platform: "google".to_string(),
                    error: Some(e),
                },
            }
        }
        _ => WebhookVerificationResult {
            valid: false,
            platform: request.platform.clone(),
            error: Some("Unknown platform".to_string()),
        },
    };

    Json(result)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Verify an Alexa Skills Kit request signature (issue #2668 — SECURITY).
///
/// The previous implementation validated only the certificate URL format and
/// the request timestamp and then returned `Ok(())` — the `Signature` header
/// was bound to `_signature` and never checked, so **any** request carrying a
/// well-formed `SignatureCertChainUrl` and a fresh timestamp was accepted
/// regardless of whether it was actually signed by Amazon. That let an attacker
/// forge arbitrary voice commands (report faults, read announcements, etc.) for
/// any linked device.
///
/// We now implement Amazon's documented verification
/// (<https://developer.amazon.com/docs/custom-skills/host-a-custom-skill-as-a-web-service.html#verifying-that-the-request-was-sent-by-alexa>):
///
/// 1. Validate the `SignatureCertChainUrl` (scheme/host/port/path — existing
///    `validate_alexa_cert_url`). This constrains the fetch to Amazon's S3
///    bucket over TLS, so the certificate origin is authenticated by the
///    transport.
/// 2. Validate the request timestamp is within 150 seconds (anti-replay).
/// 3. Fetch the PEM certificate chain from the (validated) URL, with a small
///    in-process TTL cache so a burst of requests does not re-fetch.
/// 4. Validate the signing (leaf) certificate: it is currently within its
///    validity window and its Subject Alternative Names include
///    `echo-api.amazon.com`.
/// 5. Verify the base64-decoded `Signature` header is a valid PKCS#1 v1.5
///    RSA-SHA1 signature of the **raw request body** under the leaf
///    certificate's public key.
///
/// Any failure returns `Err(_)`; the caller maps that to `401 Unauthorized`.
async fn verify_alexa_signature(headers: &HeaderMap, body: &[u8]) -> Result<(), String> {
    // Get required headers. Alexa header names are case-insensitive; `HeaderMap`
    // lookups already normalise case.
    let cert_url = headers
        .get("SignatureCertChainUrl")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing SignatureCertChainUrl header")?;

    let signature_b64 = headers
        .get("Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or("Missing Signature header")?;

    // Step 1: Validate certificate URL format (host/scheme/port/path).
    validate_alexa_cert_url(cert_url)?;

    // Step 2: Validate timestamp from request body (replay protection).
    validate_alexa_timestamp(body)?;

    // Decode the presented signature before doing any network work.
    let signature = BASE64
        .decode(signature_b64.trim().as_bytes())
        .map_err(|e| format!("Signature is not valid base64: {e}"))?;

    // Step 3: Fetch the certificate chain (validated URL only), with caching.
    let cert_pem = fetch_alexa_cert_chain(cert_url).await?;

    // Steps 4 & 5: leaf validity + SAN + RSA-SHA1 body signature.
    verify_alexa_cert_and_signature(&cert_pem, &signature, body, Utc::now())?;

    tracing::info!(cert_url = %cert_url, "Alexa request signature verified");
    Ok(())
}

/// TTL for cached Alexa signing certificates. Amazon rotates the `echo.api`
/// certificate infrequently; caching by URL for an hour avoids re-fetching on
/// every request without pinning a stale cert for long.
const ALEXA_CERT_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

/// Process-local cache of fetched certificate-chain PEM bytes keyed by URL.
#[allow(clippy::type_complexity)]
static ALEXA_CERT_CACHE: std::sync::OnceLock<
    std::sync::Mutex<
        std::collections::HashMap<String, (std::time::Instant, std::sync::Arc<Vec<u8>>)>,
    >,
> = std::sync::OnceLock::new();

/// Time budget for establishing the TCP+TLS connection to S3 when fetching the
/// Alexa signing certificate.
const ALEXA_CERT_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Overall time budget for a single certificate fetch (connect + response
/// body). Bounds how long a slow S3 can stall the webhook handler.
const ALEXA_CERT_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Shared HTTP client for Alexa certificate fetches.
///
/// Built once (process-wide) with explicit connect and request timeouts and
/// reused for every fetch. Reusing a single client keeps the connection pool
/// and TLS session cache warm and, crucially, prevents a burst of cold-cache
/// requests from spawning an unbounded number of independent TLS clients.
static ALEXA_CERT_HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

/// Return the shared, timeout-bounded HTTP client used to fetch Alexa signing
/// certificates, initialising it on first use.
fn alexa_cert_http_client() -> &'static reqwest::Client {
    ALEXA_CERT_HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(ALEXA_CERT_CONNECT_TIMEOUT)
            .timeout(ALEXA_CERT_REQUEST_TIMEOUT)
            .build()
            // A builder failure means the TLS backend could not initialise; a
            // default client would fail the same way, so fall back to it rather
            // than panicking inside a request handler.
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Fetch the PEM certificate chain from a (previously validated) Alexa cert
/// URL, memoising the result for [`ALEXA_CERT_CACHE_TTL`].
///
/// The URL is only ever reached after [`validate_alexa_cert_url`], so it always
/// points at `https://s3.amazonaws.com/echo.api/…`; TLS authenticates the S3
/// origin. Returns the raw PEM bytes (one or more concatenated certificates).
///
/// The fetch goes through a shared client ([`alexa_cert_http_client`]) with
/// explicit connect/request timeouts so a slow or unresponsive S3 cannot stall
/// the handler indefinitely.
async fn fetch_alexa_cert_chain(url: &str) -> Result<std::sync::Arc<Vec<u8>>, String> {
    let cache =
        ALEXA_CERT_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));

    // Fast path: a fresh cache entry.
    if let Ok(guard) = cache.lock() {
        if let Some((fetched_at, bytes)) = guard.get(url) {
            if fetched_at.elapsed() < ALEXA_CERT_CACHE_TTL {
                return Ok(bytes.clone());
            }
        }
    }

    let response = alexa_cert_http_client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Alexa certificate: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Alexa certificate fetch returned HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read Alexa certificate body: {e}"))?
        .to_vec();
    let bytes = std::sync::Arc::new(bytes);

    if let Ok(mut guard) = cache.lock() {
        guard.insert(url.to_string(), (std::time::Instant::now(), bytes.clone()));
    }

    Ok(bytes)
}

/// Validate the Alexa signing certificate and verify the request-body signature.
///
/// Pure (no I/O): takes the fetched PEM chain, the decoded signature bytes, the
/// raw request body, and the current time, so the security-critical logic is
/// unit-testable without a network or a real Amazon certificate. Splitting the
/// fetch out of this function is what lets the regression tests exercise the
/// forged-signature path against a locally generated certificate.
///
/// Checks, in order:
/// 1. The leaf (first) certificate parses.
/// 2. `now` is within the leaf's `notBefore..=notAfter` window.
/// 3. The leaf's Subject Alternative Names include `echo-api.amazon.com`.
/// 4. `signature` is a valid PKCS#1 v1.5 RSA-SHA1 signature of `body` under the
///    leaf's public key.
fn verify_alexa_cert_and_signature(
    cert_pem: &[u8],
    signature: &[u8],
    body: &[u8],
    now: DateTime<Utc>,
) -> Result<(), String> {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::{Pkcs1v15Sign, RsaPublicKey};
    use sha1::{Digest, Sha1};
    use x509_cert::der::{Decode, DecodePem, Encode};
    use x509_cert::ext::pkix::{name::GeneralName, SubjectAltName};
    use x509_cert::Certificate;

    // (1) Parse the leaf certificate (first PEM block in the chain).
    let cert = Certificate::from_pem(cert_pem)
        .map_err(|e| format!("Failed to parse Alexa certificate: {e}"))?;
    let tbs = &cert.tbs_certificate;

    // (2) Validity window.
    let now_s = now.timestamp();
    if now_s < 0 {
        return Err("Current time is before the Unix epoch".to_string());
    }
    let now_s = now_s as u64;
    let not_before = tbs.validity.not_before.to_unix_duration().as_secs();
    let not_after = tbs.validity.not_after.to_unix_duration().as_secs();
    if now_s < not_before {
        return Err("Alexa certificate is not yet valid".to_string());
    }
    if now_s > not_after {
        return Err("Alexa certificate has expired".to_string());
    }

    // (3) Subject Alternative Name must include echo-api.amazon.com.
    // OID 2.5.29.17 = id-ce-subjectAltName.
    let mut san_ok = false;
    if let Some(extensions) = tbs.extensions.as_ref() {
        for ext in extensions.iter() {
            if ext.extn_id.to_string() != "2.5.29.17" {
                continue;
            }
            let san = SubjectAltName::from_der(ext.extn_value.as_bytes())
                .map_err(|e| format!("Failed to parse certificate SAN: {e}"))?;
            for general_name in san.0.iter() {
                if let GeneralName::DnsName(dns) = general_name {
                    if dns.as_str().eq_ignore_ascii_case("echo-api.amazon.com") {
                        san_ok = true;
                        break;
                    }
                }
            }
            if san_ok {
                break;
            }
        }
    }
    if !san_ok {
        return Err("Alexa certificate SAN does not include echo-api.amazon.com".to_string());
    }

    // (4) Verify the RSA-SHA1 signature over the raw request body.
    let spki_der = tbs
        .subject_public_key_info
        .to_der()
        .map_err(|e| format!("Failed to encode certificate public key: {e}"))?;
    let public_key = RsaPublicKey::from_public_key_der(&spki_der)
        .map_err(|e| format!("Certificate public key is not RSA: {e}"))?;

    let digest = Sha1::digest(body);
    public_key
        .verify(Pkcs1v15Sign::new::<Sha1>(), &digest, signature)
        .map_err(|_| "Alexa request signature does not match request body".to_string())?;

    Ok(())
}

/// Validate Alexa certificate URL format.
/// The URL must:
/// - Use HTTPS protocol
/// - Use host s3.amazonaws.com with path /echo.api/
/// - Use port 443 (default)
fn validate_alexa_cert_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid certificate URL: {}", e))?;

    // Check protocol
    if parsed.scheme() != "https" {
        return Err("Certificate URL must use HTTPS".to_string());
    }

    // Check host
    let host = parsed.host_str().ok_or("Certificate URL missing host")?;

    if host != "s3.amazonaws.com" {
        return Err(format!(
            "Certificate URL host must be s3.amazonaws.com, got: {}",
            host
        ));
    }

    // Check port (must be 443 or default)
    if let Some(port) = parsed.port() {
        if port != 443 {
            return Err(format!("Certificate URL must use port 443, got: {}", port));
        }
    }

    // Check path starts with /echo.api/
    let path = parsed.path();
    if !path.starts_with("/echo.api/") {
        return Err(format!(
            "Certificate URL path must start with /echo.api/, got: {}",
            path
        ));
    }

    Ok(())
}

/// Validate Alexa request timestamp.
/// The timestamp must be within 150 seconds of current time.
fn validate_alexa_timestamp(body: &[u8]) -> Result<(), String> {
    // Parse the body to extract timestamp
    #[derive(Deserialize)]
    struct AlexaRequest {
        request: AlexaRequestTimestamp,
    }

    #[derive(Deserialize)]
    struct AlexaRequestTimestamp {
        timestamp: String,
    }

    let request: AlexaRequest = serde_json::from_slice(body)
        .map_err(|e| format!("Failed to parse Alexa request: {}", e))?;

    // Parse timestamp (ISO 8601 format)
    let timestamp = chrono::DateTime::parse_from_rfc3339(&request.request.timestamp)
        .map_err(|e| format!("Invalid timestamp format: {}", e))?;

    let now = Utc::now();
    let diff = now.signed_duration_since(timestamp);

    // Must be within 150 seconds (past or future)
    if diff.num_seconds().abs() > 150 {
        return Err(format!(
            "Request timestamp too old or too new: {} seconds difference",
            diff.num_seconds()
        ));
    }

    Ok(())
}

/// Google's OAuth2 JWKS endpoint — RS256 signing certificates for Google-issued
/// ID tokens (used by Google Assistant / Actions account linking).
const GOOGLE_CERTS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

/// Accepted `iss` values for a Google-issued ID token.
const GOOGLE_ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];

/// How long a fetched JWKS is trusted before a refresh. Google rotates signing
/// keys on the order of days; a 1h TTL keeps the hot path off the network while
/// still picking up rotations promptly (a rotated-in `kid` also forces an
/// immediate refresh regardless of TTL — see `google_decoding_key`).
const GOOGLE_JWKS_TTL: std::time::Duration = std::time::Duration::from_secs(3600);

/// One JWK entry from Google's `/oauth2/v3/certs` response (RSA public key).
#[derive(Clone, Deserialize)]
struct GoogleJwk {
    kid: String,
    n: String,
    e: String,
    #[allow(dead_code)]
    alg: Option<String>,
}

#[derive(Clone, Deserialize)]
struct GoogleJwks {
    keys: Vec<GoogleJwk>,
}

struct JwksCache {
    keys: Vec<GoogleJwk>,
    fetched_at: std::time::Instant,
}

/// Process-wide JWKS cache (1h TTL) and shared HTTP client (5s timeout).
static GOOGLE_JWKS_CACHE: OnceLock<RwLock<Option<JwksCache>>> = OnceLock::new();
static GOOGLE_JWKS_HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn google_jwks_cache() -> &'static RwLock<Option<JwksCache>> {
    GOOGLE_JWKS_CACHE.get_or_init(|| RwLock::new(None))
}

fn google_jwks_http() -> &'static reqwest::Client {
    GOOGLE_JWKS_HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            // Mirror `alexa_cert_http_client`: a builder failure means the TLS
            // backend could not initialise, and a default client would fail the
            // same way — fall back to it rather than panicking inside the
            // request-serving `verify_google_request` path.
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Fetch and parse Google's current JWKS. `error_for_status()` is deliberate:
/// a 4xx/5xx JSON error body would otherwise parse into an empty `keys[]` and
/// surface later as a confusing "unknown kid" instead of the real HTTP failure.
async fn fetch_google_jwks() -> Result<Vec<GoogleJwk>, String> {
    let resp = google_jwks_http()
        .get(GOOGLE_CERTS_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Google signing certs: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Google signing certs endpoint returned an error: {e}"))?;
    let jwks: GoogleJwks = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Google signing certs: {e}"))?;
    Ok(jwks.keys)
}

/// Resolve the RSA public [`DecodingKey`] for a `kid`, using the cached JWKS
/// when fresh and it contains the key, otherwise refreshing from Google.
/// Mirrors the JWKS resolver in `deploy-server`'s OIDC validator.
async fn google_decoding_key(kid: &str) -> Result<DecodingKey, String> {
    // Fast path: cached, fresh, and contains the requested kid.
    {
        let guard = google_jwks_cache().read().await;
        if let Some(cache) = guard.as_ref() {
            if cache.fetched_at.elapsed() < GOOGLE_JWKS_TTL {
                if let Some(k) = cache.keys.iter().find(|k| k.kid == kid) {
                    return DecodingKey::from_rsa_components(&k.n, &k.e)
                        .map_err(|e| format!("Invalid Google JWK for kid {kid}: {e}"));
                }
            }
        }
    }

    // Refresh: stale cache OR the kid was rotated in and isn't cached yet.
    let fresh = fetch_google_jwks().await?;
    let key = fresh
        .iter()
        .find(|k| k.kid == kid)
        .ok_or_else(|| format!("Unknown Google signing key id: {kid}"))
        .and_then(|k| {
            DecodingKey::from_rsa_components(&k.n, &k.e)
                .map_err(|e| format!("Invalid Google JWK for kid {kid}: {e}"))
        })?;
    *google_jwks_cache().write().await = Some(JwksCache {
        keys: fresh,
        fetched_at: std::time::Instant::now(),
    });
    Ok(key)
}

/// Verify a Google-issued ID token's RS256 signature and registered claims
/// against `key`. Pure and synchronous so it is unit-testable without a live
/// JWKS fetch. FAILS CLOSED: any bad signature, wrong audience (`aud` must
/// equal `expected_project` exactly), untrusted issuer, or expired/not-yet-valid
/// token yields `Err`.
fn verify_google_id_token(
    token: &str,
    expected_project: &str,
    key: &DecodingKey,
) -> Result<(), String> {
    let mut validation = Validation::new(Algorithm::RS256);
    // Exact audience match against the configured Actions project id.
    validation.set_audience(&[expected_project]);
    validation.set_issuer(&GOOGLE_ISSUERS);
    validation.validate_exp = true;
    validation.validate_nbf = true;

    // jsonwebtoken base64url-decodes the segments itself (no STANDARD-alphabet
    // mis-decode) and enforces signature + aud + iss + exp/nbf.
    decode::<serde_json::Value>(token, key, &validation)
        .map(|_| ())
        .map_err(|e| format!("Google ID token verification failed: {e}"))
}

/// Verify a Google Actions request (Epic 93 / Story 93.3).
///
/// The request MUST carry a Google-issued ID token in the `Authorization:
/// Bearer <jwt>` header. The token's RS256 signature is verified against
/// Google's published JWKS (`/oauth2/v3/certs`, cached 1h), and its `aud`
/// claim must equal `GOOGLE_ACTIONS_PROJECT_ID`, its `iss` must be a Google
/// issuer, and it must be unexpired.
///
/// FAILS CLOSED on every path: an unset `GOOGLE_ACTIONS_PROJECT_ID`, a missing
/// or malformed `Authorization` header, or any signature/claim failure all
/// return `Err` (mapped by the caller to `401`).
async fn verify_google_request(headers: &HeaderMap) -> Result<(), String> {
    // FAIL CLOSED: the audience we verify against must be configured.
    let expected_project = std::env::var("GOOGLE_ACTIONS_PROJECT_ID")
        .map_err(|_| "GOOGLE_ACTIONS_PROJECT_ID is not configured".to_string())?;

    // FAIL CLOSED: a Google-signed Bearer ID token is required.
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Missing Authorization header".to_string())?;
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| "Invalid Authorization header format".to_string())?;

    // Fail fast — and off the network — on a structurally invalid token.
    let header =
        decode_header(token).map_err(|e| format!("Invalid Google ID token header: {e}"))?;
    if header.alg != Algorithm::RS256 {
        return Err(format!(
            "Unexpected Google ID token algorithm: {:?}",
            header.alg
        ));
    }
    let kid = header
        .kid
        .ok_or_else(|| "Google ID token missing key id (kid)".to_string())?;

    let key = google_decoding_key(&kid).await?;
    verify_google_id_token(token, &expected_project, &key)?;

    tracing::info!(project_id = %expected_project, "Google Actions ID token verified");
    Ok(())
}

/// Constant-time byte equality for secret-derived string comparisons.
///
/// Issue #2658 (#3): comparing signatures/tokens with `==` leaks a timing
/// side-channel proportional to the shared prefix length. Mirrors the
/// constant-time comparison the portal/Airbnb webhook receivers use.
fn ct_eq_str(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Verify HMAC-SHA256 signature.
///
/// Issue #2658 (#2): fails **closed** when `VOICE_WEBHOOK_SECRET` is unset.
/// The previous `unwrap_or("default_secret")` verified against a literal
/// present in source, letting anyone forge signatures; the sibling receivers
/// (`PORTAL_WEBHOOK_SECRET`, `AIRBNB_WEBHOOK_SECRET`, `STRIPE_WEBHOOK_SECRET`)
/// all fail closed, and so do we now.
fn verify_hmac_signature(signature: &str, body: &str) -> Result<bool, String> {
    let secret = std::env::var("VOICE_WEBHOOK_SECRET")
        .map_err(|_| "VOICE_WEBHOOK_SECRET is not configured".to_string())?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("Invalid HMAC key: {}", e))?;

    mac.update(body.as_bytes());

    let expected = BASE64.encode(mac.finalize().into_bytes());

    // Constant-time compare (issue #2658 #3) instead of `signature == expected`.
    Ok(ct_eq_str(signature, &expected))
}

/// Keyed HMAC-SHA256 of a voice OAuth access token, used as a deterministic,
/// indexed lookup key (`voice_assistant_devices.access_token_hash`) so
/// `authenticate_voice_user` selects the candidate device in SQL instead of
/// decrypt-and-scanning every active device for the platform (issue #2662).
///
/// Keyed with `INTEGRATION_ENCRYPTION_KEY` — the same secret that encrypts the
/// token at rest — so the persisted value is a keyed MAC, not an offline-
/// guessable plain digest. Returns `None` when the key is unavailable: write
/// paths then persist a NULL hash and the read path falls back to the linear
/// scan, so a device is never locked out.
fn voice_access_token_hash(access_token: &str) -> Option<Vec<u8>> {
    let key = std::env::var(integrations::ENCRYPTION_KEY_ENV).ok()?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).ok()?;
    mac.update(access_token.as_bytes());
    Some(mac.finalize().into_bytes().to_vec())
}

/// Whether `presented` is the access token that owns `device`: it must match
/// the device's decrypted stored token (constant-time) and not be expired.
///
/// Pure (no I/O) so the security-critical matching logic can be unit-tested
/// without a database. See `authenticate_voice_user`.
fn voice_device_token_matches(
    crypto: &IntegrationCrypto,
    device: &db::models::VoiceAssistantDevice,
    presented: &str,
    now: DateTime<Utc>,
) -> bool {
    let Some(encrypted) = device.access_token_encrypted.as_deref() else {
        return false;
    };
    // A token we cannot decrypt (e.g. after key rotation) is simply not a match.
    let Ok(stored) = crypto.decrypt(encrypted) else {
        return false;
    };
    if !ct_eq_str(&stored, presented) {
        return false;
    }
    // Reject an expired access token.
    match device.token_expires_at {
        Some(expires_at) => expires_at > now,
        None => true,
    }
}

/// Authenticate a voice request by matching the platform-presented OAuth access
/// token against the encrypted token stored for a linked device.
///
/// Issue #2658 (#1 — broken authentication): the previous implementation
/// ignored the access token entirely and returned *any* active device for the
/// platform, authenticating every caller as whichever user most recently used
/// the platform (horizontal privilege escalation). We now fail closed unless
/// the presented token exactly matches a device's stored token:
///   * an absent/blank token is rejected outright;
///   * the integration encryption key is required — stored tokens are
///     ciphertext, so without the key we cannot validate and refuse rather than
///     authenticate anyone;
///   * each active device's stored token is decrypted and compared to the
///     presented token in constant time, and only the owning (unexpired) device
///     is returned.
async fn authenticate_voice_user(
    conn: &mut PgConnection,
    access_token: &str,
    platform: &str,
) -> Result<db::models::VoiceAssistantDevice, (StatusCode, Json<ErrorResponse>)> {
    let unauthorized = || {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_ACCESS_TOKEN",
                "Voice device not linked or access token invalid. Please complete account linking.",
            )),
        )
    };

    // Reject an absent/blank token outright.
    if access_token.trim().is_empty() {
        return Err(unauthorized());
    }

    // Stored device access tokens are encrypted at rest; without the integration
    // encryption key we cannot validate the presented token, so fail closed
    // rather than authenticate anyone.
    let crypto = IntegrationCrypto::try_from_env().ok_or_else(|| {
        tracing::error!(
            "Voice authentication unavailable: INTEGRATION_ENCRYPTION_KEY is not configured"
        );
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "ENCRYPTION_REQUIRED",
                "Voice token validation is not configured",
            )),
        )
    })?;

    let now = Utc::now();
    let db_error = || {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
        )
    };

    // Fast path (issue #2662): select the single candidate device by a keyed
    // HMAC-SHA256 of the presented token, resolved by the partial index on
    // (platform, access_token_hash), instead of AES-GCM-decrypting every active
    // device for the platform. The decrypt + constant-time verify below remains
    // the authority (defence in depth) — the hash only narrows the search, so a
    // hash collision or a stale/rotated row still cannot authenticate.
    let token_hash = voice_access_token_hash(access_token);
    if let Some(hash) = token_hash.as_deref() {
        let candidate = sqlx::query_as::<_, db::models::VoiceAssistantDevice>(
            r#"
            SELECT * FROM voice_assistant_devices
            WHERE platform = $1
              AND is_active = TRUE
              AND access_token_hash = $2
            LIMIT 1
            "#,
        )
        .bind(platform)
        .bind(hash)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            db_error()
        })?;

        if let Some(device) = candidate {
            if voice_device_token_matches(&crypto, &device, access_token, now) {
                return Ok(device);
            }
        }
    }

    // Fallback: linear decrypt-and-scan. When a hash was computed we only need
    // the rows that predate the hash column (NULL hash) — the indexed path above
    // already covered every backfilled row; when no key/hash is available we
    // scan all active token-bearing devices (legacy behaviour). Either way the
    // AES-GCM ciphertext (random nonce per encryption) cannot be matched in SQL,
    // so equality is checked after decryption.
    let scanned_hashed = token_hash.is_some();
    let devices = sqlx::query_as::<_, db::models::VoiceAssistantDevice>(
        r#"
        SELECT * FROM voice_assistant_devices
        WHERE platform = $1
          AND is_active = TRUE
          AND access_token_encrypted IS NOT NULL
          AND (NOT $2 OR access_token_hash IS NULL)
        ORDER BY last_used_at DESC NULLS LAST
        "#,
    )
    .bind(platform)
    .bind(scanned_hashed)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!("Database error: {}", e);
        db_error()
    })?;

    devices
        .into_iter()
        .find(|device| voice_device_token_matches(&crypto, device, access_token, now))
        .ok_or_else(unauthorized)
}

/// Extract command text from Alexa intent.
fn extract_alexa_command_text(intent: &AlexaIntent) -> String {
    // Map Alexa built-in intents to our commands
    match intent.name.as_str() {
        "AMAZON.HelpIntent" => "help".to_string(),
        "AMAZON.StopIntent" | "AMAZON.CancelIntent" => "goodbye".to_string(),
        "CheckBalanceIntent" => "check my balance".to_string(),
        "ReportFaultIntent" => {
            // Extract fault description from slots
            let description = intent
                .slots
                .as_ref()
                .and_then(|s| s.get("FaultDescription"))
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("a fault");
            format!("report a fault with {}", description)
        }
        "CheckAnnouncementsIntent" => "check announcements".to_string(),
        "CheckMeterIntent" => "check meter readings".to_string(),
        "ContactManagerIntent" => "contact manager".to_string(),
        _ => intent.name.clone(),
    }
}

/// Build Alexa skill response from action result.
fn build_alexa_response(result: &VoiceActionResult) -> AlexaSkillResponse {
    let output_speech = if let Some(ssml) = &result.ssml {
        AlexaOutputSpeech {
            speech_type: "SSML".to_string(),
            text: None,
            ssml: Some(ssml.clone()),
        }
    } else {
        AlexaOutputSpeech {
            speech_type: "PlainText".to_string(),
            text: Some(result.response_text.clone()),
            ssml: None,
        }
    };

    let card = result.card.as_ref().map(|c| AlexaCard {
        card_type: "Simple".to_string(),
        title: c.title.clone(),
        content: Some(c.content.clone()),
        text: None,
    });

    AlexaSkillResponse {
        version: "1.0".to_string(),
        response: AlexaResponseBody {
            output_speech,
            card,
            should_end_session: result.should_end_session,
        },
    }
}

/// Build Alexa response for account linking.
fn build_alexa_link_account_response() -> AlexaSkillResponse {
    AlexaSkillResponse {
        version: "1.0".to_string(),
        response: AlexaResponseBody {
            output_speech: AlexaOutputSpeech {
                speech_type: "PlainText".to_string(),
                text: Some(
                    "Please link your property management account in the Alexa app to use this skill."
                        .to_string(),
                ),
                ssml: None,
            },
            card: Some(AlexaCard {
                card_type: "LinkAccount".to_string(),
                title: "Link Account".to_string(),
                content: None,
                text: None,
            }),
            should_end_session: true,
        },
    }
}

/// Build Google Actions response from action result.
fn build_google_response(session_id: &str, result: &VoiceActionResult) -> GoogleActionsResponse {
    let content = result.card.as_ref().map(|c| GoogleContent {
        card: Some(db::models::GoogleCard {
            title: c.title.clone(),
            subtitle: None,
            text: c.content.clone(),
            image: None,
        }),
    });

    GoogleActionsResponse {
        session: GoogleSessionResponse {
            id: session_id.to_string(),
            params: None,
        },
        prompt: GooglePrompt {
            override_mode: false,
            first_simple: GoogleSimpleResponse {
                speech: result.response_text.clone(),
                text: Some(result.response_text.clone()),
            },
            content,
        },
        scene: if result.should_end_session {
            Some(GoogleSceneResponse {
                name: "actions.scene.END_CONVERSATION".to_string(),
            })
        } else {
            None
        },
    }
}

/// Build Google response for account linking.
fn build_google_link_account_response(session_id: &str) -> GoogleActionsResponse {
    GoogleActionsResponse {
        session: GoogleSessionResponse {
            id: session_id.to_string(),
            params: None,
        },
        prompt: GooglePrompt {
            override_mode: false,
            first_simple: GoogleSimpleResponse {
                speech: "Please link your property management account to use this action."
                    .to_string(),
                text: Some(
                    "Please link your property management account to use this action.".to_string(),
                ),
            },
            content: None,
        },
        scene: Some(GoogleSceneResponse {
            name: "AccountLinking".to_string(),
        }),
    }
}

// ============================================================================
// Tests (Epic 93: Voice Assistant webhooks)
// ============================================================================
//
// Scope: unit coverage for the infrastructure-free logic of the mounted voice
// webhook endpoints — the whole signature/verification surface, the mounted
// `/verify` handler, the OAuth error-mapping helper, and every response
// builder. The four handlers that hold a live DB connection + `AppState` +
// `AuthUser` (`alexa_webhook`, `google_actions_webhook`, `oauth_token_exchange`,
// `oauth_token_refresh`) reach Postgres and the OAuth managers, so their
// happy-paths belong in an integration harness with a seeded database rather
// than in these pure unit tests. What is exercised here is the security-
// relevant branch logic those handlers delegate to (`verify_alexa_signature`,
// `verify_google_request`, `verify_hmac_signature`, `authenticate_voice_user`
// input handling) plus the `voice_encryption_required` fail-closed mapping used
// on the OAuth token-exchange/refresh persistence path.
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    // --- helpers ---------------------------------------------------------

    fn action_result(text: &str, ssml: Option<&str>, end: bool) -> VoiceActionResult {
        VoiceActionResult {
            success: true,
            action_type: "test".to_string(),
            response_text: text.to_string(),
            ssml: ssml.map(|s| s.to_string()),
            card: None,
            should_end_session: end,
            data: None,
        }
    }

    fn action_result_with_card(text: &str) -> VoiceActionResult {
        VoiceActionResult {
            success: true,
            action_type: "test".to_string(),
            response_text: text.to_string(),
            ssml: None,
            card: Some(db::models::VoiceCard {
                title: "Balance".to_string(),
                content: "You owe 12 EUR".to_string(),
                image_url: None,
            }),
            should_end_session: false,
            data: None,
        }
    }

    fn alexa_body_with_timestamp(ts: &str) -> Vec<u8> {
        format!(r#"{{"request":{{"timestamp":"{ts}"}}}}"#).into_bytes()
    }

    // Compute a valid HMAC-SHA256 base64 signature for `body` under `secret`,
    // mirroring `verify_hmac_signature`'s algorithm exactly.
    fn hmac_sig(secret: &str, body: &str) -> String {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        BASE64.encode(mac.finalize().into_bytes())
    }

    // --- voice OAuth simulated-token fallback gate (#890) ---------------

    #[test]
    fn simulated_voice_oauth_fallback_forbidden_in_release_builds() {
        // Fail-open fix (#890): when the platform OAuth client is unconfigured,
        // a RELEASE build must fail closed — never mint or store a simulated
        // refresh token (which would also clobber the device's real stored
        // token). The refresh handler gates its simulated branch on this.
        assert!(
            !voice_simulated_oauth_allowed(false),
            "release builds must fail closed, never mint a simulated voice token"
        );
    }

    #[test]
    fn simulated_voice_oauth_fallback_allowed_in_debug_builds() {
        // Development convenience is preserved: debug/test builds may still use
        // the simulated fallback when the platform is unconfigured.
        assert!(
            voice_simulated_oauth_allowed(true),
            "debug builds may use the simulated fallback"
        );
    }

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

    // --- validate_alexa_cert_url ----------------------------------------

    #[test]
    fn cert_url_accepts_canonical_amazon_url() {
        assert!(
            validate_alexa_cert_url("https://s3.amazonaws.com/echo.api/echo-api-cert-12.pem")
                .is_ok()
        );
    }

    #[test]
    fn cert_url_accepts_explicit_port_443() {
        assert!(validate_alexa_cert_url("https://s3.amazonaws.com:443/echo.api/cert.pem").is_ok());
    }

    #[test]
    fn cert_url_rejects_non_https_scheme() {
        let err = validate_alexa_cert_url("http://s3.amazonaws.com/echo.api/cert.pem").unwrap_err();
        assert!(err.contains("HTTPS"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_wrong_host() {
        let err =
            validate_alexa_cert_url("https://evil.example.com/echo.api/cert.pem").unwrap_err();
        assert!(err.contains("s3.amazonaws.com"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_non_443_port() {
        let err =
            validate_alexa_cert_url("https://s3.amazonaws.com:8443/echo.api/cert.pem").unwrap_err();
        assert!(err.contains("443"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_wrong_path_prefix() {
        let err = validate_alexa_cert_url("https://s3.amazonaws.com/evil/cert.pem").unwrap_err();
        assert!(err.contains("/echo.api/"), "unexpected error: {err}");
    }

    #[test]
    fn cert_url_rejects_unparseable_url() {
        assert!(validate_alexa_cert_url("not a url").is_err());
    }

    // --- validate_alexa_timestamp ---------------------------------------

    #[test]
    fn timestamp_accepts_now() {
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        assert!(validate_alexa_timestamp(&body).is_ok());
    }

    #[test]
    fn timestamp_rejects_too_old() {
        let old = (Utc::now() - Duration::seconds(600)).to_rfc3339();
        let body = alexa_body_with_timestamp(&old);
        let err = validate_alexa_timestamp(&body).unwrap_err();
        assert!(err.contains("too old or too new"), "unexpected: {err}");
    }

    #[test]
    fn timestamp_rejects_far_future() {
        let future = (Utc::now() + Duration::seconds(600)).to_rfc3339();
        let body = alexa_body_with_timestamp(&future);
        assert!(validate_alexa_timestamp(&body).is_err());
    }

    #[test]
    fn timestamp_rejects_bad_format() {
        let body = alexa_body_with_timestamp("not-a-timestamp");
        let err = validate_alexa_timestamp(&body).unwrap_err();
        assert!(
            err.contains("Invalid timestamp format"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn timestamp_rejects_unparseable_json() {
        let err = validate_alexa_timestamp(b"{ not json").unwrap_err();
        assert!(err.contains("Failed to parse"), "unexpected: {err}");
    }

    // --- verify_alexa_signature -----------------------------------------

    #[tokio::test]
    async fn alexa_signature_rejects_missing_cert_header() {
        let mut headers = HeaderMap::new();
        headers.insert("signature", HeaderValue::from_static("sig"));
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        let err = verify_alexa_signature(&headers, &body).await.unwrap_err();
        assert!(err.contains("SignatureCertChainUrl"), "unexpected: {err}");
    }

    #[tokio::test]
    async fn alexa_signature_rejects_missing_signature_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "signaturecertchainurl",
            HeaderValue::from_static("https://s3.amazonaws.com/echo.api/cert.pem"),
        );
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        let err = verify_alexa_signature(&headers, &body).await.unwrap_err();
        assert!(err.contains("Signature header"), "unexpected: {err}");
    }

    #[tokio::test]
    async fn alexa_signature_rejects_bad_cert_url() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "signaturecertchainurl",
            HeaderValue::from_static("https://evil.example.com/echo.api/cert.pem"),
        );
        headers.insert("signature", HeaderValue::from_static("sig"));
        let body = alexa_body_with_timestamp(&Utc::now().to_rfc3339());
        assert!(verify_alexa_signature(&headers, &body).await.is_err());
    }

    // The Alexa cert fetch must go through a single shared, timeout-bounded
    // client rather than building a fresh `reqwest::Client` per call — a burst
    // of cold-cache requests would otherwise spawn unbounded TLS clients.
    // Guards against a regression back to `reqwest::Client::new()` per fetch.
    #[test]
    fn alexa_cert_http_client_is_shared() {
        let first = alexa_cert_http_client();
        let second = alexa_cert_http_client();
        assert!(
            std::ptr::eq(first, second),
            "expected a single process-wide Alexa cert HTTP client"
        );
    }

    // Sanity-check the fetch timeouts are ordered sensibly (connect budget is a
    // subset of the overall request budget) and are actually bounded.
    #[test]
    fn alexa_cert_timeouts_are_bounded() {
        assert!(ALEXA_CERT_CONNECT_TIMEOUT <= ALEXA_CERT_REQUEST_TIMEOUT);
        assert!(!ALEXA_CERT_REQUEST_TIMEOUT.is_zero());
    }

    // Regression for issue #2668: a valid cert URL + fresh timestamp is NO
    // LONGER sufficient. The old code returned Ok() here without ever checking
    // the signature; the request-body signature verification now lives in
    // `verify_alexa_cert_and_signature`, exercised end-to-end below against a
    // locally generated certificate. (The `verify_alexa_signature` wrapper adds
    // a live cert fetch over the network, so its happy path is covered by an
    // integration harness, not this unit test.)

    // A self-signed RSA-2048 certificate whose SAN is `echo-api.amazon.com`,
    // valid 2026..2126 (generated with `openssl req -x509 -sha1`), plus its
    // PKCS#8 private key — used to sign a body the way Alexa's edge would.
    const ALEXA_TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIDPzCCAiegAwIBAgIUc05E5fmM11dnpbFxoswJSfvairYwDQYJKoZIhvcNAQEF\n\
BQAwHjEcMBoGA1UEAwwTZWNoby1hcGkuYW1hem9uLmNvbTAgFw0yNjA4MDYwODI3\n\
MDlaGA8yMTI2MDcxMzA4MjcwOVowHjEcMBoGA1UEAwwTZWNoby1hcGkuYW1hem9u\n\
LmNvbTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAJMYjkpium0M0ozF\n\
xb2Cb+9G2KtRxsN15YOYfUpp1yWdE08R+ZJbXqk3Q4bP7axRJtDiR/a9OlsvEcJS\n\
twELoCkforsFQyJ53d5UCzPdLj2iOhXpg0jtv0Gh3u7Fg27n/ugqEhHaJukSjCiX\n\
4lao8t1Wd9lkxLhOEUCP6D7a9NYmIfu5ut3MqmyKeOfAAVBOjaVhS374Zq3+0OnY\n\
wSimT4+9FoTZMfeVmyVzyByyR5C3CSG9+VZq3sG86AJdhZ786ko7Dlsi/XppTQe+\n\
CYiYWb+gPEVoN8ezopr15cgAMREUNUKHajQBHbD8FEHkTAPnZqT0wrxD3MLleuKp\n\
pc2j/wkCAwEAAaNzMHEwHQYDVR0OBBYEFKYUUOrdDzvE0xV9fZJ+kDY6csU2MB8G\n\
A1UdIwQYMBaAFKYUUOrdDzvE0xV9fZJ+kDY6csU2MA8GA1UdEwEB/wQFMAMBAf8w\n\
HgYDVR0RBBcwFYITZWNoby1hcGkuYW1hem9uLmNvbTANBgkqhkiG9w0BAQUFAAOC\n\
AQEADWxe+1VEnu98Q4Xb7egrV/DUkSZLEfM9H2gSqux3NYDzjRa0/7nivMQCZGSw\n\
IgRswmQDAV+djLUNQcmONqVgc8VkdHsqeZb5zgr6eEiNqHgIkwGnz41OS3oz05Qb\n\
AlFFJU8TsobjnOQ9//Qt/ECtlEBpmretbGld1dxJ0cNo4mpLtCBjXkb9uBFeiwCy\n\
yUjA/5nfAXELnHCTHNN1YFC1CNBQQk9bFHwQEAc9OgWvMIojbcoXbRwZpUOaQOlE\n\
I6tLJePCUV9CHXNgSRBEPWCbufVHtFtyJQisGBSNUMzVPMQHbBouYUOmvju7H/bc\n\
i+jshHpYblmQyitGXulqctcnWA==\n\
-----END CERTIFICATE-----\n";

    const ALEXA_TEST_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCTGI5KYrptDNKM\n\
xcW9gm/vRtirUcbDdeWDmH1KadclnRNPEfmSW16pN0OGz+2sUSbQ4kf2vTpbLxHC\n\
UrcBC6ApH6K7BUMied3eVAsz3S49ojoV6YNI7b9Bod7uxYNu5/7oKhIR2ibpEowo\n\
l+JWqPLdVnfZZMS4ThFAj+g+2vTWJiH7ubrdzKpsinjnwAFQTo2lYUt++Gat/tDp\n\
2MEopk+PvRaE2TH3lZslc8gcskeQtwkhvflWat7BvOgCXYWe/OpKOw5bIv16aU0H\n\
vgmImFm/oDxFaDfHs6Ka9eXIADERFDVCh2o0AR2w/BRB5EwD52ak9MK8Q9zC5Xri\n\
qaXNo/8JAgMBAAECggEAAaTb/UHseGN07HYZkKBHM5GeAmR/SqgJX5lvaArLXjwg\n\
JGl3ObmmnZCfWHUVNdBxNRMxr59733Dvm2rXfli0tT+euYVJ6RYbLXzmDvy1OOLf\n\
OudhgbSHh0/MSSyhBwxV0DQygBYpNRW8G4g0CJjU4GCoUFWPcHRjPg6bBMMndUe9\n\
kWOB2kZ3o4oQkJCgQ0/93VnYYk2i7mWjrUNbpqhcLX2+GzzrecGnPbbF2528IKRL\n\
GAUn9VdkwvzP8fGWOpq9qa6iWw6kkteCoQLZoHR/hN1XaQhGN2HhqjKu43gxmm16\n\
eNa4nm+xfoe2f3eHHqkVZ+RkJHKfxMtO7HHBXKfcoQKBgQDHcmRYyo33erlu28gg\n\
vINI3I7D9IqRiByrtnlpnDADtEhorcJFkZXtJuV+14HxK5nBl042bY4hSS52Y3UL\n\
poW0ibpiOVUp46PvNs7grqTBew1WX5/Cf+zebbLChHOAJi6lfouWQaCwUmNn7qs7\n\
rh+lxtcOMb+dB/yDyhvq/nBn6QKBgQC8zhKsfpPgvgwgw104zk5NJ6Pyvnvn3TRq\n\
AB0YuBpgAYWRJMXomBurmB2wv7PfBduBEiXKKazHH7n3b74U+7lGhq+ydU7K/Nd2\n\
tgtC5klKo0B6cFtPZ+0URhJ7clrYobRcFnVTax/dhvNNah8J8vGvi90BTL/34GWe\n\
p6iFLSuKIQKBgFDuyG2HdGhycoDbyrAODzAn3/8AYqJ/mzLKzyXd7VXzeFaR+/2D\n\
AFXFrOb1yJL24GPAZEqN1lkHe0UrQrnBjwwdv3ZQUZC4ATP3B6gA9nZU2qqsDwY8\n\
JwBzf1CTstLTq6YYXchRRUWHiTMJlI6ZL9pzf50Q7vJn5T4Na5rGORLRAoGAfSnH\n\
q16GPfj/JUEeLahmtDNRNn0cuwsj0hmdMGPr6DVaDGxqXtVnkovXMvMDFRhW+evD\n\
7Y9PIPphWC1Vv6dYne5vz0iBIYQYenQYZxMvBzHObtzJS4zD2CrT2c5ndzFL1bh1\n\
swVTLJJn/KwbQ4cwvYVkz5XHtVWnSFQxHYhiUsECgYBTJ6GXGXWyVgCCZMEsJ52z\n\
dsIyiBSr+2cG/1bvncg2zhl45m6krGTplUO9p4K8sSwmrQxGe13nbtbijNeLuQ0D\n\
FSMLvIzM5qOlblU6uGMSaCW3G6n4FKJo/Vqm8frVJP7wvywYy0AA32smETJyqFN0\n\
dQPD++LeIpOChiX4Ru2uMQ==\n\
-----END PRIVATE KEY-----\n";

    // Sign `body` exactly as Alexa's edge does: PKCS#1 v1.5 RSA over SHA-1(body).
    fn alexa_sign(body: &[u8]) -> Vec<u8> {
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::{Pkcs1v15Sign, RsaPrivateKey};
        use sha1::{Digest, Sha1};
        let key = RsaPrivateKey::from_pkcs8_pem(ALEXA_TEST_KEY_PEM).expect("load test key");
        let digest = Sha1::digest(body);
        key.sign(Pkcs1v15Sign::new::<Sha1>(), &digest)
            .expect("sign body")
    }

    #[test]
    fn alexa_cert_and_signature_accepts_valid_signature() {
        let body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(body);
        assert!(
            verify_alexa_cert_and_signature(ALEXA_TEST_CERT_PEM.as_bytes(), &sig, body, Utc::now())
                .is_ok(),
            "a body signed with the cert's key must verify"
        );
    }

    #[test]
    fn alexa_cert_and_signature_rejects_forged_signature() {
        // Attacker presents a body they control with a bogus signature — the
        // exact forgery the old code accepted. Must be rejected.
        let body = br#"{"request":{"type":"IntentRequest","intent":{"name":"evil"}}}"#;
        let forged = vec![0u8; 256]; // right length, wrong signature
        let err = verify_alexa_cert_and_signature(
            ALEXA_TEST_CERT_PEM.as_bytes(),
            &forged,
            body,
            Utc::now(),
        )
        .unwrap_err();
        assert!(err.contains("does not match"), "unexpected: {err}");
    }

    #[test]
    fn alexa_cert_and_signature_rejects_tampered_body() {
        // Valid signature for one body must not authenticate a different body.
        let signed_body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(signed_body);
        let tampered_body = br#"{"request":{"type":"LaunchRequest"} }"#; // one byte differs
        let err = verify_alexa_cert_and_signature(
            ALEXA_TEST_CERT_PEM.as_bytes(),
            &sig,
            tampered_body,
            Utc::now(),
        )
        .unwrap_err();
        assert!(err.contains("does not match"), "unexpected: {err}");
    }

    #[test]
    fn alexa_cert_and_signature_rejects_expired_certificate() {
        // The test cert is valid 2026..2126; evaluate it far in the future.
        let body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(body);
        let far_future = Utc::now() + Duration::days(365 * 200);
        let err =
            verify_alexa_cert_and_signature(ALEXA_TEST_CERT_PEM.as_bytes(), &sig, body, far_future)
                .unwrap_err();
        assert!(err.contains("expired"), "unexpected: {err}");
    }

    #[test]
    fn alexa_cert_and_signature_rejects_unparseable_cert() {
        let body = br#"{"request":{"type":"LaunchRequest"}}"#;
        let sig = alexa_sign(body);
        assert!(verify_alexa_cert_and_signature(
            b"-----BEGIN CERTIFICATE-----\nnope\n",
            &sig,
            body,
            Utc::now()
        )
        .is_err());
    }

    // --- verify_google_request (fail-closed JWT verification) -----------
    //
    // Regression coverage for the fail-open bug: `verify_google_request` used
    // to return Ok(()) for an unset project id, a missing auth header, and a
    // forged bearer whose base64 payload merely *contained* the project id
    // (with no signature check), and it decoded JWT payloads with the STANDARD
    // (padded) base64 alphabet so real URL-safe Google tokens silently fell
    // through to Ok(()). It now verifies the RS256 signature + aud/iss/exp and
    // fails closed on every error path.

    // Fixed RSA test keypair (2048-bit) used to mint + verify signed tokens in
    // these unit tests. NOT a real Google key — the production path resolves the
    // decoding key from Google's live JWKS via `google_decoding_key`.
    const TEST_RSA_PRIV_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC7p2rNa9yZ9llX
ZmRH2tWIayNP4yIdGP72CzZx8/YxOx03BbqKTNmcfpShjw9AuaWENi0sWXe9j1TX
RR/JZ1Cff2qDHwsFfdwAwWraFtKFrFAziLG9yZXicxxY9fotLHOzEGwnPqZo4JqK
APqVW/0bZzQLIpYALCM7Snj/DA2axsrlHH1Usi5UqEjaXQUpagwGRVpyR6eS6/Sg
u/BkU50/xMZCyyv1/0ebeDKxOy3I5j+hxLW/reJZaNIOHEzZwXmARjah8b0L0/yP
C6h5Y1Xg7Rn53qBw+DI03Z2HEwFf9cldO15X0B9XQiqJ+ZdGi2Jj6eq3oOBp812w
1SIzp+V/AgMBAAECggEAAQP5l/2qxp/cAT+/Rdmb/jkA3+siwXWOgQEJMmSs7byc
KTK8El4yxJ4Kv9++UriuefYGbeRYuYs6XPqKyX7oTh9VEZCWxq4qWqFcAAIk8YQ/
4DIv2WRrOJEsPhmsA5fnUw4WXRVXC9+V9oPlge7ALT3JvPsFmh/4W4HJAIMC2oDm
iB1fsl0jq+rk+k0JYIy11yRcI6Dz6CxMzrZtkl6h66a9LXUpwfqVSPUwuHilXCwp
PpwNGja/Qi+91DFAM3AjAHy85uSzg+wV2thCKvS+OF45z2MuElXnWDbQKinW6ige
tl8lbvQkIvDTpPAecSokYsV1ZI7LX5gqT4GlO1ImQQKBgQDmgDEsdBuIJP1lJarL
NeL2/t9n9Nhs7FoxPy+qKy8+7F6W98wJzrXXCxcK6NhKSw6RmPunrFx3ha3i6aPp
hu/855ihB0ZTkvNHQxCKmHPDW0wrrN6u77ANMYQa+Uwbs/DDbe/5Bj24ECcYqwHk
7womofS/DewMsabouuk+ymxmWwKBgQDQacynrub8CjTUaCyvLqcKjIv/+Mv0i3j+
zO1Fpb8bO9NM/FCKtnB3oddIRINAQRNJ3nd1V8HrHZhGxOTbIIeLi2RmtmJ0kjBE
gVAKPDAqvhaybfgnRKsFpl0WDzeu0UmwaC2zrsBLEXpoLRfnT2JHnA4THBj3h4yG
hVoIP1VOrQKBgQDJx7DEZIPxg8gbcoT4TZz5chbqb0nC2IkAEXtNcW5znAIWEKia
cU14CepLD5jAOMJxLMYoe1ea/fhB6xwlg421DJztYmvrH3o+iPQDEABPJS4iEbwC
0iqA8jbeUhyRJ819l1D6477F0cYX7yPCYIu3VBHn6m0Yk7A0jeM/p36LfwKBgFhf
5KZeJhhOA6TmH7yRHcf9XQhH6cRiuAXjw+E6rVTRA4Kro0OOpRY1jGJamwVOEu3J
5gHeGp6mSAIKT7kTjCaCDyr2v70KmGkUJGqSpyIYxOsYcpfEKHkW2HYYMdZxbLvf
ETIWMfgjCzLNnEs7gEM5S0aTLYsY8V/BgDHrGTNpAoGBANeF2qEjS1NyJ99mJcGj
Y5TPLyQot9d9hEkIpNT2aAFnUb0baqJ0j1eyN6NvOCUAdXcqskj4S5G5kIJEHQxP
mR3t5mHnqglfXy8CSVtCuIuBxzPf32fQq8XZo0JtE6oKfbd0jnsAX61NYhnF7LDO
mFM80HjvdsbTWHj9n2QgMU5B
-----END PRIVATE KEY-----";
    const TEST_RSA_PUB_PEM: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu6dqzWvcmfZZV2ZkR9rV
iGsjT+MiHRj+9gs2cfP2MTsdNwW6ikzZnH6UoY8PQLmlhDYtLFl3vY9U10UfyWdQ
n39qgx8LBX3cAMFq2hbShaxQM4ixvcmV4nMcWPX6LSxzsxBsJz6maOCaigD6lVv9
G2c0CyKWACwjO0p4/wwNmsbK5Rx9VLIuVKhI2l0FKWoMBkVackenkuv0oLvwZFOd
P8TGQssr9f9Hm3gysTstyOY/ocS1v63iWWjSDhxM2cF5gEY2ofG9C9P8jwuoeWNV
4O0Z+d6gcPgyNN2dhxMBX/XJXTteV9AfV0IqifmXRotiY+nqt6DgafNdsNUiM6fl
fwIDAQAB
-----END PUBLIC KEY-----";

    const GOOGLE_ISS: &str = "https://accounts.google.com";

    fn test_decoding_key() -> DecodingKey {
        DecodingKey::from_rsa_pem(TEST_RSA_PUB_PEM.as_bytes()).unwrap()
    }

    // Mint an RS256-signed JWT with the test private key. `exp_offset` is
    // seconds relative to now (negative => already expired).
    fn signed_google_token(iss: &str, aud: &str, exp_offset: i64) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        let now = Utc::now().timestamp();
        let claims = serde_json::json!({
            "iss": iss,
            "aud": aud,
            "sub": "1234567890",
            "iat": now,
            "exp": now + exp_offset,
        });
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-kid".to_string());
        let key = EncodingKey::from_rsa_pem(TEST_RSA_PRIV_PEM.as_bytes()).unwrap();
        encode(&header, &claims, &key).unwrap()
    }

    // A properly-signed, correctly-audienced, unexpired Google ID token is
    // accepted. Also proves URL-safe base64url JWT segments decode for real
    // (the old STANDARD-alphabet decode silently failed and fell through).
    #[test]
    fn google_id_token_valid_signature_accepted() {
        let token = signed_google_token(GOOGLE_ISS, "my-project-123", 3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_ok());
    }

    // A forged bearer whose payload merely *contains* the project id but whose
    // signature does not verify is rejected (the core of the old fail-open bug).
    #[test]
    fn google_id_token_forged_bearer_rejected() {
        let good = signed_google_token(GOOGLE_ISS, "my-project-123", 3600);
        let parts: Vec<&str> = good.split('.').collect();
        // Keep the real header + the payload (still carries aud=my-project-123),
        // but replace the signature with garbage.
        let forged = format!("{}.{}.AAAAAAAAAAAAAAAA", parts[0], parts[1]);
        let err = verify_google_id_token(&forged, "my-project-123", &test_decoding_key())
            .expect_err("forged signature must be rejected");
        assert!(err.contains("verification failed"), "unexpected: {err}");
    }

    // Signature is valid but the audience is a different project => rejected
    // (exact project-id match, not a substring contains()).
    #[test]
    fn google_id_token_wrong_project_rejected() {
        let token = signed_google_token(GOOGLE_ISS, "someone-elses-project", 3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_err());
    }

    // Signature + audience valid but the token is expired => rejected.
    #[test]
    fn google_id_token_expired_rejected() {
        let token = signed_google_token(GOOGLE_ISS, "my-project-123", -3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_err());
    }

    // Signature + audience valid but issued by an untrusted issuer => rejected.
    #[test]
    fn google_id_token_untrusted_issuer_rejected() {
        let token = signed_google_token("https://evil.example.com", "my-project-123", 3600);
        assert!(verify_google_id_token(&token, "my-project-123", &test_decoding_key()).is_err());
    }

    // GOOGLE_ACTIONS_PROJECT_ID is process-global and read only by
    // `verify_google_request`; confining every mutation of it to this single
    // test keeps the assertions race-free under the parallel test runner. All
    // branches here fail closed *before* any JWKS network fetch.
    #[tokio::test]
    async fn google_request_fails_closed_on_bad_input() {
        // Unset project id => fail closed (previously returned Ok).
        std::env::remove_var("GOOGLE_ACTIONS_PROJECT_ID");
        let err = verify_google_request(&HeaderMap::new())
            .await
            .expect_err("must fail closed when project id unset");
        assert!(err.contains("not configured"), "unexpected: {err}");

        std::env::set_var("GOOGLE_ACTIONS_PROJECT_ID", "my-project-123");

        // Missing Authorization header => rejected (previously returned Ok).
        let err = verify_google_request(&HeaderMap::new())
            .await
            .expect_err("missing auth header must be rejected");
        assert!(err.contains("Missing Authorization"), "unexpected: {err}");

        // Malformed Authorization (no "Bearer " prefix) => rejected.
        let mut bad_scheme = HeaderMap::new();
        bad_scheme.insert("authorization", HeaderValue::from_static("Basic abc"));
        let err = verify_google_request(&bad_scheme)
            .await
            .expect_err("non-bearer scheme must be rejected");
        assert!(
            err.contains("Authorization header format"),
            "unexpected: {err}"
        );

        // Structurally invalid bearer token => rejected before any network I/O.
        let mut bad_jwt = HeaderMap::new();
        bad_jwt.insert("authorization", HeaderValue::from_static("Bearer only.two"));
        assert!(verify_google_request(&bad_jwt).await.is_err());

        std::env::remove_var("GOOGLE_ACTIONS_PROJECT_ID");
    }

    // --- verify_hmac_signature + the mounted /verify handler ------------
    //
    // VOICE_WEBHOOK_SECRET is process-global and read by `verify_hmac_signature`
    // (directly and via the `/verify` "google" branch). All of its assertions
    // live in this one test so they run sequentially and never race.
    #[tokio::test]
    async fn hmac_and_verify_endpoint_google_branch() {
        std::env::set_var("VOICE_WEBHOOK_SECRET", "unit-test-secret");
        let body = "payload-to-sign";
        let good = hmac_sig("unit-test-secret", body);

        // Direct helper: matching signature -> true, tampered -> false.
        assert!(verify_hmac_signature(&good, body).unwrap());
        assert!(!verify_hmac_signature("wrong", body).unwrap());

        // Mounted /verify endpoint, "google" platform: valid signature.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "google".to_string(),
            signature: good.clone(),
            body: body.to_string(),
            timestamp: None,
        }))
        .await;
        assert!(resp.0.valid);
        assert_eq!(resp.0.platform, "google");
        assert!(resp.0.error.is_none());

        // Mounted /verify endpoint, "google" platform: invalid signature.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "google".to_string(),
            signature: "bad".to_string(),
            body: body.to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid);
        assert_eq!(resp.0.error.as_deref(), Some("Invalid signature"));

        // Issue #2658 (#2): with the secret unset, verification fails closed
        // (Err) rather than silently verifying against a hardcoded default.
        std::env::remove_var("VOICE_WEBHOOK_SECRET");
        assert!(verify_hmac_signature(&good, body).is_err());
    }

    // --- ct_eq_str / voice_device_token_matches (issue #2658) -----------

    #[test]
    fn ct_eq_str_matches_only_identical_strings() {
        assert!(ct_eq_str("token-abc", "token-abc"));
        assert!(!ct_eq_str("token-abc", "token-abd"));
        assert!(!ct_eq_str("token-abc", "token-abc-longer"));
        assert!(!ct_eq_str("", "x"));
        assert!(ct_eq_str("", ""));
    }

    /// A `VoiceAssistantDevice` whose `access_token_encrypted` holds `plaintext`
    /// encrypted under `crypto`, expiring at `expires_at`.
    fn device_with_token(
        crypto: &IntegrationCrypto,
        plaintext: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> db::models::VoiceAssistantDevice {
        let now = Utc::now();
        db::models::VoiceAssistantDevice {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            unit_id: None,
            platform: voice_platform::ALEXA.to_string(),
            device_id: "dev-1".to_string(),
            device_name: None,
            linked_at: now,
            last_used_at: None,
            access_token_encrypted: plaintext.map(|p| crypto.encrypt(p).unwrap()),
            refresh_token_encrypted: None,
            token_expires_at: expires_at,
            is_active: true,
            capabilities: serde_json::json!([]),
            // The pure predicate ignores the lookup hash (it re-derives trust
            // from the decrypted ciphertext); the DB-backed selection is covered
            // by `authenticate_voice_user_*` below.
            access_token_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn voice_token_match_is_exact_and_rejects_mismatch_missing_and_expired() {
        // Deterministic 32-byte key; construct crypto directly (no env races).
        let crypto = IntegrationCrypto::new(&"ab".repeat(32)).expect("crypto");
        let now = Utc::now();

        // Exact match on a non-expiring token -> authenticated.
        let dev = device_with_token(&crypto, Some("real-token"), None);
        assert!(voice_device_token_matches(&crypto, &dev, "real-token", now));

        // Wrong token -> not a match (this is the broken-auth regression: the
        // old code returned this device regardless of the presented token).
        assert!(!voice_device_token_matches(&crypto, &dev, "attacker", now));

        // Device without a stored token -> never a match.
        let no_tok = device_with_token(&crypto, None, None);
        assert!(!voice_device_token_matches(
            &crypto,
            &no_tok,
            "real-token",
            now
        ));

        // Matching token but expired -> rejected.
        let expired =
            device_with_token(&crypto, Some("real-token"), Some(now - Duration::hours(1)));
        assert!(!voice_device_token_matches(
            &crypto,
            &expired,
            "real-token",
            now
        ));

        // Matching token still valid in the future -> accepted.
        let future = device_with_token(&crypto, Some("real-token"), Some(now + Duration::hours(1)));
        assert!(voice_device_token_matches(
            &crypto,
            &future,
            "real-token",
            now
        ));

        // A token encrypted under a different key can't be decrypted -> no match.
        let other = IntegrationCrypto::new(&"cd".repeat(32)).expect("crypto");
        assert!(!voice_device_token_matches(&other, &dev, "real-token", now));
    }

    // --- authenticate_voice_user (DB-backed selection path, issue #2662) ------
    //
    // Postgres-backed via `#[sqlx::test]` (runs under backend.yml CI; the cloud
    // verify gate skips it — no DATABASE_URL). This drives the real fetch +
    // selection loop the pure `voice_device_token_matches` predicate cannot
    // reach: the #2660 regression lived exactly here (the old code returned
    // *any* active device for the platform regardless of the presented token).

    /// Seed one active voice device on `platform` whose stored token is `token`
    /// (encrypted under `crypto`) and whose indexed lookup hash is the keyed
    /// HMAC of `token`, expiring at `expires_at`. Returns the new device id.
    async fn seed_voice_device(
        pool: &sqlx::PgPool,
        crypto: &IntegrationCrypto,
        platform: &str,
        token: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO voice_assistant_devices (
                id, organization_id, user_id, platform, device_id,
                access_token_encrypted, access_token_hash, token_expires_at,
                is_active, capabilities
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, TRUE, '[]'::jsonb)
            "#,
        )
        .bind(id)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(platform)
        .bind(format!("dev-{id}"))
        .bind(crypto.encrypt(token).expect("encrypt seed token"))
        .bind(voice_access_token_hash(token))
        .bind(expires_at)
        .execute(pool)
        .await
        .expect("seed voice device");
        id
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn authenticate_voice_user_selects_owner_and_rejects_others(pool: sqlx::PgPool) {
        // `authenticate_voice_user` reads the encryption key from the process
        // environment — both to decrypt stored tokens and to key the lookup
        // hash — so pin a deterministic key. This is the only in-crate test that
        // touches INTEGRATION_ENCRYPTION_KEY.
        let key = "ab".repeat(32); // 64 hex chars = 32 bytes
        std::env::set_var(integrations::ENCRYPTION_KEY_ENV, &key);
        let crypto = IntegrationCrypto::new(&key).expect("crypto");

        // Two devices on the same platform with distinct tokens (the priv-esc
        // guard: presenting A's token must never authenticate as B).
        let id_a = seed_voice_device(&pool, &crypto, voice_platform::ALEXA, "token-A", None).await;
        let _id_b = seed_voice_device(&pool, &crypto, voice_platform::ALEXA, "token-B", None).await;

        let mut conn = pool.acquire().await.expect("acquire connection");

        // (a) Presenting A's token resolves to exactly device A, never B.
        let dev = authenticate_voice_user(&mut conn, "token-A", voice_platform::ALEXA)
            .await
            .expect("token-A must authenticate as device A");
        assert_eq!(
            dev.id, id_a,
            "presented token-A must resolve to device A only"
        );

        // (b) A token matching no device -> 401 INVALID_ACCESS_TOKEN.
        let (status, body) =
            authenticate_voice_user(&mut conn, "token-UNKNOWN", voice_platform::ALEXA)
                .await
                .expect_err("unknown token must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.0.code, "INVALID_ACCESS_TOKEN");

        // (c) An expired but otherwise-matching token is rejected.
        let past = Utc::now() - Duration::hours(1);
        seed_voice_device(
            &pool,
            &crypto,
            voice_platform::ALEXA,
            "token-EXP",
            Some(past),
        )
        .await;
        let (status, body) = authenticate_voice_user(&mut conn, "token-EXP", voice_platform::ALEXA)
            .await
            .expect_err("expired token must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.0.code, "INVALID_ACCESS_TOKEN");

        // (d) An absent (empty or whitespace-only) access token is rejected
        // outright — the fail-closed early return, before any device lookup, so
        // a caller presenting no token can never be authenticated as a device.
        for absent in ["", "   "] {
            let (status, body) = authenticate_voice_user(&mut conn, absent, voice_platform::ALEXA)
                .await
                .expect_err("absent token must be rejected");
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(body.0.code, "INVALID_ACCESS_TOKEN");
        }

        std::env::remove_var(integrations::ENCRYPTION_KEY_ENV);
    }

    #[tokio::test]
    async fn verify_endpoint_alexa_branch_fails_closed() {
        // SECURITY regression: the JSON /verify endpoint must NEVER report an
        // Alexa signature as valid. It has no certificate-chain URL and no raw
        // signed body, so it cannot perform Amazon's cert-based verification;
        // the old code returned `valid: !signature.is_empty()`, accepting any
        // non-empty string as a valid Alexa signature (a forgery vector).

        // A forged, attacker-controlled non-empty "signature" is rejected — this
        // is exactly the input the old code accepted as valid.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "alexa".to_string(),
            signature: "any-forged-non-empty-string".to_string(),
            body: r#"{"request":{"type":"IntentRequest","intent":{"name":"evil"}}}"#.to_string(),
            timestamp: None,
        }))
        .await;
        assert!(
            !resp.0.valid,
            "a non-empty (forged) Alexa signature must NOT be accepted"
        );
        assert_eq!(resp.0.platform, "alexa");
        assert!(
            resp.0
                .error
                .as_deref()
                .is_some_and(|e| e.contains("cannot verify Alexa signatures")),
            "unexpected error: {:?}",
            resp.0.error
        );

        // Even a plausibly base64-shaped signature is rejected — there is no
        // input to this endpoint that yields `valid: true` for Alexa.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "alexa".to_string(),
            signature: BASE64.encode([0u8; 256]),
            body: "b".to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid, "no Alexa input may be reported as valid");

        // Empty signature -> still invalid, with the "Missing signature" hint.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "alexa".to_string(),
            signature: String::new(),
            body: "b".to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid);
        assert_eq!(resp.0.error.as_deref(), Some("Missing signature"));
    }

    #[tokio::test]
    async fn verify_endpoint_unknown_platform() {
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "siri".to_string(),
            signature: "x".to_string(),
            body: "b".to_string(),
            timestamp: None,
        }))
        .await;
        assert!(!resp.0.valid);
        assert_eq!(resp.0.platform, "siri");
        assert_eq!(resp.0.error.as_deref(), Some("Unknown platform"));
    }

    // --- alexa_health_check ---------------------------------------------

    #[tokio::test]
    async fn health_check_returns_ok() {
        assert_eq!(alexa_health_check().await, StatusCode::OK);
    }

    // --- voice_encryption_required (OAuth persistence fail-closed) -------

    #[test]
    fn encryption_required_maps_to_500() {
        let (status, body) =
            voice_encryption_required(CryptoError::KeyNotConfigured("no key".to_string()));
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.0.code, "ENCRYPTION_REQUIRED");
    }

    // --- extract_alexa_command_text -------------------------------------

    fn intent(name: &str, slots: Option<serde_json::Value>) -> AlexaIntent {
        AlexaIntent {
            name: name.to_string(),
            slots,
        }
    }

    #[test]
    fn command_text_maps_builtin_intents() {
        assert_eq!(
            extract_alexa_command_text(&intent("AMAZON.HelpIntent", None)),
            "help"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("AMAZON.StopIntent", None)),
            "goodbye"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("AMAZON.CancelIntent", None)),
            "goodbye"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("CheckBalanceIntent", None)),
            "check my balance"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("CheckAnnouncementsIntent", None)),
            "check announcements"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("CheckMeterIntent", None)),
            "check meter readings"
        );
        assert_eq!(
            extract_alexa_command_text(&intent("ContactManagerIntent", None)),
            "contact manager"
        );
    }

    #[test]
    fn command_text_extracts_fault_description_slot() {
        let slots = serde_json::json!({
            "FaultDescription": { "value": "broken heating" }
        });
        assert_eq!(
            extract_alexa_command_text(&intent("ReportFaultIntent", Some(slots))),
            "report a fault with broken heating"
        );
    }

    #[test]
    fn command_text_fault_defaults_without_slot() {
        assert_eq!(
            extract_alexa_command_text(&intent("ReportFaultIntent", None)),
            "report a fault with a fault"
        );
    }

    #[test]
    fn command_text_passes_through_unknown_intent() {
        assert_eq!(
            extract_alexa_command_text(&intent("MyCustomIntent", None)),
            "MyCustomIntent"
        );
    }

    // --- Alexa response builders ----------------------------------------

    #[test]
    fn build_alexa_response_plaintext() {
        let r = build_alexa_response(&action_result("hello", None, true));
        assert_eq!(r.response.output_speech.speech_type, "PlainText");
        assert_eq!(r.response.output_speech.text.as_deref(), Some("hello"));
        assert!(r.response.output_speech.ssml.is_none());
        assert!(r.response.should_end_session);
        assert!(r.response.card.is_none());
    }

    #[test]
    fn build_alexa_response_ssml() {
        let r = build_alexa_response(&action_result("hi", Some("<speak>hi</speak>"), false));
        assert_eq!(r.response.output_speech.speech_type, "SSML");
        assert_eq!(
            r.response.output_speech.ssml.as_deref(),
            Some("<speak>hi</speak>")
        );
        assert!(r.response.output_speech.text.is_none());
    }

    #[test]
    fn build_alexa_response_with_card() {
        let r = build_alexa_response(&action_result_with_card("your balance"));
        let card = r.response.card.expect("card present");
        assert_eq!(card.card_type, "Simple");
        assert_eq!(card.title, "Balance");
        assert_eq!(card.content.as_deref(), Some("You owe 12 EUR"));
    }

    #[test]
    fn build_alexa_link_account_response_shape() {
        let r = build_alexa_link_account_response();
        let card = r.response.card.expect("link card present");
        assert_eq!(card.card_type, "LinkAccount");
        assert!(r.response.should_end_session);
    }

    // --- Google response builders ---------------------------------------

    #[test]
    fn build_google_response_ends_conversation() {
        let r = build_google_response("sess-1", &action_result("done", None, true));
        assert_eq!(r.session.id, "sess-1");
        assert_eq!(r.prompt.first_simple.speech, "done");
        let scene = r.scene.expect("end scene present");
        assert_eq!(scene.name, "actions.scene.END_CONVERSATION");
    }

    #[test]
    fn build_google_response_continues_without_scene() {
        let r = build_google_response("sess-2", &action_result_with_card("balance"));
        assert!(r.scene.is_none());
        let content = r.prompt.content.expect("content present");
        let card = content.card.expect("card present");
        assert_eq!(card.title, "Balance");
    }

    #[test]
    fn build_google_link_account_response_shape() {
        let r = build_google_link_account_response("sess-3");
        assert_eq!(r.session.id, "sess-3");
        let scene = r.scene.expect("link scene present");
        assert_eq!(scene.name, "AccountLinking");
    }
}

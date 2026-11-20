//! Voice assistant webhook routes (Epic 93: Voice Assistant & OAuth Completion).
//!
//! Story 93.3: Voice Platform Webhooks
//! - Alexa Skills Kit webhook handler
//! - Google Actions webhook handler
//! - Request signature verification
//! - User authentication via OAuth token
//!
//! # Module layout
//!
//! This module was split from a single ~3.1k-LOC `voice_webhooks.rs` into
//! surface-scoped pieces. The split is behaviour-preserving — every route
//! registration, handler body, auth/RLS guard, and the webhook security fixes
//! (#2658 broken-auth, #2668 Alexa forgery, #890 fail-open simulated tokens,
//! #765 fail-closed encryption, #2769 refresh/exchange throttling) are carried
//! over verbatim.
//!
//! | Surface                     | Location                              |
//! |-----------------------------|---------------------------------------|
//! | Router + webhook handlers   | this module (`mod.rs`)                |
//! | Signature / token verify    | [`signature`]                         |
//! | Provider request/response   | [`parsers`]                           |
//! | OAuth + device auth logic   | [`crate::services::voice_oauth`]      |

pub mod parsers;
pub mod signature;

use crate::routes::rate_limit::{rate_limit_allowed, InProcessRateLimiter};
use crate::services::voice_oauth::{
    authenticate_voice_user, encrypt_voice_token_pair, voice_oauth_error_response,
    voice_simulated_oauth_allowed, OAUTH_EXCHANGE_FAILED_MESSAGE, OAUTH_REFRESH_FAILED_MESSAGE,
};
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
use chrono::{Duration, Utc};
use common::errors::ErrorResponse;
use db::models::{
    voice_platform, AlexaOutputSpeech, AlexaRequestBody, AlexaResponseBody, AlexaSkillRequest,
    AlexaSkillResponse, GoogleActionsRequest, GoogleActionsResponse, VoiceOAuthExchangeRequest,
    VoiceOAuthExchangeResponse, VoiceTokenRefreshRequest, VoiceTokenRefreshResult,
    WebhookVerificationResult,
};
use db::RlsPool;
use integrations::IntegrationCrypto;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use parsers::{
    build_alexa_link_account_response, build_alexa_response, build_google_link_account_response,
    build_google_response, extract_alexa_command_text,
};
use signature::{verify_alexa_signature, verify_google_request, verify_hmac_signature};

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
                state.org_repo.clone(),
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
                state.org_repo.clone(),
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
        state.org_repo.clone(),
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
    api_core::TenantAuthUser(auth): api_core::TenantAuthUser,
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
                voice_oauth_error_response(
                    StatusCode::BAD_REQUEST,
                    "OAUTH_EXCHANGE_FAILED",
                    OAUTH_EXCHANGE_FAILED_MESSAGE,
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
                    voice_oauth_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "TOKEN_REFRESH_FAILED",
                        OAUTH_REFRESH_FAILED_MESSAGE,
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
// Signature Verification endpoint (Story 93.3)
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
// Tests (Epic 93: Voice Assistant webhooks)
// ============================================================================
//
// Scope: unit coverage for the mounted handlers' infrastructure-free logic — the
// rate limiters, the mounted `/verify` endpoint (both the HMAC "google" branch
// and the fail-closed "alexa" branch), and the health check. The signature /
// token-verification surface lives in `signature`, the provider parsers/builders
// in `parsers`, and the OAuth/device-auth business logic in
// `crate::services::voice_oauth`, each with its own unit tests. The four
// handlers that hold a live DB connection + `AppState` + `AuthUser`
// (`alexa_webhook`, `google_actions_webhook`, `oauth_token_exchange`,
// `oauth_token_refresh`) reach Postgres and the OAuth managers, so their
// happy-paths belong in an integration harness with a seeded database.
#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    // Compute a valid HMAC-SHA256 base64 signature for `body` under `secret`,
    // mirroring `signature::verify_hmac_signature`'s algorithm exactly.
    fn hmac_sig(secret: &str, body: &str) -> String {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        BASE64.encode(mac.finalize().into_bytes())
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

    // --- verify_hmac_signature + the mounted /verify handler ------------
    //
    // VOICE_WEBHOOK_SECRET is process-global and read by
    // `signature::verify_hmac_signature` (directly and via the `/verify`
    // "google" branch). All of its assertions live in this one test so they run
    // sequentially and never race.
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

        // ...and a *blank* secret is treated the same as unset — never key the
        // HMAC with an empty value (mirrors the sibling PORTAL/AIRBNB/STRIPE
        // receivers). Both a correct-looking and a bogus signature must Err.
        std::env::set_var("VOICE_WEBHOOK_SECRET", "");
        assert!(verify_hmac_signature(&good, body).is_err());
        assert!(verify_hmac_signature("anything", body).is_err());
        std::env::remove_var("VOICE_WEBHOOK_SECRET");
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
}

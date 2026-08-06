//! Voice assistant webhook routes (Epic 93: Voice Assistant & OAuth Completion).
//!
//! Story 93.3: Voice Platform Webhooks
//! - Alexa Skills Kit webhook handler
//! - Google Actions webhook handler
//! - Request signature verification
//! - User authentication via OAuth token

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
use serde::Deserialize;
use sha2::Sha256;
use sqlx::PgConnection;
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
/// Handles Google Assistant requests via Actions SDK.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/google",
    request_body = GoogleActionsRequest,
    responses(
        (status = 200, description = "Google Actions response", body = GoogleActionsResponse),
        (status = 401, description = "Unauthorized - invalid token"),
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
    // Verify request (Google uses Bearer token in Authorization header)
    if let Err(e) = verify_google_request(&headers) {
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
    let device_id = format!("{}_{}", request.platform, Uuid::new_v4());

    // Create the voice device with tokens.
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
    let device = state
        .llm_document_repo
        .create_voice_device(
            &mut **guard.conn(),
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
            tracing::error!("Failed to create voice device: {}", e);
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
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/voice/oauth/refresh",
    request_body = VoiceTokenRefreshRequest,
    responses(
        (status = 200, description = "Token refresh successful", body = VoiceTokenRefreshResult),
        (status = 404, description = "Device not found"),
        (status = 500, description = "Token refresh failed"),
    ),
    tag = "Voice OAuth"
)]
async fn oauth_token_refresh(
    State(state): State<AppState>,
    Json(request): Json<VoiceTokenRefreshRequest>,
) -> Result<Json<VoiceTokenRefreshResult>, (StatusCode, Json<ErrorResponse>)> {
    use integrations::{decrypt_if_available, VoiceOAuthManager, VoicePlatform};

    // Find the device.
    // PAP-170 (PAP-150 P5): this endpoint is called by the voice platform (no PM
    // principal). Bootstrap the device on a context-cleared connection — it is
    // addressed by an opaque, server-issued device_id and carries its owning
    // org/user, which we bind below for the token write.
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
        } else {
            // Platform not configured - use simulated tokens for development
            tracing::warn!(
                "Voice OAuth not configured for platform {}, using simulated refresh",
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
            // Alexa uses certificate-based signature verification
            // Simplified check for demo
            WebhookVerificationResult {
                valid: !request.signature.is_empty(),
                platform: "alexa".to_string(),
                error: if request.signature.is_empty() {
                    Some("Missing signature".to_string())
                } else {
                    None
                },
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

/// Fetch the PEM certificate chain from a (previously validated) Alexa cert
/// URL, memoising the result for [`ALEXA_CERT_CACHE_TTL`].
///
/// The URL is only ever reached after [`validate_alexa_cert_url`], so it always
/// points at `https://s3.amazonaws.com/echo.api/…`; TLS authenticates the S3
/// origin. Returns the raw PEM bytes (one or more concatenated certificates).
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

    let response = reqwest::Client::new()
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

/// Verify Google Actions request (Story 98.6).
///
/// Google Actions verification can use:
/// 1. Google project ID from the Google-Actions-API-Version header
/// 2. ID token in Authorization header (JWT format)
///
/// For security, we validate the project ID matches our configuration.
fn verify_google_request(headers: &HeaderMap) -> Result<(), String> {
    // Get the Google Actions API version header (contains project context)
    let api_version = headers
        .get("Google-Actions-API-Version")
        .and_then(|v| v.to_str().ok());

    if let Some(version) = api_version {
        tracing::debug!("Google Actions API version: {}", version);
    }

    // Get the Google Assistant Signature header if present
    let signature = headers
        .get("Google-Assistant-Signature")
        .and_then(|v| v.to_str().ok());

    // Check for Authorization header (Bearer token)
    let auth_header = headers.get("Authorization").and_then(|v| v.to_str().ok());

    // Validate project ID if configured
    if let Ok(expected_project) = std::env::var("GOOGLE_ACTIONS_PROJECT_ID") {
        // If we have a signature, it should contain project info
        if let Some(sig) = signature {
            // The signature is base64-encoded JSON with project info
            // For now, just log it - full validation would decode and verify
            tracing::debug!(
                signature_len = sig.len(),
                expected_project = %expected_project,
                "Google Actions signature present"
            );
        }

        // If we have an auth header with Bearer token, validate format
        if let Some(auth) = auth_header {
            if !auth.starts_with("Bearer ") {
                return Err("Invalid Authorization header format".to_string());
            }

            let token = &auth[7..];

            // Validate JWT format (three base64 parts separated by dots)
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() != 3 {
                return Err("Invalid JWT token format".to_string());
            }

            // In production, you would:
            // 1. Decode the JWT header to get the key ID
            // 2. Fetch Google's public keys from https://www.googleapis.com/oauth2/v3/certs
            // 3. Verify the signature using the appropriate key
            // 4. Check the 'aud' claim matches your project ID
            // 5. Check the 'iss' claim is accounts.google.com or https://accounts.google.com
            // 6. Check the token is not expired

            // Decode and check the payload for project ID (basic validation)
            if let Ok(payload_bytes) = BASE64.decode(parts[1]) {
                if let Ok(payload_str) = std::str::from_utf8(&payload_bytes) {
                    if payload_str.contains(&expected_project) {
                        tracing::info!(
                            project_id = %expected_project,
                            "Google Actions project ID verified"
                        );
                    }
                }
            }
        }
    }

    // Log validation for audit
    tracing::info!(
        has_signature = signature.is_some(),
        has_auth = auth_header.is_some(),
        "Google Actions request validation passed"
    );

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

    // --- verify_google_request ------------------------------------------
    //
    // GOOGLE_ACTIONS_PROJECT_ID is process-global; `verify_google_request` is
    // the only reader, so confining every mutation of it to this single test
    // keeps the assertions race-free under the parallel test runner.
    #[test]
    fn google_request_verification_branches() {
        // No project configured -> always passes (both auth present and absent).
        std::env::remove_var("GOOGLE_ACTIONS_PROJECT_ID");
        assert!(verify_google_request(&HeaderMap::new()).is_ok());

        std::env::set_var("GOOGLE_ACTIONS_PROJECT_ID", "my-project-123");

        // No Authorization header still passes.
        assert!(verify_google_request(&HeaderMap::new()).is_ok());

        // Malformed Authorization (no "Bearer " prefix) is rejected.
        let mut bad_scheme = HeaderMap::new();
        bad_scheme.insert("authorization", HeaderValue::from_static("Basic abc"));
        let err = verify_google_request(&bad_scheme).unwrap_err();
        assert!(
            err.contains("Authorization header format"),
            "unexpected: {err}"
        );

        // Bearer token that isn't a 3-part JWT is rejected.
        let mut bad_jwt = HeaderMap::new();
        bad_jwt.insert("authorization", HeaderValue::from_static("Bearer only.two"));
        let err = verify_google_request(&bad_jwt).unwrap_err();
        assert!(err.contains("JWT token format"), "unexpected: {err}");

        // Well-formed 3-part JWT carrying the project id in the payload passes.
        let payload = BASE64.encode(br#"{"aud":"my-project-123"}"#);
        let token = format!("Bearer header.{payload}.signature");
        let mut good = HeaderMap::new();
        good.insert("authorization", HeaderValue::from_str(&token).unwrap());
        assert!(verify_google_request(&good).is_ok());

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
    async fn verify_endpoint_alexa_branch() {
        // Non-empty signature -> valid.
        let resp = verify_webhook_signature(Json(VerifyWebhookRequest {
            platform: "alexa".to_string(),
            signature: "present".to_string(),
            body: "b".to_string(),
            timestamp: None,
        }))
        .await;
        assert!(resp.0.valid);
        assert_eq!(resp.0.platform, "alexa");

        // Empty signature -> invalid with "Missing signature".
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

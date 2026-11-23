//! Voice-platform webhook ingest handlers (Epic 93, Story 93.3).
//!
//! Split verbatim from `voice_webhooks.rs`: the Alexa Skills Kit and Google
//! Actions receivers plus the Alexa skill-validation health check. Both
//! platform handlers verify the provider's own request signature FIRST and are
//! deliberately **not** gated by an auth/RLS extractor (a legitimate voice
//! delivery carries no Property-Management bearer token) — see the per-handler
//! comments and the webhook security fixes (#2658, #2668) carried over here
//! unchanged. The router that mounts them lives in [`super`].

use super::parsers::{
    build_alexa_link_account_response, build_alexa_response, build_google_link_account_response,
    build_google_response, extract_alexa_command_text,
};
use super::signature::{verify_alexa_signature, verify_google_request};
use crate::services::voice_oauth::authenticate_voice_user;
use crate::services::VoiceCommandProcessor;
use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use common::errors::ErrorResponse;
use db::models::{
    voice_platform, AlexaOutputSpeech, AlexaRequestBody, AlexaResponseBody, AlexaSkillRequest,
    AlexaSkillResponse, GoogleActionsRequest, GoogleActionsResponse, VoiceActionResult,
    VoiceAssistantDevice,
};
use db::RlsPool;

// ============================================================================
// Shared RLS command-processing helper
// ============================================================================

/// Run an authenticated voice command under the device owner's RLS context,
/// releasing the pooled connection on **every** return path.
///
/// Command processing reads tenant-scoped tables (faults, announcements,
/// meters, …), so it runs under the *device owner's* org/user RLS context — the
/// tenant the linked `device` belongs to, resolved from the already-authenticated
/// device by the caller (never a context-cleared or caller-derived context).
///
/// The acquired [`db::RlsGuard`] is released with `release().await` before
/// returning on BOTH the success and the error branch, so the RLS context is
/// cleared deterministically instead of relying on the guard's best-effort
/// `Drop` cleanup (which cannot await and only logs a warning). This mirrors the
/// release-on-every-path convention used across the api-server handlers and
/// closes the inconsistency where an early `?`-return after `acquire_with_rls`
/// skipped the release, risking a leaked tenant context on the connection
/// handed back to the pool. Centralising the acquire → process → release
/// sequence here — it was previously open-coded three times across the two
/// webhook handlers, which is how a branch came to miss the release — gives
/// every caller a single, guaranteed release site.
async fn run_voice_command(
    rls_pool: &RlsPool,
    processor: &VoiceCommandProcessor,
    device: &VoiceAssistantDevice,
    command_text: &str,
    locale: &str,
) -> Result<VoiceActionResult, (StatusCode, Json<ErrorResponse>)> {
    let mut guard = rls_pool
        .acquire_with_rls(device.organization_id, device.user_id, false)
        .await
        .map_err(|e| {
            tracing::error!("Failed to acquire RLS connection: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;

    // NOTE (pre-fix state — intentionally reproduces the bug this commit's test
    // pins): the command outcome is mapped with `?`, so the error branch
    // early-returns and leaves `guard` to its best-effort `Drop` cleanup rather
    // than an explicit `release().await`. The regression test below asserts the
    // "RlsGuard dropped without calling release()" warning is NOT emitted, so it
    // FAILS here and passes once the release is added.
    let (result, _) = processor
        .process_command(guard.conn(), device.id, command_text, locale)
        .await
        .map_err(|e| {
            tracing::error!("Voice command processing failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("PROCESSING_ERROR", e.to_string())),
            )
        })?;

    Ok(result)
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
pub(crate) async fn alexa_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<AlexaSkillResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify the request signature FIRST (Story 93.3). An Alexa delivery is
    // authenticated by Amazon's certificate-chain signature over the *raw* body,
    // NOT by a Property-Management session — so this handler must not be gated by
    // an auth/RLS extractor.
    //
    // It previously took `mut rls: RlsConnection` as a handler argument. That is
    // an extractor: Axum runs it (via `ValidatedTenantExtractor` — JWT +
    // tenant-membership validation) BEFORE the handler body. A legitimate Alexa
    // request carries no PM bearer token, so the extractor rejected every real
    // delivery with 401 before the body was ever read and the signature was ever
    // checked. The database connection is now acquired inside the body, after the
    // signature check and after the device is authenticated from its OAuth token.
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

    // Extract access token for user authentication.
    // Account linking not complete -> return the link card (no DB access needed).
    let Some(token) = request.session.user.access_token.as_ref() else {
        return Ok(Json(build_alexa_link_account_response()));
    };

    // Resolve the linked voice device from the presented OAuth access token. The
    // `voice_assistant_devices` lookup is not RLS-bound (migration 00226), so it
    // runs on a context-cleared public connection — mirroring `oauth_token_refresh`.
    let rls_pool = RlsPool::new(state.db.clone());
    let mut lookup = rls_pool.acquire_public().await.map_err(|e| {
        tracing::error!("Failed to acquire database connection: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
        )
    })?;
    let device = authenticate_voice_user(lookup.conn(), token, voice_platform::ALEXA).await?;
    drop(lookup);

    // Process the request based on type.
    let locale = match &request.request {
        AlexaRequestBody::LaunchRequest { locale, .. } => locale.clone(),
        AlexaRequestBody::IntentRequest { locale, .. } => locale.clone(),
        AlexaRequestBody::SessionEndedRequest { locale, .. } => locale.clone(),
    };

    // Command processing reads tenant-scoped tables (faults, announcements,
    // meters, …), so it runs under the *device owner's* org/user RLS context —
    // the tenant the linked device belongs to, resolved from the authenticated
    // device above (never a context-cleared or caller-derived context).
    // `run_voice_command` owns the RLS acquire → process → release lifecycle and
    // releases the connection on every path.
    let processor = VoiceCommandProcessor::new(
        state.llm_document_repo.clone(),
        state.fault_repo.clone(),
        state.unit_repo.clone(),
        state.announcement_repo.clone(),
        state.meter_repo.clone(),
        state.org_repo.clone(),
    );

    let response = match &request.request {
        AlexaRequestBody::LaunchRequest { .. } => {
            // Welcome message
            let result = run_voice_command(&rls_pool, &processor, &device, "help", &locale).await?;
            build_alexa_response(&result)
        }
        AlexaRequestBody::IntentRequest { intent, .. } => {
            // Process the intent
            let command_text = extract_alexa_command_text(intent);
            let result =
                run_voice_command(&rls_pool, &processor, &device, &command_text, &locale).await?;
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

    Ok(Json(response))
}

/// Alexa health check endpoint for skill validation.
pub(crate) async fn alexa_health_check() -> StatusCode {
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
pub(crate) async fn google_actions_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<GoogleActionsRequest>,
) -> Result<Json<GoogleActionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify the request FIRST: Google-issued ID token (Bearer) with RS256
    // signature + aud/iss/exp validation. Fails closed on any verification error.
    //
    // As with the Alexa handler, a Google Actions delivery authenticates via its
    // Google-signed ID token, NOT a Property-Management session — so this handler
    // must not be gated by an auth/RLS extractor. It previously took `mut rls:
    // RlsConnection`, which Axum runs (JWT + tenant-membership validation) BEFORE
    // the handler body, so every real Google delivery (which carries no PM
    // bearer) was rejected with 401 before the body was read and this check ran.
    // The connection is now acquired inside the body, after verification.
    if let Err(e) = verify_google_request(&headers).await {
        tracing::warn!("Google Actions verification failed: {}", e);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_REQUEST", &e)),
        ));
    }

    // Extract access token from user params.
    // Account linking not complete -> return the link card (no DB access needed).
    let Some(token) = request
        .user
        .params
        .as_ref()
        .and_then(|p| p.get("bearerToken"))
        .and_then(|v| v.as_str())
    else {
        return Ok(Json(build_google_link_account_response(
            &request.session.id,
        )));
    };

    // Resolve the linked voice device from the presented OAuth access token on a
    // context-cleared public connection (`voice_assistant_devices` is not
    // RLS-bound; migration 00226), mirroring `oauth_token_refresh`.
    let rls_pool = RlsPool::new(state.db.clone());
    let mut lookup = rls_pool.acquire_public().await.map_err(|e| {
        tracing::error!("Failed to acquire database connection: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
        )
    })?;
    let device =
        authenticate_voice_user(lookup.conn(), token, voice_platform::GOOGLE_ASSISTANT).await?;
    drop(lookup);

    // Get locale from session
    let locale = request.session.language_code.as_deref().unwrap_or("en-US");

    // Extract command text from intent
    let command_text = request
        .intent
        .query
        .as_deref()
        .unwrap_or(&request.handler.name);

    // Process the command under the device owner's org/user RLS context — the
    // tenant the linked device belongs to, resolved from the authenticated device
    // above (command processing reads tenant-scoped tables). `run_voice_command`
    // releases the RLS connection on every path.
    let processor = VoiceCommandProcessor::new(
        state.llm_document_repo.clone(),
        state.fault_repo.clone(),
        state.unit_repo.clone(),
        state.announcement_repo.clone(),
        state.meter_repo.clone(),
        state.org_repo.clone(),
    );
    let result = run_voice_command(&rls_pool, &processor, &device, command_text, locale).await?;

    Ok(Json(build_google_response(&request.session.id, &result)))
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use db::repositories::{
        AnnouncementRepository, FaultRepository, LlmDocumentRepository, MeterRepository,
        OrganizationRepository, UnitRepository,
    };
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use uuid::Uuid;

    // --- alexa_health_check ---------------------------------------------

    #[tokio::test]
    async fn health_check_returns_ok() {
        assert_eq!(alexa_health_check().await, StatusCode::OK);
    }

    // --- run_voice_command RLS release ----------------------------------
    //
    // Regression for the inconsistent RLS release in the voice-webhook
    // handlers: after `acquire_with_rls`, an early `?`-return on the
    // command-processing error branch dropped the `RlsGuard` without releasing
    // it, so the connection went back to the pool via the guard's best-effort
    // `Drop` (which cannot await and only logs a warning) instead of a
    // deterministic `release().await`. `run_voice_command` now releases on
    // every path. These tests capture tracing output and assert the
    // "RlsGuard dropped without calling release()" warning — emitted
    // synchronously by `RlsGuard::drop` — is NOT produced, i.e. the guard was
    // explicitly released.

    /// A tracing layer that records every event's `message` field so a test can
    /// assert on what was (or was not) logged.
    #[derive(Clone, Default)]
    struct CaptureLayer {
        messages: Arc<Mutex<Vec<String>>>,
    }

    struct MessageVisitor<'a>(&'a mut String);
    impl Visit for MessageVisitor<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                use std::fmt::Write as _;
                let _ = write!(self.0, "{value:?}");
            }
        }
    }

    impl<S: tracing::Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut message = String::new();
            event.record(&mut MessageVisitor(&mut message));
            if !message.is_empty() {
                if let Ok(mut guard) = self.messages.lock() {
                    guard.push(message);
                }
            }
        }
    }

    fn processor_with_pool(pool: sqlx::PgPool) -> VoiceCommandProcessor {
        VoiceCommandProcessor::new(
            LlmDocumentRepository::new(pool.clone()),
            FaultRepository::new(pool.clone()),
            UnitRepository::new(pool.clone()),
            AnnouncementRepository::new(pool.clone()),
            MeterRepository::new(pool.clone()),
            OrganizationRepository::new(pool),
        )
    }

    fn unlinked_device() -> VoiceAssistantDevice {
        let now = chrono::Utc::now();
        VoiceAssistantDevice {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            unit_id: None,
            platform: "alexa".to_string(),
            device_id: "test-device".to_string(),
            device_name: None,
            linked_at: now,
            last_used_at: None,
            access_token_encrypted: None,
            refresh_token_encrypted: None,
            token_expires_at: None,
            is_active: true,
            capabilities: json!([]),
            access_token_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    // The command-processing error branch (device row not found) must still
    // release the RLS connection. No device is seeded, so `process_command`
    // returns `DeviceNotFound` and `run_voice_command` takes its error path —
    // the exact early-return branch that used to drop the guard un-released.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn run_voice_command_releases_rls_on_error_path(pool: sqlx::PgPool) {
        let processor = processor_with_pool(pool.clone());
        let rls_pool = RlsPool::new(pool);
        let device = unlinked_device();

        let capture = CaptureLayer::default();
        let messages = capture.messages.clone();

        let result = {
            let _default =
                tracing::subscriber::set_default(tracing_subscriber::registry().with(capture));
            run_voice_command(&rls_pool, &processor, &device, "help", "en-US").await
        };

        assert!(
            result.is_err(),
            "an unseeded device must fail command processing (DeviceNotFound)"
        );

        let logs = messages.lock().expect("capture lock");
        assert!(
            !logs.iter().any(|m| m.contains("without calling release")),
            "the error branch must release the RlsGuard — found a Drop-cleanup \
             warning, meaning release() was skipped. Captured logs: {logs:?}"
        );
    }
}

//! Portal webhook routes (Epic 105: Portal Syndication).
//!
//! UC-32.9: Handle Portal Webhooks
//!
//! Story 105.4: Portal Webhook Receivers
//! (delivers the portal-webhook scope of Story 83.3: Real Estate Portal
//! Webhooks — these receivers are the single implementation of both story
//! ids; the parser/connection helpers live in `integrations::portals`.)
//! - Receive webhooks from external real estate portals
//! - Handle view counts, inquiries, and status updates
//! - HMAC signature verification for webhook security

use crate::state::AppState;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use common::errors::ErrorResponse;
use db::models::{listing_portal, webhook_event_type, PortalInquiryWebhook, PortalViewWebhook};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use utoipa::ToSchema;

type HmacSha256 = Hmac<Sha256>;

/// Header carrying the signed unix-seconds timestamp for replay protection.
const TIMESTAMP_HEADER: &str = "X-Webhook-Timestamp";

/// Maximum accepted skew (seconds) between a per-portal webhook's signed
/// timestamp and the receiver's clock. Mirrors the connection-scoped portal
/// receiver and the Stripe receiver (both ±300s).
const PORTAL_WEBHOOK_TOLERANCE_SECS: i64 = 300;

// ============================================================================
// Router
// ============================================================================

/// Portal webhooks router for receiving events from external portals.
pub fn router() -> Router<AppState> {
    Router::new()
        // Reality Portal webhooks
        .route("/reality-portal/views", post(reality_portal_views_webhook))
        .route(
            "/reality-portal/inquiries",
            post(reality_portal_inquiry_webhook),
        )
        // SReality webhooks
        .route("/sreality/views", post(sreality_views_webhook))
        .route("/sreality/inquiries", post(sreality_inquiry_webhook))
        // Bezrealitky webhooks
        .route("/bezrealitky/views", post(bezrealitky_views_webhook))
        .route("/bezrealitky/inquiries", post(bezrealitky_inquiry_webhook))
        // Nehnutelnosti.sk webhooks
        .route("/nehnutelnosti/views", post(nehnutelnosti_views_webhook))
        .route(
            "/nehnutelnosti/inquiries",
            post(nehnutelnosti_inquiry_webhook),
        )
        // Generic webhook endpoint (portal specified in path)
        .route("/{portal}/events", post(generic_portal_webhook))
}

// ============================================================================
// Webhook Response Types
// ============================================================================

/// Webhook acknowledgment response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WebhookAckResponse {
    /// Whether the webhook was processed successfully
    pub success: bool,
    /// Optional message
    pub message: Option<String>,
    /// Timestamp of acknowledgment
    pub acknowledged_at: chrono::DateTime<Utc>,
}

/// Generic portal webhook event.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenericPortalEvent {
    /// Event type
    pub event_type: String,
    /// External listing ID
    pub external_id: String,
    /// Event payload
    pub payload: serde_json::Value,
    /// Timestamp of the event
    pub timestamp: Option<chrono::DateTime<Utc>>,
}

// ============================================================================
// Signature Verification
// ============================================================================

/// Verify webhook signature using HMAC-SHA256.
fn verify_webhook_signature(
    headers: &HeaderMap,
    body: &Bytes,
    secret: &str,
    signature_header: &str,
) -> Result<(), String> {
    // Get signature from headers
    let signature = headers
        .get(signature_header)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "Missing signature header".to_string())?;

    // Decode expected signature
    let expected = BASE64
        .decode(signature.trim_start_matches("sha256="))
        .map_err(|e| format!("Invalid signature format: {}", e))?;

    // Calculate actual signature
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("Invalid secret key: {}", e))?;
    mac.update(body);

    // Verify
    mac.verify_slice(&expected)
        .map_err(|_| "Signature verification failed".to_string())?;

    Ok(())
}

/// Get webhook secret for a portal from environment.
///
/// SECURITY (#2263): a present-but-empty env var must read as "unconfigured",
/// not as an empty signing key. Deploy tooling forwards these vars as `""`
/// when unset (compose/blue-green passthrough from #2259), and an HMAC keyed
/// with the empty string is computable by anyone — treating it as configured
/// would flip the receiver from fail-closed to forgeable.
fn get_portal_webhook_secret(portal: &str) -> Option<String> {
    let env_key = format!("{}_WEBHOOK_SECRET", portal.to_uppercase().replace('-', "_"));
    std::env::var(&env_key).ok().filter(|s| !s.is_empty())
}

/// Verify a portal webhook, failing **closed**.
///
/// Security (#833): the webhook receivers used to skip signature verification
/// whenever the `<PORTAL>_WEBHOOK_SECRET` env var was unset — accepting any
/// unverified, attacker-forged payload. This helper inverts that contract:
///
/// - If the secret is **not** configured, the webhook is **rejected** with
///   `401 WEBHOOK_NOT_CONFIGURED` and a clear error is logged. We never accept
///   an unverified webhook.
/// - If the secret **is** configured, the HMAC-SHA256 signature in
///   `signature_header` must be present and valid (constant-time compare via
///   `Mac::verify_slice`); otherwise the webhook is rejected with
///   `401 INVALID_SIGNATURE`.
fn require_portal_webhook_verification(
    portal: &str,
    headers: &HeaderMap,
    body: &Bytes,
    signature_header: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let Some(secret) = get_portal_webhook_secret(portal) else {
        tracing::error!(
            portal = %portal,
            "Rejecting webhook: no {}_WEBHOOK_SECRET configured — refusing to accept unverified webhook",
            portal.to_uppercase().replace('-', "_"),
        );
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "WEBHOOK_NOT_CONFIGURED",
                "Webhook signature verification is not configured for this portal",
            )),
        ));
    };

    // Replay protection (issue #2330, gap 83-3): these per-portal receivers
    // PERSIST state (views/inquiries), so an unguarded body-only HMAC lets a
    // captured valid delivery be replayed indefinitely to inflate view counts
    // or duplicate inquiries. When the sender includes an `X-Webhook-Timestamp`,
    // enforce the same signed-timestamp freshness window as the connection-
    // scoped and Stripe receivers: the timestamp is folded into the signed
    // payload (`"{timestamp}.{body}"`) so it cannot be swapped independently of
    // the signature, and deliveries outside ±PORTAL_WEBHOOK_TOLERANCE_SECS are
    // rejected 401.
    //
    // Staged rollout (accept-both -> enforce): external portals currently sign
    // the bare body with no timestamp. Until those senders are migrated we still
    // accept a legacy body-only signature when no timestamp header is present,
    // logging a deprecation warning. Once every portal signs "{timestamp}.{body}"
    // this legacy branch should be removed so replay protection is mandatory.
    if let Some(timestamp_raw) = headers.get(TIMESTAMP_HEADER).and_then(|v| v.to_str().ok()) {
        return verify_timestamped_portal_webhook(
            portal,
            &secret,
            headers,
            body,
            signature_header,
            timestamp_raw,
            Utc::now().timestamp(),
            PORTAL_WEBHOOK_TOLERANCE_SECS,
        );
    }

    if let Err(e) = verify_webhook_signature(headers, body, &secret, signature_header) {
        tracing::warn!(portal = %portal, error = %e, "Webhook signature verification failed");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
        ));
    }

    tracing::warn!(
        portal = %portal,
        "Portal webhook accepted with a legacy body-only signature and no {TIMESTAMP_HEADER} — \
         replay protection is NOT in effect for this delivery. Migrate this sender to sign \
         \"{{timestamp}}.{{body}}\" and send {TIMESTAMP_HEADER} (issue #2330)."
    );

    Ok(())
}

/// Verify a per-portal webhook that carries an `X-Webhook-Timestamp`, enforcing
/// both authenticity **and** freshness (issue #2330).
///
/// The signature covers the signed payload `"{timestamp}.{body}"` (base64
/// HMAC-SHA256, matching the encoding the body-only per-portal scheme already
/// uses) so the timestamp cannot be swapped without invalidating the signature,
/// and the timestamp must be within `tolerance_secs` of `now_unix`. Every
/// rejection maps to an opaque `401 INVALID_SIGNATURE` so the reason is not
/// leaked to the caller (the discriminant is logged instead). `now_unix` is
/// injected so the window is unit-testable without wall-clock dependence.
#[allow(clippy::too_many_arguments)]
fn verify_timestamped_portal_webhook(
    portal: &str,
    secret: &str,
    headers: &HeaderMap,
    body: &Bytes,
    signature_header: &str,
    timestamp_raw: &str,
    now_unix: i64,
    tolerance_secs: i64,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    fn reject() -> (StatusCode, Json<ErrorResponse>) {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature or freshness verification failed",
            )),
        )
    }

    let Ok(timestamp) = timestamp_raw.trim().parse::<i64>() else {
        tracing::warn!(portal = %portal, "Portal webhook rejected: malformed {TIMESTAMP_HEADER}");
        return Err(reject());
    };

    // Freshness — overflow-proof shared helper (rejects adversarial i64::MIN/MAX).
    if !integrations::portals::timestamp_within_tolerance(now_unix, timestamp, tolerance_secs) {
        tracing::warn!(
            portal = %portal,
            "Portal webhook rejected: timestamp outside tolerance window (replay defense)"
        );
        return Err(reject());
    }

    let Some(signature) = headers.get(signature_header).and_then(|v| v.to_str().ok()) else {
        tracing::warn!(portal = %portal, "Portal webhook rejected: missing signature header");
        return Err(reject());
    };

    let Ok(expected) = BASE64.decode(signature.trim_start_matches("sha256=")) else {
        tracing::warn!(portal = %portal, "Portal webhook rejected: signature is not valid base64");
        return Err(reject());
    };

    // signed_payload = "{timestamp}.{raw_body}"
    let mut signed = Vec::with_capacity(body.len() + 20);
    signed.extend_from_slice(timestamp.to_string().as_bytes());
    signed.push(b'.');
    signed.extend_from_slice(body);

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return Err(reject());
    };
    mac.update(&signed);
    // `verify_slice` is constant-time.
    if mac.verify_slice(&expected).is_err() {
        tracing::warn!(
            portal = %portal,
            "Portal webhook rejected: signature mismatch over \"{{timestamp}}.{{body}}\""
        );
        return Err(reject());
    }

    Ok(())
}

// ============================================================================
// Reality Portal Webhooks
// ============================================================================

/// Handle views webhook from Reality Portal.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/reality-portal/views",
    request_body = PortalViewWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn reality_portal_views_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification("reality_portal", &headers, &body, "X-Webhook-Signature")?;

    // Parse the request
    let webhook: PortalViewWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    // Process the view event
    process_view_webhook(&state, listing_portal::REALITY_PORTAL, &webhook).await
}

/// Handle inquiry webhook from Reality Portal.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/reality-portal/inquiries",
    request_body = PortalInquiryWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn reality_portal_inquiry_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification("reality_portal", &headers, &body, "X-Webhook-Signature")?;

    // Parse the request
    let webhook: PortalInquiryWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    // Process the inquiry event
    process_inquiry_webhook(&state, listing_portal::REALITY_PORTAL, &webhook).await
}

// ============================================================================
// SReality Webhooks
// ============================================================================

/// Handle views webhook from SReality.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/sreality/views",
    request_body = PortalViewWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn sreality_views_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification("sreality", &headers, &body, "X-SReality-Signature")?;

    let webhook: PortalViewWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    process_view_webhook(&state, listing_portal::SREALITY, &webhook).await
}

/// Handle inquiry webhook from SReality.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/sreality/inquiries",
    request_body = PortalInquiryWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn sreality_inquiry_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification("sreality", &headers, &body, "X-SReality-Signature")?;

    let webhook: PortalInquiryWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    process_inquiry_webhook(&state, listing_portal::SREALITY, &webhook).await
}

// ============================================================================
// Bezrealitky Webhooks
// ============================================================================

/// Handle views webhook from Bezrealitky.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/bezrealitky/views",
    request_body = PortalViewWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn bezrealitky_views_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification("bezrealitky", &headers, &body, "X-BR-Signature")?;

    let webhook: PortalViewWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    process_view_webhook(&state, listing_portal::BEZREALITKY, &webhook).await
}

/// Handle inquiry webhook from Bezrealitky.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/bezrealitky/inquiries",
    request_body = PortalInquiryWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn bezrealitky_inquiry_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification("bezrealitky", &headers, &body, "X-BR-Signature")?;

    let webhook: PortalInquiryWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    process_inquiry_webhook(&state, listing_portal::BEZREALITKY, &webhook).await
}

// ============================================================================
// Nehnutelnosti.sk Webhooks
// ============================================================================

/// Handle views webhook from Nehnutelnosti.sk.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/nehnutelnosti/views",
    request_body = PortalViewWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn nehnutelnosti_views_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification(
        "nehnutelnosti",
        &headers,
        &body,
        "X-Nehnutelnosti-Signature",
    )?;

    let webhook: PortalViewWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    process_view_webhook(&state, listing_portal::NEHNUTELNOSTI, &webhook).await
}

/// Handle inquiry webhook from Nehnutelnosti.sk.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/nehnutelnosti/inquiries",
    request_body = PortalInquiryWebhook,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
    ),
    tag = "Portal Webhooks"
)]
async fn nehnutelnosti_inquiry_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification(
        "nehnutelnosti",
        &headers,
        &body,
        "X-Nehnutelnosti-Signature",
    )?;

    let webhook: PortalInquiryWebhook = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    process_inquiry_webhook(&state, listing_portal::NEHNUTELNOSTI, &webhook).await
}

// ============================================================================
// Generic Portal Webhook
// ============================================================================

/// Handle generic webhook from any portal.
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/portals/{portal}/events",
    params(("portal" = String, Path, description = "Portal name")),
    request_body = GenericPortalEvent,
    responses(
        (status = 200, description = "Webhook processed", body = WebhookAckResponse),
        (status = 401, description = "Invalid signature"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Unknown portal"),
    ),
    tag = "Portal Webhooks"
)]
async fn generic_portal_webhook(
    State(state): State<AppState>,
    Path(portal): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate portal
    let valid_portals = [
        listing_portal::REALITY_PORTAL,
        listing_portal::SREALITY,
        listing_portal::BEZREALITKY,
        listing_portal::NEHNUTELNOSTI,
    ];

    if !valid_portals.contains(&portal.as_str()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "UNKNOWN_PORTAL",
                format!("Unknown portal: {}", portal),
            )),
        ));
    }

    // Verify signature — fails closed if no secret is configured (#833).
    require_portal_webhook_verification(&portal, &headers, &body, "X-Webhook-Signature")?;

    // Parse the generic event
    let event: GenericPortalEvent = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_REQUEST", e.to_string())),
        )
    })?;

    // Find the syndication by external ID
    let syndication = state
        .listing_repo
        .find_syndication_by_external_id(&portal, &event.external_id)
        .await
        .map_err(|e| {
            tracing::error!(portal = %portal, error = %e, "Database error looking up syndication");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to look up syndication",
                )),
            )
        })?;

    let syndication = match syndication {
        Some(s) => s,
        None => {
            tracing::warn!(
                portal = %portal,
                external_id = %event.external_id,
                "Syndication not found for external ID"
            );
            return Ok(Json(WebhookAckResponse {
                success: false,
                message: Some("Syndication not found".to_string()),
                acknowledged_at: Utc::now(),
            }));
        }
    };

    // Record the event
    let _ = state
        .listing_repo
        .record_webhook_event(
            syndication.listing_id,
            syndication.id,
            &portal,
            &event.event_type,
            Some(&event.external_id),
            event.payload,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                portal = %portal,
                external_id = %event.external_id,
                error = %e,
                "Failed to record webhook event"
            );
        });

    tracing::info!(
        portal = %portal,
        event_type = %event.event_type,
        external_id = %event.external_id,
        listing_id = %syndication.listing_id,
        "Processed generic portal webhook"
    );

    Ok(Json(WebhookAckResponse {
        success: true,
        message: None,
        acknowledged_at: Utc::now(),
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Process a view webhook event.
async fn process_view_webhook(
    state: &AppState,
    portal: &str,
    webhook: &PortalViewWebhook,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Find the syndication by external ID
    let syndication = state
        .listing_repo
        .find_syndication_by_external_id(portal, &webhook.external_id)
        .await
        .map_err(|e| {
            tracing::error!(portal = %portal, error = %e, "Database error looking up syndication");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to look up syndication",
                )),
            )
        })?;

    let syndication = match syndication {
        Some(s) => s,
        None => {
            tracing::warn!(
                portal = %portal,
                external_id = %webhook.external_id,
                "Syndication not found for external ID"
            );
            return Ok(Json(WebhookAckResponse {
                success: false,
                message: Some("Syndication not found".to_string()),
                acknowledged_at: Utc::now(),
            }));
        }
    };

    // Record the view event
    let _ = state
        .listing_repo
        .record_webhook_event(
            syndication.listing_id,
            syndication.id,
            portal,
            webhook_event_type::VIEW,
            Some(&webhook.external_id),
            serde_json::json!({
                "views_count": webhook.views_count,
                "timestamp": webhook.timestamp,
            }),
        )
        .await
        .map_err(|e| {
            tracing::error!(
                portal = %portal,
                external_id = %webhook.external_id,
                error = %e,
                "Failed to record view event"
            );
        });

    // Update stats
    let _ = state
        .listing_repo
        .increment_syndication_stats(syndication.id, webhook.views_count, 0)
        .await;

    tracing::info!(
        portal = %portal,
        external_id = %webhook.external_id,
        listing_id = %syndication.listing_id,
        views_count = webhook.views_count,
        "Processed view webhook"
    );

    Ok(Json(WebhookAckResponse {
        success: true,
        message: None,
        acknowledged_at: Utc::now(),
    }))
}

/// Process an inquiry webhook event.
async fn process_inquiry_webhook(
    state: &AppState,
    portal: &str,
    webhook: &PortalInquiryWebhook,
) -> Result<Json<WebhookAckResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Find the syndication by external ID
    let syndication = state
        .listing_repo
        .find_syndication_by_external_id(portal, &webhook.external_id)
        .await
        .map_err(|e| {
            tracing::error!(portal = %portal, error = %e, "Database error looking up syndication");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to look up syndication",
                )),
            )
        })?;

    let syndication = match syndication {
        Some(s) => s,
        None => {
            tracing::warn!(
                portal = %portal,
                external_id = %webhook.external_id,
                "Syndication not found for external ID"
            );
            return Ok(Json(WebhookAckResponse {
                success: false,
                message: Some("Syndication not found".to_string()),
                acknowledged_at: Utc::now(),
            }));
        }
    };

    // Record the inquiry event.
    //
    // UNLIKE the view / generic paths (analytics, genuinely best-effort), an
    // inquiry carries a *lead* — sender name / email / phone / message. If the
    // write fails and we still ack the portal with `success: true` / HTTP 200,
    // the lead is silently lost: the portal treats the delivery as accepted and
    // never retries, and there is no record anywhere (#2275). So on this path we
    // must propagate a genuine persistence failure to the caller as 500 so the
    // portal retries. A duplicate delivery (unique-constraint violation) is
    // instead an idempotent success — we ack 200 so retries of an
    // already-recorded lead don't 500 forever.
    let record_result = state
        .listing_repo
        .record_webhook_event(
            syndication.listing_id,
            syndication.id,
            portal,
            webhook_event_type::INQUIRY,
            Some(&webhook.external_id),
            serde_json::json!({
                "sender_name": webhook.sender_name,
                "sender_email": webhook.sender_email,
                "sender_phone": webhook.sender_phone,
                "message": webhook.message,
                "timestamp": webhook.timestamp,
            }),
        )
        .await;

    match classify_inquiry_persist(record_result.as_ref().map(|_| ())) {
        InquiryPersistOutcome::Recorded => {}
        InquiryPersistOutcome::Duplicate => {
            // Already recorded on an earlier delivery — ack idempotently so the
            // portal stops retrying, but do not double-count stats.
            tracing::info!(
                portal = %portal,
                external_id = %webhook.external_id,
                listing_id = %syndication.listing_id,
                "Inquiry webhook already recorded (duplicate delivery) — acking idempotently"
            );
            return Ok(Json(WebhookAckResponse {
                success: true,
                message: Some("Inquiry already recorded".to_string()),
                acknowledged_at: Utc::now(),
            }));
        }
        InquiryPersistOutcome::Retry => {
            // Genuine write failure — the lead is NOT persisted. Return 5xx so
            // the portal retries instead of silently dropping the lead (#2275).
            let err = record_result.expect_err("Retry outcome implies an Err result");
            tracing::error!(
                portal = %portal,
                external_id = %webhook.external_id,
                error = %err,
                "Failed to persist inbound inquiry lead — returning 500 so the portal retries"
            );
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LEAD_PERSISTENCE_FAILED",
                    "Failed to persist inquiry lead; please retry",
                )),
            ));
        }
    }

    // Update stats
    let _ = state
        .listing_repo
        .increment_syndication_stats(syndication.id, 0, 1)
        .await;

    tracing::info!(
        portal = %portal,
        external_id = %webhook.external_id,
        listing_id = %syndication.listing_id,
        sender_email = ?webhook.sender_email,
        "Processed inquiry webhook"
    );

    Ok(Json(WebhookAckResponse {
        success: true,
        message: None,
        acknowledged_at: Utc::now(),
    }))
}

/// Classification of a `record_webhook_event` result on the inquiry-lead path.
///
/// The inquiry path carries a lead that must not be silently dropped (#2275),
/// so unlike the analytics paths it distinguishes three outcomes rather than
/// swallowing every error.
#[derive(Debug, PartialEq, Eq)]
enum InquiryPersistOutcome {
    /// The lead was written — ack the portal with 200.
    Recorded,
    /// A duplicate delivery of an already-recorded lead (unique violation).
    /// Idempotent success — ack 200 so the portal stops retrying.
    Duplicate,
    /// A genuine write failure — the lead is NOT persisted. Return 5xx so the
    /// portal retries instead of dropping the lead.
    Retry,
}

/// Whether a SQLx error is a Postgres unique-constraint violation
/// (SQLSTATE `23505`). Used to treat a duplicate inquiry delivery as an
/// idempotent success rather than a retriable write failure.
fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(
        err,
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505")
    )
}

/// Classify the result of persisting an inbound inquiry lead.
///
/// - `Ok(())` -> [`InquiryPersistOutcome::Recorded`] (ack 200)
/// - unique violation -> [`InquiryPersistOutcome::Duplicate`] (idempotent ack 200)
/// - any other error -> [`InquiryPersistOutcome::Retry`] (return 500, portal retries)
fn classify_inquiry_persist(result: Result<(), &sqlx::Error>) -> InquiryPersistOutcome {
    match result {
        Ok(()) => InquiryPersistOutcome::Recorded,
        Err(e) if is_unique_violation(e) => InquiryPersistOutcome::Duplicate,
        Err(_) => InquiryPersistOutcome::Retry,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    //! Regression for #833: portal webhook signature verification must fail
    //! **closed**.
    //!
    //! On `main`/`dev` the receivers skipped verification entirely whenever the
    //! `<PORTAL>_WEBHOOK_SECRET` env var was unset, accepting any forged
    //! payload. These tests pin the inverted contract directly at the security
    //! gate (`require_portal_webhook_verification`), which runs before body
    //! parsing and any DB access — so no pool/AppState is needed.

    use super::*;

    // `std::env::{set_var, remove_var}` are not safe to interleave with
    // `getenv` on glibc, and the unit tests in this binary run concurrently.
    // Serialize every test that touches the env behind one mutex.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const SIG_HEADER: &str = "X-Webhook-Signature";

    /// Compute the `sha256=<base64>` signature header value the verifier expects.
    fn sign(secret: &str, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        format!("sha256={}", BASE64.encode(mac.finalize().into_bytes()))
    }

    fn env_key(portal: &str) -> String {
        format!("{}_WEBHOOK_SECRET", portal.to_uppercase().replace('-', "_"))
    }

    /// Regression for #2263: a present-but-EMPTY `<PORTAL>_WEBHOOK_SECRET`
    /// (what compose/blue-green passthrough injects when the operator leaves
    /// the var unset) must read as "unconfigured" and reject — even a payload
    /// carrying a signature correctly HMAC'd with the empty key, which any
    /// attacker can compute.
    #[test]
    fn rejects_empty_secret_even_with_empty_key_signature() {
        let _guard = ENV_LOCK.lock().unwrap();
        let portal = "reality_portal";
        std::env::set_var(env_key(portal), "");

        let body = Bytes::from_static(b"{\"external_id\":\"forged\"}");
        let mut headers = HeaderMap::new();
        headers.insert(SIG_HEADER, sign("", &body).parse().unwrap());

        let res = require_portal_webhook_verification(portal, &headers, &body, SIG_HEADER);
        std::env::remove_var(env_key(portal));

        let (status, _) = res.expect_err("empty secret must read as unconfigured and REJECT");
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "empty secret must fail closed with 401, not verify against the empty key"
        );
    }

    #[test]
    fn rejects_when_no_secret_configured() {
        let _guard = ENV_LOCK.lock().unwrap();
        let portal = "reality_portal";
        std::env::remove_var(env_key(portal));

        let body = Bytes::from_static(b"{\"external_id\":\"x\"}");
        let headers = HeaderMap::new();

        let res = require_portal_webhook_verification(portal, &headers, &body, SIG_HEADER);

        let (status, _) = res.expect_err("unconfigured secret must REJECT, not accept");
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "missing secret must fail closed with 401"
        );
    }

    #[test]
    fn rejects_invalid_signature_when_secret_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        let portal = "sreality";
        std::env::set_var(env_key(portal), "super-secret-key");

        let body = Bytes::from_static(b"{\"external_id\":\"x\"}");
        let mut headers = HeaderMap::new();
        headers.insert(SIG_HEADER, "sha256=AAAA".parse().unwrap());

        let res = require_portal_webhook_verification(portal, &headers, &body, SIG_HEADER);
        std::env::remove_var(env_key(portal));

        let (status, _) = res.expect_err("invalid signature must REJECT");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn rejects_missing_signature_header_when_secret_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        let portal = "bezrealitky";
        std::env::set_var(env_key(portal), "super-secret-key");

        let body = Bytes::from_static(b"{\"external_id\":\"x\"}");
        let headers = HeaderMap::new(); // no signature header at all

        let res = require_portal_webhook_verification(portal, &headers, &body, SIG_HEADER);
        std::env::remove_var(env_key(portal));

        let (status, _) = res.expect_err("missing signature header must REJECT");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn accepts_valid_signature_when_secret_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        let portal = "nehnutelnosti";
        let secret = "super-secret-key";
        std::env::set_var(env_key(portal), secret);

        let body = Bytes::from_static(b"{\"external_id\":\"x\",\"views_count\":3}");
        let mut headers = HeaderMap::new();
        headers.insert(SIG_HEADER, sign(secret, &body).parse().unwrap());

        let res = require_portal_webhook_verification(portal, &headers, &body, SIG_HEADER);
        std::env::remove_var(env_key(portal));

        assert!(res.is_ok(), "a correctly-signed webhook must be accepted");
    }

    // ------------------------------------------------------------------
    // Regression for #2275: the inbound INQUIRY path must not silently drop
    // a lead when the DB write fails. A genuine persistence failure must map
    // to a retry (-> 500), NOT an idempotent ack (-> 200).
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // Replay protection for the per-portal receivers (issue #2330, gap 83-3).
    // These pin `verify_timestamped_portal_webhook` directly (now_unix is
    // injected) so the freshness window is deterministic and wall-clock-free.
    // ------------------------------------------------------------------

    const TS_SECRET: &str = "portal-ts-secret";
    const TS_BODY: &[u8] = br#"{"external_id":"abc","views_count":3}"#;
    const NOW: i64 = 1_700_000_000;
    const TS_SIG_HEADER: &str = "X-Webhook-Signature";

    /// Sign `"{ts}.{body}"` as base64 HMAC-SHA256 — the timestamped per-portal
    /// contract this receiver enforces.
    fn sign_ts(secret: &str, ts: i64, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{ts}.").as_bytes());
        mac.update(body);
        format!("sha256={}", BASE64.encode(mac.finalize().into_bytes()))
    }

    fn ts_headers(sig: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(TS_SIG_HEADER, sig.parse().unwrap());
        headers
    }

    fn verify_ts(ts: i64, sig: &str, now: i64) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
        verify_timestamped_portal_webhook(
            "reality_portal",
            TS_SECRET,
            &ts_headers(sig),
            &Bytes::from_static(TS_BODY),
            TS_SIG_HEADER,
            &ts.to_string(),
            now,
            PORTAL_WEBHOOK_TOLERANCE_SECS,
        )
    }

    #[test]
    fn timestamped_accepts_fresh_valid_signature() {
        let sig = sign_ts(TS_SECRET, NOW, TS_BODY);
        assert!(verify_ts(NOW, &sig, NOW).is_ok());
    }

    #[test]
    fn timestamped_accepts_exact_tolerance_boundary() {
        let ts = NOW - PORTAL_WEBHOOK_TOLERANCE_SECS;
        let sig = sign_ts(TS_SECRET, ts, TS_BODY);
        assert!(
            verify_ts(ts, &sig, NOW).is_ok(),
            "boundary must be inclusive"
        );
    }

    #[test]
    fn timestamped_rejects_stale_delivery() {
        // Correctly signed but too old — the replay case the per-portal
        // receivers previously accepted indefinitely.
        let ts = NOW - PORTAL_WEBHOOK_TOLERANCE_SECS - 1;
        let sig = sign_ts(TS_SECRET, ts, TS_BODY);
        let (status, _) = verify_ts(ts, &sig, NOW).expect_err("stale delivery must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn timestamped_rejects_timestamp_swap_replay() {
        // Attacker captures a valid (old_ts, sig) pair and presents a fresh
        // timestamp header to beat the window while reusing the old signature.
        // Because the timestamp is folded into the HMAC, the swap fails.
        let old_ts = NOW - 10_000;
        let sig = sign_ts(TS_SECRET, old_ts, TS_BODY);
        let (status, _) =
            verify_ts(NOW, &sig, NOW).expect_err("timestamp-swap replay must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn timestamped_rejects_wrong_signature() {
        let (status, _) =
            verify_ts(NOW, "sha256=AAAA", NOW).expect_err("bad signature must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn timestamped_rejects_adversarial_overflow_timestamp() {
        // Regression for the overflow edge — must reject without panicking.
        let sig = sign_ts(TS_SECRET, i64::MIN, TS_BODY);
        let (status, _) =
            verify_ts(i64::MIN, &sig, NOW).expect_err("overflow timestamp must be rejected");
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn legacy_body_only_still_accepted_without_timestamp_header() {
        // Backward-compat (accept-both phase): a sender that has not yet
        // migrated — body-only base64 signature, no X-Webhook-Timestamp — is
        // still accepted so live portal integrations don't break.
        let _guard = ENV_LOCK.lock().unwrap();
        let portal = "reality_portal";
        std::env::set_var(env_key(portal), TS_SECRET);

        let body = Bytes::from_static(TS_BODY);
        let mut headers = HeaderMap::new();
        headers.insert(SIG_HEADER, sign(TS_SECRET, &body).parse().unwrap());

        let res = require_portal_webhook_verification(portal, &headers, &body, SIG_HEADER);
        std::env::remove_var(env_key(portal));

        assert!(
            res.is_ok(),
            "legacy body-only signature must still be accepted during the accept-both rollout"
        );
    }

    #[test]
    fn inquiry_persist_success_is_recorded() {
        assert_eq!(
            classify_inquiry_persist(Ok(())),
            InquiryPersistOutcome::Recorded,
            "a successful write must ack the portal"
        );
    }

    #[test]
    fn inquiry_persist_db_failure_triggers_retry_not_silent_ack() {
        // `PoolClosed` stands in for any genuine infrastructure/write failure:
        // the lead did NOT reach the DB. On `main`/`dev` this error was swallowed
        // (`let _ = ...`) and the handler still returned `success: true` / 200,
        // silently losing the lead. It must now classify as `Retry` so the
        // handler returns 5xx and the portal re-delivers.
        let err = sqlx::Error::PoolClosed;
        assert_eq!(
            classify_inquiry_persist(Err(&err)),
            InquiryPersistOutcome::Retry,
            "a genuine write failure must trigger a retry (5xx), never a silent 200 ack"
        );
    }

    #[test]
    fn non_database_error_is_not_a_unique_violation() {
        // Only a real unique-constraint violation may be treated as an
        // idempotent duplicate; any other error must fall through to `Retry`.
        assert!(!is_unique_violation(&sqlx::Error::PoolClosed));
        assert!(!is_unique_violation(&sqlx::Error::RowNotFound));
    }
}

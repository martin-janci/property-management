//! Portal webhook routes (Epic 105: Portal Syndication).
//!
//! Story 105.4: Portal Webhook Receivers
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
fn get_portal_webhook_secret(portal: &str) -> Option<String> {
    let env_key = format!("{}_WEBHOOK_SECRET", portal.to_uppercase().replace('-', "_"));
    std::env::var(&env_key).ok()
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
    // Verify signature if secret is configured
    if let Some(secret) = get_portal_webhook_secret("reality_portal") {
        if let Err(e) = verify_webhook_signature(&headers, &body, &secret, "X-Webhook-Signature") {
            tracing::warn!(portal = "reality_portal", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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
    // Verify signature if secret is configured
    if let Some(secret) = get_portal_webhook_secret("reality_portal") {
        if let Err(e) = verify_webhook_signature(&headers, &body, &secret, "X-Webhook-Signature") {
            tracing::warn!(portal = "reality_portal", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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
    if let Some(secret) = get_portal_webhook_secret("sreality") {
        if let Err(e) = verify_webhook_signature(&headers, &body, &secret, "X-SReality-Signature") {
            tracing::warn!(portal = "sreality", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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
    if let Some(secret) = get_portal_webhook_secret("sreality") {
        if let Err(e) = verify_webhook_signature(&headers, &body, &secret, "X-SReality-Signature") {
            tracing::warn!(portal = "sreality", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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
    if let Some(secret) = get_portal_webhook_secret("bezrealitky") {
        if let Err(e) = verify_webhook_signature(&headers, &body, &secret, "X-BR-Signature") {
            tracing::warn!(portal = "bezrealitky", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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
    if let Some(secret) = get_portal_webhook_secret("bezrealitky") {
        if let Err(e) = verify_webhook_signature(&headers, &body, &secret, "X-BR-Signature") {
            tracing::warn!(portal = "bezrealitky", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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
    if let Some(secret) = get_portal_webhook_secret("nehnutelnosti") {
        if let Err(e) =
            verify_webhook_signature(&headers, &body, &secret, "X-Nehnutelnosti-Signature")
        {
            tracing::warn!(portal = "nehnutelnosti", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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
    if let Some(secret) = get_portal_webhook_secret("nehnutelnosti") {
        if let Err(e) =
            verify_webhook_signature(&headers, &body, &secret, "X-Nehnutelnosti-Signature")
        {
            tracing::warn!(portal = "nehnutelnosti", error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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

    // Verify signature if secret is configured
    if let Some(secret) = get_portal_webhook_secret(&portal) {
        if let Err(e) = verify_webhook_signature(&headers, &body, &secret, "X-Webhook-Signature") {
            tracing::warn!(portal = %portal, error = %e, "Webhook signature verification failed");
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("INVALID_SIGNATURE", &e)),
            ));
        }
    }

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

    // Record the inquiry event
    let _ = state
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
        .await
        .map_err(|e| {
            tracing::error!(
                portal = %portal,
                external_id = %webhook.external_id,
                error = %e,
                "Failed to record inquiry event"
            );
        });

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

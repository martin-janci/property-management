//! Integration webhook-surface routes.
//!
//! Covers two concerns:
//! 1. **Outbound webhook management** — CRUD for webhook subscriptions,
//!    test delivery, and delivery logs (Story 61.5).
//! 2. **Inbound webhook receivers** — endpoints that external systems POST to:
//!    - E-signature providers (DocuSign, Adobe Sign, HelloSign)
//!    - Booking.com OTA push notifications
//!    - Portal/listing-site webhooks (public, no auth)

use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use db::models::{
    esignature_provider, CreateWebhookSubscription, TestWebhookRequest, TestWebhookResponse,
    UpdateWebhookSubscription, WebhookDeliveryLog, WebhookDeliveryQuery, WebhookStatistics,
    WebhookSubscription,
};
use hmac::{Hmac, KeyInit, Mac};
use integrations::BookingClient;
use serde::Deserialize;
use sha2::Sha256;
use uuid::Uuid;

use super::install::ConnectionIdPath;
use super::sync::{
    verify_adobe_sign_signature, verify_docusign_signature, verify_hellosign_signature,
    verify_org_access, OrgIdPath, ResourceIdPath,
};
use crate::state::AppState;
use common::errors::ErrorResponse;

// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

// ==================== Inbound Webhook Payload Types ====================

/// E-signature webhook payload with provider-specific fields.
#[derive(Debug, Deserialize)]
struct ESignatureWebhookPayload {
    provider: Option<String>,
    event_type: Option<String>,
    envelope_id: Option<String>,
    event_time: Option<String>,
    event_hash: Option<String>,
    #[serde(flatten)]
    data: serde_json::Value,
}

// ==================== Router ====================

/// Create webhook-surface router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Outbound webhook subscriptions (Story 61.5)
        .route(
            "/organizations/{org_id}/webhooks",
            get(list_webhook_subscriptions),
        )
        .route(
            "/organizations/{org_id}/webhooks",
            post(create_webhook_subscription),
        )
        .route("/webhooks/{id}", get(get_webhook_subscription))
        .route(
            "/webhooks/{id}",
            axum::routing::put(update_webhook_subscription),
        )
        .route(
            "/webhooks/{id}",
            axum::routing::delete(delete_webhook_subscription),
        )
        .route("/webhooks/{id}/test", post(test_webhook))
        .route("/webhooks/{id}/logs", get(list_webhook_logs))
        .route("/webhooks/{id}/stats", get(get_webhook_stats))
        // Inbound webhook receivers
        .route("/esignatures/webhook", post(esignature_webhook))
        .route("/booking/push", post(booking_push_notification))
        .route(
            "/webhooks/portal/{connection_id}",
            post(handle_portal_webhook),
        )
}

// ==================== Outbound Webhook Subscriptions (Story 61.5) ====================

/// List webhook subscriptions for an organization.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/organizations/{org_id}/webhooks",
    params(OrgIdPath),
    responses(
        (status = 200, description = "Subscriptions retrieved", body = Vec<WebhookSubscription>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_webhook_subscriptions(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<WebhookSubscription>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let subscriptions = state
        .integration_repo
        .list_webhook_subscriptions(path.org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list webhook subscriptions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list webhook subscriptions",
                )),
            )
        })?;

    Ok(Json(subscriptions))
}

/// Create a webhook subscription.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/organizations/{org_id}/webhooks",
    params(OrgIdPath),
    request_body = CreateWebhookSubscription,
    responses(
        (status = 201, description = "Subscription created", body = WebhookSubscription),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn create_webhook_subscription(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateWebhookSubscription>,
) -> Result<(StatusCode, Json<WebhookSubscription>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, path.org_id).await?;

    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let subscription = state
        .integration_repo
        .create_webhook_subscription(path.org_id, auth.user_id, data, is_production)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create webhook subscription",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(subscription)))
}

/// Get a webhook subscription by ID.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/webhooks/{id}",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Subscription retrieved", body = WebhookSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_webhook_subscription(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let subscription = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get webhook subscription",
                )),
            )
        })?;

    match subscription {
        Some(s) => {
            verify_org_access(&state, auth.user_id, s.organization_id).await?;
            Ok(Json(s))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Webhook subscription not found",
            )),
        )),
    }
}

/// Update a webhook subscription.
#[utoipa::path(
    put,
    path = "/api/v1/integrations/webhooks/{id}",
    params(ResourceIdPath),
    request_body = UpdateWebhookSubscription,
    responses(
        (status = 200, description = "Subscription updated", body = WebhookSubscription),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn update_webhook_subscription(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<UpdateWebhookSubscription>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let existing = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            )
        })?;

    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let subscription = state
        .integration_repo
        .update_webhook_subscription(path.id, data, is_production)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to update webhook subscription",
                )),
            )
        })?;

    Ok(Json(subscription))
}

/// Delete a webhook subscription.
#[utoipa::path(
    delete,
    path = "/api/v1/integrations/webhooks/{id}",
    params(ResourceIdPath),
    responses(
        (status = 204, description = "Subscription deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not a member of the organization"),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn delete_webhook_subscription(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let existing = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            )
        })?;

    verify_org_access(&state, auth.user_id, existing.organization_id).await?;

    let deleted = state
        .integration_repo
        .delete_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to delete webhook subscription",
                )),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Webhook subscription not found",
            )),
        ))
    }
}

/// Test a webhook subscription.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/webhooks/{id}/test",
    params(ResourceIdPath),
    request_body = TestWebhookRequest,
    responses(
        (status = 200, description = "Test completed", body = TestWebhookResponse),
        (status = 404, description = "Subscription not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn test_webhook(
    State(state): State<AppState>,
    auth: api_core::AuthUser,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<TestWebhookRequest>,
) -> Result<Json<TestWebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    let subscription = state
        .integration_repo
        .get_webhook_subscription(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook subscription");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get webhook subscription",
                )),
            )
        })?;

    let subscription = match subscription {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            ))
        }
    };

    verify_org_access(&state, auth.user_id, subscription.organization_id).await?;

    let test_payload = data.payload.unwrap_or_else(|| {
        serde_json::json!({
            "event": data.event_type,
            "test": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": {
                "message": "This is a test webhook delivery"
            }
        })
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("PropertyManagement-Webhook-Test/1.0")
        .pool_max_idle_per_host(0)
        .build()
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create HTTP client");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CLIENT_ERROR",
                    "Failed to create HTTP client",
                )),
            )
        })?;

    // SSRF gate: re-validate the stored URL as defence-in-depth
    if let Err(e) = common::url_validation::validate_external_url(&subscription.url) {
        tracing::warn!(
            subscription_id = %path.id,
            url = %subscription.url,
            error = %e,
            "SSRF validation rejected webhook subscription URL"
        );
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_WEBHOOK_URL",
                format!("Webhook URL rejected: {}", e),
            )),
        ));
    }

    let mut request = client.post(&subscription.url).json(&test_payload);

    const BLOCKED_HEADERS: &[&str] = &[
        "host",
        "authorization",
        "cookie",
        "x-forwarded-for",
        "x-real-ip",
        "x-forwarded-host",
        "x-forwarded-proto",
    ];

    if let Some(headers) = &subscription.headers {
        if let Some(headers_obj) = headers.as_object() {
            for (key, value) in headers_obj {
                if BLOCKED_HEADERS.contains(&key.to_lowercase().as_str()) {
                    tracing::warn!(header = %key, "Blocked webhook header injection attempt");
                    continue;
                }
                if let Some(value_str) = value.as_str() {
                    request = request.header(key, value_str);
                }
            }
        }
    }

    if let Some(secret) = &subscription.secret {
        let payload_str = serde_json::to_string(&test_payload).unwrap_or_default();
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
            tracing::error!(error = ?e, "Failed to create HMAC for webhook test signature");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "CRYPTO_ERROR",
                    "Failed to compute webhook signature",
                )),
            )
        })?;
        mac.update(payload_str.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        request = request.header("X-Webhook-Signature", format!("sha256={}", signature));
    }

    request = request
        .header("Content-Type", "application/json")
        .header("X-Webhook-Event", &data.event_type)
        .header("X-Webhook-Test", "true")
        .header("X-Webhook-ID", Uuid::new_v4().to_string());

    let start_time = std::time::Instant::now();
    let response_result = request.send().await;
    let response_time_ms = start_time.elapsed().as_millis() as i32;

    match response_result {
        Ok(response) => {
            let status_code = response.status().as_u16() as i32;
            let success = response.status().is_success();

            let error = if !success {
                let body = response.text().await.ok();
                body.map(|b| {
                    let sanitized = b.lines().take(5).collect::<Vec<_>>().join("\n");
                    if sanitized.len() > 500 {
                        format!("{}...", &sanitized[..500])
                    } else {
                        sanitized
                    }
                })
            } else {
                None
            };

            Ok(Json(TestWebhookResponse {
                success,
                status_code: Some(status_code),
                response_time_ms: Some(response_time_ms),
                error,
            }))
        }
        Err(e) => {
            tracing::warn!(error = ?e, "Webhook test request failed");

            let error_message = if e.is_timeout() {
                "Request timed out after 30 seconds".to_string()
            } else if e.is_connect() {
                "Failed to connect to webhook URL".to_string()
            } else {
                "Request failed while testing webhook".to_string()
            };

            Ok(Json(TestWebhookResponse {
                success: false,
                status_code: None,
                response_time_ms: Some(response_time_ms),
                error: Some(error_message),
            }))
        }
    }
}

/// List webhook delivery logs.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/webhooks/{id}/logs",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Logs retrieved", body = Vec<WebhookDeliveryLog>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn list_webhook_logs(
    State(state): State<AppState>,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<Vec<WebhookDeliveryLog>>, (StatusCode, Json<ErrorResponse>)> {
    let logs = state
        .integration_repo
        .list_webhook_delivery_logs(WebhookDeliveryQuery {
            subscription_id: Some(path.id),
            event_type: None,
            status: None,
            from_date: None,
            to_date: None,
            limit: Some(100),
            offset: None,
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list webhook logs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list webhook logs",
                )),
            )
        })?;

    Ok(Json(logs))
}

/// Get webhook statistics.
#[utoipa::path(
    get,
    path = "/api/v1/integrations/webhooks/{id}/stats",
    params(ResourceIdPath),
    responses(
        (status = 200, description = "Statistics retrieved", body = WebhookStatistics),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Integrations"
)]
pub async fn get_webhook_stats(
    State(state): State<AppState>,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<WebhookStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let stats = state
        .integration_repo
        .get_webhook_statistics(path.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get webhook statistics");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get webhook statistics",
                )),
            )
        })?;

    Ok(Json(stats))
}

// ==================== Inbound: E-Signature Webhook ====================

/// E-signature webhook endpoint.
///
/// Receives webhooks from DocuSign, Adobe Sign, and HelloSign.
/// Verifies the HMAC signature before processing.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/esignatures/webhook",
    request_body(content = String, description = "Provider-specific webhook payload"),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid payload"),
        (status = 401, description = "Invalid signature"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations"
)]
pub async fn esignature_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let payload: ESignatureWebhookPayload = serde_json::from_slice(&body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse e-signature webhook payload");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PAYLOAD",
                "Invalid webhook payload",
            )),
        )
    })?;

    let provider = payload.provider.as_deref().unwrap_or_else(|| {
        if headers.contains_key("x-docusign-signature-1") {
            esignature_provider::DOCUSIGN
        } else if headers.contains_key("x-adobesign-clientid") {
            esignature_provider::ADOBE_SIGN
        } else if payload.event_hash.is_some() {
            esignature_provider::HELLOSIGN
        } else {
            "unknown"
        }
    });

    match provider {
        esignature_provider::DOCUSIGN => {
            let secret =
                std::env::var("DOCUSIGN_WEBHOOK_SECRET").unwrap_or_else(|_| String::new());
            if secret.is_empty() {
                tracing::warn!("DOCUSIGN_WEBHOOK_SECRET not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            let signature = headers
                .get("x-docusign-signature-1")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !verify_docusign_signature(&secret, &body, signature) {
                tracing::warn!("Invalid DocuSign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        esignature_provider::ADOBE_SIGN => {
            let client_secret =
                std::env::var("ADOBE_SIGN_CLIENT_SECRET").unwrap_or_else(|_| String::new());
            if client_secret.is_empty() {
                tracing::warn!("ADOBE_SIGN_CLIENT_SECRET not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            let signature = headers
                .get("x-adobesign-signature")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if !verify_adobe_sign_signature(&client_secret, &body, signature) {
                tracing::warn!("Invalid Adobe Sign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        esignature_provider::HELLOSIGN => {
            let api_key = std::env::var("HELLOSIGN_API_KEY").unwrap_or_else(|_| String::new());
            if api_key.is_empty() {
                tracing::warn!("HELLOSIGN_API_KEY not configured");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "CONFIG_ERROR",
                        "Webhook verification not configured",
                    )),
                ));
            }

            let event_time = payload.event_time.as_deref().unwrap_or("");
            let event_type = payload.event_type.as_deref().unwrap_or("");
            let event_hash = payload.event_hash.as_deref().unwrap_or("");

            if !verify_hellosign_signature(&api_key, event_time, event_type, event_hash) {
                tracing::warn!("Invalid HelloSign webhook signature");
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse::new(
                        "INVALID_SIGNATURE",
                        "Invalid webhook signature",
                    )),
                ));
            }
        }
        _ => {
            tracing::warn!(provider = %provider, "Unknown e-signature provider");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "UNKNOWN_PROVIDER",
                    "Unknown e-signature provider",
                )),
            ));
        }
    }

    tracing::info!(
        provider = %provider,
        event_type = ?payload.event_type,
        envelope_id = ?payload.envelope_id,
        "Processing verified e-signature webhook"
    );

    if let (Some(envelope_id), Some(event_type)) = (
        payload.envelope_id.as_deref(),
        payload.event_type.as_deref(),
    ) {
        let new_status = match event_type {
            "envelope-completed" | "agreement_all_signed" | "signature_request_all_signed" => {
                Some("completed")
            }
            "envelope-voided" | "agreement_cancelled" | "signature_request_canceled" => {
                Some("voided")
            }
            "envelope-declined" | "agreement_rejected" | "signature_request_declined" => {
                Some("declined")
            }
            "envelope-sent" | "agreement_created" | "signature_request_sent" => Some("sent"),
            _ => None,
        };

        if let Some(status) = new_status {
            if let Err(e) = state
                .integration_repo
                .update_esignature_workflow_by_external_id(envelope_id, status)
                .await
            {
                tracing::warn!(
                    error = %e,
                    envelope_id = %envelope_id,
                    "Failed to update workflow status from webhook"
                );
            }
        }
    }

    Ok(StatusCode::OK)
}

// ==================== Inbound: Booking.com Push ====================

/// Handle Booking.com push notification.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/booking/push",
    responses(
        (status = 200, description = "Push notification processed"),
        (status = 400, description = "Invalid notification"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations - Booking.com"
)]
pub async fn booking_push_notification(
    State(_state): State<AppState>,
    body: String,
) -> Result<Response<Body>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Received Booking.com push notification");

    let _notification = BookingClient::parse_push_notification(&body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse Booking.com notification");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PARSE_ERROR",
                "Invalid notification format",
            )),
        )
    })?;

    let response_xml = BookingClient::generate_push_response(true, None);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(response_xml))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "RESPONSE_ERROR",
                    "Failed to build response",
                )),
            )
        })
}

// ==================== Inbound: Portal Webhook ====================

/// Handle incoming portal webhook (public endpoint, no auth required).
#[utoipa::path(
    post,
    path = "/api/v1/integrations/webhooks/portal/{connection_id}",
    params(ConnectionIdPath),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid webhook"),
        (status = 401, description = "Invalid signature"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations - Portals"
)]
pub async fn handle_portal_webhook(
    State(_state): State<AppState>,
    Path(path): Path<ConnectionIdPath>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        connection_id = %path.connection_id,
        "Received portal webhook"
    );

    let _: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse webhook body");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid JSON body")),
        )
    })?;

    if let Some(signature) = headers.get("X-Webhook-Signature") {
        let _sig = signature.to_str().unwrap_or_default();
        tracing::debug!("Webhook signature present");
    }

    Ok(StatusCode::OK)
}

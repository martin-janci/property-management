//! Integration webhook-surface routes.
//!
//! Covers two concerns:
//! 1. **Outbound webhook management** — CRUD for webhook subscriptions,
//!    test delivery, and delivery logs (Story 61.5). UNMOUNTED (PAP-122):
//!    the backing schema exists in no migration; see `router()` for the
//!    remount conditions. The live subscription surface is the
//!    enhanced-webhook CRUD in `routes/api_ecosystem.rs`.
//! 2. **Inbound webhook receivers** — endpoints that external systems POST to:
//!    - E-signature providers (DocuSign, Adobe Sign, HelloSign) — UNMOUNTED
//!      (PAP-122), writes the migration-less `esignature_workflows` table
//!    - Booking.com OTA push notifications
//!    - Portal/listing-site webhooks (public, no auth)
//!
//! # RLS routing (PAP-105 / PAP-80)
//!
//! `webhook_subscriptions` runs under `FORCE ROW LEVEL SECURITY` (migration
//! `00179`), so every outbound-subscription handler acquires an
//! [`RlsConnection`] and runs its queries on that context-set connection; the
//! `{org_id}` path segment must equal `rls.tenant_id()` so the SQL org filter
//! and the policy can never disagree, and by-id reads of another tenant's
//! subscription resolve to `None` → `404` via RLS. The inbound receivers have
//! no request principal: the e-signature receiver writes the non-FORCE
//! `esignature_workflows` table on the pool (scoped by the provider-signed
//! envelope id), and must NEVER touch `webhook_subscriptions` that way.
//! Every authenticated path calls `rls.release().await` before returning.

use api_core::extractors::RlsConnection;
use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::post,
    Json, Router,
};
use db::models::{
    esignature_provider,
    infrastructure::{job_type, queue, CreateBackgroundJob},
    CreateWebhookSubscription, TestWebhookRequest, TestWebhookResponse, UpdateWebhookSubscription,
    WebhookDeliveryLog, WebhookDeliveryQuery, WebhookStatistics, WebhookSubscription,
};
use db::RlsPool;
use hmac::{Hmac, KeyInit, Mac};
use integrations::booking::ota_xml;
use integrations::{AirbnbClient, AirbnbWebhookEventType};
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
        // ROADMAP(PAP-122): outbound webhook-subscription CRUD (Story 61.5)
        // unmounted — the `IntegrationRepository` SQL behind it expects
        // `status` / `retry_policy` columns and a `webhook_delivery_logs`
        // table that exist in no migration, and it duplicates the live,
        // schema-aligned enhanced-webhook surface in `routes/api_ecosystem.rs`
        // (which is what `/organizations/{org_id}/webhooks` should keep
        // serving). Remount only after the Epic-61 migrations land AND the
        // two surfaces are reconciled onto one column convention.
        //
        // ROADMAP(PAP-122): the e-signature inbound receiver is unmounted with
        // the rest of the e-signature surface — its handler writes
        // `esignature_workflows`, which exists in no migration.
        //
        // Inbound webhook receivers (live)
        .route("/booking/push", post(booking_push_notification))
        .route(
            "/webhooks/portal/{connection_id}",
            post(handle_portal_webhook),
        )
        // Gap 83-1: Airbnb inbound webhook
        .route("/airbnb/webhook", post(handle_airbnb_webhook))
        // Story 11.5 (BIT-181): payment-gateway confirmation webhook
        .route(
            "/webhooks/payments/{provider}",
            post(handle_payment_webhook),
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
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<WebhookSubscription>>, (StatusCode, Json<ErrorResponse>)> {
    // PAP-105 (PAP-80): webhook_subscriptions is FORCE-RLS, so the path org
    // must be the org the RLS context is bound to — a mismatching path org
    // would silently read as empty. Membership in the tenant is validated by
    // the extractor.
    if path.org_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You are not a member of this organization",
            )),
        ));
    }

    let result = state
        .integration_repo
        .list_webhook_subscriptions(&mut **rls.conn(), path.org_id)
        .await;
    rls.release().await;

    let subscriptions = result.map_err(|e| {
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
    mut rls: RlsConnection,
    Path(path): Path<OrgIdPath>,
    Json(data): Json<CreateWebhookSubscription>,
) -> Result<(StatusCode, Json<WebhookSubscription>), (StatusCode, Json<ErrorResponse>)> {
    // PAP-105 (PAP-80): webhook_subscriptions is FORCE-RLS — the INSERT's
    // org must be the org the RLS context is bound to or the policy
    // WITH CHECK rejects the write. Membership in the tenant is validated by
    // the extractor.
    if path.org_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You are not a member of this organization",
            )),
        ));
    }
    let user_id = rls.user_id();

    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let result = state
        .integration_repo
        .create_webhook_subscription(&mut **rls.conn(), path.org_id, user_id, data, is_production)
        .await;
    rls.release().await;

    let subscription = result.map_err(|e| {
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
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // FORCE-RLS scopes the by-id read: another tenant's subscription resolves
    // to None → 404, indistinguishable from a missing one.
    let result = state
        .integration_repo
        .get_webhook_subscription(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let subscription = result.map_err(|e| {
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
            // Defense-in-depth: RLS already guarantees the row is the
            // caller's org, but keep the explicit membership check.
            verify_org_access(&state, rls.user_id(), s.organization_id).await?;
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
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<UpdateWebhookSubscription>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // FORCE-RLS scopes the by-id read: another tenant's subscription resolves
    // to None → 404 before any write is attempted.
    let existing = match state
        .integration_repo
        .get_webhook_subscription(&mut **rls.conn(), path.id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!(error = %e, "Failed to get webhook subscription");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            ));
        }
    };

    if let Err(e) = verify_org_access(&state, rls.user_id(), existing.organization_id).await {
        rls.release().await;
        return Err(e);
    }

    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let result = state
        .integration_repo
        .update_webhook_subscription(&mut **rls.conn(), path.id, data, is_production)
        .await;
    rls.release().await;

    let subscription = result.map_err(|e| {
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
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // FORCE-RLS scopes the by-id read: another tenant's subscription resolves
    // to None → 404 before any delete is attempted.
    let existing = match state
        .integration_repo
        .get_webhook_subscription(&mut **rls.conn(), path.id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Webhook subscription not found",
                )),
            ));
        }
        Err(e) => {
            rls.release().await;
            tracing::error!(error = %e, "Failed to get webhook subscription");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            ));
        }
    };

    if let Err(e) = verify_org_access(&state, rls.user_id(), existing.organization_id).await {
        rls.release().await;
        return Err(e);
    }

    let result = state
        .integration_repo
        .delete_webhook_subscription(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let deleted = result.map_err(|e| {
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
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
    Json(data): Json<TestWebhookRequest>,
) -> Result<Json<TestWebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    // FORCE-RLS scopes the by-id read: another tenant's subscription resolves
    // to None → 404. PAP-105 (PAP-80): release the RLS connection right after
    // the lookup so the (up to 30s) outbound test POST below does not pin a
    // pool connection.
    let result = state
        .integration_repo
        .get_webhook_subscription(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let subscription = result.map_err(|e| {
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

    verify_org_access(&state, rls.user_id(), subscription.organization_id).await?;

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
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<Vec<WebhookDeliveryLog>>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .list_webhook_delivery_logs(
            &mut **rls.conn(),
            WebhookDeliveryQuery {
                subscription_id: Some(path.id),
                event_type: None,
                status: None,
                from_date: None,
                to_date: None,
                limit: Some(100),
                offset: None,
            },
        )
        .await;
    rls.release().await;

    let logs = result.map_err(|e| {
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
    mut rls: RlsConnection,
    Path(path): Path<ResourceIdPath>,
) -> Result<Json<WebhookStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .integration_repo
        .get_webhook_statistics(&mut **rls.conn(), path.id)
        .await;
    rls.release().await;

    let stats = result.map_err(|e| {
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
            let secret = std::env::var("DOCUSIGN_WEBHOOK_SECRET").unwrap_or_else(|_| String::new());
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
            // PAP-170 (PAP-150 P5): inbound provider webhook — no request
            // principal. Resolve the owning org from the provider-signed
            // envelope id, then apply the status update under that tenant's RLS
            // context (defense in depth — the write is confined to the resolved
            // org instead of running on the raw pool).
            let rls_pool = RlsPool::new(state.db.clone());

            // Bootstrap read on a context-cleared connection: esignature_workflows
            // is keyed by the unguessable, provider-signed envelope id.
            let workflow = match rls_pool.acquire_public().await {
                Ok(mut lookup) => state
                    .integration_repo
                    .find_esignature_workflow_by_external_id(&mut **lookup.conn(), envelope_id)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            error = %e,
                            envelope_id = %envelope_id,
                            "Failed to resolve workflow org from webhook envelope id"
                        );
                        None
                    }),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to acquire connection for e-signature webhook"
                    );
                    None
                }
            };

            if let Some(wf) = workflow {
                match rls_pool
                    .acquire_with_rls(wf.organization_id, wf.created_by, false)
                    .await
                {
                    Ok(mut guard) => {
                        if let Err(e) = state
                            .integration_repo
                            .update_esignature_workflow_by_external_id(
                                &mut **guard.conn(),
                                envelope_id,
                                status,
                            )
                            .await
                        {
                            tracing::warn!(
                                error = %e,
                                envelope_id = %envelope_id,
                                "Failed to update workflow status from webhook"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            envelope_id = %envelope_id,
                            "Failed to bind RLS context for e-signature webhook update"
                        );
                    }
                }
            } else {
                tracing::warn!(
                    envelope_id = %envelope_id,
                    "E-signature webhook: no workflow matches envelope id, ignoring"
                );
            }
        }
    }

    Ok(StatusCode::OK)
}

// ==================== Inbound: Booking.com Push ====================

/// Maximum accepted skew (seconds) between a Booking.com push's signed
/// timestamp and the receiver's clock. Mirrors the portal/Stripe receivers'
/// 5-minute replay-protection window.
const BOOKING_WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// In-process, TTL-bounded replay ledger for the Booking.com push receiver
/// (audit R1 dedup leg).
///
/// The signed-timestamp window bounds the replay *duration* (±tolerance) but not
/// the *count*: within the window a captured valid delivery could still be
/// replayed N times. Booking.com guarantees at-least-once delivery, so a genuine
/// retry is byte-identical and yields the same dedup key. This guard suppresses
/// both — a duplicate key seen inside the retention window is treated as
/// already-processed and acked idempotently.
///
/// It is deliberately in-process (no migration): the receiver has no state
/// mutation today (audit F1 — it only parses OTA XML and acks), so a persistent,
/// cross-replica ledger (the `airbnb_webhook_events` pattern) is only *required*
/// before this receiver is wired to persist reservations (audit R1 blocking
/// condition). Retention is pinned to the freshness window so an entry never
/// outlives the timestamp tolerance that would let the same delivery back in
/// anyway, which also bounds the map size.
///
/// The decision logic ([`BookingReplayGuard::check_and_record`]) takes an
/// injected `now`, so it is unit-testable without wall-clock dependence.
#[derive(Default)]
struct BookingReplayGuard {
    seen: std::sync::Mutex<std::collections::HashMap<String, i64>>,
}

impl BookingReplayGuard {
    /// Record `key` as seen at `now_unix`. Returns `true` when the key is
    /// first-seen (caller should process), `false` when it is a duplicate still
    /// inside the `retention_secs` window (caller should ack idempotently).
    /// Expired entries are opportunistically evicted on each call so the map
    /// cannot grow without bound.
    fn check_and_record(&self, key: &str, now_unix: i64, retention_secs: i64) -> bool {
        let mut seen = match self.seen.lock() {
            Ok(g) => g,
            // A poisoned lock must not wedge the receiver. The dedup layer is
            // defense-in-depth (signature + freshness already gate authenticity),
            // so recover the guard and continue rather than fail the request.
            Err(p) => p.into_inner(),
        };
        // Evict entries older than the retention window (overflow-proof).
        seen.retain(|_, &mut ts| {
            now_unix
                .checked_sub(ts)
                .map(|skew| skew <= retention_secs)
                .unwrap_or(false)
        });
        if seen.contains_key(key) {
            return false;
        }
        seen.insert(key.to_string(), now_unix);
        true
    }
}

/// Process-global Booking.com replay ledger (see [`BookingReplayGuard`]).
static BOOKING_REPLAY_GUARD: std::sync::OnceLock<BookingReplayGuard> = std::sync::OnceLock::new();

fn booking_replay_guard() -> &'static BookingReplayGuard {
    BOOKING_REPLAY_GUARD.get_or_init(BookingReplayGuard::default)
}

/// Compute the idempotency/dedup key for an inbound Booking.com push delivery.
///
/// Booking.com redelivers the byte-identical signed body on an at-least-once
/// retry, so a SHA-256 over the raw body is a stable key across redeliveries
/// while differing for any distinct notification. Reservation ids parsed from
/// the OTA payload are folded in first (for observability and stability against
/// insignificant reordering), with the body hash as the collision-proof tail.
/// Mirrors the Airbnb synthetic-key derivation ([`airbnb_dedup_key`]).
fn booking_dedup_key(body: &str, res_ids: &[String]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    for id in res_ids {
        hasher.update(id.as_bytes());
        hasher.update(b"|");
    }
    hasher.update(body.as_bytes());
    format!("booking:{}", hex::encode(hasher.finalize()))
}

/// Handle Booking.com push notification.
///
/// Fail-closed inbound receiver (audit F1/R1). The endpoint has no request
/// principal — Booking.com POSTs OTA `OTA_HotelResNotifRQ` XML to the URL handed
/// to them at connection time — so authenticity is established by verifying an
/// HMAC-SHA256 `X-Booking-Signature` over `"{X-Booking-Timestamp}.{raw_body}"`
/// against the shared `BOOKING_WEBHOOK_SECRET`, exactly like the sibling portal
/// ([`handle_portal_webhook`]) / Airbnb ([`handle_airbnb_webhook`]) / Stripe
/// ([`handle_payment_webhook`]) receivers, **before** the payload is parsed.
///
/// The receiver fails closed: an unset secret is `503 NOT_CONFIGURED`; a
/// missing/malformed timestamp, a stale/future timestamp, or an invalid/absent
/// signature are all `401 INVALID_SIGNATURE`. Timestamp freshness closes the
/// replay window and an in-process dedup ledger ([`BookingReplayGuard`])
/// suppresses at-least-once redeliveries within that window; both are idempotent
/// (a duplicate is acked with the same OTA `<Success/>`).
///
/// Raw `Bytes` are taken so the HMAC is computed over the exact bytes signed —
/// UTF-8 decoding happens after signature verification.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/booking/push",
    params(
        ("X-Booking-Timestamp" = String, Header,
         description = "Unix-seconds timestamp of the delivery. Signed as part of \"{timestamp}.{body}\"; deliveries outside ±300s of server time are rejected 401 (replay defense)."),
        ("X-Booking-Signature" = String, Header,
         description = "Hex HMAC-SHA256 (optionally `sha256=`-prefixed) over \"{X-Booking-Timestamp}.{raw_body}\" keyed by BOOKING_WEBHOOK_SECRET."),
    ),
    responses(
        (status = 200, description = "Push notification processed"),
        (status = 400, description = "Invalid notification"),
        (status = 401, description = "Invalid signature or stale timestamp"),
        (status = 500, description = "Internal server error"),
        (status = 503, description = "Webhook signature verification not configured")
    ),
    tag = "Integrations - Booking.com"
)]
pub async fn booking_push_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Received Booking.com push notification");

    // Fail closed when the signing secret is not configured — never accept an
    // unverified OTA push (audit F1/R1).
    let secret = state.booking_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("BOOKING_WEBHOOK_SECRET is not configured — refusing Booking.com push");
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Booking.com push signature verification is not configured",
            )),
        ));
    }

    // Decode for signing over the exact received bytes (HMAC is over the raw
    // body; a non-UTF-8 body can never be a valid OTA XML push anyway).
    let body_str = std::str::from_utf8(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ENCODING",
                "Request body is not valid UTF-8",
            )),
        )
    })?;

    // Verify HMAC signature AND timestamp freshness over the raw body BEFORE
    // parsing or acting. A missing/malformed timestamp fails closed (an attacker
    // cannot strip the header to defeat replay protection).
    let timestamp = headers
        .get("X-Booking-Timestamp")
        .and_then(|v| v.to_str().ok())
        .and_then(|t| t.trim().parse::<i64>().ok());
    let signature = headers
        .get("X-Booking-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let now_unix = chrono::Utc::now().timestamp();
    let verified = match timestamp {
        Some(ts) => integrations::verify_timestamped_signature(
            secret,
            ts,
            body_str,
            signature,
            now_unix,
            BOOKING_WEBHOOK_TOLERANCE_SECS,
        ),
        None => Err(integrations::TimestampedSignatureError::BadSignature),
    };
    if let Err(reason) = verified {
        tracing::warn!(?reason, "Booking.com push rejected (signature/freshness)");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature or freshness verification failed",
            )),
        ));
    }

    // Parse only AFTER authenticity + freshness are established.
    let notifications = ota_xml::parse_res_notif_rq_raw(body_str).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse Booking.com notification");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PARSE_ERROR",
                "Invalid notification format",
            )),
        )
    })?;

    // Dedup / replay-count defense: suppress an at-least-once redelivery within
    // the freshness window. Harmless-but-first-class today (no side effects);
    // becomes load-bearing the moment this receiver mutates state (audit R1).
    let res_ids: Vec<String> = notifications.iter().map(|(id, _, _)| id.clone()).collect();
    let dedup_key = booking_dedup_key(body_str, &res_ids);
    let first_seen = booking_replay_guard().check_and_record(
        &dedup_key,
        now_unix,
        BOOKING_WEBHOOK_TOLERANCE_SECS,
    );
    if first_seen {
        tracing::debug!(
            dedup_key = %dedup_key,
            reservation_count = notifications.len(),
            "Booking.com push: first-seen delivery recorded in replay guard"
        );
    } else {
        // Idempotent ack — no reprocessing, same OTA <Success/> response.
        tracing::info!(
            dedup_key = %dedup_key,
            "Booking.com push: duplicate delivery suppressed by replay guard, acking idempotently"
        );
    }

    let response_xml = ota_xml::build_res_notif_rs(true, None).map_err(|e| {
        tracing::error!(error = %e, "Failed to build Booking.com response");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "RESPONSE_ERROR",
                "Failed to build response",
            )),
        )
    })?;

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

/// Maximum accepted skew (seconds) between an inbound portal webhook's signed
/// timestamp and the receiver's clock. Mirrors the Stripe receiver's 5-minute
/// replay-protection window ([`crate::services::stripe::DEFAULT_SIGNATURE_TOLERANCE_SECS`]).
const PORTAL_WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Why an inbound portal webhook was rejected. Every variant fails closed; the
/// handler maps all of them to `401 INVALID_SIGNATURE` (a single opaque status
/// so the reason is not leaked to the caller) while logging the discriminant.
#[derive(Debug, PartialEq, Eq)]
enum PortalWebhookError {
    /// `X-Webhook-Timestamp` was absent or not an integer unix-seconds value.
    MalformedTimestamp,
    /// The signed timestamp is outside the tolerance window — a stale capture
    /// or an absurd future-dated delivery (replay defense, gap 83-3).
    StaleTimestamp,
    /// `X-Webhook-Signature` was absent or did not match the HMAC over the
    /// `"{timestamp}.{body}"` signed payload.
    BadSignature,
}

/// Verify an inbound portal webhook's authenticity **and** freshness.
///
/// *Authenticity*: `X-Webhook-Signature` is the hex-encoded HMAC-SHA256
/// (optionally `sha256=`-prefixed) over the signed payload `"{timestamp}.{body}"`
/// keyed by the shared `PORTAL_WEBHOOK_SECRET`.
///
/// NOTE: this **inbound** contract signs `"{timestamp}.{body}"`. It is
/// deliberately *not* the same as the receiver's own **outbound** test-delivery
/// signer ([`test_webhook`]), which signs the bare body with no timestamp — do
/// not use the outbound signer as a reference for this inbound format (the two
/// diverged when gap 83-3 folded the timestamp into the inbound signature).
/// The timestamp is bound *into* the signed material rather than trusted as a
/// bare header — this is what makes the freshness check meaningful: an attacker
/// replaying a captured delivery cannot swap in a fresh timestamp without
/// invalidating the signature.
///
/// *Freshness* (gap 83-3): the `X-Webhook-Timestamp` header (unix seconds) must
/// be within `tolerance_secs` of `now_unix` in either direction, rejecting both
/// stale replays and future-dated deliveries. HMAC alone proves authenticity
/// but not freshness.
///
/// `now_unix`/`tolerance_secs` are injected so the window is unit-testable
/// without wall-clock dependence, matching
/// [`crate::services::stripe::verify_signature`]. The freshness + constant-time
/// HMAC check is delegated to the shared
/// [`integrations::verify_timestamped_signature`] helper (the 2026-07 audit R4
/// consolidation) so every `"{ts}.{body}"` hex receiver runs one reviewed
/// implementation; this wrapper only adapts header parsing and the opaque
/// error taxonomy.
fn verify_portal_webhook(
    secret: &str,
    body: &str,
    timestamp_header: Option<&str>,
    signature_header: Option<&str>,
    now_unix: i64,
    tolerance_secs: i64,
) -> Result<(), PortalWebhookError> {
    // Defense-in-depth: the shared helper computes a valid HMAC even for an empty
    // key, so an unset secret must be rejected here too and not only in the
    // handler's up-front `500 CONFIG_ERROR` guard — otherwise a caller signing
    // with the empty key would verify. (The helper also rejects it, but keeping
    // the guard preserves the original error ordering pinned by the unit tests.)
    if secret.is_empty() {
        return Err(PortalWebhookError::BadSignature);
    }

    let timestamp: i64 = timestamp_header
        .and_then(|t| t.trim().parse::<i64>().ok())
        .ok_or(PortalWebhookError::MalformedTimestamp)?;

    let signature = signature_header.ok_or(PortalWebhookError::BadSignature)?;

    // Freshness + authenticity over "{timestamp}.{raw_body}" — same construction
    // as the Stripe receiver — via the shared helper (overflow-proof window,
    // constant-time compare). Map its opaque error variants onto this receiver's.
    match integrations::verify_timestamped_signature(
        secret,
        timestamp,
        body,
        signature,
        now_unix,
        tolerance_secs,
    ) {
        Ok(()) => Ok(()),
        Err(integrations::TimestampedSignatureError::StaleTimestamp) => {
            Err(PortalWebhookError::StaleTimestamp)
        }
        Err(integrations::TimestampedSignatureError::BadSignature) => {
            Err(PortalWebhookError::BadSignature)
        }
    }
}

/// Handle incoming portal webhook (public endpoint, no session auth).
///
/// The endpoint has no request principal — external portals POST here using the
/// URL handed to them at connection time. Authenticity is instead established by
/// verifying the `X-Webhook-Signature` HMAC-SHA256 over the signed payload
/// `"{X-Webhook-Timestamp}.{raw_body}"` against the shared `PORTAL_WEBHOOK_SECRET`
/// **before** the payload is parsed or acted on, exactly like the sibling Airbnb
/// ([`handle_airbnb_webhook`]) and Stripe ([`handle_payment_webhook`]) receivers.
///
/// Replay/freshness (gap 83-3): HMAC alone proves authenticity but not freshness
/// — a captured valid delivery could otherwise be replayed indefinitely. The
/// signed `X-Webhook-Timestamp` is checked against a
/// [`PORTAL_WEBHOOK_TOLERANCE_SECS`] window (rejecting stale/future deliveries),
/// and because the timestamp is folded into the HMAC input it cannot be forged
/// independently of the signature.
///
/// The receiver fails closed: an unset secret is a `500 CONFIG_ERROR`, and a
/// missing/malformed timestamp, a stale/future timestamp, or an invalid/absent
/// signature are all `401 INVALID_SIGNATURE` — an unverified or replayed payload
/// is never processed.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/webhooks/portal/{connection_id}",
    params(
        ConnectionIdPath,
        ("X-Webhook-Timestamp" = String, Header,
         description = "Unix-seconds timestamp of the delivery. Signed as part of \"{timestamp}.{body}\"; deliveries whose timestamp is outside ±300s of server time are rejected 401 (replay defense, gap 83-3)."),
        ("X-Webhook-Signature" = String, Header,
         description = "Hex HMAC-SHA256 (optionally `sha256=`-prefixed) over \"{X-Webhook-Timestamp}.{raw_body}\" keyed by PORTAL_WEBHOOK_SECRET."),
    ),
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
    State(state): State<AppState>,
    Path(path): Path<ConnectionIdPath>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        connection_id = %path.connection_id,
        "Received portal webhook"
    );

    // Fail closed when the signing secret is not configured — never accept an
    // unverified webhook.
    let secret = state.portal_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("PORTAL_WEBHOOK_SECRET is not configured — refusing portal webhook");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "CONFIG_ERROR",
                "Portal webhook signature verification is not configured",
            )),
        ));
    }

    // Verify the HMAC signature AND timestamp freshness over the raw body BEFORE
    // parsing or acting. HMAC proves authenticity; the signed `X-Webhook-Timestamp`
    // closes the replay window (gap 83-3) — a captured delivery can no longer be
    // replayed once its timestamp falls outside the tolerance window, and the
    // timestamp cannot be refreshed without invalidating the signature.
    let timestamp = headers
        .get("X-Webhook-Timestamp")
        .and_then(|v| v.to_str().ok());
    let signature = headers
        .get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok());

    let now_unix = chrono::Utc::now().timestamp();
    if let Err(reason) = verify_portal_webhook(
        secret,
        &body,
        timestamp,
        signature,
        now_unix,
        PORTAL_WEBHOOK_TOLERANCE_SECS,
    ) {
        tracing::warn!(
            connection_id = %path.connection_id,
            ?reason,
            "Portal webhook rejected (signature/freshness)"
        );
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature or freshness verification failed",
            )),
        ));
    }

    let _: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse webhook body");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid JSON body")),
        )
    })?;

    // FOLLOW-UP (issue #2330, originally #2196): the signed-timestamp window
    // bounds the replay *duration* (±PORTAL_WEBHOOK_TOLERANCE_SECS) but NOT the
    // *count* — within the tolerance a captured valid delivery can still be
    // replayed N times. Harmless today because this receiver only parses the
    // body and has no side effects, but BEFORE it is ever wired to mutate state
    // (persist views/inquiries like the per-portal receiver), add an
    // idempotency/nonce dedup keyed on a delivery id (e.g. an `X-Webhook-ID`
    // header folded into the signed payload, or a payload `event_id`), reusing
    // the Airbnb dedup-ledger pattern (`airbnb_dedup_key` +
    // `record_airbnb_webhook_event`) with retention ≥ the tolerance window.
    Ok(StatusCode::OK)
}

// ==================== Gap 83-1: Airbnb Inbound Webhook ====================

/// Compute the idempotency key for an inbound Airbnb webhook delivery.
///
/// Prefers the Airbnb-assigned `event_id`. When absent (older webhook schema
/// versions omit it), derives a deterministic synthetic key from the stable,
/// HMAC-signed fields of the delivery so that an at-least-once redelivery of
/// the same payload maps to the same key and is suppressed by the dedup
/// ledger. The `synthetic:` prefix keeps these keys from ever colliding with
/// a real Airbnb `event_id`.
fn airbnb_dedup_key(event: &integrations::AirbnbWebhookEvent) -> String {
    if let Some(event_id) = event.event_id.as_deref() {
        return event_id.to_string();
    }

    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", event.event_type).as_bytes());
    hasher.update(b"|");
    hasher.update(event.listing_id.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(event.confirmation_code.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(event.timestamp.to_rfc3339().as_bytes());
    format!("synthetic:{}", hex::encode(hasher.finalize()))
}

/// Maximum accepted skew (seconds) between an inbound Airbnb webhook's signed
/// timestamp and the receiver's clock, when the delivery carries an
/// `X-Airbnb-Timestamp` header. Mirrors the portal/Stripe receivers' 5-minute
/// replay-protection window (audit R2).
const AIRBNB_WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Establish the authenticity (and, when timestamped, freshness) of an inbound
/// Airbnb delivery — the staged accept-both decision (audit R2), extracted so it
/// is unit-testable without wall-clock dependence (`now_unix` is injected).
///
/// * `X-Airbnb-Timestamp` **present** ⇒ verify the signature over
///   `"{timestamp}.{body}"` and require the timestamp within `tolerance_secs`
///   (shared [`integrations::verify_timestamped_signature`]). A present-but-
///   malformed timestamp fails closed, so an attacker cannot send a garbage
///   header to fall back to the no-freshness path.
/// * **absent** ⇒ accept a legacy body-only signature (migration window); replay
///   for those deliveries stays neutralized by the persistent dedup ledger.
///
/// Returns `true` when the delivery is authentic (and fresh, if timestamped).
fn verify_airbnb_delivery(
    secret: &str,
    timestamp_header: Option<&str>,
    signature: &str,
    body: &str,
    now_unix: i64,
    tolerance_secs: i64,
) -> bool {
    match timestamp_header {
        Some(ts_raw) => match ts_raw.trim().parse::<i64>() {
            Ok(ts) => integrations::verify_timestamped_signature(
                secret,
                ts,
                body,
                signature,
                now_unix,
                tolerance_secs,
            )
            .is_ok(),
            Err(_) => false,
        },
        None => AirbnbClient::verify_webhook_signature(signature, body, secret),
    }
}

/// Receive and dispatch an inbound Airbnb webhook event.
///
/// Airbnb signs every delivery with HMAC-SHA256 keyed by the shared secret from
/// `AIRBNB_WEBHOOK_SECRET`; the signature (`X-Airbnb-Signature`) is verified
/// before the payload is parsed to reject unauthenticated callers.
///
/// Replay freshness (audit R2, staged accept-both): when the delivery carries an
/// `X-Airbnb-Timestamp` header the signature is verified over the timestamped
/// payload `"{timestamp}.{body}"` (via the shared
/// [`integrations::verify_timestamped_signature`]) and the timestamp must fall
/// within [`AIRBNB_WEBHOOK_TOLERANCE_SECS`] of server time, giving the same
/// replay posture as the portal/Stripe receivers. Until Airbnb senders are
/// migrated to send that header, a legacy **body-only** signature is still
/// accepted (with a deprecation warning); replay for those deliveries remains
/// neutralized by the persistent dedup ledger in step 4. Once every sender sends
/// the header the legacy branch should be removed so freshness is mandatory.
///
/// Raw `Bytes` are used so the HMAC is computed over the exact bytes Airbnb
/// signed — UTF-8 decoding happens after signature verification.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/airbnb/webhook",
    request_body(content = String, content_type = "application/json", description = "Airbnb webhook event payload"),
    params(
        ("X-Airbnb-Timestamp" = Option<String>, Header,
         description = "Optional unix-seconds timestamp. When present the signature covers \"{timestamp}.{body}\" and deliveries outside ±300s of server time are rejected 401 (replay defense, audit R2). Absent ⇒ legacy body-only signature (accepted during the accept-both rollout)."),
    ),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 401, description = "Invalid signature or stale timestamp"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations - Airbnb"
)]
pub async fn handle_airbnb_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // 1. Load the shared secret from the cached AppState config (issue #711).
    //    The env var is read once at server startup; per-request env reads
    //    were a minor perf concern and made misconfiguration only visible
    //    once a real delivery arrived.
    let secret = state.airbnb_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("AIRBNB_WEBHOOK_SECRET is not configured");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb webhook secret is not configured",
            )),
        ));
    }

    // 2. Verify HMAC-SHA256 signature over raw bytes.
    let signature = headers
        .get("X-Airbnb-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let body_str = std::str::from_utf8(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ENCODING",
                "Request body is not valid UTF-8",
            )),
        )
    })?;

    // Staged accept-both (audit R2): when the delivery carries an
    // `X-Airbnb-Timestamp` header, enforce the same `"{timestamp}.{body}"`
    // freshness window as the portal/Stripe receivers (folding the timestamp
    // into the signed material so it cannot be swapped without invalidating the
    // signature). Absent header ⇒ legacy body-only signature is still accepted
    // during the migration; replay for those is already neutralized by the
    // dedup ledger in step 4.
    let ts_header = headers
        .get("X-Airbnb-Timestamp")
        .and_then(|v| v.to_str().ok());
    let now_unix = chrono::Utc::now().timestamp();
    let authentic = verify_airbnb_delivery(
        secret,
        ts_header,
        signature,
        body_str,
        now_unix,
        AIRBNB_WEBHOOK_TOLERANCE_SECS,
    );
    if authentic && ts_header.is_none() {
        tracing::warn!(
            "Airbnb webhook accepted with a legacy body-only signature and no \
             X-Airbnb-Timestamp — replay freshness is NOT in effect for this delivery \
             (dedup ledger still applies). Migrate this sender to sign \
             \"{{timestamp}}.{{body}}\" and send X-Airbnb-Timestamp (audit R2)."
        );
    }

    if !authentic {
        tracing::warn!("Airbnb webhook signature verification failed");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature verification failed",
            )),
        ));
    }

    // 3. Parse the event.
    let event = AirbnbClient::parse_webhook_event(body_str).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse Airbnb webhook event");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid webhook payload")),
        )
    })?;

    // 4. Persistent deduplication (issue #711, bug-webhook-airbnb-dup-sync-jobs).
    //
    // Airbnb guarantees at-least-once delivery. Without persistent dedup,
    // duplicate ReservationCreated/Updated deliveries enqueue racing
    // SYNC_EXTERNAL jobs, and ReservationCancelled deliveries hammer the
    // booking-status guard. Migration 00169 created
    // `airbnb_webhook_events(event_id PRIMARY KEY, event_type, received_at)`;
    // we attempt to insert the delivery's dedup key and bail out with 200 on
    // conflict (idempotent acknowledgement, no side effects).
    //
    // The dedup key is the Airbnb-assigned `event_id` when present. Older
    // webhook schema versions omit it; in that case we derive a *synthetic*
    // key from the stable, signed fields of the delivery (event type +
    // listing id + confirmation code + timestamp). Airbnb redelivers the
    // byte-identical signed payload, so a redelivery yields the same
    // synthetic key and is suppressed — closing the previously-unguarded
    // event_id-absent path that could still double-enqueue SYNC_EXTERNAL.
    let dedup_key = airbnb_dedup_key(&event);
    let event_type_label = format!("{:?}", event.event_type);
    // PAP-170 (PAP-150): the dedup-ledger insert now lives in the repository
    // layer (`airbnb_webhook_events` is a global, tenant-less table) instead of
    // a raw `state.db` pool access here. `Ok(false)` ⇒ already seen ⇒ suppress.
    let recorded = state
        .rental_repo
        .record_airbnb_webhook_event(&dedup_key, &event_type_label)
        .await;

    match recorded {
        Ok(false) => {
            tracing::info!(
                dedup_key = %dedup_key,
                event_type = %event_type_label,
                "Airbnb webhook: duplicate delivery suppressed by dedup ledger"
            );
            return Ok(StatusCode::OK);
        }
        Ok(true) => {
            tracing::debug!(
                dedup_key = %dedup_key,
                event_type = %event_type_label,
                "Airbnb webhook: delivery recorded in dedup ledger"
            );
        }
        Err(e) => {
            // Best-effort: if the ledger insert fails (DB blip, table
            // not yet migrated on a stale env, etc.) we degrade to the
            // pre-#711 behaviour and process the event. Failing closed
            // here would let a transient DB issue silently drop real
            // reservation updates, which is worse than a rare double-
            // processing (downstream handlers are idempotent upserts).
            tracing::warn!(
                error = %e,
                dedup_key = %dedup_key,
                "Airbnb webhook: dedup ledger insert failed, processing anyway"
            );
        }
    }

    // 5. Dispatch by event type.
    match event.event_type {
        AirbnbWebhookEventType::ReservationCreated | AirbnbWebhookEventType::ReservationUpdated => {
            if let Some(listing_id) = &event.listing_id {
                match state
                    .rental_repo
                    .find_airbnb_connection_by_listing_id(listing_id)
                    .await
                {
                    Ok(Some(conn)) => {
                        let payload = serde_json::json!({
                            "org_id": conn.organization_id,
                            "connection_id": conn.id,
                            "sync_type": "reservations",
                            "trigger": "webhook",
                            "event_id": event.event_id,
                        });
                        // Idempotency note: at-least-once redeliveries are
                        // suppressed up-front by the dedup ledger in step 4
                        // (Airbnb event_id, or a synthetic key for legacy
                        // deliveries that omit it), so reaching this point
                        // means a first-seen delivery and the SYNC_EXTERNAL
                        // job is enqueued exactly once per delivery.
                        let job_data = CreateBackgroundJob {
                            job_type: job_type::SYNC_EXTERNAL.to_string(),
                            priority: Some(1),
                            payload,
                            scheduled_at: None,
                            queue: Some(queue::LOW_PRIORITY.to_string()),
                            max_attempts: Some(3),
                            org_id: Some(conn.organization_id),
                        };
                        if let Err(e) = state.background_job_repo.create(job_data, None).await {
                            tracing::error!(
                                error = %e,
                                listing_id = %listing_id,
                                event_type = ?event.event_type,
                                "Failed to enqueue Airbnb reservation sync job from webhook"
                            );
                        } else {
                            tracing::info!(
                                listing_id = %listing_id,
                                org_id = %conn.organization_id,
                                event_type = ?event.event_type,
                                "Airbnb webhook: enqueued reservation sync job"
                            );
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            listing_id = %listing_id,
                            "Airbnb webhook: no active connection found for listing, ignoring"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            listing_id = %listing_id,
                            "Airbnb webhook: DB error looking up connection"
                        );
                    }
                }
            }
        }
        AirbnbWebhookEventType::ReservationCancelled => {
            if let Some(code) = &event.confirmation_code {
                match state
                    .rental_repo
                    .find_booking_by_external_id("airbnb", code)
                    .await
                {
                    Ok(Some(booking)) => {
                        if booking.status == db::models::rental::booking_status::CANCELLED {
                            tracing::warn!(
                                booking_id = %booking.id,
                                confirmation_code = %code,
                                event_id = ?event.event_id,
                                "Airbnb webhook: ReservationCancelled for already-cancelled booking, likely duplicate delivery — ignoring"
                            );
                        } else {
                            let cancel_data = db::models::rental::UpdateBookingStatus {
                                status: db::models::rental::booking_status::CANCELLED.to_string(),
                                cancellation_reason: Some(
                                    "Cancelled via Airbnb webhook".to_string(),
                                ),
                            };
                            // PAP-141: key the status mutation to the booking's
                            // own organization (`update_booking_status_for_org`)
                            // rather than the bare id, so a forged/replayed
                            // webhook can never flip a booking the lookup did
                            // not legitimately resolve. `booking` was just found
                            // by (`platform`, `external_id`), so its
                            // `organization_id` is the authoritative owner.
                            match state
                                .rental_repo
                                .update_booking_status_for_org(
                                    booking.organization_id,
                                    booking.id,
                                    cancel_data,
                                )
                                .await
                            {
                                Ok(Some(_)) => {
                                    tracing::info!(
                                        booking_id = %booking.id,
                                        confirmation_code = %code,
                                        "Airbnb webhook: booking cancelled"
                                    );
                                }
                                Ok(None) => {
                                    tracing::warn!(
                                        booking_id = %booking.id,
                                        org_id = %booking.organization_id,
                                        confirmation_code = %code,
                                        "Airbnb webhook: booking not cancelled (org mismatch on status update)"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        booking_id = %booking.id,
                                        confirmation_code = %code,
                                        "Airbnb webhook: failed to cancel booking"
                                    );
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            confirmation_code = %code,
                            "Airbnb webhook: ReservationCancelled for unknown booking, ignoring"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            confirmation_code = %code,
                            "Airbnb webhook: DB error looking up booking for cancellation"
                        );
                    }
                }
            }
        }
        AirbnbWebhookEventType::ListingUpdated => {
            tracing::info!(listing_id = ?event.listing_id, "Airbnb webhook: ListingUpdated (not yet handled)");
        }
        AirbnbWebhookEventType::MessageReceived => {
            tracing::info!(listing_id = ?event.listing_id, "Airbnb webhook: MessageReceived (not yet handled)");
        }
        AirbnbWebhookEventType::ReviewReceived => {
            tracing::info!(listing_id = ?event.listing_id, "Airbnb webhook: ReviewReceived (not yet handled)");
        }
    }

    Ok(StatusCode::OK)
}

// ==================== Payment-gateway webhook (Story 11.5, BIT-181) ====================

/// Inbound payment-gateway confirmation webhook.
///
/// `POST /api/v1/integrations/webhooks/payments/{provider}` (currently only
/// `stripe`). Mirrors the other inbound receivers: load the signing secret
/// from `AppState` (never the request), **fail closed** when it is unset,
/// verify the signature over the raw body before parsing, and acknowledge
/// idempotently.
///
/// On a verified `checkout.session.completed` with `payment_status == "paid"`,
/// the originating `online_payment_sessions` row is looked up by its Stripe
/// session id, the invoice is settled via the gateway-settlement path (records
/// a `payments` row + allocation; the DB trigger marks the invoice `paid`), and
/// the session is marked `completed`. Already-completed sessions are a no-op so
/// Stripe's at-least-once retries cannot double-settle.
///
/// Tenant isolation: the session id is the only attacker-influenced input and
/// it is resolved server-side to its owning organization/invoice — no org id is
/// trusted from the payload, and a forged session id that matches nothing is
/// acknowledged without side effects.
pub async fn handle_payment_webhook(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    use crate::services::stripe;

    // Only Stripe is wired today; reject unknown providers explicitly.
    if provider != "stripe" {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "UNKNOWN_PROVIDER",
                "Unsupported payment provider",
            )),
        ));
    }

    // Fail closed when the signing secret is not configured.
    let secret = state.stripe_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("STRIPE_WEBHOOK_SECRET is not configured — refusing webhook");
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Payment webhook secret is not configured",
            )),
        ));
    }

    let signature = headers
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let now_unix = chrono::Utc::now().timestamp();
    if let Err(e) = stripe::verify_signature(
        &body,
        signature,
        secret,
        now_unix,
        stripe::DEFAULT_SIGNATURE_TOLERANCE_SECS,
    ) {
        tracing::warn!(?e, "Stripe webhook signature verification failed");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature verification failed",
            )),
        ));
    }

    let event = stripe::parse_event(&body).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse Stripe webhook event");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid webhook payload")),
        )
    })?;

    // Only settlement events are actionable; acknowledge everything else so
    // Stripe does not retry events we deliberately ignore.
    if event.event_type != "checkout.session.completed" {
        return Ok(StatusCode::OK);
    }
    if event.data.object.payment_status.as_deref() != Some("paid") {
        return Ok(StatusCode::OK);
    }

    let provider_session_id = &event.data.object.id;
    let session = match state
        .financial_repo
        .get_payment_session_by_provider_id(provider_session_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            // Unknown session — ack so Stripe stops retrying; nothing to do.
            tracing::warn!(session_id = %provider_session_id, "Stripe webhook for unknown session");
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            tracing::error!("Failed to load payment session: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to load payment session",
                )),
            ));
        }
    };

    // Idempotency: a session already settled is a no-op (Stripe retries).
    if session.status == "completed" {
        return Ok(StatusCode::OK);
    }

    // Defense-in-depth: settlement uses our server-stored `session.amount`/
    // `currency` (never a client-supplied amount), but if the amount Stripe
    // reports collecting disagrees with what we recorded, something is wrong —
    // refuse to settle and ack (200) so Stripe stops retrying a mismatched
    // event. Older payloads may omit these fields; when absent we keep the
    // prior behaviour and settle on the stored amount.
    {
        use rust_decimal::prelude::ToPrimitive;
        use rust_decimal::Decimal;

        let factor = stripe::minor_unit_factor(&session.currency);
        let expected_minor = (session.amount * Decimal::from(factor)).round().to_i64();

        if let (Some(expected), Some(reported)) = (expected_minor, event.data.object.amount_total) {
            if expected != reported {
                tracing::warn!(
                    session_id = %provider_session_id,
                    expected_minor = expected,
                    reported_minor = reported,
                    "Stripe webhook amount_total disagrees with stored session amount — not settling"
                );
                return Ok(StatusCode::OK);
            }
        }

        if let Some(reported_currency) = event.data.object.currency.as_deref() {
            if !reported_currency.eq_ignore_ascii_case(&session.currency) {
                tracing::warn!(
                    session_id = %provider_session_id,
                    expected_currency = %session.currency,
                    reported_currency = %reported_currency,
                    "Stripe webhook currency disagrees with stored session currency — not settling"
                );
                return Ok(StatusCode::OK);
            }
        }
    }

    let invoice = match state.financial_repo.get_invoice(session.invoice_id).await {
        Ok(Some(inv)) => inv,
        Ok(None) => {
            tracing::error!(invoice_id = %session.invoice_id, "Webhook session references a missing invoice");
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            tracing::error!("Failed to load invoice for webhook: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to load invoice")),
            ));
        }
    };

    // Prefer the PaymentIntent id as the gateway reference; fall back to the
    // Checkout Session id.
    let gateway_ref = event
        .data
        .object
        .payment_intent
        .clone()
        .unwrap_or_else(|| session.session_id.clone());

    let settled = state
        .financial_repo
        .settle_invoice_from_gateway(&session, &invoice, &gateway_ref)
        .await
        .map_err(|e| {
            tracing::error!("Failed to settle invoice from gateway webhook: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to settle invoice")),
            )
        })?;

    match settled {
        Some(_) => tracing::info!(
            invoice_id = %invoice.id,
            session_id = %session.session_id,
            "Invoice settled from Stripe Checkout webhook"
        ),
        // The atomic idempotency claim found the session already settled by a
        // concurrent delivery. Ack so Stripe stops retrying; no double-credit.
        None => tracing::info!(
            session_id = %session.session_id,
            "Stripe webhook duplicate: session already settled, no-op"
        ),
    }

    Ok(StatusCode::OK)
}

// ==================== Gap 83-1 Unit Tests ====================

#[cfg(test)]
mod airbnb_webhook_tests {
    use integrations::AirbnbClient;

    #[test]
    fn test_signature_valid() {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let secret = "test_secret_key";
        let body = r#"{"event_type":"reservation_created","listing_id":"123","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;

        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(AirbnbClient::verify_webhook_signature(
            &signature, body, secret
        ));
    }

    #[test]
    fn test_signature_invalid() {
        assert!(!AirbnbClient::verify_webhook_signature(
            "deadbeef",
            r#"{"event_type":"listing_updated","listing_id":"42","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#,
            "some_secret",
        ));
    }

    #[test]
    fn test_event_parse_valid() {
        let body = r#"{"event_type":"listing_updated","listing_id":"abc123","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
        let event = AirbnbClient::parse_webhook_event(body);
        assert!(event.is_ok(), "parse failed: {:?}", event.err());
        let ev = event.unwrap();
        assert_eq!(ev.listing_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_event_parse_invalid() {
        let result = AirbnbClient::parse_webhook_event("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_event_parse_with_event_id() {
        let body = r#"{
            "event_type": "reservation_created",
            "event_id": "evt_abc123",
            "listing_id": "listing_42",
            "timestamp": "2026-01-01T00:00:00Z",
            "payload": {}
        }"#;
        let event = AirbnbClient::parse_webhook_event(body);
        assert!(event.is_ok(), "parse failed: {:?}", event.err());
        let ev = event.unwrap();
        assert_eq!(ev.event_id.as_deref(), Some("evt_abc123"));
        assert_eq!(ev.listing_id.as_deref(), Some("listing_42"));
    }

    #[test]
    fn test_event_parse_without_event_id() {
        let body = r#"{
            "event_type": "listing_updated",
            "listing_id": "listing_99",
            "timestamp": "2026-01-01T00:00:00Z",
            "payload": {}
        }"#;
        let event = AirbnbClient::parse_webhook_event(body);
        assert!(event.is_ok(), "parse failed: {:?}", event.err());
        let ev = event.unwrap();
        assert!(ev.event_id.is_none());
    }

    // ---- Dedup-key regression tests (bug-webhook-airbnb-dup-sync-jobs) ----
    //
    // The dedup key is what makes the `airbnb_webhook_events` ledger
    // (INSERT ... ON CONFLICT DO NOTHING) suppress at-least-once
    // redeliveries before any SYNC_EXTERNAL job is enqueued. A redelivery
    // MUST map to the same key as the original delivery, otherwise the
    // ledger lets it through and a duplicate sync job is created.

    fn parse(body: &str) -> integrations::AirbnbWebhookEvent {
        AirbnbClient::parse_webhook_event(body).expect("valid event")
    }

    const RESERVATION_WITH_ID: &str = r#"{
        "event_type": "reservation_created",
        "event_id": "evt_abc123",
        "listing_id": "listing_42",
        "confirmation_code": "HMABC123",
        "timestamp": "2026-01-01T00:00:00Z",
        "payload": {}
    }"#;

    const RESERVATION_NO_ID: &str = r#"{
        "event_type": "reservation_created",
        "listing_id": "listing_42",
        "confirmation_code": "HMABC123",
        "timestamp": "2026-01-01T00:00:00Z",
        "payload": {}
    }"#;

    #[test]
    fn dedup_key_uses_event_id_verbatim_when_present() {
        let key = super::airbnb_dedup_key(&parse(RESERVATION_WITH_ID));
        assert_eq!(key, "evt_abc123");
    }

    #[test]
    fn dedup_key_redelivery_with_event_id_is_stable() {
        // Same event_id redelivered => identical key => ledger conflict =>
        // no second SYNC_EXTERNAL enqueue.
        let first = super::airbnb_dedup_key(&parse(RESERVATION_WITH_ID));
        let redelivery = super::airbnb_dedup_key(&parse(RESERVATION_WITH_ID));
        assert_eq!(first, redelivery);
    }

    #[test]
    fn dedup_key_synthetic_when_event_id_absent() {
        let key = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        assert!(
            key.starts_with("synthetic:"),
            "expected synthetic key, got {key}"
        );
        // Synthetic keys can never collide with a real Airbnb event_id.
        assert_ne!(key, "evt_abc123");
    }

    #[test]
    fn dedup_key_synthetic_redelivery_is_stable() {
        // The previously-unguarded path: an event WITHOUT event_id, redelivered
        // byte-for-byte, must still produce the same key so the duplicate
        // SYNC_EXTERNAL enqueue is suppressed.
        let first = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        let redelivery = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        assert_eq!(first, redelivery);
    }

    #[test]
    fn dedup_key_synthetic_differs_for_distinct_deliveries() {
        let a = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        // Different listing => different delivery => different key.
        let other = RESERVATION_NO_ID.replace("listing_42", "listing_99");
        let b = super::airbnb_dedup_key(&parse(&other));
        assert_ne!(a, b);
        // Different timestamp => different delivery => different key.
        let later = RESERVATION_NO_ID.replace("00:00:00Z", "00:05:00Z");
        let c = super::airbnb_dedup_key(&parse(&later));
        assert_ne!(a, c);
    }
}

// ==================== Portal webhook auth + freshness tests ====================
//
// Regression cover for two properties of the connection-scoped portal webhook
// receiver:
//   1. authenticity — the `X-Webhook-Signature` HMAC must be verified (the
//      latent auth-bypass where any unsigned caller was accepted with 200);
//   2. freshness (gap 83-3) — a signed `X-Webhook-Timestamp` must fall inside a
//      tolerance window so a captured valid delivery cannot be replayed, and the
//      timestamp is folded into the HMAC so it cannot be refreshed independently.
//
// These pin the contract of `verify_portal_webhook`, the guard the handler runs
// before parsing/acting on the payload.
#[cfg(test)]
mod portal_webhook_signature_tests {
    use super::{verify_portal_webhook, PortalWebhookError, PORTAL_WEBHOOK_TOLERANCE_SECS};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    const SECRET: &str = "portal_webhook_secret";
    const BODY: &str = r#"{"event":"listing.viewed","external_id":"abc123"}"#;
    // Fixed reference "now" so the window is deterministic and wall-clock-free.
    const NOW: i64 = 1_700_000_000;

    /// Sign `"{timestamp}.{body}"` — the same signed-payload construction the
    /// handler verifies (mirrors the Stripe receiver).
    fn sign_at(secret: &str, ts: i64, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(format!("{ts}.{body}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    // ---- authenticity ----

    #[test]
    fn accepts_valid_hex_signature() {
        let sig = sign_at(SECRET, NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn accepts_valid_signature_with_sha256_prefix() {
        let sig = format!("sha256={}", sign_at(SECRET, NOW, BODY));
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_missing_signature_header() {
        // The original auth-bypass bug: no signature header was still accepted.
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                None,
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_wrong_signature() {
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some("deadbeef"),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_signature_for_tampered_body() {
        let sig = sign_at(SECRET, NOW, BODY);
        let tampered = r#"{"event":"listing.viewed","external_id":"evil999"}"#;
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                tampered,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_signature_under_wrong_secret() {
        let sig = sign_at("attacker_secret", NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_when_secret_empty() {
        // Defense-in-depth: even a syntactically valid signature cannot pass
        // when the server secret is unset (the handler also fails closed).
        let sig = sign_at("", NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                "",
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    // ---- freshness / replay protection (gap 83-3) ----

    #[test]
    fn rejects_stale_timestamp() {
        // A correctly-signed delivery captured and replayed once its signed
        // timestamp is older than the tolerance window must be rejected — HMAC
        // is still valid but the request is no longer fresh.
        let stale = NOW - PORTAL_WEBHOOK_TOLERANCE_SECS - 1;
        let sig = sign_at(SECRET, stale, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&stale.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::StaleTimestamp)
        );
    }

    #[test]
    fn accepts_fresh_timestamp_within_window() {
        // A delivery whose signed timestamp is inside the window is accepted.
        let fresh = NOW - (PORTAL_WEBHOOK_TOLERANCE_SECS - 1);
        let sig = sign_at(SECRET, fresh, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&fresh.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn accepts_timestamp_at_exact_tolerance_boundary() {
        // The freshness gate is inclusive (`|now - ts| == tolerance` passes).
        // Pin both edges so an off-by-one refactor (`<=` -> `<`) is caught.
        for boundary in [
            NOW - PORTAL_WEBHOOK_TOLERANCE_SECS,
            NOW + PORTAL_WEBHOOK_TOLERANCE_SECS,
        ] {
            let sig = sign_at(SECRET, boundary, BODY);
            assert_eq!(
                verify_portal_webhook(
                    SECRET,
                    BODY,
                    Some(&boundary.to_string()),
                    Some(&sig),
                    NOW,
                    PORTAL_WEBHOOK_TOLERANCE_SECS,
                ),
                Ok(()),
                "a delivery exactly {PORTAL_WEBHOOK_TOLERANCE_SECS}s away must still be fresh"
            );
        }
    }

    #[test]
    fn rejects_adversarial_overflow_timestamp_without_panicking() {
        // Regression for issue #2330: a parseable but adversarial timestamp near
        // i64::MIN/MAX must be rejected as stale WITHOUT panicking under
        // overflow-checks builds (the old `(now - ts).abs()` overflowed).
        for ts in [i64::MIN, i64::MIN + 1, i64::MAX, i64::MAX - 1] {
            let sig = sign_at(SECRET, ts, BODY);
            assert_eq!(
                verify_portal_webhook(
                    SECRET,
                    BODY,
                    Some(&ts.to_string()),
                    Some(&sig),
                    NOW,
                    PORTAL_WEBHOOK_TOLERANCE_SECS,
                ),
                Err(PortalWebhookError::StaleTimestamp),
                "adversarial timestamp {ts} must be rejected as stale, not panic"
            );
        }
    }

    #[test]
    fn rejects_future_timestamp_beyond_tolerance() {
        // An absurd future-dated delivery is rejected in the same way.
        let future = NOW + PORTAL_WEBHOOK_TOLERANCE_SECS + 1;
        let sig = sign_at(SECRET, future, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&future.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::StaleTimestamp)
        );
    }

    #[test]
    fn rejects_missing_timestamp_header() {
        // Without a timestamp there is no freshness to check — fail closed so an
        // attacker cannot strip the header to defeat replay protection.
        let sig = sign_at(SECRET, NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                None,
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::MalformedTimestamp)
        );
    }

    #[test]
    fn rejects_non_numeric_timestamp() {
        let sig = sign_at(SECRET, NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some("not-a-number"),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::MalformedTimestamp)
        );
    }

    #[test]
    fn rejects_timestamp_swap_on_captured_signature() {
        // The core replay defense: an attacker captures a valid (timestamp, sig)
        // pair, then presents a *fresh* timestamp header to beat the window while
        // reusing the old signature. Because the timestamp is folded into the
        // HMAC, the signature no longer matches and the swap is rejected.
        let old_ts = NOW - 10_000; // well outside the window
        let sig = sign_at(SECRET, old_ts, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()), // fresh header, but not what was signed
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }
}

// ==================== Booking.com push: auth + freshness + dedup tests ====================
//
// Regression cover for the audit F1/R1 fix: the Booking.com push receiver used
// to perform NO signature, timestamp, or dedup check — any caller could POST OTA
// XML and receive a `<Success/>` ack. These pin the three properties it now
// enforces before parsing/acting:
//   1. authenticity + freshness over "{X-Booking-Timestamp}.{body}" (shared
//      `verify_timestamped_signature` at BOOKING_WEBHOOK_TOLERANCE_SECS);
//   2. a stable dedup key across at-least-once redeliveries;
//   3. an in-process replay guard that suppresses a duplicate within the window.
#[cfg(test)]
mod booking_webhook_tests {
    use super::{booking_dedup_key, BookingReplayGuard, BOOKING_WEBHOOK_TOLERANCE_SECS};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    const SECRET: &str = "booking_webhook_secret";
    const BODY: &str = r#"<OTA_HotelResNotifRQ><HotelReservation ResStatus="Commit"><HotelReservationID ResID_Value="R1"/></HotelReservation></OTA_HotelResNotifRQ>"#;
    const NOW: i64 = 1_700_000_000;

    fn sign_ts(secret: &str, ts: i64, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(format!("{ts}.{body}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    // ---- authenticity + freshness (the F1 auth-bypass fix) ----

    #[test]
    fn accepts_valid_signed_and_fresh_delivery() {
        let sig = sign_ts(SECRET, NOW, BODY);
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                NOW,
                BODY,
                &sig,
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_forged_or_unsigned_delivery() {
        // The pre-fix bypass: an arbitrary/forged signature must not verify.
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                NOW,
                BODY,
                "deadbeef",
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(integrations::TimestampedSignatureError::BadSignature)
        );
    }

    #[test]
    fn rejects_stale_replay_and_timestamp_swap() {
        // Stale capture outside the window.
        let stale = NOW - BOOKING_WEBHOOK_TOLERANCE_SECS - 1;
        let stale_sig = sign_ts(SECRET, stale, BODY);
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                stale,
                BODY,
                &stale_sig,
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(integrations::TimestampedSignatureError::StaleTimestamp)
        );
        // Captured old signature re-presented with a fresh timestamp: the
        // folded timestamp invalidates the HMAC.
        let old_sig = sign_ts(SECRET, NOW - 10_000, BODY);
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                NOW,
                BODY,
                &old_sig,
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(integrations::TimestampedSignatureError::BadSignature)
        );
    }

    // ---- dedup key ----

    #[test]
    fn dedup_key_is_stable_across_byte_identical_redelivery() {
        let a = booking_dedup_key(BODY, &["R1".to_string()]);
        let b = booking_dedup_key(BODY, &["R1".to_string()]);
        assert_eq!(a, b);
        assert!(a.starts_with("booking:"), "unexpected key: {a}");
    }

    #[test]
    fn dedup_key_differs_for_distinct_deliveries() {
        let a = booking_dedup_key(BODY, &["R1".to_string()]);
        let other_body = BODY.replace("R1", "R2");
        let b = booking_dedup_key(&other_body, &["R2".to_string()]);
        assert_ne!(a, b);
    }

    // ---- in-process replay guard ----

    #[test]
    fn replay_guard_first_seen_then_duplicate_suppressed() {
        let guard = BookingReplayGuard::default();
        let key = "booking:abc";
        // First delivery is processed; an immediate redelivery is suppressed.
        assert!(guard.check_and_record(key, NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
        assert!(!guard.check_and_record(key, NOW + 1, BOOKING_WEBHOOK_TOLERANCE_SECS));
    }

    #[test]
    fn replay_guard_reaccepts_after_retention_window() {
        let guard = BookingReplayGuard::default();
        let key = "booking:abc";
        assert!(guard.check_and_record(key, NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
        // Once the entry ages past the retention window it is evicted; a delivery
        // that far in the future would itself be stale at the signature gate, so
        // re-accepting here does not widen the replay window.
        assert!(guard.check_and_record(
            key,
            NOW + BOOKING_WEBHOOK_TOLERANCE_SECS + 1,
            BOOKING_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn replay_guard_distinct_keys_are_independent() {
        let guard = BookingReplayGuard::default();
        assert!(guard.check_and_record("booking:a", NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
        assert!(guard.check_and_record("booking:b", NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
    }
}

// ==================== Airbnb timestamp-parity (staged accept-both) tests ====================
//
// Regression cover for the audit R2 fix: the Airbnb receiver verified a body-only
// HMAC with no freshness window. It now enforces "{timestamp}.{body}" freshness
// when the delivery carries `X-Airbnb-Timestamp`, while still accepting a legacy
// body-only signature during the migration. These pin `verify_airbnb_delivery`.
#[cfg(test)]
mod airbnb_timestamp_parity_tests {
    use super::{verify_airbnb_delivery, AIRBNB_WEBHOOK_TOLERANCE_SECS};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    const SECRET: &str = "airbnb_webhook_secret";
    const BODY: &str = r#"{"event_type":"reservation_created","listing_id":"L1","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
    const NOW: i64 = 1_700_000_000;

    fn sign_ts(secret: &str, ts: i64, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(format!("{ts}.{body}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    fn sign_body(secret: &str, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(body.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn timestamped_delivery_within_window_is_accepted() {
        let sig = sign_ts(SECRET, NOW, BODY);
        assert!(verify_airbnb_delivery(
            SECRET,
            Some(&NOW.to_string()),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn timestamped_delivery_outside_window_is_rejected() {
        let stale = NOW - AIRBNB_WEBHOOK_TOLERANCE_SECS - 1;
        let sig = sign_ts(SECRET, stale, BODY);
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some(&stale.to_string()),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn timestamp_swap_on_captured_signature_is_rejected() {
        let sig = sign_ts(SECRET, NOW - 10_000, BODY);
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some(&NOW.to_string()),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn present_but_malformed_timestamp_fails_closed() {
        // A body-only signature paired with a garbage timestamp header must not
        // fall back to the no-freshness path.
        let sig = sign_body(SECRET, BODY);
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some("not-a-number"),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn legacy_body_only_signature_accepted_when_no_timestamp() {
        // Accept-both migration window: no header ⇒ legacy body-only signature.
        let sig = sign_body(SECRET, BODY);
        assert!(verify_airbnb_delivery(
            SECRET,
            None,
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn forged_signature_rejected_on_both_paths() {
        assert!(!verify_airbnb_delivery(
            SECRET,
            None,
            "deadbeef",
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some(&NOW.to_string()),
            "deadbeef",
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }
}

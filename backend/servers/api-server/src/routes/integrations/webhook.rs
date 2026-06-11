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
            // PAP-105 (PAP-80): inbound provider webhook — no request
            // principal, so no RLS context to bind. esignature_workflows is
            // not FORCE-RLS and the write is scoped by the provider-signed
            // envelope id, so the pool is the executor here.
            if let Err(e) = state
                .integration_repo
                .update_esignature_workflow_by_external_id(&state.db, envelope_id, status)
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

    let _notifications = ota_xml::parse_res_notif_rq_raw(&body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse Booking.com notification");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PARSE_ERROR",
                "Invalid notification format",
            )),
        )
    })?;

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

/// Receive and dispatch an inbound Airbnb webhook event.
///
/// Airbnb signs every delivery with HMAC-SHA256 over the raw body using the
/// shared secret from `AIRBNB_WEBHOOK_SECRET`. The signature is verified
/// before the payload is parsed to reject unauthenticated callers.
///
/// Raw `Bytes` are used so the HMAC is computed over the exact bytes Airbnb
/// signed — UTF-8 decoding happens after signature verification.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/airbnb/webhook",
    request_body(content = String, content_type = "application/json", description = "Airbnb webhook event payload"),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 401, description = "Invalid signature"),
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

    if !AirbnbClient::verify_webhook_signature(signature, body_str, secret) {
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
    let insert = sqlx::query(
        "INSERT INTO airbnb_webhook_events (event_id, event_type) \
         VALUES ($1, $2) ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(&dedup_key)
    .bind(&event_type_label)
    .execute(&state.db)
    .await;

    match insert {
        Ok(result) if result.rows_affected() == 0 => {
            tracing::info!(
                dedup_key = %dedup_key,
                event_type = %event_type_label,
                "Airbnb webhook: duplicate delivery suppressed by dedup ledger"
            );
            return Ok(StatusCode::OK);
        }
        Ok(_) => {
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

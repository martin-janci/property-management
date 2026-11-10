//! Outbound webhook-subscription management (Story 61.5).
//!
//! UNMOUNTED (PAP-122): see the parent module's `router()` for the remount
//! conditions. Split out of the former monolithic `webhook.rs` (behaviour
//! preserved).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use common::errors::ErrorResponse;
use db::models::{
    CreateWebhookSubscription, TestWebhookRequest, TestWebhookResponse, UpdateWebhookSubscription,
    WebhookDeliveryLog, WebhookDeliveryQuery, WebhookStatistics, WebhookSubscription,
};

use crate::routes::integrations::sync::{verify_org_access, OrgIdPath, ResourceIdPath};
use crate::state::AppState;

// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

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

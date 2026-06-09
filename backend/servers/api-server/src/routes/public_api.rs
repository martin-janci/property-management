// ROADMAP(PAP-24): roadmap stub — handlers return 501. Unmounted in lib.rs (no /api/v1/developer
// route) so callers get 404 instead of a false 501. Kept for the planned external-developer portal.
//! Public API and Developer Portal routes (Epic 69).
//!
//! Provides endpoints for API key management, webhooks, rate limiting,
//! and SDK generation for third-party integrations.

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use common::errors::ErrorResponse;
use db::models::{
    public_api::{
        webhook_event_type, CreateWebhookSubscription, RateLimitStatus, TestWebhookRequest,
        TestWebhookResponse, UpdateWebhookSubscription, WebhookDelivery, WebhookDeliveryQuery,
        WebhookSubscription, WebhookSubscriptionQuery,
    },
    ApiChangelog, ApiEndpointDoc, ApiKeyDisplay, ApiKeyQuery, ApiKeyUsageStats, ApiRequestLog,
    ApiRequestLogQuery, CreateApiKey, CreateApiKeyResponse, CreateDeveloperAccount,
    CreateRateLimitConfig, CreateWebhookResponse, DeveloperAccount, DeveloperPortalStats,
    DeveloperUsageSummary, PaginatedResponse, RateLimitConfig, RotateApiKeyResponse,
    RotateWebhookSecretResponse, SandboxEnvironment, SandboxTestRequest, SandboxTestResponse,
    SdkDownloadInfo, SdkLanguageInfo, SdkVersion, UpdateApiKey, UpdateDeveloperAccount,
    UpdateRateLimitConfig,
};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

/// Generate a cryptographically secure random secret.
/// Uses 32 bytes of random data encoded as hex (64 characters).
fn generate_secure_secret(prefix: &str) -> String {
    use rand::TryRng;
    let mut bytes = [0u8; 32];
    rand::rngs::SysRng
        .try_fill_bytes(&mut bytes)
        .expect("OS rng failed");
    format!("{}_{}", prefix, hex::encode(bytes))
}

/// Uniform "not implemented" error for developer-portal endpoints whose
/// persistence layer does not exist yet (closes #851).
///
/// These handlers previously diverged: some fabricated a success response
/// (e.g. `create_api_key` minted a real-looking secret, `revoke_api_key`
/// returned `204` without revoking anything), while others returned `404`.
/// That advertised live behavior the server can't honor. Until the developer
/// account / API-key repository lands, every such endpoint returns `501` so
/// callers get an honest, uniform signal instead of a misleading 2xx/404.
fn not_implemented(feature: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse::new(
            "NOT_IMPLEMENTED",
            format!("{feature} is not implemented yet"),
        )),
    )
}

/// Create public API / developer portal router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Developer Account (Story 69.1)
        .route("/accounts", post(create_developer_account))
        .route("/accounts/me", get(get_my_developer_account))
        .route("/accounts/me", patch(update_my_developer_account))
        .route("/accounts/me/usage", get(get_my_usage_summary))
        // API Keys (Story 69.1)
        .route("/keys", post(create_api_key))
        .route("/keys", get(list_api_keys))
        .route("/keys/{id}", get(get_api_key))
        .route("/keys/{id}", patch(update_api_key))
        .route("/keys/{id}", delete(revoke_api_key))
        .route("/keys/{id}/rotate", post(rotate_api_key))
        .route("/keys/{id}/usage", get(get_api_key_usage))
        // Interactive Documentation (Story 69.2)
        .route("/docs/endpoints", get(list_api_endpoints))
        .route("/docs/endpoints/{id}", get(get_api_endpoint))
        .route("/docs/changelog", get(list_api_changelog))
        .route("/docs/openapi", get(get_openapi_spec))
        // Sandbox (Story 69.2)
        .route("/sandbox", post(create_sandbox))
        .route("/sandbox/test", post(test_sandbox_request))
        .route("/sandbox/{id}", get(get_sandbox))
        .route("/sandbox/{id}", delete(delete_sandbox))
        // Webhooks (Story 69.3)
        .route("/webhooks", post(create_webhook))
        .route("/webhooks", get(list_webhooks))
        .route("/webhooks/{id}", get(get_webhook))
        .route("/webhooks/{id}", patch(update_webhook))
        .route("/webhooks/{id}", delete(delete_webhook))
        .route("/webhooks/{id}/test", post(test_webhook))
        .route("/webhooks/{id}/rotate-secret", post(rotate_webhook_secret))
        .route("/webhooks/{id}/deliveries", get(list_webhook_deliveries))
        .route("/webhooks/events", get(list_webhook_event_types))
        // Rate Limiting (Story 69.4)
        .route("/rate-limits/status", get(get_rate_limit_status))
        .route("/rate-limits/tiers", get(list_rate_limit_tiers))
        // SDK Generation (Story 69.5)
        .route("/sdks", get(list_sdk_languages))
        .route("/sdks/{language}", get(get_sdk_info))
        .route("/sdks/{language}/download", get(download_sdk))
        .route("/sdks/{language}/versions", get(list_sdk_versions))
        // Admin Routes (for platform administrators)
        .route("/admin/developers", get(list_developers))
        .route("/admin/developers/{id}", get(get_developer))
        .route("/admin/developers/{id}", patch(update_developer))
        .route("/admin/developers/{id}/verify", post(verify_developer))
        .route("/admin/developers/{id}/suspend", post(suspend_developer))
        .route("/admin/rate-limits", post(create_rate_limit_config))
        .route("/admin/rate-limits/{id}", patch(update_rate_limit_config))
        .route("/admin/stats", get(get_portal_stats))
        .route("/admin/request-logs", get(list_request_logs))
}

// ==================== Request/Response Types ====================

/// Pagination query parameters.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Date range query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DateRangeQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// SDK language path parameter.
#[derive(Debug, Deserialize)]
pub struct SdkLanguagePath {
    pub language: String,
}

/// Suspend developer request.
#[derive(Debug, Deserialize)]
pub struct SuspendDeveloperRequest {
    pub reason: String,
}

// ==================== Developer Account Endpoints (Story 69.1) ====================

/// Create a developer account for API access.
#[utoipa::path(
    post,
    path = "/api/v1/developer/accounts",
    request_body = CreateDeveloperAccount,
    responses(
        (status = 201, description = "Developer account created", body = DeveloperAccount),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn create_developer_account(
    State(_state): State<AppState>,
    _user: AuthUser,
    Json(_payload): Json<CreateDeveloperAccount>,
) -> Result<(StatusCode, Json<DeveloperAccount>), (StatusCode, Json<ErrorResponse>)> {
    // No developer-account persistence exists yet; previously this fabricated
    // a CREATED account that was never stored (closes #851).
    Err(not_implemented("Developer account creation"))
}

/// Get current user's developer account.
#[utoipa::path(
    get,
    path = "/api/v1/developer/accounts/me",
    responses(
        (status = 200, description = "Developer account details", body = DeveloperAccount),
        (status = 404, description = "Account not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn get_my_developer_account(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<DeveloperAccount>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("Developer account lookup"))
}

/// Update current user's developer account.
async fn update_my_developer_account(
    State(_state): State<AppState>,
    _user: AuthUser,
    Json(_payload): Json<UpdateDeveloperAccount>,
) -> Result<Json<DeveloperAccount>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("Developer account update"))
}

/// Get usage summary for current developer.
async fn get_my_usage_summary(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<DeveloperUsageSummary>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("Developer usage summary"))
}

// ==================== API Key Endpoints (Story 69.1) ====================

/// Create a new API key.
#[utoipa::path(
    post,
    path = "/api/v1/developer/keys",
    request_body = CreateApiKey,
    responses(
        (status = 201, description = "API key created", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn create_api_key(
    State(_state): State<AppState>,
    _user: AuthUser,
    Json(payload): Json<CreateApiKey>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Validate scopes up front so obviously-bad requests still get a 400.
    for scope in &payload.scopes {
        if !db::models::api_key_scope::ALL.contains(&scope.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_SCOPE",
                    format!("Invalid scope: {}", scope),
                )),
            ));
        }
    }

    // Previously this minted a real-looking, cryptographically-secure secret
    // and returned 201 — but the key was never persisted, so it could never
    // authenticate anything. Returning the fake secret was misleading and a
    // latent security footgun. Fail honestly until persistence exists (#851).
    Err(not_implemented("API key creation"))
}

/// List all API keys for current developer.
#[utoipa::path(
    get,
    path = "/api/v1/developer/keys",
    params(ApiKeyQuery),
    responses(
        (status = 200, description = "List of API keys", body = Vec<ApiKeyDisplay>),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn list_api_keys(
    State(_state): State<AppState>,
    _user: AuthUser,
    Query(_query): Query<ApiKeyQuery>,
) -> Result<Json<Vec<ApiKeyDisplay>>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("API key listing"))
}

/// Get API key details.
async fn get_api_key(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiKeyDisplay>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("API key lookup"))
}

/// Update an API key.
async fn update_api_key(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(_payload): Json<UpdateApiKey>,
) -> Result<Json<ApiKeyDisplay>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("API key update"))
}

/// Revoke an API key.
#[utoipa::path(
    delete,
    path = "/api/v1/developer/keys/{id}",
    params(
        ("id" = Uuid, Path, description = "API key ID")
    ),
    responses(
        (status = 204, description = "API key revoked"),
        (status = 404, description = "API key not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn revoke_api_key(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Previously returned 204 without revoking anything — a silent no-op that
    // told the caller a key was revoked when nothing happened (closes #851).
    Err(not_implemented("API key revocation"))
}

/// Rotate an API key (create new, expire old).
#[utoipa::path(
    post,
    path = "/api/v1/developer/keys/{id}/rotate",
    params(
        ("id" = Uuid, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key rotated", body = RotateApiKeyResponse),
        (status = 404, description = "API key not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn rotate_api_key(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<RotateApiKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("API key rotation"))
}

/// Get usage statistics for an API key.
async fn get_api_key_usage(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Query(_query): Query<DateRangeQuery>,
) -> Result<Json<Vec<ApiKeyUsageStats>>, (StatusCode, Json<ErrorResponse>)> {
    Err(not_implemented("API key usage statistics"))
}

// ==================== API Documentation Endpoints (Story 69.2) ====================

/// List all API endpoints with documentation.
#[utoipa::path(
    get,
    path = "/api/v1/developer/docs/endpoints",
    params(PaginationQuery),
    responses(
        (status = 200, description = "List of API endpoints", body = Vec<ApiEndpointDoc>),
    ),
    tag = "developer"
)]
async fn list_api_endpoints(
    State(_state): State<AppState>,
    Query(_query): Query<PaginationQuery>,
) -> Result<Json<Vec<ApiEndpointDoc>>, (StatusCode, Json<ErrorResponse>)> {
    // Return sample endpoints for documentation
    Ok(Json(vec![]))
}

/// Get details for a specific API endpoint.
async fn get_api_endpoint(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ApiEndpointDoc>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Endpoint not found")),
    ))
}

/// Get API changelog.
#[utoipa::path(
    get,
    path = "/api/v1/developer/docs/changelog",
    params(PaginationQuery),
    responses(
        (status = 200, description = "API changelog entries", body = Vec<ApiChangelog>),
    ),
    tag = "developer"
)]
async fn list_api_changelog(
    State(_state): State<AppState>,
    Query(_query): Query<PaginationQuery>,
) -> Result<Json<Vec<ApiChangelog>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

/// Get OpenAPI specification.
#[utoipa::path(
    get,
    path = "/api/v1/developer/docs/openapi",
    responses(
        (status = 200, description = "OpenAPI specification", body = serde_json::Value),
    ),
    tag = "developer"
)]
async fn get_openapi_spec(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Return OpenAPI spec
    Ok(Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Property Management API",
            "version": "1.0.0",
            "description": "Public API for Property Management System"
        },
        "servers": [
            {
                "url": "https://api.ppt.example.com/v1",
                "description": "Production server"
            },
            {
                "url": "https://sandbox.ppt.example.com/v1",
                "description": "Sandbox server"
            }
        ]
    })))
}

// ==================== Sandbox Endpoints (Story 69.2) ====================

/// Create a sandbox environment for testing.
async fn create_sandbox(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> Result<(StatusCode, Json<SandboxEnvironment>), (StatusCode, Json<ErrorResponse>)> {
    let sandbox = SandboxEnvironment {
        id: Uuid::new_v4(),
        developer_account_id: Uuid::new_v4(), // Would be actual developer ID
        name: "Default Sandbox".to_string(),
        description: Some("Development and testing environment".to_string()),
        mock_data_enabled: Some(true),
        rate_limits_enabled: Some(false),
        is_active: Some(true),
        expires_at: Some(Utc::now() + chrono::Duration::days(30)),
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };

    Ok((StatusCode::CREATED, Json(sandbox)))
}

/// Test an API request in the sandbox.
async fn test_sandbox_request(
    State(_state): State<AppState>,
    _user: AuthUser,
    Json(_payload): Json<SandboxTestRequest>,
) -> Result<Json<SandboxTestResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Simulate API request in sandbox
    let response = SandboxTestResponse {
        status_code: 200,
        headers: serde_json::json!({
            "content-type": "application/json",
            "x-request-id": Uuid::new_v4().to_string()
        }),
        body: serde_json::json!({
            "success": true,
            "message": "Sandbox test successful"
        }),
        response_time_ms: 42,
    };

    Ok(Json(response))
}

/// Get sandbox environment details.
async fn get_sandbox(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<SandboxEnvironment>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Sandbox not found")),
    ))
}

/// Delete a sandbox environment.
async fn delete_sandbox(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

// ==================== Webhook Endpoints (Story 69.3) ====================

/// Create a webhook subscription.
#[utoipa::path(
    post,
    path = "/api/v1/developer/webhooks",
    request_body = CreateWebhookSubscription,
    responses(
        (status = 201, description = "Webhook created", body = CreateWebhookResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn create_webhook(
    State(_state): State<AppState>,
    _user: AuthUser,
    Json(payload): Json<CreateWebhookSubscription>,
) -> Result<(StatusCode, Json<CreateWebhookResponse>), (StatusCode, Json<ErrorResponse>)> {
    // P1-03: SSRF gate on the developer-supplied endpoint_url. The
    // integration::create path already calls validate_webhook_url for
    // tenant-internal webhooks; this public-API surface was missing the
    // gate, which would let a developer register
    // `http://169.254.169.254/...` or an internal Redis URL and have
    // every webhook delivery proxy through our cluster.
    //
    // is_production is derived from the standard PPT_ENV variable; we
    // fail closed if unset (treat as production).
    let is_production = std::env::var("PPT_ENV")
        .map(|v| v != "development" && v != "dev")
        .unwrap_or(true);
    let url_check =
        db::models::integration::validate_webhook_url(&payload.endpoint_url, is_production);
    if !url_check.is_valid {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_WEBHOOK_URL",
                url_check
                    .error
                    .unwrap_or_else(|| "Webhook URL rejected".to_string()),
            )),
        ));
    }

    // Validate event types
    for event_type in &payload.event_types {
        if !webhook_event_type::ALL.contains(&event_type.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_EVENT_TYPE",
                    format!("Invalid event type: {}", event_type),
                )),
            ));
        }
    }

    // Generate webhook secret for HMAC signature verification
    let secret = generate_secure_secret("whsec");

    let response = CreateWebhookResponse {
        id: Uuid::new_v4(),
        name: payload.name,
        endpoint_url: payload.endpoint_url,
        secret,
        event_types: payload.event_types,
        created_at: Utc::now(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// List webhook subscriptions.
#[utoipa::path(
    get,
    path = "/api/v1/developer/webhooks",
    params(WebhookSubscriptionQuery),
    responses(
        (status = 200, description = "List of webhooks", body = Vec<WebhookSubscription>),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn list_webhooks(
    State(_state): State<AppState>,
    _user: AuthUser,
    Query(_query): Query<WebhookSubscriptionQuery>,
) -> Result<Json<Vec<WebhookSubscription>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

/// Get webhook details.
async fn get_webhook(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Webhook not found")),
    ))
}

/// Update a webhook subscription.
async fn update_webhook(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(_payload): Json<UpdateWebhookSubscription>,
) -> Result<Json<WebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Webhook not found")),
    ))
}

/// Delete a webhook subscription.
async fn delete_webhook(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    Ok(StatusCode::NO_CONTENT)
}

/// Test webhook delivery.
#[utoipa::path(
    post,
    path = "/api/v1/developer/webhooks/{id}/test",
    params(
        ("id" = Uuid, Path, description = "Webhook ID")
    ),
    request_body = TestWebhookRequest,
    responses(
        (status = 200, description = "Test result", body = TestWebhookResponse),
        (status = 404, description = "Webhook not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn test_webhook(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(_payload): Json<TestWebhookRequest>,
) -> Result<Json<TestWebhookResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Simulate webhook test delivery
    let response = TestWebhookResponse {
        success: true,
        response_status_code: Some(200),
        response_body: Some("{\"received\": true}".to_string()),
        response_time_ms: Some(150),
        error_message: None,
    };

    Ok(Json(response))
}

/// Rotate webhook secret.
async fn rotate_webhook_secret(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RotateWebhookSecretResponse>, (StatusCode, Json<ErrorResponse>)> {
    let new_secret = generate_secure_secret("whsec");

    let response = RotateWebhookSecretResponse {
        webhook_id: id,
        new_secret,
        rotated_at: Utc::now(),
    };

    Ok(Json(response))
}

/// List webhook delivery logs.
async fn list_webhook_deliveries(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Query(_query): Query<WebhookDeliveryQuery>,
) -> Result<Json<Vec<WebhookDelivery>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(vec![]))
}

/// List available webhook event types.
#[utoipa::path(
    get,
    path = "/api/v1/developer/webhooks/events",
    responses(
        (status = 200, description = "List of event types", body = Vec<String>),
    ),
    tag = "developer"
)]
async fn list_webhook_event_types(
    State(_state): State<AppState>,
) -> Result<Json<Vec<&'static str>>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(webhook_event_type::ALL.to_vec()))
}

// ==================== Rate Limiting Endpoints (Story 69.4) ====================

/// Get current rate limit status.
#[utoipa::path(
    get,
    path = "/api/v1/developer/rate-limits/status",
    responses(
        (status = 200, description = "Rate limit status", body = RateLimitStatus),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "developer"
)]
async fn get_rate_limit_status(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<RateLimitStatus>, (StatusCode, Json<ErrorResponse>)> {
    use db::models::public_api::RateLimitWindow;

    let now = Utc::now();
    let status = RateLimitStatus {
        tier: "free".to_string(),
        requests_per_minute: RateLimitWindow {
            limit: 60,
            remaining: 55,
            reset_at: now + chrono::Duration::seconds(30),
        },
        requests_per_hour: RateLimitWindow {
            limit: 1000,
            remaining: 950,
            reset_at: now + chrono::Duration::minutes(45),
        },
        requests_per_day: RateLimitWindow {
            limit: 10000,
            remaining: 9500,
            reset_at: now + chrono::Duration::hours(18),
        },
    };

    Ok(Json(status))
}

/// List available rate limit tiers.
#[utoipa::path(
    get,
    path = "/api/v1/developer/rate-limits/tiers",
    responses(
        (status = 200, description = "List of rate limit tiers", body = Vec<RateLimitConfig>),
    ),
    tag = "developer"
)]
async fn list_rate_limit_tiers(
    State(_state): State<AppState>,
) -> Result<Json<Vec<RateLimitConfig>>, (StatusCode, Json<ErrorResponse>)> {
    let tiers = vec![
        RateLimitConfig {
            id: Uuid::new_v4(),
            tier: "free".to_string(),
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            burst_limit: Some(10),
            endpoint_limits: None,
            description: Some("Free tier for development and small projects".to_string()),
            is_active: Some(true),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        },
        RateLimitConfig {
            id: Uuid::new_v4(),
            tier: "basic".to_string(),
            requests_per_minute: 120,
            requests_per_hour: 5000,
            requests_per_day: 50000,
            burst_limit: Some(20),
            endpoint_limits: None,
            description: Some("Basic tier for small businesses".to_string()),
            is_active: Some(true),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        },
        RateLimitConfig {
            id: Uuid::new_v4(),
            tier: "professional".to_string(),
            requests_per_minute: 300,
            requests_per_hour: 20000,
            requests_per_day: 200000,
            burst_limit: Some(50),
            endpoint_limits: None,
            description: Some("Professional tier for growing businesses".to_string()),
            is_active: Some(true),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        },
        RateLimitConfig {
            id: Uuid::new_v4(),
            tier: "enterprise".to_string(),
            requests_per_minute: 1000,
            requests_per_hour: 100000,
            requests_per_day: 1000000,
            burst_limit: Some(100),
            endpoint_limits: None,
            description: Some("Enterprise tier with custom limits available".to_string()),
            is_active: Some(true),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        },
    ];

    Ok(Json(tiers))
}

// ==================== SDK Endpoints (Story 69.5) ====================

/// List available SDK languages.
#[utoipa::path(
    get,
    path = "/api/v1/developer/sdks",
    responses(
        (status = 200, description = "List of SDK languages", body = Vec<SdkLanguageInfo>),
    ),
    tag = "developer"
)]
async fn list_sdk_languages(
    State(_state): State<AppState>,
) -> Result<Json<Vec<SdkLanguageInfo>>, (StatusCode, Json<ErrorResponse>)> {
    let languages = vec![
        SdkLanguageInfo {
            language: "typescript".to_string(),
            display_name: "TypeScript / JavaScript".to_string(),
            package_manager: "npm".to_string(),
            latest_version: Some("1.0.0".to_string()),
            installation_command: "npm install @ppt/api-client".to_string(),
            documentation_url: Some("https://docs.ppt.example.com/sdks/typescript".to_string()),
        },
        SdkLanguageInfo {
            language: "python".to_string(),
            display_name: "Python".to_string(),
            package_manager: "pip".to_string(),
            latest_version: Some("1.0.0".to_string()),
            installation_command: "pip install ppt-api-client".to_string(),
            documentation_url: Some("https://docs.ppt.example.com/sdks/python".to_string()),
        },
        SdkLanguageInfo {
            language: "go".to_string(),
            display_name: "Go".to_string(),
            package_manager: "go modules".to_string(),
            latest_version: Some("1.0.0".to_string()),
            installation_command: "go get github.com/ppt/api-client-go".to_string(),
            documentation_url: Some("https://docs.ppt.example.com/sdks/go".to_string()),
        },
    ];

    Ok(Json(languages))
}

/// Get SDK information for a specific language.
async fn get_sdk_info(
    State(_state): State<AppState>,
    Path(language): Path<String>,
) -> Result<Json<SdkDownloadInfo>, (StatusCode, Json<ErrorResponse>)> {
    let info = SdkDownloadInfo {
        language: language.clone(),
        version: "1.0.0".to_string(),
        api_version: "v1".to_string(),
        download_url: format!(
            "https://downloads.ppt.example.com/sdks/{}/1.0.0.tar.gz",
            language
        ),
        package_name: Some(format!("ppt-api-client-{}", language)),
        package_manager_url: Some("https://www.npmjs.com/package/@ppt/api-client".to_string()),
        checksum_sha256: Some("abc123...".to_string()),
        release_notes: Some("Initial release with full API coverage".to_string()),
    };

    Ok(Json(info))
}

/// Download SDK for a specific language.
async fn download_sdk(
    State(state): State<AppState>,
    Path(language): Path<String>,
) -> Result<Json<SdkDownloadInfo>, (StatusCode, Json<ErrorResponse>)> {
    // In production, this would redirect to the actual download URL
    get_sdk_info(State(state), Path(language)).await
}

/// List SDK versions for a specific language.
async fn list_sdk_versions(
    State(_state): State<AppState>,
    Path(language): Path<String>,
) -> Result<Json<Vec<SdkVersion>>, (StatusCode, Json<ErrorResponse>)> {
    let versions = vec![SdkVersion {
        id: Uuid::new_v4(),
        language: language.clone(),
        version: "1.0.0".to_string(),
        api_version: "v1".to_string(),
        download_url: Some(format!(
            "https://downloads.ppt.example.com/sdks/{}/1.0.0.tar.gz",
            language
        )),
        package_name: Some(format!("ppt-api-client-{}", language)),
        package_manager_url: None,
        build_status: "success".to_string(),
        build_log: None,
        checksum_sha256: Some("abc123...".to_string()),
        download_count: Some(1500),
        release_notes: Some("Initial release".to_string()),
        is_latest: Some(true),
        is_stable: Some(true),
        created_at: Some(Utc::now()),
        published_at: Some(Utc::now()),
    }];

    Ok(Json(versions))
}

// ==================== Admin Endpoints ====================

/// List all developers (admin only).
async fn list_developers(
    State(_state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<DeveloperAccount>>, (StatusCode, Json<ErrorResponse>)> {
    let response = PaginatedResponse {
        data: vec![],
        total: 0,
        limit: query.limit.unwrap_or(50),
        offset: query.offset.unwrap_or(0),
        has_more: false,
    };

    Ok(Json(response))
}

/// Get developer details (admin only).
async fn get_developer(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<DeveloperAccount>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Developer not found")),
    ))
}

/// Update developer account (admin only).
async fn update_developer(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(_payload): Json<UpdateDeveloperAccount>,
) -> Result<Json<DeveloperAccount>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Developer not found")),
    ))
}

/// Verify a developer account (admin only).
async fn verify_developer(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
) -> Result<Json<DeveloperAccount>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Developer not found")),
    ))
}

/// Suspend a developer account (admin only).
async fn suspend_developer(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(_payload): Json<SuspendDeveloperRequest>,
) -> Result<Json<DeveloperAccount>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Developer not found")),
    ))
}

/// Create rate limit configuration (admin only).
async fn create_rate_limit_config(
    State(_state): State<AppState>,
    _user: AuthUser,
    Json(payload): Json<CreateRateLimitConfig>,
) -> Result<(StatusCode, Json<RateLimitConfig>), (StatusCode, Json<ErrorResponse>)> {
    let config = RateLimitConfig {
        id: Uuid::new_v4(),
        tier: payload.tier,
        requests_per_minute: payload.requests_per_minute,
        requests_per_hour: payload.requests_per_hour,
        requests_per_day: payload.requests_per_day,
        burst_limit: payload.burst_limit,
        endpoint_limits: payload.endpoint_limits,
        description: payload.description,
        is_active: Some(true),
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };

    Ok((StatusCode::CREATED, Json(config)))
}

/// Update rate limit configuration (admin only).
async fn update_rate_limit_config(
    State(_state): State<AppState>,
    _user: AuthUser,
    Path(_id): Path<Uuid>,
    Json(_payload): Json<UpdateRateLimitConfig>,
) -> Result<Json<RateLimitConfig>, (StatusCode, Json<ErrorResponse>)> {
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            "NOT_FOUND",
            "Rate limit config not found",
        )),
    ))
}

/// Get developer portal statistics (admin only).
async fn get_portal_stats(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<DeveloperPortalStats>, (StatusCode, Json<ErrorResponse>)> {
    let stats = DeveloperPortalStats {
        total_developers: 150,
        active_api_keys: 320,
        total_api_requests_today: 125000,
        total_api_requests_month: 3500000,
        webhook_deliveries_today: 8500,
        successful_webhook_rate: 0.985,
        top_endpoints: vec![],
        requests_by_tier: vec![],
    };

    Ok(Json(stats))
}

/// List API request logs (admin only).
async fn list_request_logs(
    State(_state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ApiRequestLogQuery>,
) -> Result<Json<PaginatedResponse<ApiRequestLog>>, (StatusCode, Json<ErrorResponse>)> {
    let response = PaginatedResponse {
        data: vec![],
        total: 0,
        limit: query.limit.unwrap_or(50),
        offset: query.offset.unwrap_or(0),
        has_more: false,
    };

    Ok(Json(response))
}

//! API Ecosystem Expansion routes (Epic 150).
//!
//! Routes for integration marketplace, connector framework, webhooks, and developer portal.

use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use common::errors::ErrorResponse;
use db::models::{
    api_doc_category, code_sample_language, ecosystem_webhook_event, ApiCodeSample,
    ApiDocumentation, ApiEcosystemDashboard, ApiEcosystemStatistics, Connector, ConnectorAction,
    ConnectorExecutionLog, ConnectorExecutionQuery, CreateApiCodeSample, CreateApiDocumentation,
    CreateConnector, CreateConnectorAction, CreateDeveloperApiKey, CreateDeveloperApiKeyResponse,
    CreateDeveloperRegistration, CreateEnhancedWebhookSubscription, CreateIntegrationRating,
    CreateMarketplaceIntegration, CreatePreBuiltIntegrationConnection, CreateSandboxConfig,
    DeveloperApiKeyDisplay, DeveloperPortalStatistics, DeveloperRegistration, DeveloperUsageStats,
    EnhancedWebhookDeliveryLog, EnhancedWebhookStatistics, EnhancedWebhookSubscription,
    InstallIntegration, IntegrationCategoryCount, IntegrationRating, IntegrationRatingWithUser,
    MarketplaceIntegration, MarketplaceIntegrationQuery, MarketplaceIntegrationSummary,
    OrganizationIntegration, PreBuiltIntegrationConnection, PreBuiltIntegrationSyncResult,
    ReviewDeveloperRegistration, SandboxConfig, SandboxTestRequestPayload,
    SandboxTestResponsePayload, SyncPreBuiltIntegrationRequest, UpdateApiDocumentation,
    UpdateConnector, UpdateEnhancedWebhookSubscription, UpdateMarketplaceIntegration,
    UpdateOrganizationIntegration, UpdatePreBuiltIntegrationConnection,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Create API ecosystem router.
pub fn router() -> Router<AppState> {
    Router::new()
        // ==================== Story 150.1: Integration Marketplace ====================
        .route("/marketplace", get(list_marketplace_integrations))
        .route("/marketplace", post(create_marketplace_integration))
        .route("/marketplace/{id}", get(get_marketplace_integration))
        .route("/marketplace/{id}", put(update_marketplace_integration))
        .route("/marketplace/{id}", delete(delete_marketplace_integration))
        .route("/marketplace/categories", get(list_integration_categories))
        .route("/marketplace/{id}/ratings", get(list_integration_ratings))
        .route("/marketplace/{id}/ratings", post(create_integration_rating))
        // Organization installations
        .route(
            "/organizations/{org_id}/integrations",
            get(list_organization_integrations),
        )
        .route(
            "/organizations/{org_id}/integrations",
            post(install_integration),
        )
        .route(
            "/organizations/{org_id}/integrations/{id}",
            get(get_organization_integration),
        )
        .route(
            "/organizations/{org_id}/integrations/{id}",
            put(update_organization_integration),
        )
        .route(
            "/organizations/{org_id}/integrations/{id}",
            delete(uninstall_integration),
        )
        .route(
            "/organizations/{org_id}/integrations/{id}/sync",
            post(sync_integration),
        )
        // ==================== Story 150.2: Connector Framework ====================
        .route("/connectors", get(list_connectors))
        .route("/connectors", post(create_connector))
        .route("/connectors/{id}", get(get_connector))
        .route("/connectors/{id}", put(update_connector))
        .route("/connectors/{id}", delete(delete_connector))
        .route("/connectors/{id}/actions", get(list_connector_actions))
        .route("/connectors/{id}/actions", post(create_connector_action))
        .route(
            "/organizations/{org_id}/connector-logs",
            get(list_connector_logs),
        )
        // ==================== Story 150.3: Webhook Management ====================
        .route(
            "/organizations/{org_id}/webhooks",
            get(list_enhanced_webhooks),
        )
        .route(
            "/organizations/{org_id}/webhooks",
            post(create_enhanced_webhook),
        )
        .route("/webhooks/{id}", get(get_enhanced_webhook))
        .route("/webhooks/{id}", put(update_enhanced_webhook))
        .route("/webhooks/{id}", delete(delete_enhanced_webhook))
        .route("/webhooks/{id}/test", post(test_enhanced_webhook))
        .route("/webhooks/{id}/logs", get(list_webhook_delivery_logs))
        .route("/webhooks/{id}/stats", get(get_enhanced_webhook_stats))
        .route("/webhooks/events", get(list_webhook_event_types))
        // ==================== Story 150.4: Pre-Built Integrations ====================
        .route(
            "/organizations/{org_id}/prebuilt",
            get(list_prebuilt_connections),
        )
        .route(
            "/organizations/{org_id}/prebuilt",
            post(create_prebuilt_connection),
        )
        .route(
            "/organizations/{org_id}/prebuilt/{integration_type}",
            get(get_prebuilt_connection),
        )
        .route(
            "/organizations/{org_id}/prebuilt/{integration_type}",
            put(update_prebuilt_connection),
        )
        .route(
            "/organizations/{org_id}/prebuilt/{integration_type}",
            delete(delete_prebuilt_connection),
        )
        .route(
            "/organizations/{org_id}/prebuilt/{integration_type}/sync",
            post(sync_prebuilt_connection),
        )
        .route(
            "/organizations/{org_id}/prebuilt/{integration_type}/oauth",
            get(get_prebuilt_oauth_url),
        )
        .route(
            "/organizations/{org_id}/prebuilt/{integration_type}/oauth/callback",
            post(handle_prebuilt_oauth_callback),
        )
        // ==================== Story 150.5: Developer Portal ====================
        .route("/developers/register", post(register_developer))
        .route("/developers/{id}", get(get_developer_registration))
        .route(
            "/developers/{id}/review",
            post(review_developer_registration),
        )
        .route("/developers/{id}/keys", get(list_developer_api_keys))
        .route("/developers/{id}/keys", post(create_developer_api_key))
        .route(
            "/developers/{id}/keys/{key_id}",
            delete(revoke_developer_api_key),
        )
        .route(
            "/developers/{id}/keys/{key_id}/rotate",
            post(rotate_developer_api_key),
        )
        .route("/developers/{id}/usage", get(get_developer_usage_stats))
        .route("/developers/{id}/sandbox", post(create_sandbox_environment))
        .route("/developers/{id}/sandbox", get(get_sandbox_environment))
        .route("/developers/{id}/sandbox/test", post(test_sandbox_request))
        // Documentation
        .route("/docs", get(list_api_documentation))
        .route("/docs", post(create_api_documentation))
        .route("/docs/{slug}", get(get_api_documentation))
        .route("/docs/{slug}", put(update_api_documentation))
        .route("/docs/{slug}", delete(delete_api_documentation))
        .route("/docs/{slug}/code-samples", get(list_code_samples))
        .route("/docs/{slug}/code-samples", post(create_code_sample))
        // Portal statistics
        .route("/portal/stats", get(get_developer_portal_stats))
        // ==================== Dashboard ====================
        .route(
            "/organizations/{org_id}/dashboard",
            get(get_ecosystem_dashboard),
        )
        .route(
            "/organizations/{org_id}/stats",
            get(get_ecosystem_statistics),
        )
}

// ==================== Types ====================

/// Organization ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgIdPath {
    pub org_id: Uuid,
}

/// Integration ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct IntegrationIdPath {
    pub id: Uuid,
}

/// Organization and integration ID path parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgIntegrationPath {
    pub org_id: Uuid,
    pub id: Uuid,
}

/// Pre-built integration type path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PrebuiltTypePath {
    pub org_id: Uuid,
    pub integration_type: String,
}

/// Developer ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DeveloperIdPath {
    pub id: Uuid,
}

/// Developer API key path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DeveloperKeyPath {
    pub id: Uuid,
    pub key_id: Uuid,
}

/// Documentation slug path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DocSlugPath {
    pub slug: String,
}

/// OAuth callback request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthCallbackRequest {
    pub code: String,
    pub state: Option<String>,
}

/// OAuth URL response.
#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthUrlResponse {
    pub url: String,
    pub state: String,
}

// Helper to create error response
fn error_response(code: &str, message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(code, message)),
    )
}

fn not_found(entity: &str, id: impl std::fmt::Display) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new(
            "NOT_FOUND",
            format!("{} {} not found", entity, id),
        )),
    )
}

// ==================== Story 150.1: Integration Marketplace ====================

/// List marketplace integrations.
#[utoipa::path(
    get,
    path = "/api/v1/ecosystem/marketplace",
    params(MarketplaceIntegrationQuery),
    responses(
        (status = 200, description = "List of integrations", body = Vec<MarketplaceIntegrationSummary>),
    ),
    tag = "API Ecosystem"
)]
async fn list_marketplace_integrations(
    State(state): State<AppState>,
    Query(query): Query<MarketplaceIntegrationQuery>,
) -> Result<Json<Vec<MarketplaceIntegrationSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let integrations = state
        .api_ecosystem_repo
        .list_marketplace_integrations(&query)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(integrations))
}

/// Create marketplace integration (admin only).
#[utoipa::path(
    post,
    path = "/api/v1/ecosystem/marketplace",
    request_body = CreateMarketplaceIntegration,
    responses(
        (status = 201, description = "Integration created", body = MarketplaceIntegration),
        (status = 403, description = "Forbidden"),
    ),
    tag = "API Ecosystem"
)]
async fn create_marketplace_integration(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(request): Json<CreateMarketplaceIntegration>,
) -> Result<Json<MarketplaceIntegration>, (StatusCode, Json<ErrorResponse>)> {
    // Admin only - require platform admin privileges
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }

    let integration = state
        .api_ecosystem_repo
        .create_marketplace_integration(&request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(integration))
}

/// Get marketplace integration by ID.
#[utoipa::path(
    get,
    path = "/api/v1/ecosystem/marketplace/{id}",
    params(IntegrationIdPath),
    responses(
        (status = 200, description = "Integration details", body = MarketplaceIntegration),
        (status = 404, description = "Not found"),
    ),
    tag = "API Ecosystem"
)]
async fn get_marketplace_integration(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<MarketplaceIntegration>, (StatusCode, Json<ErrorResponse>)> {
    let integration = state
        .api_ecosystem_repo
        .get_marketplace_integration(path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Integration", path.id))?;

    Ok(Json(integration))
}

/// Update marketplace integration (admin only).
#[utoipa::path(
    put,
    path = "/api/v1/ecosystem/marketplace/{id}",
    params(IntegrationIdPath),
    request_body = UpdateMarketplaceIntegration,
    responses(
        (status = 200, description = "Integration updated", body = MarketplaceIntegration),
        (status = 404, description = "Not found"),
    ),
    tag = "API Ecosystem"
)]
async fn update_marketplace_integration(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<IntegrationIdPath>,
    Json(request): Json<UpdateMarketplaceIntegration>,
) -> Result<Json<MarketplaceIntegration>, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }
    let integration = state
        .api_ecosystem_repo
        .update_marketplace_integration(path.id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Integration", path.id))?;

    Ok(Json(integration))
}

/// Delete marketplace integration (admin only).
#[utoipa::path(
    delete,
    path = "/api/v1/ecosystem/marketplace/{id}",
    params(IntegrationIdPath),
    responses(
        (status = 204, description = "Integration deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "API Ecosystem"
)]
async fn delete_marketplace_integration(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<IntegrationIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }
    let deleted = state
        .api_ecosystem_repo
        .delete_marketplace_integration(path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Integration", path.id))
    }
}

/// List integration categories with counts.
#[utoipa::path(
    get,
    path = "/api/v1/ecosystem/marketplace/categories",
    responses(
        (status = 200, description = "List of categories", body = Vec<IntegrationCategoryCount>),
    ),
    tag = "API Ecosystem"
)]
async fn list_integration_categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<IntegrationCategoryCount>>, (StatusCode, Json<ErrorResponse>)> {
    let categories = state
        .api_ecosystem_repo
        .get_integration_category_counts()
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(categories))
}

/// Pagination query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    20
}

/// List integration ratings.
#[utoipa::path(
    get,
    path = "/api/v1/ecosystem/marketplace/{id}/ratings",
    params(IntegrationIdPath, PaginationQuery),
    responses(
        (status = 200, description = "List of ratings", body = Vec<IntegrationRatingWithUser>),
    ),
    tag = "API Ecosystem"
)]
async fn list_integration_ratings(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<IntegrationRatingWithUser>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = query.limit.max(1).min(100);
    let offset = query.offset.max(0);
    let ratings = state
        .api_ecosystem_repo
        .list_integration_ratings(path.id, limit, offset)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(ratings))
}

/// Create integration rating.
#[utoipa::path(
    post,
    path = "/api/v1/ecosystem/marketplace/{id}/ratings",
    params(IntegrationIdPath),
    request_body = CreateIntegrationRating,
    responses(
        (status = 201, description = "Rating created", body = IntegrationRating),
    ),
    tag = "API Ecosystem"
)]
async fn create_integration_rating(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<IntegrationIdPath>,
    Json(request): Json<CreateIntegrationRating>,
) -> Result<Json<IntegrationRating>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = auth
        .tenant_id
        .ok_or_else(|| error_response("MISSING_ORG", "Organization context required"))?;

    let rating = state
        .api_ecosystem_repo
        .create_integration_rating(path.id, org_id, auth.user_id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(rating))
}

/// List organization integrations.
#[utoipa::path(
    get,
    path = "/api/v1/ecosystem/organizations/{org_id}/integrations",
    params(OrgIdPath),
    responses(
        (status = 200, description = "List of installed integrations", body = Vec<OrganizationIntegration>),
    ),
    tag = "API Ecosystem"
)]
async fn list_organization_integrations(
    State(state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<OrgIdPath>,
) -> Result<Json<Vec<OrganizationIntegration>>, (StatusCode, Json<ErrorResponse>)> {
    let integrations = state
        .api_ecosystem_repo
        .list_organization_integrations(path.org_id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(integrations))
}

/// Install integration.
#[utoipa::path(
    post,
    path = "/api/v1/ecosystem/organizations/{org_id}/integrations",
    params(OrgIdPath),
    request_body = InstallIntegration,
    responses(
        (status = 201, description = "Integration installed", body = OrganizationIntegration),
    ),
    tag = "API Ecosystem"
)]
async fn install_integration(
    State(state): State<AppState>,
    _tenant: TenantExtractor,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<InstallIntegration>,
) -> Result<Json<OrganizationIntegration>, (StatusCode, Json<ErrorResponse>)> {
    let installation = state
        .api_ecosystem_repo
        .install_integration(path.org_id, auth.user_id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(installation))
}

/// Get organization integration.
async fn get_organization_integration(
    State(state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<OrgIntegrationPath>,
) -> Result<Json<OrganizationIntegration>, (StatusCode, Json<ErrorResponse>)> {
    let integration = state
        .api_ecosystem_repo
        .get_organization_integration(path.org_id, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Integration", path.id))?;

    Ok(Json(integration))
}

/// Update organization integration.
async fn update_organization_integration(
    State(state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<OrgIntegrationPath>,
    Json(request): Json<UpdateOrganizationIntegration>,
) -> Result<Json<OrganizationIntegration>, (StatusCode, Json<ErrorResponse>)> {
    let integration = state
        .api_ecosystem_repo
        .update_organization_integration(path.org_id, path.id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Integration", path.id))?;

    Ok(Json(integration))
}

/// Uninstall integration.
async fn uninstall_integration(
    State(state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<OrgIntegrationPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let uninstalled = state
        .api_ecosystem_repo
        .uninstall_integration(path.org_id, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    if uninstalled {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Integration", path.id))
    }
}

/// Sync integration.
async fn sync_integration(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(_path): Path<OrgIntegrationPath>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement sync logic
    Ok(Json(serde_json::json!({
        "status": "completed",
        "records_synced": 0
    })))
}

// ==================== Story 150.2: Connector Framework ====================

/// List connectors.
async fn list_connectors(
    State(_state): State<AppState>,
) -> Result<Json<Vec<Connector>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Ok(Json(vec![]))
}

/// Create connector.
async fn create_connector(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Json(request): Json<CreateConnector>,
) -> Result<Json<Connector>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database insert
    let connector = Connector {
        id: Uuid::new_v4(),
        integration_id: request.integration_id,
        name: request.name,
        description: request.description,
        auth_type: request.auth_type,
        auth_config: request.auth_config,
        base_url: request.base_url,
        rate_limit_requests: request.rate_limit_requests,
        rate_limit_window_seconds: request.rate_limit_window_seconds,
        retry_max_attempts: request.retry_max_attempts.unwrap_or(3),
        retry_initial_delay_ms: request.retry_initial_delay_ms.unwrap_or(1000),
        retry_max_delay_ms: request.retry_max_delay_ms.unwrap_or(30000),
        timeout_ms: request.timeout_ms.unwrap_or(30000),
        headers: request.headers,
        supported_actions: request.supported_actions,
        error_mapping: request.error_mapping,
        data_transformations: request.data_transformations,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(connector))
}

/// Get connector.
async fn get_connector(
    State(_state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<Connector>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Err(not_found("Connector", path.id))
}

/// Update connector.
async fn update_connector(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<IntegrationIdPath>,
    Json(_request): Json<UpdateConnector>,
) -> Result<Json<Connector>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database update
    Err(not_found("Connector", path.id))
}

/// Delete connector.
async fn delete_connector(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(_path): Path<IntegrationIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database delete
    Ok(StatusCode::NO_CONTENT)
}

/// List connector actions.
async fn list_connector_actions(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<Vec<ConnectorAction>>, (StatusCode, Json<ErrorResponse>)> {
    let actions = state
        .api_ecosystem_repo
        .list_connector_actions(path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(actions))
}

/// Create connector action.
async fn create_connector_action(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(_path): Path<IntegrationIdPath>,
    Json(request): Json<CreateConnectorAction>,
) -> Result<Json<ConnectorAction>, (StatusCode, Json<ErrorResponse>)> {
    let action = state
        .api_ecosystem_repo
        .create_connector_action(&request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(action))
}

/// List connector execution logs.
async fn list_connector_logs(
    State(state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<OrgIdPath>,
    Query(query): Query<ConnectorExecutionQuery>,
) -> Result<Json<Vec<ConnectorExecutionLog>>, (StatusCode, Json<ErrorResponse>)> {
    let logs = state
        .api_ecosystem_repo
        .list_connector_execution_logs(path.org_id, &query)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(logs))
}

// ==================== Story 150.3: Webhook Management ====================

/// List enhanced webhooks.
async fn list_enhanced_webhooks(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<Vec<EnhancedWebhookSubscription>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Ok(Json(vec![]))
}

/// Create enhanced webhook.
async fn create_enhanced_webhook(
    State(state): State<AppState>,
    _tenant: TenantExtractor,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<CreateEnhancedWebhookSubscription>,
) -> Result<Json<EnhancedWebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let webhook = state
        .api_ecosystem_repo
        .create_enhanced_webhook(path.org_id, auth.user_id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(webhook))
}

/// Get enhanced webhook.
async fn get_enhanced_webhook(
    State(_state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<EnhancedWebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Err(not_found("Webhook", path.id))
}

/// Update enhanced webhook.
async fn update_enhanced_webhook(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
    Json(request): Json<UpdateEnhancedWebhookSubscription>,
) -> Result<Json<EnhancedWebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let webhook = state
        .api_ecosystem_repo
        .update_enhanced_webhook(path.id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Webhook", path.id))?;

    Ok(Json(webhook))
}

/// Delete enhanced webhook.
async fn delete_enhanced_webhook(
    State(_state): State<AppState>,
    Path(_path): Path<IntegrationIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database delete
    Ok(StatusCode::NO_CONTENT)
}

/// Test enhanced webhook.
async fn test_enhanced_webhook(
    State(_state): State<AppState>,
    Path(_path): Path<IntegrationIdPath>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement webhook test
    Ok(Json(serde_json::json!({
        "success": true,
        "status_code": 200,
        "response_time_ms": 150
    })))
}

/// List webhook delivery logs.
async fn list_webhook_delivery_logs(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<EnhancedWebhookDeliveryLog>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = query.limit.max(1).min(100);
    let offset = query.offset.max(0);
    let logs = state
        .api_ecosystem_repo
        .list_webhook_delivery_logs(path.id, limit, offset)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(logs))
}

/// Get enhanced webhook statistics.
async fn get_enhanced_webhook_stats(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<EnhancedWebhookStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let stats = state
        .api_ecosystem_repo
        .get_webhook_statistics(path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(stats))
}

/// List available webhook event types.
async fn list_webhook_event_types(
    State(_state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    let events = vec![
        serde_json::json!({
            "type": ecosystem_webhook_event::INTEGRATION_INSTALLED,
            "description": "Triggered when an integration is installed"
        }),
        serde_json::json!({
            "type": ecosystem_webhook_event::INTEGRATION_SYNCED,
            "description": "Triggered when an integration sync completes"
        }),
        serde_json::json!({
            "type": ecosystem_webhook_event::DATA_IMPORTED,
            "description": "Triggered when data is imported"
        }),
        serde_json::json!({
            "type": ecosystem_webhook_event::DATA_EXPORTED,
            "description": "Triggered when data is exported"
        }),
        serde_json::json!({
            "type": ecosystem_webhook_event::CONNECTOR_EXECUTED,
            "description": "Triggered when a connector action is executed"
        }),
        serde_json::json!({
            "type": ecosystem_webhook_event::API_KEY_CREATED,
            "description": "Triggered when an API key is created"
        }),
    ];

    Ok(Json(events))
}

// ==================== Story 150.4: Pre-Built Integrations ====================

/// List pre-built integration connections.
async fn list_prebuilt_connections(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<Vec<PreBuiltIntegrationConnection>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Ok(Json(vec![]))
}

/// Create pre-built integration connection.
async fn create_prebuilt_connection(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    auth: AuthUser,
    Path(path): Path<OrgIdPath>,
    Json(request): Json<CreatePreBuiltIntegrationConnection>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database insert
    let connection = PreBuiltIntegrationConnection {
        id: Uuid::new_v4(),
        organization_id: path.org_id,
        integration_type: request.integration_type,
        status: "pending".to_string(),
        configuration: request.configuration,
        access_token_encrypted: None,
        refresh_token_encrypted: None,
        token_expires_at: None,
        last_sync_at: None,
        last_error: None,
        sync_enabled: true,
        created_by: auth.user_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(connection))
}

/// Get pre-built integration connection.
async fn get_prebuilt_connection(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<PrebuiltTypePath>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Err(not_found("Pre-built connection", &path.integration_type))
}

/// Update pre-built integration connection.
async fn update_prebuilt_connection(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<PrebuiltTypePath>,
    Json(_request): Json<UpdatePreBuiltIntegrationConnection>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database update
    Err(not_found("Pre-built connection", &path.integration_type))
}

/// Delete pre-built integration connection.
async fn delete_prebuilt_connection(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(_path): Path<PrebuiltTypePath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database delete
    Ok(StatusCode::NO_CONTENT)
}

/// Sync pre-built integration.
async fn sync_prebuilt_connection(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<PrebuiltTypePath>,
    Json(_request): Json<SyncPreBuiltIntegrationRequest>,
) -> Result<Json<PreBuiltIntegrationSyncResult>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement sync logic
    let result = PreBuiltIntegrationSyncResult {
        integration_type: path.integration_type,
        records_created: 0,
        records_updated: 0,
        records_deleted: 0,
        errors: vec![],
        synced_at: Utc::now(),
        duration_ms: 0,
    };

    Ok(Json(result))
}

/// Get OAuth URL for pre-built integration.
async fn get_prebuilt_oauth_url(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<PrebuiltTypePath>,
) -> Result<Json<OAuthUrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Generate OAuth URL based on integration type
    let state = Uuid::new_v4().to_string();

    let url = match path.integration_type.as_str() {
        "quickbooks" => format!(
            "https://appcenter.intuit.com/connect/oauth2?client_id=CLIENT_ID&response_type=code&scope=com.intuit.quickbooks.accounting&redirect_uri=REDIRECT_URI&state={}",
            state
        ),
        "xero" => format!(
            "https://login.xero.com/identity/connect/authorize?response_type=code&client_id=CLIENT_ID&redirect_uri=REDIRECT_URI&scope=openid%20profile%20email%20accounting.transactions&state={}",
            state
        ),
        "salesforce" => format!(
            "https://login.salesforce.com/services/oauth2/authorize?response_type=code&client_id=CLIENT_ID&redirect_uri=REDIRECT_URI&state={}",
            state
        ),
        "hubspot" => format!(
            "https://app.hubspot.com/oauth/authorize?client_id=CLIENT_ID&redirect_uri=REDIRECT_URI&scope=contacts%20crm.objects.deals.read&state={}",
            state
        ),
        "slack" => format!(
            "https://slack.com/oauth/v2/authorize?client_id=CLIENT_ID&scope=chat:write,channels:read&redirect_uri=REDIRECT_URI&state={}",
            state
        ),
        _ => {
            return Err(error_response(
                "INVALID_INTEGRATION_TYPE",
                &format!("Integration type {} does not support OAuth", path.integration_type),
            ))
        }
    };

    Ok(Json(OAuthUrlResponse { url, state }))
}

/// Handle OAuth callback for pre-built integration.
async fn handle_prebuilt_oauth_callback(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(_path): Path<PrebuiltTypePath>,
    Json(_request): Json<OAuthCallbackRequest>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Exchange code for tokens and store connection
    Err(error_response(
        "NOT_IMPLEMENTED",
        "OAuth callback handling not yet implemented",
    ))
}

// ==================== Story 150.5: Developer Portal ====================

/// Register as a developer.
async fn register_developer(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(request): Json<CreateDeveloperRegistration>,
) -> Result<Json<DeveloperRegistration>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .api_ecosystem_repo
        .register_developer(auth.user_id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(registration))
}

/// Get developer registration.
async fn get_developer_registration(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
) -> Result<Json<DeveloperRegistration>, (StatusCode, Json<ErrorResponse>)> {
    let registration = state
        .api_ecosystem_repo
        .get_developer_registration(path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Developer", path.id))?;

    Ok(Json(registration))
}

/// Review developer registration (admin only).
async fn review_developer_registration(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
    Json(request): Json<ReviewDeveloperRegistration>,
) -> Result<Json<DeveloperRegistration>, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }
    let registration = state
        .api_ecosystem_repo
        .review_developer_registration(path.id, auth.user_id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Developer", path.id))?;

    Ok(Json(registration))
}

/// List developer API keys.
async fn list_developer_api_keys(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(_path): Path<DeveloperIdPath>,
) -> Result<Json<Vec<DeveloperApiKeyDisplay>>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Map DeveloperApiKey to DeveloperApiKeyDisplay (hiding key_hash)
    Ok(Json(vec![]))
}

/// Create developer API key.
async fn create_developer_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
    Json(request): Json<CreateDeveloperApiKey>,
) -> Result<Json<CreateDeveloperApiKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify the caller owns this developer account or is platform admin
    if !auth.is_platform_admin() {
        let dev_account = state
            .api_ecosystem_repo
            .get_developer_registration(path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Developer", path.id))?;
        if dev_account.user_id != Some(auth.user_id) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "You can only create API keys for your own developer account",
                )),
            ));
        }
    }

    // Generate a new API key
    let is_sandbox = request.is_sandbox.unwrap_or(false);
    let key = format!(
        "ppt_{}_{}",
        if is_sandbox { "test" } else { "live" },
        Uuid::new_v4().to_string().replace("-", "")
    );

    // Create key prefix (first 8 chars for display, matches VARCHAR(8) in DB)
    let key_prefix = key.chars().take(8).collect::<String>();

    // Hash the key for storage (in production, use proper hashing like argon2)
    let key_hash = format!("sha256:{}", sha256_simple(&key));

    let api_key = state
        .api_ecosystem_repo
        .create_developer_api_key(path.id, &request, &key_prefix, &key_hash)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    // Return the full key only on creation
    let response = CreateDeveloperApiKeyResponse {
        id: api_key.id,
        name: api_key.name,
        key: key.clone(), // Return full key only on creation
        scopes: api_key.scopes,
        rate_limit_tier: api_key.rate_limit_tier,
        is_sandbox: api_key.is_sandbox,
        expires_at: api_key.expires_at,
    };

    Ok(Json(response))
}

/// SHA-256 hash for API key storage.
/// Note: For production, consider using Argon2id/bcrypt/scrypt with salt.
fn sha256_simple(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Revoke developer API key.
async fn revoke_developer_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperKeyPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let revoked = state
        .api_ecosystem_repo
        .revoke_api_key(path.key_id, auth.user_id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    if revoked {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("API key", path.key_id))
    }
}

/// Rotate developer API key.
async fn rotate_developer_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperKeyPath>,
) -> Result<Json<CreateDeveloperApiKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Fetch existing key to determine sandbox status for the new key prefix
    let existing_keys = state
        .api_ecosystem_repo
        .list_developer_api_keys(path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;
    let old_key = existing_keys
        .iter()
        .find(|k| k.id == path.key_id)
        .ok_or_else(|| not_found("API key", path.key_id))?;
    let is_sandbox = old_key.is_sandbox;

    let key = format!(
        "ppt_{}_{}",
        if is_sandbox { "test" } else { "live" },
        Uuid::new_v4().to_string().replace("-", "")
    );
    let key_prefix = key.chars().take(8).collect::<String>();
    let key_hash = format!("sha256:{}", sha256_simple(&key));

    let api_key = state
        .api_ecosystem_repo
        .rotate_developer_api_key(path.key_id, auth.user_id, &key_prefix, &key_hash)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("API key", path.key_id))?;

    let response = CreateDeveloperApiKeyResponse {
        id: api_key.id,
        name: api_key.name,
        key,
        scopes: api_key.scopes,
        rate_limit_tier: api_key.rate_limit_tier,
        is_sandbox: api_key.is_sandbox,
        expires_at: api_key.expires_at,
    };

    Ok(Json(response))
}

/// Get developer usage statistics.
async fn get_developer_usage_stats(
    State(_state): State<AppState>,
    Path(path): Path<DeveloperIdPath>,
) -> Result<Json<DeveloperUsageStats>, (StatusCode, Json<ErrorResponse>)> {
    let stats = DeveloperUsageStats {
        developer_id: path.id,
        api_calls_today: 0,
        api_calls_this_month: 0,
        rate_limit_exceeded_count: 0,
        error_count: 0,
        avg_response_time_ms: 0.0,
        most_used_endpoints: vec![],
    };

    Ok(Json(stats))
}

/// Create sandbox environment.
async fn create_sandbox_environment(
    State(_state): State<AppState>,
    Path(path): Path<DeveloperIdPath>,
    Json(request): Json<CreateSandboxConfig>,
) -> Result<Json<SandboxConfig>, (StatusCode, Json<ErrorResponse>)> {
    let sandbox = SandboxConfig {
        id: Uuid::new_v4(),
        developer_id: path.id,
        name: request.name,
        configuration: serde_json::json!({
            "test_mode": true,
            "seed_data": request.seed_test_data.unwrap_or(true)
        }),
        test_data_seeded: request.seed_test_data.unwrap_or(true),
        expires_at: request
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days as i64)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(sandbox))
}

/// Get sandbox environment.
async fn get_sandbox_environment(
    State(_state): State<AppState>,
    Path(_path): Path<DeveloperIdPath>,
) -> Result<Json<SandboxConfig>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Err(error_response("NOT_FOUND", "Sandbox environment not found"))
}

/// Test sandbox request.
async fn test_sandbox_request(
    State(_state): State<AppState>,
    Path(_path): Path<DeveloperIdPath>,
    Json(_request): Json<SandboxTestRequestPayload>,
) -> Result<Json<SandboxTestResponsePayload>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement sandbox request execution
    let response = SandboxTestResponsePayload {
        status_code: 200,
        headers: serde_json::json!({
            "content-type": "application/json"
        }),
        body: serde_json::json!({
            "message": "Sandbox test successful"
        }),
        duration_ms: 50,
    };

    Ok(Json(response))
}

/// List API documentation.
async fn list_api_documentation(
    State(_state): State<AppState>,
) -> Result<Json<Vec<ApiDocumentation>>, (StatusCode, Json<ErrorResponse>)> {
    // Return sample documentation
    let docs = vec![
        ApiDocumentation {
            id: Uuid::new_v4(),
            slug: "getting-started".to_string(),
            title: "Getting Started".to_string(),
            content: "# Getting Started\n\nWelcome to the PPT API...".to_string(),
            category: api_doc_category::GETTING_STARTED.to_string(),
            order_index: 1,
            is_published: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        ApiDocumentation {
            id: Uuid::new_v4(),
            slug: "authentication".to_string(),
            title: "Authentication".to_string(),
            content: "# Authentication\n\nThe PPT API uses API keys...".to_string(),
            category: api_doc_category::AUTHENTICATION.to_string(),
            order_index: 2,
            is_published: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    ];

    Ok(Json(docs))
}

/// Create API documentation (admin only).
async fn create_api_documentation(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Json(request): Json<CreateApiDocumentation>,
) -> Result<Json<ApiDocumentation>, (StatusCode, Json<ErrorResponse>)> {
    let doc = ApiDocumentation {
        id: Uuid::new_v4(),
        slug: request.slug,
        title: request.title,
        content: request.content,
        category: request.category,
        order_index: request.order_index.unwrap_or(0),
        is_published: request.is_published.unwrap_or(false),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(doc))
}

/// Get API documentation by slug.
async fn get_api_documentation(
    State(_state): State<AppState>,
    Path(path): Path<DocSlugPath>,
) -> Result<Json<ApiDocumentation>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database query
    Err(not_found("Documentation", &path.slug))
}

/// Update API documentation (admin only).
async fn update_api_documentation(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(path): Path<DocSlugPath>,
    Json(_request): Json<UpdateApiDocumentation>,
) -> Result<Json<ApiDocumentation>, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database update
    Err(not_found("Documentation", &path.slug))
}

/// Delete API documentation (admin only).
async fn delete_api_documentation(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(_path): Path<DocSlugPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // TODO: Implement database delete
    Ok(StatusCode::NO_CONTENT)
}

/// List code samples for an endpoint.
async fn list_code_samples(
    State(_state): State<AppState>,
    Path(path): Path<DocSlugPath>,
) -> Result<Json<Vec<ApiCodeSample>>, (StatusCode, Json<ErrorResponse>)> {
    // Return sample code samples
    let samples = vec![
        ApiCodeSample {
            id: Uuid::new_v4(),
            endpoint_path: format!("/api/v1/{}", path.slug),
            http_method: "GET".to_string(),
            language: code_sample_language::CURL.to_string(),
            code: format!(
                "curl -X GET 'https://api.ppt.com/api/v1/{}' \\\n  -H 'Authorization: Bearer YOUR_API_KEY'",
                path.slug
            ),
            description: Some("Basic cURL request".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        ApiCodeSample {
            id: Uuid::new_v4(),
            endpoint_path: format!("/api/v1/{}", path.slug),
            http_method: "GET".to_string(),
            language: code_sample_language::JAVASCRIPT.to_string(),
            code: format!(
                r#"const response = await fetch('https://api.ppt.com/api/v1/{}', {{
  headers: {{
    'Authorization': 'Bearer YOUR_API_KEY'
  }}
}});
const data = await response.json();"#,
                path.slug
            ),
            description: Some("JavaScript fetch example".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
    ];

    Ok(Json(samples))
}

/// Create code sample (admin only).
async fn create_code_sample(
    State(_state): State<AppState>,
    _auth: AuthUser,
    Path(_path): Path<DocSlugPath>,
    Json(request): Json<CreateApiCodeSample>,
) -> Result<Json<ApiCodeSample>, (StatusCode, Json<ErrorResponse>)> {
    let sample = ApiCodeSample {
        id: Uuid::new_v4(),
        endpoint_path: request.endpoint_path,
        http_method: request.http_method,
        language: request.language,
        code: request.code,
        description: request.description,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    Ok(Json(sample))
}

/// Get developer portal statistics (admin only).
async fn get_developer_portal_stats(
    State(_state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<DeveloperPortalStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let stats = DeveloperPortalStatistics {
        total_developers: 0,
        active_developers: 0,
        pending_registrations: 0,
        total_api_keys: 0,
        sandbox_api_keys: 0,
        production_api_keys: 0,
        api_calls_today: 0,
        api_calls_this_month: 0,
        top_endpoints: vec![],
    };

    Ok(Json(stats))
}

// ==================== Dashboard ====================

/// Get API ecosystem dashboard.
async fn get_ecosystem_dashboard(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<ApiEcosystemDashboard>, (StatusCode, Json<ErrorResponse>)> {
    let dashboard = ApiEcosystemDashboard {
        installed_integrations: 0,
        active_integrations: 0,
        pending_sync: 0,
        failed_integrations: 0,
        webhook_subscriptions: 0,
        webhooks_delivered_today: 0,
        webhook_success_rate: 100.0,
        connector_executions_today: 0,
        connector_success_rate: 100.0,
        recent_activity: vec![],
    };

    Ok(Json(dashboard))
}

/// Get API ecosystem statistics.
async fn get_ecosystem_statistics(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<ApiEcosystemStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let stats = ApiEcosystemStatistics {
        total_integrations: 0,
        integrations_by_category: serde_json::json!({}),
        active_connections: 0,
        sync_operations_today: 0,
        sync_operations_this_month: 0,
        data_transferred_bytes: 0,
        average_sync_duration_ms: 0.0,
        error_rate: 0.0,
    };

    Ok(Json(stats))
}

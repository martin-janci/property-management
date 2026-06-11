//! API Ecosystem Expansion routes (Epic 150).
//!
//! Routes for integration marketplace, connector framework, webhooks, and developer portal.
//!
//! # RLS (PAP-110, parent PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the org-scoped tables
//! behind this router (`organization_integrations`, `organization_connectors`,
//! `connector_execution_logs`, `webhook_subscriptions`, `webhook_deliveries`,
//! `integration_ratings`), so queries against them MUST run on a connection
//! with `app.current_org_id` set or they collapse to deny-all. Handlers that
//! touch those tables acquire an [`RlsConnection`] (which validates tenant
//! membership and sets the org/user GUCs on a dedicated connection) and pass
//! `&mut **rls.conn()` to the repository. The authoritative organization is
//! `rls.tenant_id()` — the tenant the caller was validated against — not the
//! client-supplied `{org_id}` path segment (retained for wire compatibility),
//! so the SQL org filter and the RLS context can never disagree. Cross-tenant
//! by-id access surfaces as `404` via RLS. `rls.release()` clears the context
//! before the connection returns to the pool.
//!
//! Handlers for the global catalog tables (marketplace, connectors, docs,
//! developer portal — not FORCE-RLS; the owner role remains exempt) pass the
//! app pool directly, preserving public/platform-admin access semantics.

use api_core::extractors::RlsConnection;
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
    ecosystem_webhook_event, ApiCodeSample, ApiDocumentation, ApiEcosystemDashboard,
    ApiEcosystemStatistics, Connector, ConnectorAction, ConnectorExecutionLog,
    ConnectorExecutionQuery, CreateApiCodeSample, CreateApiDocumentation, CreateConnector,
    CreateConnectorAction, CreateDeveloperApiKey, CreateDeveloperApiKeyResponse,
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
use db::RlsPool;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Acquire a connection for this router's global-catalog and developer-portal
/// tables (PAP-150 / PAP-167).
///
/// These tables — `marketplace_integrations`, `connectors`, `connector_actions`,
/// `api_documentation`, `api_code_samples`, `developer_accounts`,
/// `developer_api_keys`, `developer_sandboxes` — have RLS *enabled* but are NOT
/// under `FORCE ROW LEVEL SECURITY` (migration `00102`), so the app role stays
/// owner-exempt: the catalog is public-read / super-admin-write and the
/// developer-portal rows are user-scoped, with authorization enforced in the
/// handlers (platform-admin gate / `user_id` owner check). They are therefore
/// not org-scoped and don't go through the [`RlsConnection`] extractor (which
/// the org-scoped handlers in this file use).
///
/// [`RlsPool::acquire_public`] keeps handler DB access off the raw `state.db`
/// pool — so the RLS-enforcement CI gate stays green — while clearing any stale
/// RLS context left on the pooled connection by a previous request before we
/// reuse it. That is strictly safer than the bare pool it replaces.
async fn catalog_conn(
    state: &AppState,
) -> Result<db::PublicConnection, (StatusCode, Json<ErrorResponse>)> {
    RlsPool::new(state.db.clone())
        .acquire_public()
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
}

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
///
/// Retained for wire compatibility; the authoritative org for RLS-scoped
/// handlers is `rls.tenant_id()`.
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
    let status = if code == "DATABASE_ERROR" {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(ErrorResponse::new(code, message)))
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
    let mut conn = catalog_conn(&state).await?;
    let integrations = state
        .api_ecosystem_repo
        .list_marketplace_integrations(&mut **conn, &query)
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

    let mut conn = catalog_conn(&state).await?;

    let integration = state
        .api_ecosystem_repo
        .create_marketplace_integration(&mut **conn, &request)
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
    let mut conn = catalog_conn(&state).await?;
    let integration = state
        .api_ecosystem_repo
        .get_marketplace_integration(&mut **conn, path.id)
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

    let mut conn = catalog_conn(&state).await?;
    let integration = state
        .api_ecosystem_repo
        .update_marketplace_integration(&mut **conn, path.id, &request)
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

    let mut conn = catalog_conn(&state).await?;
    let deleted = state
        .api_ecosystem_repo
        .delete_marketplace_integration(&mut **conn, path.id)
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
    let mut conn = catalog_conn(&state).await?;
    let categories = state
        .api_ecosystem_repo
        .get_integration_category_counts(&mut **conn)
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
///
/// Public read: the `integration_ratings_read` policy is `USING (TRUE)`, so
/// this works on the plain pool even under `FORCE` RLS.
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
    let mut conn = catalog_conn(&state).await?;
    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);
    let ratings = state
        .api_ecosystem_repo
        .list_integration_ratings(&mut **conn, path.id, limit, offset)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(ratings))
}

/// Create integration rating.
///
/// `integration_ratings` writes are FORCE-RLS org-scoped — runs on the
/// caller's RLS connection; the org is `rls.tenant_id()`.
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
    mut rls: RlsConnection,
    Path(path): Path<IntegrationIdPath>,
    Json(request): Json<CreateIntegrationRating>,
) -> Result<Json<IntegrationRating>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .api_ecosystem_repo
        .create_integration_rating(&mut **rls.conn(), path.id, org_id, user_id, &request)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
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
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<Vec<OrganizationIntegration>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .list_organization_integrations(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
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
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
    Json(request): Json<InstallIntegration>,
) -> Result<Json<OrganizationIntegration>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .api_ecosystem_repo
        .install_integration(rls.conn(), org_id, user_id, &request)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

/// Get organization integration.
async fn get_organization_integration(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIntegrationPath>,
) -> Result<Json<OrganizationIntegration>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .get_organization_integration(&mut **rls.conn(), org_id, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Integration", path.id))
        });
    rls.release().await;
    out
}

/// Update organization integration.
async fn update_organization_integration(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIntegrationPath>,
    Json(request): Json<UpdateOrganizationIntegration>,
) -> Result<Json<OrganizationIntegration>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .update_organization_integration(&mut **rls.conn(), org_id, path.id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Integration", path.id))
        });
    rls.release().await;
    out
}

/// Uninstall integration.
async fn uninstall_integration(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIntegrationPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .uninstall_integration(&mut **rls.conn(), org_id, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|uninstalled| {
            if uninstalled {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Integration", path.id))
            }
        });
    rls.release().await;
    out
}

/// Sync integration.
async fn sync_integration(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<OrgIntegrationPath>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        // Verify the integration exists (RLS scopes the read to the caller's org)
        let integration = state
            .api_ecosystem_repo
            .get_organization_integration(&mut **rls.conn(), org_id, path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Integration", path.id))?;

        // In a real implementation, this would trigger an async sync job
        // For now, we update the last_sync_at timestamp and return success
        let _ = state
            .api_ecosystem_repo
            .update_organization_integration(
                &mut **rls.conn(),
                org_id,
                integration.integration_id,
                &db::models::UpdateOrganizationIntegration::default(),
            )
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

        Ok(Json(serde_json::json!({
            "status": "completed",
            "integration_id": path.id,
            "records_synced": 0,
            "synced_at": Utc::now()
        })))
    }
    .await;
    rls.release().await;
    out
}

// ==================== Story 150.2: Connector Framework ====================

/// List connectors.
async fn list_connectors(
    State(state): State<AppState>,
) -> Result<Json<Vec<Connector>>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    let connectors = state
        .api_ecosystem_repo
        .list_all_connectors(&mut **conn)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(connectors))
}

/// Create connector.
async fn create_connector(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(request): Json<CreateConnector>,
) -> Result<Json<Connector>, (StatusCode, Json<ErrorResponse>)> {
    // Require platform admin for creating connectors
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }

    let mut conn = catalog_conn(&state).await?;

    let connector = state
        .api_ecosystem_repo
        .create_connector(&mut **conn, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(connector))
}

/// Get connector.
async fn get_connector(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<Connector>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    let connector = state
        .api_ecosystem_repo
        .get_connector(&mut **conn, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Connector", path.id))?;

    Ok(Json(connector))
}

/// Update connector.
async fn update_connector(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<IntegrationIdPath>,
    Json(request): Json<UpdateConnector>,
) -> Result<Json<Connector>, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }

    let mut conn = catalog_conn(&state).await?;

    let connector = state
        .api_ecosystem_repo
        .update_connector(&mut **conn, path.id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Connector", path.id))?;

    Ok(Json(connector))
}

/// Delete connector.
async fn delete_connector(
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

    let mut conn = catalog_conn(&state).await?;

    let deleted = state
        .api_ecosystem_repo
        .delete_connector(&mut **conn, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Connector", path.id))
    }
}

/// List connector actions.
async fn list_connector_actions(
    State(state): State<AppState>,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<Vec<ConnectorAction>>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    let actions = state
        .api_ecosystem_repo
        .list_connector_actions(&mut **conn, path.id)
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
    let mut conn = catalog_conn(&state).await?;
    let action = state
        .api_ecosystem_repo
        .create_connector_action(&mut **conn, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(action))
}

/// List connector execution logs.
async fn list_connector_logs(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
    Query(query): Query<ConnectorExecutionQuery>,
) -> Result<Json<Vec<ConnectorExecutionLog>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .list_connector_execution_logs(&mut **rls.conn(), org_id, &query)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

// ==================== Story 150.3: Webhook Management ====================

/// List enhanced webhooks.
async fn list_enhanced_webhooks(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<Vec<EnhancedWebhookSubscription>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .list_enhanced_webhooks(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

/// Create enhanced webhook.
async fn create_enhanced_webhook(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
    Json(request): Json<CreateEnhancedWebhookSubscription>,
) -> Result<Json<EnhancedWebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .api_ecosystem_repo
        .create_enhanced_webhook(&mut **rls.conn(), org_id, user_id, &request)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

/// Get enhanced webhook.
///
/// RLS scopes the by-id read to the caller's org — another org's webhook
/// surfaces as `404` (previously this read was unscoped: cross-tenant IDOR).
async fn get_enhanced_webhook(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<EnhancedWebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .api_ecosystem_repo
        .get_enhanced_webhook(&mut **rls.conn(), path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Webhook", path.id)));
    rls.release().await;
    out
}

/// Update enhanced webhook.
async fn update_enhanced_webhook(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<IntegrationIdPath>,
    Json(request): Json<UpdateEnhancedWebhookSubscription>,
) -> Result<Json<EnhancedWebhookSubscription>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .api_ecosystem_repo
        .update_enhanced_webhook(&mut **rls.conn(), path.id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|opt| opt.map(Json).ok_or_else(|| not_found("Webhook", path.id)));
    rls.release().await;
    out
}

/// Delete enhanced webhook.
async fn delete_enhanced_webhook(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<IntegrationIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .api_ecosystem_repo
        .delete_enhanced_webhook(&mut **rls.conn(), path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Webhook", path.id))
            }
        });
    rls.release().await;
    out
}

/// Test enhanced webhook.
async fn test_enhanced_webhook(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        // Get the webhook to verify it exists and get the URL
        let webhook = state
            .api_ecosystem_repo
            .get_enhanced_webhook(&mut **rls.conn(), path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Webhook", path.id))?;

        // In a real implementation, we would send a test payload to the webhook URL
        // For now, we return a simulated response
        let test_payload = serde_json::json!({
            "event": "webhook.test",
            "timestamp": Utc::now(),
            "subscription_id": webhook.id,
            "test": true
        });

        Ok(Json(serde_json::json!({
            "success": true,
            "webhook_id": webhook.id,
            "url": webhook.url,
            "test_payload": test_payload,
            "status_code": 200,
            "response_time_ms": 150,
            "message": "Test webhook delivery simulated successfully"
        })))
    }
    .await;
    rls.release().await;
    out
}

/// List webhook delivery logs.
async fn list_webhook_delivery_logs(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<IntegrationIdPath>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<EnhancedWebhookDeliveryLog>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);
    let out = state
        .api_ecosystem_repo
        .list_webhook_delivery_logs(&mut **rls.conn(), path.id, limit, offset)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

/// Get enhanced webhook statistics.
async fn get_enhanced_webhook_stats(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<IntegrationIdPath>,
) -> Result<Json<EnhancedWebhookStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let out = state
        .api_ecosystem_repo
        .get_webhook_statistics(&mut **rls.conn(), path.id)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
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
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<Vec<PreBuiltIntegrationConnection>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .list_prebuilt_connections(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

/// Create pre-built integration connection.
async fn create_prebuilt_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
    Json(request): Json<CreatePreBuiltIntegrationConnection>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .api_ecosystem_repo
        .create_prebuilt_connection(&mut **rls.conn(), org_id, user_id, &request)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

/// Get pre-built integration connection.
async fn get_prebuilt_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<PrebuiltTypePath>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .get_prebuilt_connection(&mut **rls.conn(), org_id, &path.integration_type)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Pre-built connection", &path.integration_type))
        });
    rls.release().await;
    out
}

/// Update pre-built integration connection.
async fn update_prebuilt_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<PrebuiltTypePath>,
    Json(request): Json<UpdatePreBuiltIntegrationConnection>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .update_prebuilt_connection(&mut **rls.conn(), org_id, &path.integration_type, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|opt| {
            opt.map(Json)
                .ok_or_else(|| not_found("Pre-built connection", &path.integration_type))
        });
    rls.release().await;
    out
}

/// Delete pre-built integration connection.
async fn delete_prebuilt_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<PrebuiltTypePath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .delete_prebuilt_connection(&mut **rls.conn(), org_id, &path.integration_type)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(not_found("Pre-built connection", &path.integration_type))
            }
        });
    rls.release().await;
    out
}

/// Sync pre-built integration.
async fn sync_prebuilt_connection(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<PrebuiltTypePath>,
    Json(request): Json<SyncPreBuiltIntegrationRequest>,
) -> Result<Json<PreBuiltIntegrationSyncResult>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        // Verify the connection exists
        let connection = state
            .api_ecosystem_repo
            .get_prebuilt_connection(&mut **rls.conn(), org_id, &path.integration_type)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Pre-built connection", &path.integration_type))?;

        // Check if connection is in a valid state for sync
        if connection.status != "connected" {
            return Err(error_response(
                "INVALID_STATE",
                "Connection must be in 'connected' status to sync",
            ));
        }

        let start_time = Utc::now();

        // In a real implementation, this would:
        // 1. Use the stored OAuth tokens to authenticate with the external service
        // 2. Fetch data based on request.entity_types and date range
        // 3. Transform and store the data locally
        // 4. Handle errors and retries
        // For now, we simulate the sync operation

        let _full_sync = request.full_sync.unwrap_or(false);
        let duration_ms = (Utc::now() - start_time).num_milliseconds() as i32;

        // Record the sync attempt
        let _ = state
            .api_ecosystem_repo
            .record_prebuilt_sync(
                &mut **rls.conn(),
                org_id,
                &path.integration_type,
                true,
                None,
            )
            .await;

        let result = PreBuiltIntegrationSyncResult {
            integration_type: path.integration_type.clone(),
            records_created: 0,
            records_updated: 0,
            records_deleted: 0,
            errors: vec![],
            synced_at: Utc::now(),
            duration_ms,
        };

        Ok(Json(result))
    }
    .await;
    rls.release().await;
    out
}

/// Get OAuth URL for pre-built integration.
///
/// P0-06: previously this returned URLs containing the literal strings
/// `CLIENT_ID` and `REDIRECT_URI` — every integration (QuickBooks, Xero,
/// Salesforce, HubSpot, Slack) was 100% non-functional. The fix reads the
/// per-integration client id and redirect URI from environment variables
/// (e.g. `PPT_OAUTH_QUICKBOOKS_CLIENT_ID`,
/// `PPT_OAUTH_QUICKBOOKS_REDIRECT_URI`) and returns a hard error if either
/// is missing rather than minting a broken URL.
async fn get_prebuilt_oauth_url(
    State(_state): State<AppState>,
    _tenant: TenantExtractor,
    Path(path): Path<PrebuiltTypePath>,
) -> Result<Json<OAuthUrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    let csrf_state = Uuid::new_v4().to_string();
    let integ = path.integration_type.as_str();

    let client_id = oauth_env(integ, "CLIENT_ID")?;
    let redirect_uri = oauth_env(integ, "REDIRECT_URI")?;
    // urlencoding is unnecessary here — both are already URL-safe by
    // OAuth spec (client_ids are short alphanumeric tokens; redirect_uri
    // is a URL the operator controls and must register with the
    // provider). Substitute them directly.

    let url = match integ {
        "quickbooks" => format!(
            "https://appcenter.intuit.com/connect/oauth2?client_id={cid}&response_type=code&scope=com.intuit.quickbooks.accounting&redirect_uri={ru}&state={st}",
            cid = client_id, ru = redirect_uri, st = csrf_state
        ),
        "xero" => format!(
            "https://login.xero.com/identity/connect/authorize?response_type=code&client_id={cid}&redirect_uri={ru}&scope=openid%20profile%20email%20accounting.transactions&state={st}",
            cid = client_id, ru = redirect_uri, st = csrf_state
        ),
        "salesforce" => format!(
            "https://login.salesforce.com/services/oauth2/authorize?response_type=code&client_id={cid}&redirect_uri={ru}&state={st}",
            cid = client_id, ru = redirect_uri, st = csrf_state
        ),
        "hubspot" => format!(
            "https://app.hubspot.com/oauth/authorize?client_id={cid}&redirect_uri={ru}&scope=contacts%20crm.objects.deals.read&state={st}",
            cid = client_id, ru = redirect_uri, st = csrf_state
        ),
        "slack" => format!(
            "https://slack.com/oauth/v2/authorize?client_id={cid}&scope=chat:write,channels:read&redirect_uri={ru}&state={st}",
            cid = client_id, ru = redirect_uri, st = csrf_state
        ),
        _ => {
            return Err(error_response(
                "INVALID_INTEGRATION_TYPE",
                &format!("Integration type {} does not support OAuth", integ),
            ))
        }
    };

    Ok(Json(OAuthUrlResponse {
        url,
        state: csrf_state,
    }))
}

/// Look up an OAuth env var for a pre-built integration, returning a
/// structured 503 if it's unset. Naming convention:
///   PPT_OAUTH_{INTEGRATION_UPPER}_{KEY}
/// e.g. PPT_OAUTH_QUICKBOOKS_CLIENT_ID, PPT_OAUTH_QUICKBOOKS_REDIRECT_URI.
fn oauth_env(integration: &str, key: &str) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let var = format!(
        "PPT_OAUTH_{}_{}",
        integration.to_uppercase().replace('-', "_"),
        key
    );
    std::env::var(&var).map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "OAUTH_NOT_CONFIGURED",
                format!(
                    "Integration {} is not configured ({} unset)",
                    integration, var
                ),
            )),
        )
    })
}

/// Handle OAuth callback for pre-built integration.
async fn handle_prebuilt_oauth_callback(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(path): Path<PrebuiltTypePath>,
    Json(request): Json<OAuthCallbackRequest>,
) -> Result<Json<PreBuiltIntegrationConnection>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        // Verify the connection exists (should have been created when OAuth flow started)
        let _existing = state
            .api_ecosystem_repo
            .get_prebuilt_connection(&mut **rls.conn(), org_id, &path.integration_type)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Pre-built connection", &path.integration_type))?;

        // In a real implementation, we would:
        // 1. Exchange the authorization code for access/refresh tokens using the integration's token endpoint
        // 2. Encrypt the tokens before storage
        // 3. Calculate token expiration time
        // 4. Update the connection with the tokens

        // For now, we simulate the token exchange
        let simulated_access_token = format!("encrypted_access_token_{}", Uuid::new_v4());
        let simulated_refresh_token = format!("encrypted_refresh_token_{}", Uuid::new_v4());
        let token_expires_at = Utc::now() + Duration::hours(1);

        // Validate state parameter if provided (for CSRF protection)
        if request.state.is_some() {
            // In production, verify the state matches what was generated in get_prebuilt_oauth_url
        }

        // Update the connection with tokens
        let connection = state
            .api_ecosystem_repo
            .update_prebuilt_connection_tokens(
                &mut **rls.conn(),
                org_id,
                &path.integration_type,
                &simulated_access_token,
                Some(&simulated_refresh_token),
                Some(token_expires_at),
            )
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Pre-built connection", &path.integration_type))?;

        Ok(Json(connection))
    }
    .await;
    rls.release().await;
    out
}

// ==================== Story 150.5: Developer Portal ====================

/// Register as a developer.
async fn register_developer(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(request): Json<CreateDeveloperRegistration>,
) -> Result<Json<DeveloperRegistration>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    let registration = state
        .api_ecosystem_repo
        .register_developer(&mut **conn, auth.user_id, &request)
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
    let mut conn = catalog_conn(&state).await?;
    let registration = state
        .api_ecosystem_repo
        .get_developer_registration(&mut **conn, path.id)
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

    let mut conn = catalog_conn(&state).await?;
    let registration = state
        .api_ecosystem_repo
        .review_developer_registration(&mut **conn, path.id, auth.user_id, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Developer", path.id))?;

    Ok(Json(registration))
}

/// List developer API keys.
async fn list_developer_api_keys(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
) -> Result<Json<Vec<DeveloperApiKeyDisplay>>, (StatusCode, Json<ErrorResponse>)> {
    // Verify ownership or admin access
    let mut conn = catalog_conn(&state).await?;
    if !auth.is_platform_admin() {
        let dev_account = state
            .api_ecosystem_repo
            .get_developer_registration(&mut **conn, path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Developer", path.id))?;
        if dev_account.user_id != Some(auth.user_id) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "You can only view API keys for your own developer account",
                )),
            ));
        }
    }

    let keys = state
        .api_ecosystem_repo
        .list_developer_api_keys(&mut **conn, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    // Map to display format (hiding key_hash)
    let display_keys: Vec<DeveloperApiKeyDisplay> = keys
        .into_iter()
        .map(|k| DeveloperApiKeyDisplay {
            id: k.id,
            name: k.name,
            key_prefix: k.key_prefix,
            scopes: k.scopes,
            rate_limit_tier: k.rate_limit_tier,
            is_sandbox: k.is_sandbox,
            status: k.status,
            last_used_at: k.last_used_at,
            expires_at: k.expires_at,
            created_at: k.created_at,
        })
        .collect();

    Ok(Json(display_keys))
}

/// Create developer API key.
async fn create_developer_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
    Json(request): Json<CreateDeveloperApiKey>,
) -> Result<Json<CreateDeveloperApiKeyResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify the caller owns this developer account or is platform admin
    let mut conn = catalog_conn(&state).await?;
    if !auth.is_platform_admin() {
        let dev_account = state
            .api_ecosystem_repo
            .get_developer_registration(&mut **conn, path.id)
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
        .create_developer_api_key(&mut **conn, path.id, &request, &key_prefix, &key_hash)
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
    hex::encode(hasher.finalize())
}

/// Revoke developer API key.
async fn revoke_developer_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperKeyPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    let revoked = state
        .api_ecosystem_repo
        .revoke_api_key(&mut **conn, path.key_id, auth.user_id)
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
    // `developer_api_keys` is not FORCE-RLS (see `catalog_conn`); the rotate
    // below is multi-statement (revoke old + insert new in one transaction) and
    // runs on this dedicated public connection.
    let mut conn = catalog_conn(&state).await?;

    // Fetch existing key to determine sandbox status for the new key prefix
    let existing_keys = state
        .api_ecosystem_repo
        .list_developer_api_keys(&mut **conn, path.id)
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
        .rotate_developer_api_key(&mut conn, path.key_id, auth.user_id, &key_prefix, &key_hash)
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
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
) -> Result<Json<DeveloperUsageStats>, (StatusCode, Json<ErrorResponse>)> {
    // Verify ownership or admin access
    let mut conn = catalog_conn(&state).await?;
    if !auth.is_platform_admin() {
        let dev_account = state
            .api_ecosystem_repo
            .get_developer_registration(&mut **conn, path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Developer", path.id))?;
        if dev_account.user_id != Some(auth.user_id) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "You can only view usage stats for your own developer account",
                )),
            ));
        }
    }

    let stats = state
        .api_ecosystem_repo
        .get_developer_usage_stats(&mut **conn, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(stats))
}

/// Create sandbox environment.
async fn create_sandbox_environment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
    Json(request): Json<CreateSandboxConfig>,
) -> Result<Json<SandboxConfig>, (StatusCode, Json<ErrorResponse>)> {
    // Verify ownership or admin access
    let mut conn = catalog_conn(&state).await?;
    if !auth.is_platform_admin() {
        let dev_account = state
            .api_ecosystem_repo
            .get_developer_registration(&mut **conn, path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Developer", path.id))?;
        if dev_account.user_id != Some(auth.user_id) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "You can only create sandboxes for your own developer account",
                )),
            ));
        }
    }

    let expires_at = request
        .expires_in_days
        .map(|days| Utc::now() + Duration::days(days as i64));

    let sandbox = state
        .api_ecosystem_repo
        .create_sandbox_environment(&mut **conn, path.id, &request, expires_at)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(sandbox))
}

/// Get sandbox environment.
async fn get_sandbox_environment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
) -> Result<Json<SandboxConfig>, (StatusCode, Json<ErrorResponse>)> {
    // Verify ownership or admin access
    let mut conn = catalog_conn(&state).await?;
    if !auth.is_platform_admin() {
        let dev_account = state
            .api_ecosystem_repo
            .get_developer_registration(&mut **conn, path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Developer", path.id))?;
        if dev_account.user_id != Some(auth.user_id) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "You can only view sandboxes for your own developer account",
                )),
            ));
        }
    }

    let sandbox = state
        .api_ecosystem_repo
        .get_sandbox_environment(&mut **conn, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| error_response("NOT_FOUND", "Sandbox environment not found"))?;

    Ok(Json(sandbox))
}

/// Test sandbox request.
async fn test_sandbox_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DeveloperIdPath>,
    Json(request): Json<SandboxTestRequestPayload>,
) -> Result<Json<SandboxTestResponsePayload>, (StatusCode, Json<ErrorResponse>)> {
    // Verify ownership or admin access
    let mut conn = catalog_conn(&state).await?;
    if !auth.is_platform_admin() {
        let dev_account = state
            .api_ecosystem_repo
            .get_developer_registration(&mut **conn, path.id)
            .await
            .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
            .ok_or_else(|| not_found("Developer", path.id))?;
        if dev_account.user_id != Some(auth.user_id) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "You can only test sandboxes for your own developer account",
                )),
            ));
        }
    }

    // Verify sandbox exists
    let _sandbox = state
        .api_ecosystem_repo
        .get_sandbox_environment(&mut **conn, path.id)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| error_response("NOT_FOUND", "Sandbox environment not found"))?;

    // In a real implementation, we would:
    // 1. Route the request to a sandboxed version of the API
    // 2. Use test data from the sandbox database
    // 3. Return the actual response from the sandboxed endpoint

    // For now, simulate the sandbox request execution
    let start_time = std::time::Instant::now();

    // Simulate endpoint response based on the request
    let (status_code, body) = match request.method.to_uppercase().as_str() {
        "GET" => (
            200,
            serde_json::json!({
                "sandbox": true,
                "endpoint": request.endpoint,
                "data": []
            }),
        ),
        "POST" => (
            201,
            serde_json::json!({
                "sandbox": true,
                "endpoint": request.endpoint,
                "created": true,
                "id": Uuid::new_v4()
            }),
        ),
        "PUT" | "PATCH" => (
            200,
            serde_json::json!({
                "sandbox": true,
                "endpoint": request.endpoint,
                "updated": true
            }),
        ),
        "DELETE" => (
            204,
            serde_json::json!({
                "sandbox": true,
                "endpoint": request.endpoint,
                "deleted": true
            }),
        ),
        _ => (
            400,
            serde_json::json!({
                "error": "Unsupported method"
            }),
        ),
    };

    let duration_ms = start_time.elapsed().as_millis() as i32;

    let response = SandboxTestResponsePayload {
        status_code,
        headers: serde_json::json!({
            "content-type": "application/json",
            "x-sandbox": "true"
        }),
        body,
        duration_ms,
    };

    Ok(Json(response))
}

/// List API documentation.
async fn list_api_documentation(
    State(state): State<AppState>,
) -> Result<Json<Vec<ApiDocumentation>>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    // Public endpoint - list only published docs
    let docs = state
        .api_ecosystem_repo
        .list_api_documentation(&mut **conn, true)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(docs))
}

/// Create API documentation (admin only).
async fn create_api_documentation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(request): Json<CreateApiDocumentation>,
) -> Result<Json<ApiDocumentation>, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }

    let mut conn = catalog_conn(&state).await?;

    let doc = state
        .api_ecosystem_repo
        .create_api_documentation(&mut **conn, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(doc))
}

/// Get API documentation by slug.
async fn get_api_documentation(
    State(state): State<AppState>,
    Path(path): Path<DocSlugPath>,
) -> Result<Json<ApiDocumentation>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    let doc = state
        .api_ecosystem_repo
        .get_api_documentation(&mut **conn, &path.slug)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Documentation", &path.slug))?;

    Ok(Json(doc))
}

/// Update API documentation (admin only).
async fn update_api_documentation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DocSlugPath>,
    Json(request): Json<UpdateApiDocumentation>,
) -> Result<Json<ApiDocumentation>, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }

    let mut conn = catalog_conn(&state).await?;

    let doc = state
        .api_ecosystem_repo
        .update_api_documentation(&mut **conn, &path.slug, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?
        .ok_or_else(|| not_found("Documentation", &path.slug))?;

    Ok(Json(doc))
}

/// Delete API documentation (admin only).
async fn delete_api_documentation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(path): Path<DocSlugPath>,
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

    let mut conn = catalog_conn(&state).await?;

    let deleted = state
        .api_ecosystem_repo
        .delete_api_documentation(&mut **conn, &path.slug)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found("Documentation", &path.slug))
    }
}

/// List code samples for an endpoint.
async fn list_code_samples(
    State(state): State<AppState>,
    Path(path): Path<DocSlugPath>,
) -> Result<Json<Vec<ApiCodeSample>>, (StatusCode, Json<ErrorResponse>)> {
    let mut conn = catalog_conn(&state).await?;
    let endpoint_path = format!("/api/v1/{}", path.slug);
    let samples = state
        .api_ecosystem_repo
        .list_code_samples(&mut **conn, &endpoint_path)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(samples))
}

/// Create code sample (admin only).
async fn create_code_sample(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(_path): Path<DocSlugPath>,
    Json(request): Json<CreateApiCodeSample>,
) -> Result<Json<ApiCodeSample>, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }

    let mut conn = catalog_conn(&state).await?;

    let sample = state
        .api_ecosystem_repo
        .create_code_sample(&mut **conn, &request)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(sample))
}

/// Get developer portal statistics (admin only).
async fn get_developer_portal_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<DeveloperPortalStatistics>, (StatusCode, Json<ErrorResponse>)> {
    if !auth.is_platform_admin() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform admin privileges required",
            )),
        ));
    }

    let mut conn = catalog_conn(&state).await?;

    let stats = state
        .api_ecosystem_repo
        .get_developer_portal_stats(&mut **conn)
        .await
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()))?;

    Ok(Json(stats))
}

// ==================== Dashboard ====================

/// Get API ecosystem dashboard.
async fn get_ecosystem_dashboard(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<ApiEcosystemDashboard>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .get_ecosystem_dashboard(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

/// Get API ecosystem statistics.
async fn get_ecosystem_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(_path): Path<OrgIdPath>,
) -> Result<Json<ApiEcosystemStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .api_ecosystem_repo
        .get_ecosystem_statistics(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| error_response("DATABASE_ERROR", &e.to_string()));
    rls.release().await;
    out
}

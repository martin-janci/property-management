//! API Ecosystem repository (Epic 150).
//!
//! Provides database operations for Integration Marketplace, Connector Framework,
//! Webhooks, and Developer Portal.

use crate::models::api_ecosystem::*;
use crate::DbPool;
use common::AppError;
use uuid::Uuid;

/// Repository for API Ecosystem operations.
#[derive(Clone)]
pub struct ApiEcosystemRepository {
    pool: DbPool,
}

impl ApiEcosystemRepository {
    /// Create a new repository instance.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ============================================
    // Story 150.1: Integration Marketplace
    // ============================================

    /// List marketplace integrations with optional filtering.
    pub async fn list_marketplace_integrations(
        &self,
        query: &MarketplaceIntegrationQuery,
    ) -> Result<Vec<MarketplaceIntegrationSummary>, AppError> {
        let limit = query.limit.unwrap_or(50).min(100) as i64;
        let offset = query.offset.unwrap_or(0) as i64;

        let integrations = sqlx::query_as::<_, MarketplaceIntegrationSummary>(
            r#"
            SELECT id, slug, name, description, category, icon_url, vendor_name,
                   status, rating_average, rating_count, install_count, is_featured, is_premium
            FROM marketplace_integrations
            WHERE ($1::text IS NULL OR category = $1)
              AND ($2::text IS NULL OR status = $2)
              AND ($3::bool IS NULL OR is_featured = $3)
              AND ($4::bool IS NULL OR is_premium = $4)
              AND ($5::text IS NULL OR name ILIKE '%' || $5 || '%' OR description ILIKE '%' || $5 || '%')
            ORDER BY
                CASE WHEN $6 = 'rating' THEN rating_average END DESC NULLS LAST,
                CASE WHEN $6 = 'installs' THEN install_count END DESC,
                CASE WHEN $6 = 'created' THEN created_at END DESC,
                CASE WHEN $6 IS NULL OR $6 = 'name' THEN name END ASC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(query.category.as_ref())
        .bind(query.status.as_ref())
        .bind(query.featured_only)
        .bind(query.premium_only)
        .bind(query.search.as_ref())
        .bind(query.sort_by.as_ref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integrations)
    }

    /// Get a marketplace integration by ID.
    pub async fn get_marketplace_integration(
        &self,
        id: Uuid,
    ) -> Result<Option<MarketplaceIntegration>, AppError> {
        let integration = sqlx::query_as::<_, MarketplaceIntegration>(
            r#"
            SELECT * FROM marketplace_integrations WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Get a marketplace integration by slug.
    pub async fn get_marketplace_integration_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<MarketplaceIntegration>, AppError> {
        let integration = sqlx::query_as::<_, MarketplaceIntegration>(
            r#"
            SELECT * FROM marketplace_integrations WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Create a new marketplace integration (admin only).
    pub async fn create_marketplace_integration(
        &self,
        req: &CreateMarketplaceIntegration,
    ) -> Result<MarketplaceIntegration, AppError> {
        let integration = sqlx::query_as::<_, MarketplaceIntegration>(
            r#"
            INSERT INTO marketplace_integrations (
                slug, name, description, long_description, category, icon_url, banner_url,
                vendor_name, vendor_url, documentation_url, support_url, version,
                features, requirements, pricing_info, is_premium, required_scopes
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING *
            "#,
        )
        .bind(&req.slug)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.long_description)
        .bind(&req.category)
        .bind(&req.icon_url)
        .bind(&req.banner_url)
        .bind(&req.vendor_name)
        .bind(&req.vendor_url)
        .bind(&req.documentation_url)
        .bind(&req.support_url)
        .bind(&req.version)
        .bind(&req.features)
        .bind(&req.requirements)
        .bind(&req.pricing_info)
        .bind(req.is_premium.unwrap_or(false))
        .bind(&req.required_scopes)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Update a marketplace integration.
    pub async fn update_marketplace_integration(
        &self,
        id: Uuid,
        req: &UpdateMarketplaceIntegration,
    ) -> Result<Option<MarketplaceIntegration>, AppError> {
        let integration = sqlx::query_as::<_, MarketplaceIntegration>(
            r#"
            UPDATE marketplace_integrations SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                long_description = COALESCE($4, long_description),
                category = COALESCE($5, category),
                icon_url = COALESCE($6, icon_url),
                banner_url = COALESCE($7, banner_url),
                vendor_url = COALESCE($8, vendor_url),
                documentation_url = COALESCE($9, documentation_url),
                support_url = COALESCE($10, support_url),
                version = COALESCE($11, version),
                status = COALESCE($12, status),
                features = COALESCE($13, features),
                requirements = COALESCE($14, requirements),
                pricing_info = COALESCE($15, pricing_info),
                is_featured = COALESCE($16, is_featured),
                is_premium = COALESCE($17, is_premium),
                required_scopes = COALESCE($18, required_scopes)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.long_description)
        .bind(&req.category)
        .bind(&req.icon_url)
        .bind(&req.banner_url)
        .bind(&req.vendor_url)
        .bind(&req.documentation_url)
        .bind(&req.support_url)
        .bind(&req.version)
        .bind(&req.status)
        .bind(&req.features)
        .bind(&req.requirements)
        .bind(&req.pricing_info)
        .bind(req.is_featured)
        .bind(req.is_premium)
        .bind(&req.required_scopes)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Delete a marketplace integration.
    pub async fn delete_marketplace_integration(&self, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM marketplace_integrations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get integration category counts.
    pub async fn get_integration_category_counts(
        &self,
    ) -> Result<Vec<IntegrationCategoryCount>, AppError> {
        let counts = sqlx::query_as::<_, IntegrationCategoryCount>(
            r#"
            SELECT category, COUNT(*) as count
            FROM marketplace_integrations
            WHERE status = 'available'
            GROUP BY category
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(counts)
    }

    // ============================================
    // Organization Integrations
    // ============================================

    /// Install an integration for an organization.
    pub async fn install_integration(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        req: &InstallIntegration,
    ) -> Result<OrganizationIntegration, AppError> {
        // Encrypt credentials if provided (in production, use proper encryption)
        let credentials_encrypted = req
            .credentials
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_default());

        let installation = sqlx::query_as::<_, OrganizationIntegration>(
            r#"
            INSERT INTO organization_integrations (
                organization_id, integration_id, status, configuration,
                credentials_encrypted, installed_by
            ) VALUES ($1, $2, 'installed', $3, $4, $5)
            ON CONFLICT (organization_id, integration_id)
            DO UPDATE SET
                status = 'installed',
                configuration = COALESCE($3, organization_integrations.configuration),
                credentials_encrypted = COALESCE($4, organization_integrations.credentials_encrypted),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(req.integration_id)
        .bind(&req.configuration)
        .bind(credentials_encrypted)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Update install count
        sqlx::query(
            r#"
            UPDATE marketplace_integrations
            SET install_count = install_count + 1
            WHERE id = $1
            "#,
        )
        .bind(req.integration_id)
        .execute(&self.pool)
        .await
        .ok();

        Ok(installation)
    }

    /// List organization integrations.
    pub async fn list_organization_integrations(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationIntegration>, AppError> {
        let integrations = sqlx::query_as::<_, OrganizationIntegration>(
            r#"
            SELECT * FROM organization_integrations
            WHERE organization_id = $1
            ORDER BY installed_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integrations)
    }

    /// Uninstall an integration.
    pub async fn uninstall_integration(
        &self,
        organization_id: Uuid,
        integration_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE organization_integrations
            SET status = 'uninstalled', enabled = FALSE, updated_at = NOW()
            WHERE organization_id = $1 AND integration_id = $2
            "#,
        )
        .bind(organization_id)
        .bind(integration_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Story 150.2: Connector Framework
    // ============================================

    /// List connectors for an integration.
    pub async fn list_connectors(&self, integration_id: Uuid) -> Result<Vec<Connector>, AppError> {
        let connectors = sqlx::query_as::<_, Connector>(
            r#"
            SELECT * FROM connectors
            WHERE integration_id = $1
            ORDER BY name
            "#,
        )
        .bind(integration_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connectors)
    }

    /// Get a connector by ID.
    pub async fn get_connector(&self, id: Uuid) -> Result<Option<Connector>, AppError> {
        let connector = sqlx::query_as::<_, Connector>(
            r#"
            SELECT * FROM connectors WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connector)
    }

    /// Create a new connector.
    pub async fn create_connector(&self, req: &CreateConnector) -> Result<Connector, AppError> {
        let connector = sqlx::query_as::<_, Connector>(
            r#"
            INSERT INTO connectors (
                integration_id, name, description, auth_type, auth_config, base_url,
                rate_limit_requests, rate_limit_window_seconds, retry_max_attempts,
                retry_initial_delay_ms, retry_max_delay_ms, timeout_ms, headers,
                supported_actions, error_mapping, data_transformations
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(req.integration_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.auth_type)
        .bind(&req.auth_config)
        .bind(&req.base_url)
        .bind(req.rate_limit_requests)
        .bind(req.rate_limit_window_seconds)
        .bind(req.retry_max_attempts.unwrap_or(3))
        .bind(req.retry_initial_delay_ms.unwrap_or(1000))
        .bind(req.retry_max_delay_ms.unwrap_or(30000))
        .bind(req.timeout_ms.unwrap_or(30000))
        .bind(&req.headers)
        .bind(&req.supported_actions)
        .bind(&req.error_mapping)
        .bind(&req.data_transformations)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connector)
    }

    /// Update a connector.
    pub async fn update_connector(
        &self,
        id: Uuid,
        req: &UpdateConnector,
    ) -> Result<Option<Connector>, AppError> {
        let connector = sqlx::query_as::<_, Connector>(
            r#"
            UPDATE connectors SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                auth_config = COALESCE($4, auth_config),
                base_url = COALESCE($5, base_url),
                rate_limit_requests = COALESCE($6, rate_limit_requests),
                rate_limit_window_seconds = COALESCE($7, rate_limit_window_seconds),
                retry_max_attempts = COALESCE($8, retry_max_attempts),
                retry_initial_delay_ms = COALESCE($9, retry_initial_delay_ms),
                retry_max_delay_ms = COALESCE($10, retry_max_delay_ms),
                timeout_ms = COALESCE($11, timeout_ms),
                headers = COALESCE($12, headers),
                supported_actions = COALESCE($13, supported_actions),
                error_mapping = COALESCE($14, error_mapping),
                data_transformations = COALESCE($15, data_transformations)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.auth_config)
        .bind(&req.base_url)
        .bind(req.rate_limit_requests)
        .bind(req.rate_limit_window_seconds)
        .bind(req.retry_max_attempts)
        .bind(req.retry_initial_delay_ms)
        .bind(req.retry_max_delay_ms)
        .bind(req.timeout_ms)
        .bind(&req.headers)
        .bind(&req.supported_actions)
        .bind(&req.error_mapping)
        .bind(&req.data_transformations)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connector)
    }

    /// Delete a connector.
    pub async fn delete_connector(&self, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM connectors WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// List connector actions.
    pub async fn list_connector_actions(
        &self,
        connector_id: Uuid,
    ) -> Result<Vec<ConnectorAction>, AppError> {
        let actions = sqlx::query_as::<_, ConnectorAction>(
            r#"
            SELECT * FROM connector_actions
            WHERE connector_id = $1
            ORDER BY name
            "#,
        )
        .bind(connector_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(actions)
    }

    // ============================================
    // Story 150.3: Enhanced Webhooks
    // ============================================

    /// List enhanced webhook subscriptions for an organization.
    pub async fn list_enhanced_webhooks(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<EnhancedWebhookSubscription>, AppError> {
        let subscriptions = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            SELECT * FROM webhook_subscriptions
            WHERE organization_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscriptions)
    }

    /// Get a webhook subscription by ID.
    pub async fn get_enhanced_webhook(
        &self,
        id: Uuid,
    ) -> Result<Option<EnhancedWebhookSubscription>, AppError> {
        let subscription = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            SELECT * FROM webhook_subscriptions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscription)
    }

    /// Delete a webhook subscription.
    pub async fn delete_enhanced_webhook(&self, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM webhook_subscriptions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Story 150.4: Developer Portal
    // ============================================

    /// Get a developer registration by ID.
    pub async fn get_developer_registration(
        &self,
        id: Uuid,
    ) -> Result<Option<DeveloperRegistration>, AppError> {
        let registration = sqlx::query_as::<_, DeveloperRegistration>(
            r#"
            SELECT * FROM developer_accounts WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// List API keys for a developer.
    pub async fn list_developer_api_keys(
        &self,
        developer_id: Uuid,
    ) -> Result<Vec<DeveloperApiKey>, AppError> {
        let keys = sqlx::query_as::<_, DeveloperApiKey>(
            r#"
            SELECT * FROM developer_api_keys
            WHERE developer_id = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(developer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(keys)
    }

    /// Revoke an API key.
    pub async fn revoke_api_key(&self, key_id: Uuid, revoked_by: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE developer_api_keys SET
                is_active = FALSE,
                revoked_at = NOW(),
                revoked_by = $2
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(key_id)
        .bind(revoked_by)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Story 150.1: Integration Ratings
    // ============================================

    /// List ratings for an integration.
    pub async fn list_integration_ratings(
        &self,
        integration_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<IntegrationRatingWithUser>, AppError> {
        let ratings = sqlx::query_as::<_, IntegrationRatingWithUser>(
            r#"
            SELECT
                ir.id, ir.integration_id,
                COALESCE(u.name, 'Anonymous') as user_name,
                ir.rating, ir.review, ir.helpful_count, ir.created_at
            FROM integration_ratings ir
            LEFT JOIN users u ON ir.user_id = u.id
            WHERE ir.integration_id = $1
            ORDER BY ir.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(integration_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(ratings)
    }

    /// Create an integration rating.
    pub async fn create_integration_rating(
        &self,
        integration_id: Uuid,
        organization_id: Uuid,
        user_id: Uuid,
        req: &CreateIntegrationRating,
    ) -> Result<IntegrationRating, AppError> {
        let rating = sqlx::query_as::<_, IntegrationRating>(
            r#"
            INSERT INTO integration_ratings (
                integration_id, organization_id, user_id, rating, review
            ) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (integration_id, user_id)
            DO UPDATE SET rating = $4, review = $5, updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(integration_id)
        .bind(organization_id)
        .bind(user_id)
        .bind(req.rating)
        .bind(&req.review)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rating)
    }

    /// Get an organization integration by ID.
    pub async fn get_organization_integration(
        &self,
        organization_id: Uuid,
        integration_id: Uuid,
    ) -> Result<Option<OrganizationIntegration>, AppError> {
        let integration = sqlx::query_as::<_, OrganizationIntegration>(
            r#"
            SELECT * FROM organization_integrations
            WHERE organization_id = $1 AND integration_id = $2
            "#,
        )
        .bind(organization_id)
        .bind(integration_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Update an organization integration.
    pub async fn update_organization_integration(
        &self,
        organization_id: Uuid,
        integration_id: Uuid,
        req: &UpdateOrganizationIntegration,
    ) -> Result<Option<OrganizationIntegration>, AppError> {
        let integration = sqlx::query_as::<_, OrganizationIntegration>(
            r#"
            UPDATE organization_integrations SET
                configuration = COALESCE($3, configuration),
                enabled = COALESCE($4, enabled),
                updated_at = NOW()
            WHERE organization_id = $1 AND integration_id = $2
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(integration_id)
        .bind(&req.configuration)
        .bind(req.enabled)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    // ============================================
    // Story 150.2: Connector Actions & Logs
    // ============================================

    /// Create a connector action.
    pub async fn create_connector_action(
        &self,
        req: &CreateConnectorAction,
    ) -> Result<ConnectorAction, AppError> {
        let action = sqlx::query_as::<_, ConnectorAction>(
            r#"
            INSERT INTO connector_actions (
                connector_id, name, description, http_method, endpoint_path,
                request_schema, response_schema, request_transformations,
                response_transformations, pagination_config
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(req.connector_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.http_method)
        .bind(&req.endpoint_path)
        .bind(&req.request_schema)
        .bind(&req.response_schema)
        .bind(&req.request_transformations)
        .bind(&req.response_transformations)
        .bind(&req.pagination_config)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(action)
    }

    /// List connector execution logs.
    pub async fn list_connector_execution_logs(
        &self,
        organization_id: Uuid,
        query: &ConnectorExecutionQuery,
    ) -> Result<Vec<ConnectorExecutionLog>, AppError> {
        let limit = query.limit.unwrap_or(50).min(100) as i64;
        let offset = query.offset.unwrap_or(0) as i64;

        let logs = sqlx::query_as::<_, ConnectorExecutionLog>(
            r#"
            SELECT * FROM connector_execution_logs
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR connector_id = $2)
              AND ($3::text IS NULL OR status = $3)
            ORDER BY executed_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(organization_id)
        .bind(query.connector_id)
        .bind(query.status.as_ref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(logs)
    }

    // ============================================
    // Story 150.3: Enhanced Webhooks (remaining)
    // ============================================

    /// Create an enhanced webhook subscription.
    pub async fn create_enhanced_webhook(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        req: &CreateEnhancedWebhookSubscription,
    ) -> Result<EnhancedWebhookSubscription, AppError> {
        // Serialize retry_policy as JSON
        let retry_policy_json = req
            .retry_policy
            .as_ref()
            .and_then(|rp| serde_json::to_value(rp).ok());

        let subscription = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            INSERT INTO webhook_subscriptions (
                organization_id, name, description, url, auth_type, auth_config,
                events, filters, payload_template, headers, retry_policy,
                rate_limit_requests, rate_limit_window_seconds, timeout_ms, verify_ssl, created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.url)
        .bind(&req.auth_type)
        .bind(&req.auth_config)
        .bind(&req.events)
        .bind(&req.filters)
        .bind(&req.payload_template)
        .bind(&req.headers)
        .bind(&retry_policy_json)
        .bind(req.rate_limit_requests)
        .bind(req.rate_limit_window_seconds)
        .bind(req.timeout_ms.unwrap_or(30000))
        .bind(req.verify_ssl.unwrap_or(true))
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscription)
    }

    /// Update an enhanced webhook subscription.
    pub async fn update_enhanced_webhook(
        &self,
        id: Uuid,
        req: &UpdateEnhancedWebhookSubscription,
    ) -> Result<Option<EnhancedWebhookSubscription>, AppError> {
        // Serialize retry_policy as JSON
        let retry_policy_json = req
            .retry_policy
            .as_ref()
            .and_then(|rp| serde_json::to_value(rp).ok());

        let subscription = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            UPDATE webhook_subscriptions SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                url = COALESCE($4, url),
                events = COALESCE($5, events),
                filters = COALESCE($6, filters),
                payload_template = COALESCE($7, payload_template),
                headers = COALESCE($8, headers),
                retry_policy = COALESCE($9, retry_policy),
                status = COALESCE($10, status),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.url)
        .bind(&req.events)
        .bind(&req.filters)
        .bind(&req.payload_template)
        .bind(&req.headers)
        .bind(&retry_policy_json)
        .bind(&req.status)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscription)
    }

    /// List webhook delivery logs.
    pub async fn list_webhook_delivery_logs(
        &self,
        webhook_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<EnhancedWebhookDeliveryLog>, AppError> {
        let logs = sqlx::query_as::<_, EnhancedWebhookDeliveryLog>(
            r#"
            SELECT * FROM webhook_deliveries
            WHERE subscription_id = $1
            ORDER BY delivered_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(webhook_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(logs)
    }

    /// Get webhook statistics.
    pub async fn get_webhook_statistics(
        &self,
        webhook_id: Uuid,
    ) -> Result<EnhancedWebhookStatistics, AppError> {
        // Query basic stats
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'success') as success,
                COUNT(*) FILTER (WHERE status = 'failed') as failed,
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE status = 'retrying') as retrying,
                COALESCE(AVG(response_time_ms), 0)::float8 as avg_time,
                COUNT(*) FILTER (WHERE delivered_at > NOW() - INTERVAL '24 hours') as last_24h,
                COUNT(*) FILTER (WHERE delivered_at > NOW() - INTERVAL '24 hours' AND status = 'failed') as last_24h_failed
            FROM webhook_deliveries
            WHERE subscription_id = $1
            "#,
        )
        .bind(webhook_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        use sqlx::Row;
        let total: i64 = row.get("total");
        let success: i64 = row.get("success");
        let failed: i64 = row.get("failed");
        let pending: i64 = row.get("pending");
        let retrying: i64 = row.get("retrying");
        let avg_time: f64 = row.get("avg_time");
        let last_24h: i64 = row.get("last_24h");
        let last_24h_failed: i64 = row.get("last_24h_failed");

        let success_rate = if total > 0 {
            (success as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(EnhancedWebhookStatistics {
            subscription_id: webhook_id,
            total_deliveries: total,
            successful_deliveries: success,
            failed_deliveries: failed,
            pending_deliveries: pending,
            retrying_deliveries: retrying,
            average_response_time_ms: Some(avg_time),
            success_rate,
            last_24h_deliveries: last_24h,
            last_24h_failures: last_24h_failed,
            events_by_type: serde_json::json!({}),
        })
    }

    // ============================================
    // Story 150.4: Developer Portal (remaining)
    // ============================================

    /// Register a developer.
    pub async fn register_developer(
        &self,
        user_id: Uuid,
        req: &CreateDeveloperRegistration,
    ) -> Result<DeveloperRegistration, AppError> {
        let registration = sqlx::query_as::<_, DeveloperRegistration>(
            r#"
            INSERT INTO developer_accounts (
                user_id, email, company_name, website, use_case
            ) VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&req.email)
        .bind(&req.company_name)
        .bind(&req.website)
        .bind(&req.use_case)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// Get developer registration by user ID.
    pub async fn get_developer_registration_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<DeveloperRegistration>, AppError> {
        let registration = sqlx::query_as::<_, DeveloperRegistration>(
            r#"
            SELECT * FROM developer_accounts WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// Review a developer registration (approve/reject).
    pub async fn review_developer_registration(
        &self,
        developer_id: Uuid,
        reviewer_id: Uuid,
        req: &ReviewDeveloperRegistration,
    ) -> Result<Option<DeveloperRegistration>, AppError> {
        let registration = sqlx::query_as::<_, DeveloperRegistration>(
            r#"
            UPDATE developer_accounts SET
                status = $2,
                approved_at = CASE WHEN $2 = 'approved' THEN NOW() ELSE NULL END,
                approved_by = CASE WHEN $2 = 'approved' THEN $3 ELSE NULL END
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(developer_id)
        .bind(&req.status)
        .bind(reviewer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// Create a developer API key.
    pub async fn create_developer_api_key(
        &self,
        developer_id: Uuid,
        req: &CreateDeveloperApiKey,
        key_prefix: &str,
        key_hash: &str,
    ) -> Result<DeveloperApiKey, AppError> {
        // Calculate expiration from expires_in_days
        let expires_at = req
            .expires_in_days
            .map(|days| chrono::Utc::now() + chrono::Duration::days(days as i64));

        let api_key = sqlx::query_as::<_, DeveloperApiKey>(
            r#"
            INSERT INTO developer_api_keys (
                developer_id, name, key_prefix, key_hash, scopes,
                is_sandbox, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(developer_id)
        .bind(&req.name)
        .bind(key_prefix)
        .bind(key_hash)
        .bind(&req.scopes)
        .bind(req.is_sandbox.unwrap_or(false))
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(api_key)
    }

    /// Rotate (replace) a developer API key.
    pub async fn rotate_developer_api_key(
        &self,
        key_id: Uuid,
        revoked_by: Uuid,
        new_key_prefix: &str,
        new_key_hash: &str,
    ) -> Result<Option<DeveloperApiKey>, AppError> {
        // First, get the existing key info
        let existing = sqlx::query_as::<_, DeveloperApiKey>(
            r#"SELECT * FROM developer_api_keys WHERE id = $1 AND revoked_at IS NULL"#,
        )
        .bind(key_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let Some(old_key) = existing else {
            return Ok(None);
        };

        // Revoke the old key
        sqlx::query(
            r#"
            UPDATE developer_api_keys SET
                is_active = FALSE,
                revoked_at = NOW(),
                revoked_by = $2
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .bind(revoked_by)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Create a new key with the same settings
        let new_key = sqlx::query_as::<_, DeveloperApiKey>(
            r#"
            INSERT INTO developer_api_keys (
                developer_id, name, key_prefix, key_hash, scopes,
                rate_limit_tier, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(old_key.developer_id)
        .bind(format!("{} (rotated)", old_key.name))
        .bind(new_key_prefix)
        .bind(new_key_hash)
        .bind(&old_key.scopes)
        .bind(&old_key.rate_limit_tier)
        .bind(old_key.expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Some(new_key))
    }
}

//! API Ecosystem repository (Epic 150).
//!
//! Provides database operations for Integration Marketplace, Connector Framework,
//! Webhooks, and Developer Portal.
//!
//! # RLS Integration (PAP-110, parent PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on six tables this repo queries:
//! `organization_integrations`, `organization_connectors`,
//! `connector_execution_logs`, `webhook_subscriptions`, `webhook_deliveries`,
//! and `integration_ratings` (write side). Under `FORCE` the api-server's
//! owner connection is no longer exempt, so a query issued on a connection
//! without `app.current_org_id` set collapses to deny-all (own-org reads
//! return empty, writes fail).
//!
//! The repository therefore holds **no pool**: every method takes an executor
//! supplied by the caller. Handlers touching the org-scoped tables above pass
//! the `RlsConnection` extractor's context-set connection (`&mut **rls.conn()`);
//! handlers for the global, non-`FORCE` catalog tables (marketplace,
//! connectors, docs, developer portal) pass the app pool directly — the owner
//! role remains exempt there, matching prior behavior. Single-statement
//! methods take a generic `Executor`; multi-statement methods take
//! `&mut PgConnection` and run inside an explicit transaction. This mirrors
//! the `work_order.rs` / `budget.rs` / `document.rs` precedent.

use crate::models::api_ecosystem::*;
use crate::DbPool;
use common::AppError;
use sqlx::{Connection, Executor, PgConnection, Postgres};
use uuid::Uuid;

/// Repository for API Ecosystem operations.
///
/// Stateless: every method receives an executor from the caller. Org-scoped
/// (FORCE-RLS) queries must run on an RLS-context-set connection.
#[derive(Clone)]
pub struct ApiEcosystemRepository;

impl ApiEcosystemRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs).
    pub fn new(_pool: DbPool) -> Self {
        Self
    }

    // ============================================
    // Story 150.1: Integration Marketplace
    // ============================================

    /// List marketplace integrations with optional filtering.
    pub async fn list_marketplace_integrations<'e, E>(
        &self,
        executor: E,
        query: &MarketplaceIntegrationQuery,
    ) -> Result<Vec<MarketplaceIntegrationSummary>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integrations)
    }

    /// Get a marketplace integration by ID.
    pub async fn get_marketplace_integration<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<MarketplaceIntegration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let integration = sqlx::query_as::<_, MarketplaceIntegration>(
            r#"
            SELECT * FROM marketplace_integrations WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Get a marketplace integration by slug.
    pub async fn get_marketplace_integration_by_slug<'e, E>(
        &self,
        executor: E,
        slug: &str,
    ) -> Result<Option<MarketplaceIntegration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let integration = sqlx::query_as::<_, MarketplaceIntegration>(
            r#"
            SELECT * FROM marketplace_integrations WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Create a new marketplace integration (admin only).
    pub async fn create_marketplace_integration<'e, E>(
        &self,
        executor: E,
        req: &CreateMarketplaceIntegration,
    ) -> Result<MarketplaceIntegration, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Update a marketplace integration.
    pub async fn update_marketplace_integration<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        req: &UpdateMarketplaceIntegration,
    ) -> Result<Option<MarketplaceIntegration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Delete a marketplace integration.
    pub async fn delete_marketplace_integration<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM marketplace_integrations WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get integration category counts.
    pub async fn get_integration_category_counts<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<IntegrationCategoryCount>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let counts = sqlx::query_as::<_, IntegrationCategoryCount>(
            r#"
            SELECT category, COUNT(*) as count
            FROM marketplace_integrations
            WHERE status = 'available'
            GROUP BY category
            ORDER BY count DESC
            "#,
        )
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(counts)
    }

    // ============================================
    // Organization Integrations
    // ============================================

    /// Install an integration for an organization.
    /// Uses a transaction to ensure atomicity between installation and install count increment.
    /// Only increments install_count for new installations, not reinstalls.
    ///
    /// Multi-statement: takes `&mut PgConnection` (an RLS-context-set
    /// connection — `organization_integrations` is FORCE-RLS) and opens the
    /// transaction on it.
    pub async fn install_integration(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
        user_id: Uuid,
        req: &InstallIntegration,
    ) -> Result<OrganizationIntegration, AppError> {
        // Encrypt credentials if provided (in production, use proper encryption)
        let credentials_encrypted = req
            .credentials
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_default());

        // Use a transaction to ensure atomicity
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Check if this is a new installation or a reinstall
        let existing = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*) FROM organization_integrations
               WHERE organization_id = $1 AND integration_id = $2"#,
        )
        .bind(organization_id)
        .bind(req.integration_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let is_new_install = existing == 0;

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
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Only increment install count for new installations
        if is_new_install {
            sqlx::query(
                r#"
                UPDATE marketplace_integrations
                SET install_count = install_count + 1
                WHERE id = $1
                "#,
            )
            .bind(req.integration_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(installation)
    }

    /// List organization integrations.
    pub async fn list_organization_integrations<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<OrganizationIntegration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let integrations = sqlx::query_as::<_, OrganizationIntegration>(
            r#"
            SELECT * FROM organization_integrations
            WHERE organization_id = $1
            ORDER BY installed_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integrations)
    }

    /// Uninstall an integration.
    /// Note: `org_integration_id` is the primary key (id) of the organization_integrations record,
    /// not the marketplace integration_id.
    pub async fn uninstall_integration<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        org_integration_id: Uuid,
    ) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE organization_integrations
            SET status = 'uninstalled', enabled = FALSE, updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(organization_id)
        .bind(org_integration_id)
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Story 150.2: Connector Framework
    // ============================================

    /// List connectors for an integration.
    pub async fn list_connectors<'e, E>(
        &self,
        executor: E,
        integration_id: Uuid,
    ) -> Result<Vec<Connector>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connectors = sqlx::query_as::<_, Connector>(
            r#"
            SELECT * FROM connectors
            WHERE integration_id = $1
            ORDER BY name
            "#,
        )
        .bind(integration_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connectors)
    }

    /// Get a connector by ID.
    pub async fn get_connector<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Connector>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connector = sqlx::query_as::<_, Connector>(
            r#"
            SELECT * FROM connectors WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connector)
    }

    /// Create a new connector.
    pub async fn create_connector<'e, E>(
        &self,
        executor: E,
        req: &CreateConnector,
    ) -> Result<Connector, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connector)
    }

    /// Update a connector.
    pub async fn update_connector<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        req: &UpdateConnector,
    ) -> Result<Option<Connector>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connector)
    }

    /// Delete a connector.
    pub async fn delete_connector<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM connectors WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// List connector actions.
    /// Maps DB columns to model fields for schema compatibility.
    pub async fn list_connector_actions<'e, E>(
        &self,
        executor: E,
        connector_id: Uuid,
    ) -> Result<Vec<ConnectorAction>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let actions = sqlx::query_as::<_, ConnectorAction>(
            r#"
            SELECT
                id, connector_id, name, description,
                method as http_method, path as endpoint_path,
                input_schema as request_schema, output_schema as response_schema,
                NULL::jsonb as request_transformations, NULL::jsonb as response_transformations,
                pagination_config, created_at, created_at as updated_at
            FROM connector_actions
            WHERE connector_id = $1
            ORDER BY name
            "#,
        )
        .bind(connector_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(actions)
    }

    // ============================================
    // Story 150.3: Enhanced Webhooks
    // ============================================

    /// List enhanced webhook subscriptions for an organization.
    ///
    /// Selects with the same column aliasing as the create/update RETURNING:
    /// the DB schema (00102) has no `auth_type`/`status`/`retry_policy`/...
    /// columns, so a `SELECT *` cannot decode into the enhanced model. This
    /// was masked pre-fix because the raw-pool read returned zero rows under
    /// FORCE RLS (deny-all) and never reached the decoder.
    pub async fn list_enhanced_webhooks<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<EnhancedWebhookSubscription>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let subscriptions = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            SELECT
                id, organization_id, name, description, url,
                'hmac_sha256' as auth_type, NULL::jsonb as auth_config,
                events, NULL::jsonb as filters, NULL::jsonb as payload_template,
                CASE WHEN is_active THEN 'active' ELSE 'inactive' END as status,
                headers, retry_config as retry_policy,
                NULL::int4 as rate_limit_requests, NULL::int4 as rate_limit_window_seconds,
                30000 as timeout_ms, TRUE as verify_ssl,
                created_by, created_at, updated_at
            FROM webhook_subscriptions
            WHERE organization_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscriptions)
    }

    /// Get a webhook subscription by ID.
    ///
    /// RLS scopes this by-id read to the connection's org: another org's
    /// subscription is invisible and surfaces as `None` (`404`). Uses the
    /// same column aliasing as [`Self::list_enhanced_webhooks`] — see there.
    pub async fn get_enhanced_webhook<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<EnhancedWebhookSubscription>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let subscription = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            SELECT
                id, organization_id, name, description, url,
                'hmac_sha256' as auth_type, NULL::jsonb as auth_config,
                events, NULL::jsonb as filters, NULL::jsonb as payload_template,
                CASE WHEN is_active THEN 'active' ELSE 'inactive' END as status,
                headers, retry_config as retry_policy,
                NULL::int4 as rate_limit_requests, NULL::int4 as rate_limit_window_seconds,
                30000 as timeout_ms, TRUE as verify_ssl,
                created_by, created_at, updated_at
            FROM webhook_subscriptions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscription)
    }

    /// Delete a webhook subscription.
    pub async fn delete_enhanced_webhook<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM webhook_subscriptions WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Story 150.4: Developer Portal
    // ============================================

    /// Get a developer registration by ID.
    pub async fn get_developer_registration<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<DeveloperRegistration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let registration = sqlx::query_as::<_, DeveloperRegistration>(
            r#"
            SELECT * FROM developer_accounts WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// List API keys for a developer.
    pub async fn list_developer_api_keys<'e, E>(
        &self,
        executor: E,
        developer_id: Uuid,
    ) -> Result<Vec<DeveloperApiKey>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let keys = sqlx::query_as::<_, DeveloperApiKey>(
            r#"
            SELECT * FROM developer_api_keys
            WHERE developer_id = $1 AND revoked_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(developer_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(keys)
    }

    /// Revoke an API key.
    pub async fn revoke_api_key<'e, E>(
        &self,
        executor: E,
        key_id: Uuid,
        revoked_by: Uuid,
    ) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Story 150.1: Integration Ratings
    // ============================================

    /// List ratings for an integration.
    pub async fn list_integration_ratings<'e, E>(
        &self,
        executor: E,
        integration_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<IntegrationRatingWithUser>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(ratings)
    }

    /// Create an integration rating.
    pub async fn create_integration_rating<'e, E>(
        &self,
        executor: E,
        integration_id: Uuid,
        organization_id: Uuid,
        user_id: Uuid,
        req: &CreateIntegrationRating,
    ) -> Result<IntegrationRating, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rating)
    }

    /// Get an organization integration by ID.
    pub async fn get_organization_integration<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        integration_id: Uuid,
    ) -> Result<Option<OrganizationIntegration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let integration = sqlx::query_as::<_, OrganizationIntegration>(
            r#"
            SELECT * FROM organization_integrations
            WHERE organization_id = $1 AND integration_id = $2
            "#,
        )
        .bind(organization_id)
        .bind(integration_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    /// Update an organization integration.
    pub async fn update_organization_integration<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        integration_id: Uuid,
        req: &UpdateOrganizationIntegration,
    ) -> Result<Option<OrganizationIntegration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(integration)
    }

    // ============================================
    // Story 150.2: Connector Actions & Logs
    // ============================================

    /// Create a connector action.
    /// Note: Maps model fields to DB columns:
    /// - http_method -> method
    /// - endpoint_path -> path
    /// - request_schema -> input_schema
    /// - response_schema -> output_schema
    pub async fn create_connector_action<'e, E>(
        &self,
        executor: E,
        req: &CreateConnectorAction,
    ) -> Result<ConnectorAction, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let action = sqlx::query_as::<_, ConnectorAction>(
            r#"
            INSERT INTO connector_actions (
                connector_id, name, display_name, description, method, path,
                input_schema, output_schema, pagination_config
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, connector_id, name, description,
                method as http_method, path as endpoint_path,
                input_schema as request_schema, output_schema as response_schema,
                NULL::jsonb as request_transformations, NULL::jsonb as response_transformations,
                pagination_config, created_at, created_at as updated_at
            "#,
        )
        .bind(req.connector_id)
        .bind(&req.name)
        .bind(&req.name) // display_name defaults to name
        .bind(&req.description)
        .bind(&req.http_method)
        .bind(&req.endpoint_path)
        .bind(&req.request_schema)
        .bind(&req.response_schema)
        .bind(&req.pagination_config)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(action)
    }

    /// List connector execution logs.
    /// Note: DB schema uses org_connector_id (FK to organization_connectors), not organization_id.
    /// We join through organization_connectors to filter by organization.
    pub async fn list_connector_execution_logs<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: &ConnectorExecutionQuery,
    ) -> Result<Vec<ConnectorExecutionLog>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50).min(100) as i64;
        let offset = query.offset.unwrap_or(0).max(0) as i64;

        let logs = sqlx::query_as::<_, ConnectorExecutionLog>(
            r#"
            SELECT
                cel.id,
                oc.organization_id,
                oc.connector_id,
                cel.action_name,
                CASE
                    WHEN cel.status_code >= 200 AND cel.status_code < 300 THEN 'success'
                    WHEN cel.status_code >= 400 THEN 'error'
                    ELSE 'unknown'
                END as status,
                cel.request_payload,
                cel.response_payload,
                cel.error_message,
                NULL::text as error_code,
                COALESCE(cel.duration_ms, 0) as duration_ms,
                0 as retry_count,
                FALSE as rate_limited,
                cel.executed_at as created_at
            FROM connector_execution_logs cel
            JOIN organization_connectors oc ON oc.id = cel.org_connector_id
            WHERE oc.organization_id = $1
              AND ($2::uuid IS NULL OR oc.connector_id = $2)
              AND ($3::text IS NULL OR
                   CASE
                       WHEN cel.status_code >= 200 AND cel.status_code < 300 THEN 'success'
                       WHEN cel.status_code >= 400 THEN 'error'
                       ELSE 'unknown'
                   END = $3)
            ORDER BY cel.executed_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(organization_id)
        .bind(query.connector_id)
        .bind(query.status.as_ref())
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(logs)
    }

    // ============================================
    // Story 150.3: Enhanced Webhooks (remaining)
    // ============================================

    /// Create an enhanced webhook subscription.
    /// Note: DB schema has simpler columns than the enhanced model. We map enhanced
    /// fields to what the DB supports and return a compatible struct.
    pub async fn create_enhanced_webhook<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        req: &CreateEnhancedWebhookSubscription,
    ) -> Result<EnhancedWebhookSubscription, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Serialize retry_policy as JSON for retry_config column
        let retry_config_json = req
            .retry_policy
            .as_ref()
            .and_then(|rp| serde_json::to_value(rp).ok());

        // Generate a webhook secret for HMAC signing
        let secret = format!("whsec_{}", Uuid::new_v4().to_string().replace("-", ""));

        // Insert using DB schema columns, return with aliased columns for model compatibility
        let subscription = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            INSERT INTO webhook_subscriptions (
                organization_id, name, description, url, secret, events, headers,
                retry_config, created_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id, organization_id, name, description, url,
                'hmac_sha256' as auth_type, NULL::jsonb as auth_config,
                events, NULL::jsonb as filters, NULL::jsonb as payload_template,
                CASE WHEN is_active THEN 'active' ELSE 'inactive' END as status,
                headers, retry_config as retry_policy,
                NULL::int4 as rate_limit_requests, NULL::int4 as rate_limit_window_seconds,
                30000 as timeout_ms, TRUE as verify_ssl,
                created_by, created_at, updated_at
            "#,
        )
        .bind(organization_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.url)
        .bind(&secret)
        .bind(&req.events)
        .bind(&req.headers)
        .bind(&retry_config_json)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscription)
    }

    /// Update an enhanced webhook subscription.
    pub async fn update_enhanced_webhook<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        req: &UpdateEnhancedWebhookSubscription,
    ) -> Result<Option<EnhancedWebhookSubscription>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Serialize retry_policy as JSON
        let retry_policy_json = req
            .retry_policy
            .as_ref()
            .and_then(|rp| serde_json::to_value(rp).ok());

        // Map the status field to is_active boolean for DB compatibility
        let is_active: Option<bool> = req.status.as_ref().map(|s| s == "active");

        let subscription = sqlx::query_as::<_, EnhancedWebhookSubscription>(
            r#"
            UPDATE webhook_subscriptions SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                url = COALESCE($4, url),
                events = COALESCE($5, events),
                headers = COALESCE($6, headers),
                retry_config = COALESCE($7, retry_config),
                is_active = COALESCE($8, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, organization_id, name, description, url,
                'hmac_sha256' as auth_type, NULL::jsonb as auth_config,
                events, NULL::jsonb as filters, NULL::jsonb as payload_template,
                CASE WHEN is_active THEN 'active' ELSE 'inactive' END as status,
                headers, retry_config as retry_policy,
                NULL::int4 as rate_limit_requests, NULL::int4 as rate_limit_window_seconds,
                30000 as timeout_ms, TRUE as verify_ssl,
                created_by, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.url)
        .bind(&req.events)
        .bind(&req.headers)
        .bind(&retry_policy_json)
        .bind(is_active)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(subscription)
    }

    /// List webhook delivery logs.
    pub async fn list_webhook_delivery_logs<'e, E>(
        &self,
        executor: E,
        webhook_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<EnhancedWebhookDeliveryLog>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Aliased projection: the 00102 `webhook_deliveries` schema has no
        // `event_id`/`attempts`/`created_at`/... columns the enhanced model
        // expects, so a `SELECT *` cannot decode (masked pre-fix by the
        // FORCE-RLS deny-all returning zero rows).
        let logs = sqlx::query_as::<_, EnhancedWebhookDeliveryLog>(
            r#"
            SELECT
                id, subscription_id, event_type,
                id as event_id,
                payload, NULL::jsonb as transformed_payload,
                status,
                attempt_number as attempts,
                delivered_at as last_attempt_at,
                next_retry_at,
                NULL::jsonb as request_headers,
                response_status, response_headers, response_body,
                error_message, NULL::text as error_code,
                duration_ms, NULL::text as ip_address,
                delivered_at as created_at
            FROM webhook_deliveries
            WHERE subscription_id = $1
            ORDER BY delivered_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(webhook_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(logs)
    }

    /// Get webhook statistics.
    pub async fn get_webhook_statistics<'e, E>(
        &self,
        executor: E,
        webhook_id: Uuid,
    ) -> Result<EnhancedWebhookStatistics, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Query basic stats
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'delivered') as success,
                COUNT(*) FILTER (WHERE status = 'failed') as failed,
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE status = 'retrying') as retrying,
                COALESCE(AVG(duration_ms), 0)::float8 as avg_time,
                COUNT(*) FILTER (WHERE delivered_at > NOW() - INTERVAL '24 hours') as last_24h,
                COUNT(*) FILTER (WHERE delivered_at > NOW() - INTERVAL '24 hours' AND status = 'failed') as last_24h_failed
            FROM webhook_deliveries
            WHERE subscription_id = $1
            "#,
        )
        .bind(webhook_id)
        .fetch_one(executor)
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
    pub async fn register_developer<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        req: &CreateDeveloperRegistration,
    ) -> Result<DeveloperRegistration, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// Get developer registration by user ID.
    pub async fn get_developer_registration_by_user<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Option<DeveloperRegistration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let registration = sqlx::query_as::<_, DeveloperRegistration>(
            r#"
            SELECT * FROM developer_accounts WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// Review a developer registration (approve/reject).
    pub async fn review_developer_registration<'e, E>(
        &self,
        executor: E,
        developer_id: Uuid,
        reviewer_id: Uuid,
        req: &ReviewDeveloperRegistration,
    ) -> Result<Option<DeveloperRegistration>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(registration)
    }

    /// Create a developer API key.
    pub async fn create_developer_api_key<'e, E>(
        &self,
        executor: E,
        developer_id: Uuid,
        req: &CreateDeveloperApiKey,
        key_prefix: &str,
        key_hash: &str,
    ) -> Result<DeveloperApiKey, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
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
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(api_key)
    }

    /// Rotate (replace) a developer API key.
    /// Preserves all settings from the old key including is_sandbox.
    ///
    /// Multi-statement: takes `&mut PgConnection` and runs the
    /// revoke-old/insert-new pair inside a transaction so a failure cannot
    /// leave the developer with the old key revoked and no replacement.
    pub async fn rotate_developer_api_key(
        &self,
        conn: &mut PgConnection,
        key_id: Uuid,
        revoked_by: Uuid,
        new_key_prefix: &str,
        new_key_hash: &str,
    ) -> Result<Option<DeveloperApiKey>, AppError> {
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        // First, get the existing key info
        let existing = sqlx::query_as::<_, DeveloperApiKey>(
            r#"SELECT * FROM developer_api_keys WHERE id = $1 AND revoked_at IS NULL"#,
        )
        .bind(key_id)
        .fetch_optional(&mut *tx)
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
                status = 'revoked',
                revoked_at = NOW(),
                revoked_by = $2
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .bind(revoked_by)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Create a new key with the same settings, preserving is_sandbox
        let new_key = sqlx::query_as::<_, DeveloperApiKey>(
            r#"
            INSERT INTO developer_api_keys (
                developer_id, name, key_prefix, key_hash, scopes,
                rate_limit_tier, is_sandbox, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(old_key.developer_id)
        .bind(format!("{} (rotated)", old_key.name))
        .bind(new_key_prefix)
        .bind(new_key_hash)
        .bind(&old_key.scopes)
        .bind(&old_key.rate_limit_tier)
        .bind(old_key.is_sandbox) // Preserve sandbox setting
        .bind(old_key.expires_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Some(new_key))
    }

    // ============================================
    // Story 150.4: Pre-Built Integrations
    // ============================================

    /// List pre-built integration connections for an organization.
    pub async fn list_prebuilt_connections<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<PreBuiltIntegrationConnection>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connections = sqlx::query_as::<_, PreBuiltIntegrationConnection>(
            r#"
            SELECT * FROM prebuilt_integration_connections
            WHERE organization_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connections)
    }

    /// Get a pre-built integration connection by type.
    pub async fn get_prebuilt_connection<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        integration_type: &str,
    ) -> Result<Option<PreBuiltIntegrationConnection>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connection = sqlx::query_as::<_, PreBuiltIntegrationConnection>(
            r#"
            SELECT * FROM prebuilt_integration_connections
            WHERE organization_id = $1 AND integration_type = $2
            "#,
        )
        .bind(organization_id)
        .bind(integration_type)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connection)
    }

    /// Create a pre-built integration connection.
    pub async fn create_prebuilt_connection<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        req: &CreatePreBuiltIntegrationConnection,
    ) -> Result<PreBuiltIntegrationConnection, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connection = sqlx::query_as::<_, PreBuiltIntegrationConnection>(
            r#"
            INSERT INTO prebuilt_integration_connections (
                organization_id, integration_type, status, configuration, created_by
            ) VALUES ($1, $2, 'pending', $3, $4)
            ON CONFLICT (organization_id, integration_type)
            DO UPDATE SET
                configuration = COALESCE($3, prebuilt_integration_connections.configuration),
                status = 'pending',
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(&req.integration_type)
        .bind(&req.configuration)
        .bind(user_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connection)
    }

    /// Update a pre-built integration connection.
    pub async fn update_prebuilt_connection<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        integration_type: &str,
        req: &UpdatePreBuiltIntegrationConnection,
    ) -> Result<Option<PreBuiltIntegrationConnection>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connection = sqlx::query_as::<_, PreBuiltIntegrationConnection>(
            r#"
            UPDATE prebuilt_integration_connections SET
                configuration = COALESCE($3, configuration),
                sync_enabled = COALESCE($4, sync_enabled),
                updated_at = NOW()
            WHERE organization_id = $1 AND integration_type = $2
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(integration_type)
        .bind(&req.configuration)
        .bind(req.sync_enabled)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connection)
    }

    /// Delete a pre-built integration connection.
    pub async fn delete_prebuilt_connection<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        integration_type: &str,
    ) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"DELETE FROM prebuilt_integration_connections WHERE organization_id = $1 AND integration_type = $2"#,
        )
        .bind(organization_id)
        .bind(integration_type)
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Update pre-built integration connection with OAuth tokens.
    pub async fn update_prebuilt_connection_tokens<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        integration_type: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Option<PreBuiltIntegrationConnection>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connection = sqlx::query_as::<_, PreBuiltIntegrationConnection>(
            r#"
            UPDATE prebuilt_integration_connections SET
                access_token_encrypted = $3,
                refresh_token_encrypted = $4,
                token_expires_at = $5,
                status = 'connected',
                updated_at = NOW()
            WHERE organization_id = $1 AND integration_type = $2
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(integration_type)
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connection)
    }

    /// Record sync result for pre-built integration.
    pub async fn record_prebuilt_sync<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        integration_type: &str,
        success: bool,
        error: Option<&str>,
    ) -> Result<(), AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE prebuilt_integration_connections SET
                last_sync_at = NOW(),
                last_error = $4,
                status = CASE WHEN $3 THEN 'connected' ELSE 'error' END,
                updated_at = NOW()
            WHERE organization_id = $1 AND integration_type = $2
            "#,
        )
        .bind(organization_id)
        .bind(integration_type)
        .bind(success)
        .bind(error)
        .execute(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ============================================
    // Story 150.5: Developer Portal (Documentation)
    // ============================================

    /// List all published API documentation.
    pub async fn list_api_documentation<'e, E>(
        &self,
        executor: E,
        published_only: bool,
    ) -> Result<Vec<ApiDocumentation>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let docs = if published_only {
            sqlx::query_as::<_, ApiDocumentation>(
                r#"
                SELECT * FROM api_documentation
                WHERE is_published = TRUE
                ORDER BY category, order_index, title
                "#,
            )
            .fetch_all(executor)
            .await
        } else {
            sqlx::query_as::<_, ApiDocumentation>(
                r#"
                SELECT * FROM api_documentation
                ORDER BY category, order_index, title
                "#,
            )
            .fetch_all(executor)
            .await
        }
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(docs)
    }

    /// Get API documentation by slug.
    pub async fn get_api_documentation<'e, E>(
        &self,
        executor: E,
        slug: &str,
    ) -> Result<Option<ApiDocumentation>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let doc = sqlx::query_as::<_, ApiDocumentation>(
            r#"
            SELECT * FROM api_documentation WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(doc)
    }

    /// Create API documentation.
    pub async fn create_api_documentation<'e, E>(
        &self,
        executor: E,
        req: &CreateApiDocumentation,
    ) -> Result<ApiDocumentation, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let doc = sqlx::query_as::<_, ApiDocumentation>(
            r#"
            INSERT INTO api_documentation (slug, title, content, category, order_index, is_published)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&req.slug)
        .bind(&req.title)
        .bind(&req.content)
        .bind(&req.category)
        .bind(req.order_index.unwrap_or(0))
        .bind(req.is_published.unwrap_or(false))
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(doc)
    }

    /// Update API documentation.
    pub async fn update_api_documentation<'e, E>(
        &self,
        executor: E,
        slug: &str,
        req: &UpdateApiDocumentation,
    ) -> Result<Option<ApiDocumentation>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let doc = sqlx::query_as::<_, ApiDocumentation>(
            r#"
            UPDATE api_documentation SET
                title = COALESCE($2, title),
                content = COALESCE($3, content),
                category = COALESCE($4, category),
                order_index = COALESCE($5, order_index),
                is_published = COALESCE($6, is_published),
                updated_at = NOW()
            WHERE slug = $1
            RETURNING *
            "#,
        )
        .bind(slug)
        .bind(&req.title)
        .bind(&req.content)
        .bind(&req.category)
        .bind(req.order_index)
        .bind(req.is_published)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(doc)
    }

    /// Delete API documentation.
    pub async fn delete_api_documentation<'e, E>(
        &self,
        executor: E,
        slug: &str,
    ) -> Result<bool, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM api_documentation WHERE slug = $1")
            .bind(slug)
            .execute(executor)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// List code samples for an endpoint.
    pub async fn list_code_samples<'e, E>(
        &self,
        executor: E,
        endpoint_path: &str,
    ) -> Result<Vec<ApiCodeSample>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let samples = sqlx::query_as::<_, ApiCodeSample>(
            r#"
            SELECT * FROM api_code_samples
            WHERE endpoint_path = $1
            ORDER BY language
            "#,
        )
        .bind(endpoint_path)
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(samples)
    }

    /// Create a code sample.
    pub async fn create_code_sample<'e, E>(
        &self,
        executor: E,
        req: &CreateApiCodeSample,
    ) -> Result<ApiCodeSample, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sample = sqlx::query_as::<_, ApiCodeSample>(
            r#"
            INSERT INTO api_code_samples (endpoint_path, http_method, language, code, description)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&req.endpoint_path)
        .bind(&req.http_method)
        .bind(&req.language)
        .bind(&req.code)
        .bind(&req.description)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sample)
    }

    // ============================================
    // Story 150.5: Developer Portal (Sandbox)
    // ============================================

    /// Get sandbox environment for a developer.
    pub async fn get_sandbox_environment<'e, E>(
        &self,
        executor: E,
        developer_id: Uuid,
    ) -> Result<Option<SandboxConfig>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sandbox = sqlx::query_as::<_, SandboxConfig>(
            r#"
            SELECT * FROM developer_sandboxes
            WHERE developer_id = $1
            AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(developer_id)
        .fetch_optional(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sandbox)
    }

    /// Create sandbox environment for a developer.
    pub async fn create_sandbox_environment<'e, E>(
        &self,
        executor: E,
        developer_id: Uuid,
        req: &CreateSandboxConfig,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<SandboxConfig, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let configuration = serde_json::json!({
            "test_mode": true,
            "seed_data": req.seed_test_data.unwrap_or(true)
        });

        let sandbox = sqlx::query_as::<_, SandboxConfig>(
            r#"
            INSERT INTO developer_sandboxes (developer_id, name, configuration, test_data_seeded, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(developer_id)
        .bind(&req.name)
        .bind(&configuration)
        .bind(req.seed_test_data.unwrap_or(true))
        .bind(expires_at)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sandbox)
    }

    // ============================================
    // Statistics & Dashboard
    // ============================================

    /// Get developer usage statistics.
    pub async fn get_developer_usage_stats<'e, E>(
        &self,
        executor: E,
        developer_id: Uuid,
    ) -> Result<DeveloperUsageStats, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Query API usage statistics for the developer
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN date = CURRENT_DATE THEN request_count ELSE 0 END), 0) as api_calls_today,
                COALESCE(SUM(CASE WHEN date >= date_trunc('month', CURRENT_DATE) THEN request_count ELSE 0 END), 0) as api_calls_this_month,
                COALESCE(SUM(rate_limit_exceeded_count), 0) as rate_limit_exceeded,
                COALESCE(SUM(error_count), 0) as errors,
                COALESCE(AVG(avg_response_time_ms), 0)::float8 as avg_response_time
            FROM developer_api_usage
            WHERE developer_id = $1
            AND date >= CURRENT_DATE - INTERVAL '30 days'
            "#,
        )
        .bind(developer_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        use sqlx::Row;
        Ok(DeveloperUsageStats {
            developer_id,
            api_calls_today: row.get::<i64, _>("api_calls_today"),
            api_calls_this_month: row.get::<i64, _>("api_calls_this_month"),
            rate_limit_exceeded_count: row.get::<i64, _>("rate_limit_exceeded"),
            error_count: row.get::<i64, _>("errors"),
            avg_response_time_ms: row.get::<f64, _>("avg_response_time"),
            most_used_endpoints: vec![],
        })
    }

    /// Get developer portal statistics (admin).
    pub async fn get_developer_portal_stats<'e, E>(
        &self,
        executor: E,
    ) -> Result<DeveloperPortalStatistics, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM developer_accounts) as total_developers,
                (SELECT COUNT(*) FROM developer_accounts WHERE status = 'approved') as active_developers,
                (SELECT COUNT(*) FROM developer_accounts WHERE status = 'pending') as pending_registrations,
                (SELECT COUNT(*) FROM developer_api_keys WHERE revoked_at IS NULL) as total_api_keys,
                (SELECT COUNT(*) FROM developer_api_keys WHERE is_sandbox = TRUE AND revoked_at IS NULL) as sandbox_api_keys,
                (SELECT COUNT(*) FROM developer_api_keys WHERE is_sandbox = FALSE AND revoked_at IS NULL) as production_api_keys,
                COALESCE((SELECT SUM(request_count) FROM developer_api_usage WHERE date = CURRENT_DATE), 0) as api_calls_today,
                COALESCE((SELECT SUM(request_count) FROM developer_api_usage WHERE date >= date_trunc('month', CURRENT_DATE)), 0) as api_calls_this_month
            "#,
        )
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        use sqlx::Row;
        Ok(DeveloperPortalStatistics {
            total_developers: row.get("total_developers"),
            active_developers: row.get("active_developers"),
            pending_registrations: row.get("pending_registrations"),
            total_api_keys: row.get("total_api_keys"),
            sandbox_api_keys: row.get("sandbox_api_keys"),
            production_api_keys: row.get("production_api_keys"),
            api_calls_today: row.get("api_calls_today"),
            api_calls_this_month: row.get("api_calls_this_month"),
            top_endpoints: vec![],
        })
    }

    /// Get ecosystem dashboard for an organization.
    pub async fn get_ecosystem_dashboard<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<ApiEcosystemDashboard, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM organization_integrations WHERE organization_id = $1 AND status = 'installed') as installed_integrations,
                (SELECT COUNT(*) FROM organization_integrations WHERE organization_id = $1 AND status = 'installed' AND enabled = TRUE) as active_integrations,
                (SELECT COUNT(*) FROM organization_integrations WHERE organization_id = $1 AND status = 'pending') as pending_sync,
                (SELECT COUNT(*) FROM organization_integrations WHERE organization_id = $1 AND status = 'failed') as failed_integrations,
                (SELECT COUNT(*) FROM webhook_subscriptions WHERE organization_id = $1) as webhook_subscriptions,
                COALESCE((SELECT COUNT(*) FROM webhook_deliveries wd
                    JOIN webhook_subscriptions ws ON wd.subscription_id = ws.id
                    WHERE ws.organization_id = $1 AND wd.delivered_at >= CURRENT_DATE), 0) as webhooks_delivered_today,
                COALESCE((SELECT COUNT(*) FROM connector_execution_logs cel
                    JOIN organization_connectors oc ON cel.org_connector_id = oc.id
                    WHERE oc.organization_id = $1 AND cel.executed_at >= CURRENT_DATE), 0) as connector_executions_today
            "#,
        )
        .bind(organization_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        use sqlx::Row;
        let webhooks_delivered: i64 = row.get("webhooks_delivered_today");

        // Calculate success rates (simplified)
        let webhook_success_rate = if webhooks_delivered > 0 { 95.0 } else { 100.0 };
        let connector_success_rate = 95.0;

        Ok(ApiEcosystemDashboard {
            installed_integrations: row.get::<i64, _>("installed_integrations") as i32,
            active_integrations: row.get::<i64, _>("active_integrations") as i32,
            pending_sync: row.get::<i64, _>("pending_sync") as i32,
            failed_integrations: row.get::<i64, _>("failed_integrations") as i32,
            webhook_subscriptions: row.get::<i64, _>("webhook_subscriptions") as i32,
            webhooks_delivered_today: webhooks_delivered,
            webhook_success_rate,
            connector_executions_today: row.get("connector_executions_today"),
            connector_success_rate,
            recent_activity: vec![],
        })
    }

    /// Get ecosystem statistics for an organization.
    pub async fn get_ecosystem_statistics<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<ApiEcosystemStatistics, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM organization_integrations WHERE organization_id = $1) as total_integrations,
                (SELECT COUNT(*) FROM prebuilt_integration_connections WHERE organization_id = $1 AND status = 'connected') as active_connections,
                COALESCE((SELECT COUNT(*) FROM connector_execution_logs cel
                    JOIN organization_connectors oc ON cel.org_connector_id = oc.id
                    WHERE oc.organization_id = $1 AND cel.executed_at >= CURRENT_DATE), 0) as sync_operations_today,
                COALESCE((SELECT COUNT(*) FROM connector_execution_logs cel
                    JOIN organization_connectors oc ON cel.org_connector_id = oc.id
                    WHERE oc.organization_id = $1 AND cel.executed_at >= date_trunc('month', CURRENT_DATE)), 0) as sync_operations_this_month,
                COALESCE((SELECT AVG(duration_ms) FROM connector_execution_logs cel
                    JOIN organization_connectors oc ON cel.org_connector_id = oc.id
                    WHERE oc.organization_id = $1), 0)::float8 as avg_sync_duration
            "#,
        )
        .bind(organization_id)
        .fetch_one(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        use sqlx::Row;
        Ok(ApiEcosystemStatistics {
            total_integrations: row.get("total_integrations"),
            integrations_by_category: serde_json::json!({}),
            active_connections: row.get("active_connections"),
            sync_operations_today: row.get("sync_operations_today"),
            sync_operations_this_month: row.get("sync_operations_this_month"),
            data_transferred_bytes: 0,
            average_sync_duration_ms: row.get("avg_sync_duration"),
            error_rate: 0.0,
        })
    }

    /// List all connectors (no integration filter).
    pub async fn list_all_connectors<'e, E>(&self, executor: E) -> Result<Vec<Connector>, AppError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let connectors = sqlx::query_as::<_, Connector>(
            r#"
            SELECT * FROM connectors
            ORDER BY name
            "#,
        )
        .fetch_all(executor)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(connectors)
    }
}

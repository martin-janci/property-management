//! Agency domain repository (Phase 1: Tenant Resolution Keystone).
//!
//! # RLS Integration
//!
//! This repository supports two usage patterns:
//!
//! 1. **System lookups** (`*_system`): used by the host-resolution middleware
//!    BEFORE any tenant context exists. They acquire their own connection,
//!    set a system (super-admin / RLS-bypassing) context for exactly one
//!    constrained `SELECT`, then clear the context before returning the
//!    connection to the pool. This is the ONLY sanctioned way to read
//!    `agency_domains` / `organizations` pre-auth.
//!
//! 2. **RLS-aware** (`*_rls`): methods that accept an executor whose RLS
//!    context is already set (e.g. from an `RlsConnection` extractor). These
//!    are used by authenticated, tenant-scoped handlers (domain management UI).
//!
//! ## Example (system resolution — middleware)
//!
//! ```rust,ignore
//! let repo = AgencyDomainRepository::new(pool.clone());
//! if let Some(org_id) = repo.resolve_host_system("acme.example.com").await? {
//!     // host belongs to org_id
//! }
//! ```

use crate::models::agency_domain::{AgencyDomain, CreateAgencyDomain};
use crate::DbPool;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for agency domain operations.
#[derive(Clone)]
pub struct AgencyDomainRepository {
    pool: DbPool,
}

impl AgencyDomainRepository {
    /// Create a new AgencyDomainRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // System lookups (pre-auth, RLS-bypassing — used by middleware)
    // ========================================================================

    /// Resolve an inbound Host header to its owning organization.
    ///
    /// THE tenant-resolution lookup. Acquires a dedicated connection, sets a
    /// system context (super-admin, no org/user), runs a single host-constrained
    /// `SELECT`, then clears the context before the connection returns to the
    /// pool. Only `verified` domains resolve.
    pub async fn resolve_host_system(&self, host: &str) -> Result<Option<Uuid>, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        // System context: super-admin, no tenant. Required because this query
        // runs before any tenant context exists, and `agency_domains` has RLS.
        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT organization_id
            FROM agency_domains
            WHERE host = $1 AND verification_state = 'verified'
            "#,
        )
        .bind(host)
        .fetch_optional(&mut *conn)
        .await;

        // Always clear context before the connection returns to the pool,
        // even if the query failed.
        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::warn!(
                error = %e,
                "Failed to clear system RLS context after resolve_host_system"
            );
        }

        result
    }

    /// Resolve an organization slug to its ID (dev-mode `/a/{slug}` routing).
    ///
    /// Same system-context pattern as [`resolve_host_system`]. Only `active`
    /// organizations resolve.
    pub async fn resolve_slug_system(&self, slug: &str) -> Result<Option<Uuid>, SqlxError> {
        let mut conn = self.pool.acquire().await?;

        crate::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id
            FROM organizations
            WHERE slug = $1 AND status = 'active'
            "#,
        )
        .bind(slug)
        .fetch_optional(&mut *conn)
        .await;

        if let Err(e) = crate::tenant_context::clear_request_context(&mut *conn).await {
            tracing::warn!(
                error = %e,
                "Failed to clear system RLS context after resolve_slug_system"
            );
        }

        result
    }

    // ========================================================================
    // RLS-aware methods (authenticated, tenant-scoped handlers)
    // ========================================================================

    /// Create a new agency domain mapping with RLS context.
    pub async fn create_rls<'e, E>(
        &self,
        executor: E,
        data: CreateAgencyDomain,
    ) -> Result<AgencyDomain, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let kind = data
            .kind
            .map(|k| k.as_str())
            .unwrap_or("subdomain")
            .to_string();

        let domain = sqlx::query_as::<_, AgencyDomain>(
            r#"
            INSERT INTO agency_domains (organization_id, host, kind, is_primary)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(&data.host)
        .bind(&kind)
        .bind(data.is_primary)
        .fetch_one(executor)
        .await?;

        Ok(domain)
    }

    /// List all domain mappings for an organization with RLS context.
    pub async fn list_by_org_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<AgencyDomain>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let domains = sqlx::query_as::<_, AgencyDomain>(
            r#"
            SELECT * FROM agency_domains
            WHERE organization_id = $1
            ORDER BY is_primary DESC, created_at ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await?;

        Ok(domains)
    }

    /// Find a domain mapping by its host with RLS context.
    ///
    /// Note: under RLS this only returns rows for the caller's own tenant.
    /// For pre-auth resolution use [`resolve_host_system`].
    pub async fn find_by_host_rls<'e, E>(
        &self,
        executor: E,
        host: &str,
    ) -> Result<Option<AgencyDomain>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let domain = sqlx::query_as::<_, AgencyDomain>(
            r#"
            SELECT * FROM agency_domains WHERE host = $1
            "#,
        )
        .bind(host)
        .fetch_optional(executor)
        .await?;

        Ok(domain)
    }
}

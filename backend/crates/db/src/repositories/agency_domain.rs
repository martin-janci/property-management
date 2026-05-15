//! Agency domain repository (Phase 1: Tenant Resolution Keystone).
//!
//! # RLS Integration
//!
//! This repository supports two usage patterns:
//!
//! 1. **System lookups** (`*_system`): used by the host-resolution middleware
//!  BEFORE any tenant context exists. They acquire their own connection,
//!  set a system (super-admin / RLS-bypassing) context for exactly one
//!  constrained `SELECT`, then clear the context before returning the
//!  connection to the pool. This is the ONLY sanctioned way to read
//!  `agency_domains` / `organizations` pre-auth.
//!
//! 2. **RLS-aware** (`*_rls`): methods that accept an executor whose RLS
//!  context is already set (e.g. from an `RlsConnection` extractor). These
//!  are used by authenticated, tenant-scoped handlers (domain management UI).
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
use async_trait::async_trait;
use sqlx::{Error as SqlxError, Executor, Postgres};
use std::sync::Arc;
use uuid::Uuid;

/// Defense for N6 — host -> tenant cache invalidation hook.
///
/// `TenantResolutionCache` (in `api-core::middleware::host_tenant`) caches the
/// `agency_domains` resolution for up to its TTL (5 min positive / 30 s
/// negative). After a domain mutation that changes the host -> org mapping
/// (insert, release, verification flip, …), an unsuspecting subsequent
/// request can hit the cache and see the stale resolution. Worse, an
/// attacker who registers a domain that was just released can briefly
/// impersonate the previous tenant.
///
/// To break that window, every write method in [`AgencyDomainRepository`]
/// invokes [`AgencyDomainCacheInvalidator::invalidate`] for the host that
/// was touched. The trait lives here (in `db`) so the repository does not
/// need a dependency on `api-core` — the binary wires the impl by passing
/// an `Arc<dyn AgencyDomainCacheInvalidator>` to [`AgencyDomainRepository::with_cache`].
///
/// Tests in `db` can plug a [`NoopDomainCacheInvalidator`] so the invariant
/// "writes invalidate the cache" can be exercised without an `api-core`
/// dependency.
#[async_trait]
pub trait AgencyDomainCacheInvalidator: Send + Sync {
    /// Drop the cached resolution for `host` (no-op if not cached).
    async fn invalidate(&self, host: &str);
    /// Drop EVERY cached resolution. Used after bulk operations.
    async fn invalidate_all(&self);
}

/// Default no-op invalidator. Used when no cache is wired (tests, scripts,
/// the `system` resolution paths that do not need invalidation).
#[derive(Debug, Clone, Default)]
pub struct NoopDomainCacheInvalidator;

#[async_trait]
impl AgencyDomainCacheInvalidator for NoopDomainCacheInvalidator {
    async fn invalidate(&self, _host: &str) {}
    async fn invalidate_all(&self) {}
}

/// Repository for agency domain operations.
#[derive(Clone)]
pub struct AgencyDomainRepository {
    pool: DbPool,
    cache: Arc<dyn AgencyDomainCacheInvalidator>,
}

impl AgencyDomainRepository {
    /// Create a new AgencyDomainRepository with no cache wiring (writes do
    /// not invalidate any cache). Convenient for scripts and tests.
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            cache: Arc::new(NoopDomainCacheInvalidator),
        }
    }

    /// Builder hook: wire a cache invalidator. Production code SHOULD use
    /// this — it is what defends against the "stale resolution after a
    /// release/verify" window described on
    /// [`AgencyDomainCacheInvalidator`].
    pub fn with_cache(mut self, cache: Arc<dyn AgencyDomainCacheInvalidator>) -> Self {
        self.cache = cache;
        self
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
    ///
    /// # Cache invalidation (N6)
    ///
    /// On success, invalidates the cached resolution for `data.host`. The
    /// invalidation runs eagerly in the same task; it MUST happen even if
    /// the surrounding transaction has not yet committed, because the cache
    /// holds a *negative* entry while the host was unknown (TTL 30 s) and
    /// we want the next request post-commit to go to the DB. Worst case the
    /// caller's tx rolls back — the cache simply repopulates the negative
    /// entry on the next request, no correctness loss.
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

        // N6: drop any cached (negative) entry for this host so the next
        // resolution falls through to the DB.
        self.cache.invalidate(&domain.host).await;

        Ok(domain)
    }

    /// Release (delete) an agency domain mapping with RLS context.
    ///
    /// Used when a custom domain is unbound from a tenant (host returns to
    /// the platform's pool of available hosts). Returns the deleted host so
    /// callers can audit-log it; returns `None` if no row matched.
    ///
    /// # Cache invalidation (N6)
    ///
    /// On success, invalidates the cached positive resolution. Without this,
    /// a request that hits the cache during the TTL window resolves to the
    /// previous tenant — and an attacker who races to register the just-
    /// released host can briefly impersonate the previous tenant.
    pub async fn release_rls<'e, E>(
        &self,
        executor: E,
        domain_id: Uuid,
    ) -> Result<Option<String>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row: Option<(String,)> = sqlx::query_as(
            r#"
            DELETE FROM agency_domains
            WHERE id = $1
            RETURNING host
            "#,
        )
        .bind(domain_id)
        .fetch_optional(executor)
        .await?;

        if let Some((ref host,)) = row {
            // N6: drop the cached positive entry. Critical: an attacker who
            // races to register this host elsewhere must NOT see the
            // previous tenant's resolution from the cache.
            self.cache.invalidate(host).await;
        }

        Ok(row.map(|(h,)| h))
    }

    /// Flip an agency domain's `verification_state` (e.g. pending -> verified)
    /// with RLS context. Returns the updated row, or `None` if no row matched.
    ///
    /// # Cache invalidation (N6)
    ///
    /// On success, invalidates the cached resolution: a negative cache entry
    /// from when the domain was `pending` would otherwise hide the freshly
    /// `verified` row for up to the negative TTL.
    pub async fn update_verification_state_rls<'e, E>(
        &self,
        executor: E,
        domain_id: Uuid,
        new_state: &str,
    ) -> Result<Option<AgencyDomain>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let updated = sqlx::query_as::<_, AgencyDomain>(
            r#"
            UPDATE agency_domains
            SET verification_state = $2,
                verified_at = CASE WHEN $2 = 'verified' THEN NOW() ELSE verified_at END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(domain_id)
        .bind(new_state)
        .fetch_optional(executor)
        .await?;

        if let Some(ref row) = updated {
            // N6: drop the cached entry (negative or positive) so the next
            // resolution sees the new state.
            self.cache.invalidate(&row.host).await;
        }

        Ok(updated)
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

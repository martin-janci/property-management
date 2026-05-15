//! RLS-enabled database connection extractor.
//!
//! This module provides a request-scoped database connection with Row-Level Security
//! context properly set. Unlike pool-level approaches, this ensures RLS context is
//! bound to the exact connection used for queries.
//!
//! # Security
//!
//! The connection is acquired with RLS context set before being handed to handlers.
//! **IMPORTANT**: Call `release()` when done to clear RLS context before returning
//! the connection to the pool. This prevents context bleeding between requests.
//!
//! # Usage
//!
//! ```rust,ignore
//! async fn handler(
//!     mut rls: RlsConnection,
//! ) -> Result<Json<Response>, StatusCode> {
//!     // Use rls.conn() for all database operations
//!     let items = sqlx::query_as("SELECT * FROM items")
//!         .fetch_all(rls.conn())
//!         .await?;
//!
//!     // IMPORTANT: Release clears RLS context before returning connection to pool
//!     rls.release().await;
//!
//!     Ok(Json(items))
//! }
//! ```
//!
//! # Context Bleeding Prevention
//!
//! Without calling `release()`, the RLS context (including super-admin privileges)
//! could persist on the pooled connection and affect subsequent requests that don't
//! use RLS extractors. Always call `release()` at the end of handler logic.

use crate::extractors::tenant::TenantMembershipProvider;
use crate::extractors::ValidatedTenantExtractor;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use common::TenantRole;
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

/// A database connection with RLS context set for the current request.
///
/// This extractor:
/// 1. Validates authentication via `ValidatedTenantExtractor`
/// 2. Acquires a dedicated connection from the pool
/// 3. Sets PostgreSQL RLS context (`set_request_context`) on that connection
/// 4. Provides the connection to handlers
///
/// # Connection Lifecycle
///
/// **CRITICAL**: Call `release()` when done with the connection to clear RLS context
/// before returning to the pool. This prevents privilege escalation from context bleeding.
///
/// # Example
///
/// ```rust,ignore
/// async fn get_building(
///     mut rls: RlsConnection,
///     Path(building_id): Path<Uuid>,
/// ) -> Result<Json<Building>, StatusCode> {
///     // RLS policies automatically filter by tenant
///     let building = sqlx::query_as::<_, Building>(
///         "SELECT * FROM buildings WHERE id = $1"
///     )
///     .bind(building_id)
///     .fetch_optional(rls.conn())
///     .await
///     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
///     .ok_or(StatusCode::NOT_FOUND)?;
///
///     // Clear RLS context before returning connection to pool
///     rls.release().await;
///
///     Ok(Json(building))
/// }
/// ```
pub struct RlsConnection {
    conn: Option<PoolConnection<Postgres>>,
    tenant_id: Uuid,
    user_id: Uuid,
    role: TenantRole,
    released: bool,
}

impl RlsConnection {
    /// Get a mutable reference to the underlying connection.
    ///
    /// Use this for all database queries to ensure RLS context is applied.
    ///
    /// # Panics
    ///
    /// Panics if called after `release()` or `into_inner()`.
    pub fn conn(&mut self) -> &mut PoolConnection<Postgres> {
        self.conn.as_mut().expect("RlsConnection already released")
    }

    /// Get the tenant ID for this request.
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Get the user ID for this request.
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Get the user's role in this tenant.
    pub fn role(&self) -> TenantRole {
        self.role
    }

    /// Check if the user has at least the specified role level.
    pub fn has_role(&self, required: TenantRole) -> bool {
        self.role.level() >= required.level()
    }

    /// Check if the user is a super admin (bypasses RLS).
    pub fn is_super_admin(&self) -> bool {
        matches!(
            self.role,
            TenantRole::SuperAdmin | TenantRole::PlatformAdmin
        )
    }

    /// Release the connection back to the pool after clearing RLS context.
    ///
    /// **IMPORTANT**: Always call this when done with database operations to prevent
    /// RLS context from bleeding into subsequent requests using this pooled connection.
    ///
    /// This method:
    /// 1. Calls `clear_request_context()` on the connection
    /// 2. Returns the connection to the pool
    ///
    /// After calling `release()`, the connection can no longer be used.
    pub async fn release(&mut self) {
        if self.released {
            return;
        }

        if let Some(mut conn) = self.conn.take() {
            // Clear RLS context before returning to pool
            if let Err(e) = db::tenant_context::clear_request_context(&mut *conn).await {
                tracing::warn!(
                    error = %e,
                    tenant_id = %self.tenant_id,
                    user_id = %self.user_id,
                    "Failed to clear RLS context on release"
                );
            } else {
                tracing::trace!(
                    tenant_id = %self.tenant_id,
                    user_id = %self.user_id,
                    "RLS context cleared, connection released to pool"
                );
            }
            // Connection is dropped here, returning to pool
        }

        self.released = true;
    }

    /// Consume the RlsConnection and return the raw connection WITHOUT clearing context.
    ///
    /// **WARNING**: This bypasses the safety mechanism. Only use if you need to pass
    /// the connection to code that will handle cleanup itself.
    ///
    /// Prefer `release()` in normal usage.
    pub fn into_inner(mut self) -> Option<PoolConnection<Postgres>> {
        self.released = true; // Prevent Drop from warning
        self.conn.take()
    }
}

impl Deref for RlsConnection {
    type Target = PoolConnection<Postgres>;

    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().expect("RlsConnection already released")
    }
}

impl DerefMut for RlsConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().expect("RlsConnection already released")
    }
}

/// When dropped without calling release(), spawn a task to clear context.
impl Drop for RlsConnection {
    fn drop(&mut self) {
        if !self.released {
            if let Some(conn) = self.conn.take() {
                // Can't do async cleanup in Drop, so we spawn a task
                tracing::warn!(
                    tenant_id = %self.tenant_id,
                    user_id = %self.user_id,
                    role = ?self.role,
                    "RlsConnection dropped without calling release() - spawning cleanup task. \
                     Call rls.release().await before handler returns for guaranteed cleanup."
                );

                // Spawn async task to clear context before returning connection to pool
                db::tenant_context::spawn_clear_context(
                    conn,
                    format!(
                        "RlsConnection(tenant={}, user={}, role={:?})",
                        self.tenant_id, self.user_id, self.role
                    ),
                );
            }
        }
    }
}

impl<S> FromRequestParts<S> for RlsConnection
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Step 1: Validate authentication and tenant membership
        // This extractor does all the security checks (JWT validation, membership, etc.)
        let tenant = ValidatedTenantExtractor::from_request_parts(parts, state).await?;

        let tenant_id = tenant.tenant_id;
        let user_id = tenant.user_id;
        let role = tenant.role;
        let is_super_admin = matches!(role, TenantRole::SuperAdmin | TenantRole::PlatformAdmin);

        // Step 2: Acquire a dedicated connection from the pool
        let mut conn = state.db_pool().acquire().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to acquire database connection for RLS");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database connection unavailable",
            )
        })?;

        // Step 3: Set RLS context on THIS specific connection
        // This is the critical fix: we set context on the connection we'll use,
        // not on the pool.
        db::tenant_context::set_request_context(
            &mut *conn,
            Some(tenant_id),
            Some(user_id),
            is_super_admin,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                tenant_id = %tenant_id,
                user_id = %user_id,
                "Failed to set RLS context"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to set security context",
            )
        })?;

        tracing::debug!(
            tenant_id = %tenant_id,
            user_id = %user_id,
            role = ?role,
            is_super_admin = is_super_admin,
            "RLS context set on connection"
        );

        Ok(RlsConnection {
            conn: Some(conn),
            tenant_id,
            user_id,
            role,
            released: false,
        })
    }
}

/// A simpler RLS connection for routes that only need tenant context (no membership validation).
///
/// Use this for less sensitive routes where you trust the JWT-based tenant ID.
/// For sensitive routes, use `RlsConnection` which validates database membership.
///
/// **IMPORTANT**: Call `release()` when done to clear RLS context.
pub struct SimpleRlsConnection {
    conn: Option<PoolConnection<Postgres>>,
    tenant_id: Uuid,
    user_id: Uuid,
    role: TenantRole,
    released: bool,
}

impl SimpleRlsConnection {
    /// Get a mutable reference to the underlying connection.
    pub fn conn(&mut self) -> &mut PoolConnection<Postgres> {
        self.conn
            .as_mut()
            .expect("SimpleRlsConnection already released")
    }

    /// Get the tenant ID for this request.
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Get the user ID for this request.
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Get the user's role.
    pub fn role(&self) -> TenantRole {
        self.role
    }

    /// Release the connection back to the pool after clearing RLS context.
    ///
    /// See `RlsConnection::release()` for details.
    pub async fn release(&mut self) {
        if self.released {
            return;
        }

        if let Some(mut conn) = self.conn.take() {
            if let Err(e) = db::tenant_context::clear_request_context(&mut *conn).await {
                tracing::warn!(
                    error = %e,
                    tenant_id = %self.tenant_id,
                    "Failed to clear RLS context on release"
                );
            }
        }

        self.released = true;
    }
}

impl Deref for SimpleRlsConnection {
    type Target = PoolConnection<Postgres>;

    fn deref(&self) -> &Self::Target {
        self.conn
            .as_ref()
            .expect("SimpleRlsConnection already released")
    }
}

impl DerefMut for SimpleRlsConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn
            .as_mut()
            .expect("SimpleRlsConnection already released")
    }
}

impl Drop for SimpleRlsConnection {
    fn drop(&mut self) {
        if !self.released {
            if let Some(conn) = self.conn.take() {
                tracing::warn!(
                    tenant_id = %self.tenant_id,
                    "SimpleRlsConnection dropped without calling release() - spawning cleanup task"
                );

                db::tenant_context::spawn_clear_context(
                    conn,
                    format!("SimpleRlsConnection(tenant={})", self.tenant_id),
                );
            }
        }
    }
}

impl<S> FromRequestParts<S> for SimpleRlsConnection
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        use crate::extractors::TenantExtractor;

        // Use simple tenant extractor (no DB membership check)
        let tenant = TenantExtractor::from_request_parts(parts, state).await?;

        let tenant_id = tenant.tenant_id;
        let user_id = tenant.user_id;
        let role = tenant.role;
        let is_super_admin = matches!(role, TenantRole::SuperAdmin | TenantRole::PlatformAdmin);

        // Acquire connection and set RLS context
        let mut conn = state.db_pool().acquire().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to acquire database connection");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database connection unavailable",
            )
        })?;

        db::tenant_context::set_request_context(
            &mut *conn,
            Some(tenant_id),
            Some(user_id),
            is_super_admin,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to set RLS context");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to set security context",
            )
        })?;

        Ok(SimpleRlsConnection {
            conn: Some(conn),
            tenant_id,
            user_id,
            role,
            released: false,
        })
    }
}

/// An RLS connection whose tenant context comes from host resolution, NOT from
/// a JWT.
///
/// Unlike [`RlsConnection`] / [`SimpleRlsConnection`] (which derive the tenant
/// from an authenticated token), this extractor reads the `ResolvedTenant` that
/// `host_tenant_middleware` injected into the request extensions. It is the
/// bridge between Phase 1 host resolution and RLS-scoped queries: use it on
/// routes that are tenant-scoped by *host* rather than by *login* (e.g. public
/// per-agency listing pages on the reality portal).
///
/// It carries the SAME `release()` + `Drop`/`spawn_clear_context` discipline as
/// [`SimpleRlsConnection`] — always call `release().await` before the handler
/// returns to clear RLS context before the connection returns to the pool.
///
/// # Phase 4: PlatformHost (4th RLS context)
///
/// When the resolved tenant's `source` is
/// [`crate::middleware::host_tenant::TenantSource::PlatformHost`] this
/// extractor opens the connection in the **global-read** context instead of
/// in a tenant-scoped one:
/// * `app.current_org_id` is UNSET (empty) — `get_current_org_id()` returns
///   NULL, so the second branch of the listings policy collapses.
/// * `app.global_read = on` — the third branch of the listings policy
///   (`is_published AND is_global_read_context()`) opens up.
/// * Writes still fail (the policy's WITH CHECK omits the global branch).
///
/// `clear_request_context()` resets BOTH the tenant trio and `app.global_read`,
/// so the same `release()` call works for either path. Defense for leak #2
/// (context bleeding between requests).
///
/// # Rejection
///
/// Rejects with `500` if no `ResolvedTenant` extension is present — that means
/// `host_tenant_middleware` did not run (or the host was unresolved, in which
/// case the middleware itself already returned `404`).
pub struct HostRlsConnection {
    conn: Option<PoolConnection<Postgres>>,
    organization_id: Uuid,
    is_platform_host: bool,
    released: bool,
}

impl HostRlsConnection {
    /// Get a mutable reference to the underlying connection.
    pub fn conn(&mut self) -> &mut PoolConnection<Postgres> {
        self.conn
            .as_mut()
            .expect("HostRlsConnection already released")
    }

    /// Get the resolved organization (tenant) ID for this request.
    ///
    /// For PlatformHost requests this is `Uuid::nil()` — the sentinel meaning
    /// "no specific tenant." Branch on [`Self::is_platform_host`] before
    /// treating this as a real org id.
    pub fn organization_id(&self) -> Uuid {
        self.organization_id
    }

    /// True if this connection was opened in the PlatformHost (global-read)
    /// context — the 4th RLS context introduced in Phase 4.
    pub fn is_platform_host(&self) -> bool {
        self.is_platform_host
    }

    /// Release the connection back to the pool after clearing RLS context.
    ///
    /// See [`RlsConnection::release()`] for details. The Phase 4 extension to
    /// `clear_request_context()` (migration 00136) also resets `app.global_read`
    /// so the same call cleans up PlatformHost connections.
    pub async fn release(&mut self) {
        if self.released {
            return;
        }

        if let Some(mut conn) = self.conn.take() {
            if let Err(e) = db::tenant_context::clear_request_context(&mut *conn).await {
                tracing::warn!(
                    error = %e,
                    organization_id = %self.organization_id,
                    is_platform_host = self.is_platform_host,
                    "Failed to clear RLS context on release"
                );
            }
        }

        self.released = true;
    }
}

impl Deref for HostRlsConnection {
    type Target = PoolConnection<Postgres>;

    fn deref(&self) -> &Self::Target {
        self.conn
            .as_ref()
            .expect("HostRlsConnection already released")
    }
}

impl DerefMut for HostRlsConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn
            .as_mut()
            .expect("HostRlsConnection already released")
    }
}

impl Drop for HostRlsConnection {
    fn drop(&mut self) {
        if !self.released {
            if let Some(conn) = self.conn.take() {
                tracing::warn!(
                    organization_id = %self.organization_id,
                    is_platform_host = self.is_platform_host,
                    "HostRlsConnection dropped without calling release() - spawning cleanup task"
                );

                db::tenant_context::spawn_clear_context(
                    conn,
                    format!(
                        "HostRlsConnection(org={}, platform_host={})",
                        self.organization_id, self.is_platform_host
                    ),
                );
            }
        }
    }
}

impl<S> FromRequestParts<S> for HostRlsConnection
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Step 1: Read the tenant resolved by host_tenant_middleware.
        let resolved = parts
            .extensions
            .get::<crate::middleware::host_tenant::ResolvedTenant>()
            .copied()
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Tenant not resolved (host_tenant_middleware did not run)",
            ))?;

        let organization_id = resolved.organization_id;
        let is_platform_host = resolved.is_platform_host();

        // Step 2: Acquire a dedicated connection from the pool.
        let mut conn = state.db_pool().acquire().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to acquire database connection for host RLS");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database connection unavailable",
            )
        })?;

        // Step 3: Set RLS context on THIS specific connection. The two paths:
        //
        // * PlatformHost (Phase 4, 4th RLS context) — `app.current_org_id`
        //   stays UNSET (we still call set_request_context to clear any stale
        //   value from a pooled connection), and `app.global_read = on` opens
        //   the global-read OR-clause on `listings`. WITH CHECK still denies
        //   writes (no global branch), so this is read-only by construction.
        //
        // * Tenant-scoped (Subdomain / CustomDomain / DevPath) — set the org
        //   id, leave `app.global_read = off`. Same one-org behaviour as
        //   Phase 1. Defense-in-depth: explicitly setting global_read = off
        //   prevents a stale `on` from a previous PlatformHost request leaking
        //   into a tenant-scoped one (leak #2). The Drop / release path
        //   resets both via clear_request_context().
        if is_platform_host {
            db::tenant_context::set_request_context(&mut *conn, None, None, false)
                .await
                .map_err(|e| {
                    tracing::error!(
                        error = %e,
                        "Failed to clear RLS context for PlatformHost"
                    );
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to set security context",
                    )
                })?;
            db::tenant_context::set_global_read_context(&mut *conn, true)
                .await
                .map_err(|e| {
                    tracing::error!(
                        error = %e,
                        "Failed to enable global-read context for PlatformHost"
                    );
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to set security context",
                    )
                })?;
        } else {
            db::tenant_context::set_request_context(
                &mut *conn,
                Some(organization_id),
                None,
                false,
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    organization_id = %organization_id,
                    "Failed to set RLS context for host-resolved tenant"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to set security context",
                )
            })?;
            // Defense for leak #2: ensure global_read is OFF on every
            // tenant-scoped request, even if the connection arrived with a
            // stale ON from an earlier PlatformHost handler that crashed
            // before release().
            db::tenant_context::set_global_read_context(&mut *conn, false)
                .await
                .map_err(|e| {
                    tracing::error!(
                        error = %e,
                        organization_id = %organization_id,
                        "Failed to disable global-read context for tenant-scoped host"
                    );
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to set security context",
                    )
                })?;
        }

        Ok(HostRlsConnection {
            conn: Some(conn),
            organization_id,
            is_platform_host,
            released: false,
        })
    }
}

/// Helper macro to ensure RLS connection is released even on early returns.
///
/// Usage:
/// ```rust,ignore
/// async fn handler(mut rls: RlsConnection) -> Result<Json<Data>, AppError> {
///     let result = with_rls_release!(rls, {
///         // Your handler logic here
///         let data = sqlx::query_as("SELECT * FROM items")
///             .fetch_all(rls.conn())
///             .await?;
///         Ok(Json(data))
///     });
///     result
/// }
/// ```
#[macro_export]
macro_rules! with_rls_release {
    ($rls:expr, $body:expr) => {{
        let result = $body;
        $rls.release().await;
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_level_check() {
        // This test verifies the role hierarchy is correctly applied
        assert!(TenantRole::Manager.level() > TenantRole::Owner.level());
        assert!(TenantRole::SuperAdmin.level() > TenantRole::Manager.level());
        assert!(TenantRole::Guest.level() < TenantRole::Resident.level());
    }
}

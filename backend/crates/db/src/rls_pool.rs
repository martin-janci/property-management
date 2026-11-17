//! RLS-enforcing database pool wrapper.
//!
//! This module provides a type-safe wrapper around the database pool that
//! enforces RLS context on all acquired connections. It prevents accidental
//! bypass of RLS by making raw pool access impossible.
//!
//! # Design Goals
//!
//! 1. **Type-level enforcement**: You can't accidentally use the raw pool
//! 2. **Automatic cleanup**: Guards clear context on drop
//! 3. **Drop-in replacement**: Same API as raw pool for RLS-aware code
//!
//! # Usage
//!
//! ```rust,ignore
//! // In AppState, use RlsPool instead of DbPool
//! pub struct AppState {
//!     pub db: RlsPool,  // Instead of DbPool
//! }
//!
//! // In handlers, acquire with context
//! async fn handler(State(state): State<AppState>, auth: AuthUser) -> Result<...> {
//!     let mut guard = state.db.acquire_with_rls(
//!         auth.tenant_id,
//!         auth.user_id,
//!         auth.is_super_admin(),
//!     ).await?;
//!
//!     let items = sqlx::query("SELECT * FROM items")
//!         .fetch_all(guard.conn())
//!         .await?;
//!
//!     // Context is automatically cleared when guard is dropped
//!     Ok(Json(items))
//! }
//! ```

use crate::tenant_context::{clear_or_discard, clear_request_context, set_request_context};
use crate::DbPool;
use sqlx::pool::PoolConnection;
use sqlx::Postgres;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

/// A wrapper around DbPool that enforces RLS on all connections.
///
/// This type intentionally does NOT implement `Deref<Target=DbPool>` to prevent
/// accidental raw pool access. All database access must go through
/// `acquire_with_rls()` which sets and clears RLS context.
#[derive(Clone)]
pub struct RlsPool {
    inner: DbPool,
}

impl RlsPool {
    /// Create a new RLS-enforcing pool wrapper.
    pub fn new(pool: DbPool) -> Self {
        Self { inner: pool }
    }

    /// Acquire a connection with RLS context set.
    ///
    /// This is the ONLY way to get a connection from RlsPool.
    /// The returned guard will automatically clear RLS context when dropped.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` - The tenant/organization ID for RLS filtering
    /// * `user_id` - The user ID for RLS policies
    /// * `is_super_admin` - Whether the user bypasses RLS
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut guard = pool.acquire_with_rls(tenant_id, user_id, false).await?;
    /// let rows = sqlx::query("SELECT * FROM items")
    ///     .fetch_all(guard.conn())
    ///     .await?;
    /// // Context cleared when guard drops
    /// ```
    pub async fn acquire_with_rls(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        is_super_admin: bool,
    ) -> Result<RlsGuard, sqlx::Error> {
        let mut conn = self.inner.acquire().await?;

        // Set RLS context on the connection
        set_request_context(&mut *conn, Some(tenant_id), Some(user_id), is_super_admin).await?;

        tracing::debug!(
            tenant_id = %tenant_id,
            user_id = %user_id,
            is_super_admin = is_super_admin,
            "RLS context set on pooled connection"
        );

        Ok(RlsGuard {
            conn: Some(conn),
            tenant_id,
            user_id,
        })
    }

    /// Acquire a connection WITHOUT RLS context (for public/unauthenticated routes).
    ///
    /// Use this sparingly - only for routes that genuinely don't need tenant isolation:
    /// - Health checks
    /// - Public listing search (reality portal)
    /// - Authentication endpoints
    ///
    /// The returned connection is still wrapped in a guard for consistency,
    /// but no RLS context is set.
    ///
    /// # Safety
    ///
    /// This method explicitly clears any stale RLS context from a previous request
    /// to prevent context bleeding. If that clear **fails**, the connection may
    /// still carry the RLS-bypassing super-admin / global-read context of the
    /// previous request, so this method does NOT hand it to the public caller:
    /// it discards the connection (detach + hard-close, via
    /// [`clear_or_discard`]'s sibling [`crate::tenant_context::discard_poisoned_connection`])
    /// and returns the error. The pool opens a fresh, context-free connection on
    /// the next acquire (issue #3583).
    pub async fn acquire_public(&self) -> Result<PublicConnection, sqlx::Error> {
        let mut conn = self.inner.acquire().await?;

        // Clear any stale RLS context from previous requests to prevent context bleeding.
        // This is critical: a connection might have super-admin context from a previous
        // request. A *failed* clear is NOT recoverable by logging — the connection could
        // run the public request with tenant isolation disabled — so we discard it and
        // fail the acquire rather than return a possibly-elevated connection.
        if let Err(e) = clear_request_context(&mut *conn).await {
            tracing::error!(
                error = %e,
                "SECURITY: failed to clear stale RLS context on public connection acquire - \
                 discarding connection so a stale super-admin/elevated context cannot bleed \
                 into a public (unauthenticated) request"
            );
            crate::tenant_context::discard_poisoned_connection(conn, "RlsPool::acquire_public")
                .await;
            return Err(e);
        }

        Ok(PublicConnection { conn })
    }

    /// Get the underlying pool for health checks and diagnostics only.
    ///
    /// This returns a reference that can be used for pool statistics,
    /// but NOT for acquiring connections directly.
    pub fn pool_ref(&self) -> &DbPool {
        &self.inner
    }

    /// Check if the pool is closed.
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }

    /// Close the pool.
    pub async fn close(&self) {
        self.inner.close().await
    }
}

/// A guard that holds a connection with RLS context set.
///
/// When dropped, the guard automatically clears the RLS context before
/// returning the connection to the pool. This prevents context bleeding.
pub struct RlsGuard {
    conn: Option<PoolConnection<Postgres>>,
    tenant_id: Uuid,
    user_id: Uuid,
}

impl RlsGuard {
    /// Get a mutable reference to the underlying connection.
    ///
    /// Use this for all database queries.
    pub fn conn(&mut self) -> &mut PoolConnection<Postgres> {
        self.conn.as_mut().expect("RlsGuard already released")
    }

    /// Get the tenant ID for this connection.
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Get the user ID for this connection.
    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Explicitly release the connection, clearing RLS context.
    ///
    /// This is called automatically on drop, but you can call it explicitly
    /// if you want to release the connection early. If the context clear fails,
    /// the connection is discarded (not returned to the pool) to prevent an
    /// elevated RLS context from bleeding across requests (issue #3583).
    pub async fn release(&mut self) {
        if let Some(conn) = self.conn.take() {
            clear_or_discard(
                conn,
                &format!("RlsGuard(tenant={}, user={})", self.tenant_id, self.user_id),
            )
            .await;
        }
    }
}

impl Deref for RlsGuard {
    type Target = PoolConnection<Postgres>;

    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().expect("RlsGuard already released")
    }
}

impl DerefMut for RlsGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().expect("RlsGuard already released")
    }
}

impl Drop for RlsGuard {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            // We can't do async cleanup in Drop, so we spawn a task to clear context.
            // This provides best-effort cleanup to prevent context bleeding.
            //
            // For guaranteed cleanup, handlers should call release() explicitly.
            tracing::warn!(
                tenant_id = %self.tenant_id,
                user_id = %self.user_id,
                "RlsGuard dropped without calling release() - spawning cleanup task. \
                 Call guard.release().await for guaranteed cleanup."
            );

            // Spawn async task to clear context before returning connection to pool
            crate::tenant_context::spawn_clear_context(
                conn,
                format!("RlsGuard(tenant={}, user={})", self.tenant_id, self.user_id),
            );
        }
    }
}

/// A connection for public/unauthenticated routes.
///
/// This is a simple wrapper without RLS context management.
/// Use for health checks, public search, etc.
pub struct PublicConnection {
    conn: PoolConnection<Postgres>,
}

impl PublicConnection {
    /// Get a mutable reference to the underlying connection.
    pub fn conn(&mut self) -> &mut PoolConnection<Postgres> {
        &mut self.conn
    }
}

impl Deref for PublicConnection {
    type Target = PoolConnection<Postgres>;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl DerefMut for PublicConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rls_pool_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RlsPool>();
    }
}

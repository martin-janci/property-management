//! Tenant context utilities for Row-Level Security.
//!
//! This module provides utilities to set tenant context for database operations,
//! enabling PostgreSQL RLS policies to enforce data isolation.

use sqlx::pool::PoolConnection;
use sqlx::{Connection, Error as SqlxError, Executor, Postgres};
use tracing::Instrument;
use uuid::Uuid;

/// Set the tenant context for the current database session.
/// This must be called before any tenant-scoped queries.
///
/// # Arguments
/// * `executor` - A database executor (pool, connection, or transaction)
/// * `org_id` - The organization ID for tenant context
/// * `user_id` - The user ID for RLS policies
/// * `is_super_admin` - Whether the user is a super admin (bypasses RLS)
pub async fn set_request_context<'e, E>(
    executor: E,
    org_id: Option<Uuid>,
    user_id: Option<Uuid>,
    is_super_admin: bool,
) -> Result<(), SqlxError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(is_super_admin)
        .execute(executor)
        .await?;

    Ok(())
}

/// Clear the tenant context after a request.
pub async fn clear_request_context<'e, E>(executor: E) -> Result<(), SqlxError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("SELECT clear_request_context()")
        .execute(executor)
        .await?;

    Ok(())
}

/// Clear the RLS request context on a pooled connection as it is released, and
/// **discard the connection if the clear fails** so elevated context can never
/// bleed to the next request (issue #3583, follow-up to #3577).
///
/// This consumes the owned [`PoolConnection`]. On the happy path the connection
/// is dropped normally after a successful `clear_request_context`, returning to
/// the pool with a clean (non-super-admin, non-global-read) context. If the
/// clear **fails**, the connection may still hold the RLS-bypassing super-admin
/// / global-read context it was placed into, so returning it to the pool would
/// let the next checkout run with tenant isolation disabled. To prevent that we
/// [`PoolConnection::detach`] it — detaching severs it from the pool, so
/// dropping it closes the socket instead of recycling it — and best-effort
/// hard-close it. The pool then opens a fresh, context-free connection on next
/// demand.
///
/// This is the single hardened primitive every connection-release site should
/// use (`RlsGuard::release`, the `Drop` path via [`spawn_clear_context`], and
/// SYSTEM-context route handlers), so the "clear or discard" discipline lives
/// in one place.
pub async fn clear_or_discard(mut conn: PoolConnection<Postgres>, context_info: &str) {
    match clear_request_context(&mut *conn).await {
        Ok(()) => {
            tracing::trace!(
                context = %context_info,
                "RLS context cleared; connection returning to pool"
            );
            // `conn` drops here -> returns to the pool with a clean context.
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                context = %context_info,
                "SECURITY: failed to clear RLS context - discarding connection to \
                 prevent super-admin/elevated context from bleeding across the pool"
            );
            // Detach from the pool so the (possibly still-elevated) connection
            // is NOT recycled, then hard-close it. The pool opens a fresh,
            // context-free connection on the next acquire.
            let detached = conn.detach();
            if let Err(close_err) = detached.close().await {
                tracing::warn!(
                    error = %close_err,
                    context = %context_info,
                    "failed to hard-close discarded connection (already detached \
                     from the pool, so it will not be reused regardless)"
                );
            }
        }
    }
}

/// Spawn a task to clear RLS context on a connection.
///
/// This is used by Drop implementations that cannot await. The task runs
/// asynchronously and, via [`clear_or_discard`], **discards** the connection
/// if the clear fails rather than letting an elevated context bleed back into
/// the pool.
///
/// # Safety
///
/// This provides best-effort cleanup. For guaranteed cleanup, always call
/// `release().await` explicitly before dropping the guard.
pub fn spawn_clear_context(conn: PoolConnection<Postgres>, context_info: String) {
    let span_context = context_info.clone();
    tokio::spawn(
        async move {
            clear_or_discard(conn, &context_info).await;
        }
        .instrument(tracing::info_span!(
            "bg.tenant_context_eviction",
            context = %span_context,
        )),
    );
}

/// Set only the tenant (organization) context.
pub async fn set_tenant_context<'e, E>(executor: E, org_id: Uuid) -> Result<(), SqlxError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("SELECT set_tenant_context($1)")
        .bind(org_id)
        .execute(executor)
        .await?;

    Ok(())
}

/// Phase 4: opt the current connection into (or out of) the 4th RLS context
/// (PlatformHost / global-read).
///
/// Always pair this with [`set_request_context`] (with no org / no user / not
/// super-admin) so the connection's state is fully explicit. The migrations
/// extend [`clear_request_context`] to also reset this flag — defense for
/// leak #2 (context bleeding between requests).
pub async fn set_global_read_context<'e, E>(executor: E, enabled: bool) -> Result<(), SqlxError>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query("SELECT set_global_read_context($1)")
        .bind(enabled)
        .execute(executor)
        .await?;
    Ok(())
}

/// Check if a user has a specific permission in an organization.
pub async fn user_has_permission<'e, E>(
    executor: E,
    user_id: Uuid,
    org_id: Uuid,
    permission: &str,
) -> Result<bool, SqlxError>
where
    E: Executor<'e, Database = Postgres>,
{
    let result = sqlx::query_scalar::<_, bool>("SELECT user_has_permission($1, $2, $3)")
        .bind(user_id)
        .bind(org_id)
        .bind(permission)
        .fetch_one(executor)
        .await?;

    Ok(result)
}

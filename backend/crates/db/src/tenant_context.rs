//! Tenant context utilities for Row-Level Security.
//!
//! This module provides utilities to set tenant context for database operations,
//! enabling PostgreSQL RLS policies to enforce data isolation.

use sqlx::{Error as SqlxError, Executor, Postgres};
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

/// Spawn a task to clear RLS context on a connection.
///
/// This is used by Drop implementations that cannot await. The task runs
/// asynchronously and logs any errors.
///
/// # Safety
///
/// This provides best-effort cleanup. For guaranteed cleanup, always call
/// `release().await` explicitly before dropping the guard.
pub fn spawn_clear_context(mut conn: sqlx::pool::PoolConnection<Postgres>, context_info: String) {
    let span_context = context_info.clone();
    tokio::spawn(
        async move {
            if let Err(e) = clear_request_context(&mut *conn).await {
                tracing::error!(
                    error = %e,
                    context = %context_info,
                    "SECURITY: Failed to clear RLS context in Drop cleanup task - context may bleed"
                );
            } else {
                tracing::debug!(
                    context = %context_info,
                    "RLS context cleared via Drop cleanup task"
                );
            }
            // Connection is dropped here, returning to pool with cleared context
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

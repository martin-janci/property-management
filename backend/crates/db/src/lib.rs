#![allow(clippy::doc_overindented_list_items)]
//! Database layer: models and repositories.
//!
//! # Row-Level Security (RLS)
//!
//! This crate provides RLS-aware database access patterns to ensure tenant isolation.
//!
//! ## Key Types
//!
//! - [`RlsPool`]: Type-safe pool wrapper that enforces RLS on all connections
//! - [`RlsGuard`]: Connection guard that automatically clears context on drop
//! - [`PublicConnection`]: For unauthenticated routes (clears stale context on acquire)
//!
//! ## Security Guarantees
//!
//! 1. **Drop Safety**: If `release()` is not called, a background task clears context
//! 2. **Public Connection Safety**: `acquire_public()` clears any stale RLS context
//! 3. **CI Enforcement**: RLS violations are caught by `check-rls-enforcement.sh`
//! 4. **PR Smoke Tests**: Basic tenant isolation is verified on every PR
//!
//! ## Migration Path (Type-Level Enforcement)
//!
//! The long-term goal is type-level enforcement where repositories require
//! `RlsConnection` instead of raw `DbPool`. This eliminates the footgun of
//! accidentally using the pool without RLS context.
//!
//! Current state: Handlers use `RlsConnection` extractor, repositories still
//! accept raw pool but have RLS-aware `*_rls()` method variants.
//!
//! Future state: Replace `DbPool` with `RlsPool` in `AppState` and update
//! repository signatures to require `&mut RlsGuard` or similar.

pub mod models;
pub mod repositories;
pub mod rls_pool;
pub mod seed;
pub mod tenant_context;

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

pub type DbPool = sqlx::PgPool;

/// Re-export sqlx error for use in route handlers.
pub use sqlx::Error as SqlxError;

// Re-export tenant context utilities
pub use tenant_context::{
    clear_request_context, set_request_context, set_tenant_context, spawn_clear_context,
    user_has_permission,
};

// Re-export RLS pool types for type-level enforcement
pub use rls_pool::{PublicConnection, RlsGuard, RlsPool};

// Re-export repositories for direct import
pub use repositories::FormRepository;

/// Run all pending SQL migrations against the given pool.
///
/// Applies the 100+ migrations under `backend/crates/db/migrations/` in
/// numeric order. The migrator embeds every file in the binary at compile
/// time via `sqlx::migrate!`, so the running container doesn't need disk
/// access to `crates/db/migrations`.
///
/// Idempotent and concurrency-safe: sqlx tracks applied versions in a
/// `_sqlx_migrations` table and uses a Postgres advisory lock around the
/// run, so two backends starting at the same time (api-server +
/// reality-server in the same blue/green color) won't race — the second
/// caller waits and then sees zero pending migrations.
///
/// Call this once on each server's startup, after creating the pool and
/// before serving traffic. The first deploy of a target turns its empty
/// `ppt_<target>` database into the full schema; subsequent deploys
/// no-op as long as no new migrations are added.
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Embedded migrator. Use as `#[sqlx::test(migrator = "db::MIGRATOR")]` from
/// integration tests in other crates so they get the same migration set as
/// the production binary.
///
/// # Getting the full schema into a `#[sqlx::test]` (test-author guide)
///
/// `#[sqlx::test]` creates a **fresh, empty, uniquely-named database per test
/// function** and only applies migrations when a migrator is supplied. There
/// are two supported idioms, and they apply the *same* migration set — pick by
/// what the test needs:
///
/// 1. **Schema-full tests** (touch any table — register a user, seed an org,
///    etc.) — attach the migrator. Either form works and is equivalent:
///    - `#[sqlx::test(migrator = "db::MIGRATOR")]` — the macro emits
///      `args.migrator(&db::MIGRATOR)`, and sqlx runs `MIGRATOR.run_direct(..)`
///      during per-test DB setup. This is the default idiom (~100 api-server
///      test binaries use it).
///    - bare `#[sqlx::test]` + `db::run_migrations(&pool).await` as the first
///      line of the test body — runs `MIGRATOR.run(pool)` (the *production*
///      path: idempotent + advisory-locked). Equivalent schema, and turns any
///      migration failure into a loud, attributable panic *inside* the test.
///      Use this when you want the production migration entrypoint exercised,
///      or to keep a binary uniform with schema-less tests (see below).
///
/// 2. **Schema-less tests** (auth-rejection / smoke tests that assert
///    401/400/404 *before* any DB query) — bare `#[sqlx::test]` with **no**
///    migrator. The macro's `InferredPath` looks for a `./migrations` dir
///    relative to the *test crate* (`servers/api-server/migrations`), which
///    does not exist, so no migrator is attached and the test runs against an
///    empty database. This is correct and intended for these tests.
///
/// ## Gotcha: don't mix the two idioms ad-hoc in one body
///
/// A bare `#[sqlx::test]` whose body *does* hit a table without first calling
/// `db::run_migrations(&pool)` will fail with `relation "..." does not exist`
/// (the DB is empty). This bit the voting PDF-report tests (#1656/#1665): they
/// were authored bare alongside schema-less auth tests in `voting_tests.rs`.
/// The macro expands each test fn independently, so a `migrator = ` test is
/// never silently disabled by a sibling schema-less test — but a bare
/// schema-full test *will* see an empty DB. The fix was to call
/// `db::run_migrations(&pool)` explicitly (idiom 1, second form). If you ever
/// see an unexplained missing-relation 500 in a `#[sqlx::test]`, check that the
/// failing test actually attaches a migrator by one of the two idioms above.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Create database connection pool.
///
/// This creates a standard pool without RLS cleanup hooks.
/// For production use with RLS, prefer `create_rls_safe_pool`.
pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Create an RLS-safe database connection pool with automatic context cleanup.
///
/// This pool uses `after_release` hook to ensure RLS context is always cleared
/// before a connection is returned to the pool, providing defense-in-depth
/// against context bleeding even if `release()` is not called.
///
/// # Security
///
/// The `after_release` hook runs `clear_request_context()` on every connection
/// before it's returned to the idle pool. This guarantees that:
/// 1. No stale tenant context can bleed between requests
/// 2. Super-admin privileges cannot persist on pooled connections
/// 3. Runtime shutdown edge cases are handled (hook runs synchronously in pool)
///
/// # Example
///
/// ```rust,ignore
/// let pool = create_rls_safe_pool(&database_url).await?;
/// // All connections returned to this pool will have RLS context cleared
/// ```
pub async fn create_rls_safe_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    use sqlx::Executor;

    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .after_release(|conn, _meta| {
            Box::pin(async move {
                // Clear RLS context before returning connection to pool
                // This is defense-in-depth: even if release() wasn't called,
                // the connection will be scrubbed before reuse
                match conn.execute("SELECT clear_request_context()").await {
                    Ok(_) => {
                        tracing::trace!("RLS context cleared via pool after_release hook");
                        Ok(true) // Return connection to pool
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "SECURITY: Failed to clear RLS context in after_release hook - closing connection"
                        );
                        // Return false to close the connection rather than reuse it
                        // with potentially stale RLS context
                        Ok(false)
                    }
                }
            })
        })
        .connect(database_url)
        .await
}

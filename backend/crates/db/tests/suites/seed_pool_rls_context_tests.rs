//! Regression: the seed pool must bind RLS context to the connection that runs
//! the queries.
//!
//! Code-review finding "api-core RLS context: set on the pool, not the
//! per-request connection". `SeedRunner::run` sets a super-admin RLS context
//! once and then issues many writes (and `pool.begin()` transactions) through
//! `&pool`. If those writes can land on a *different* pooled connection than
//! the one the context was set on — or if a pool `after_release` hook scrubs
//! the context between statements — the super-admin RLS bypass silently does
//! not apply to the actual query connection.
//!
//! `db::seed_pool_options()` must guarantee that binding: a single-connection
//! pool that does NOT clear RLS context on release, so the super-admin context
//! set before seeding survives across every subsequent acquire until the
//! seeder clears it explicitly at the end of the run.

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn seed_pool_keeps_super_admin_context_on_the_query_connection(
    _pool_opts: PgPoolOptions,
    connect_opts: PgConnectOptions,
) {
    // Build the pool exactly as the seed binary does.
    let pool = db::seed_pool_options()
        .connect_with(connect_opts)
        .await
        .expect("build seed pool");

    // 1) Set the super-admin RLS context at the POOL level, exactly as
    //    SeedRunner::run does before invoking the factories.
    db::set_request_context(&pool, None, None, true)
        .await
        .expect("set super-admin context");

    // 2) A subsequent, independent query on the same pool must still observe the
    //    context — i.e. it must run on the same physical connection the context
    //    was set on, and the context must not have been scrubbed on release.
    let is_super: Option<String> =
        sqlx::query_scalar("SELECT current_setting('app.is_super_admin', true)")
            .fetch_one(&pool)
            .await
            .expect("read app.is_super_admin");

    assert_eq!(
        is_super.as_deref(),
        Some("true"),
        "seed pool must keep the super-admin RLS context bound to the connection \
         that runs subsequent queries (pool-level context must survive across acquires)"
    );

    // Leave the pool clean, mirroring the seeder's explicit teardown.
    let _ = db::clear_request_context(&pool).await;
}

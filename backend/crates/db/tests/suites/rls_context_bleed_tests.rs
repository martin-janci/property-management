//! B13 — RLS context bleed tests (Phase 6, leak #2).
//!
//! Verifies that `RlsGuard` (and `RlsPool`) leave NO GUC state on a
//! connection after the guard is dropped — even if the drop is triggered
//! by a panic mid-request.
//!
//! Three invariants tested:
//!
//!  1. Normal drop: acquire → set GUC → drop normally → re-acquire → GUC is cleared.
//!  2. Panic drop: acquire → set GUC → panic (via `catch_unwind`) while
//!     holding guard → re-acquire → GUC is cleared.
//!  3. Same for `app.current_user_id` and the super-admin flag.
//!
//! `#[sqlx::test]` provides an isolated Postgres database with all migrations
//! applied. The pool is owned by the `postgres` superuser, so RLS is bypassed;
//! we only need to observe the GUC values, not the RLS filtering behaviour.
//!
//! IMPORTANT: Because `RlsGuard::drop` spawns a Tokio task to clear the GUC
//! asynchronously (best-effort), the test must yield control to the runtime
//! after the drop so that cleanup task can execute before we re-acquire the
//! connection. We do this with a brief `tokio::time::sleep`.

use db::rls_pool::RlsPool;
use db::tenant_context::{clear_or_discard, set_request_context};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a GUC from the current connection session.  Returns `None` when the
/// setting is absent or is the empty string (both indicate "not set").
async fn read_guc(pool: &PgPool, name: &str) -> Option<String> {
    // `current_setting(name, true)` is the missing_ok form — returns NULL
    // (NOT '') when the setting has never been set on this session, so the
    // scalar must be decoded as Option<String>.
    let raw: Option<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT current_setting('{name}', true)"
    )))
    .fetch_one(pool)
    .await
    .expect("read_guc query");

    match raw {
        Some(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// Read `app.current_org_id` from the given single-connection pool.
async fn read_org_id(pool: &PgPool) -> Option<String> {
    read_guc(pool, "app.current_org_id").await
}

/// Read `app.current_user_id` from the given single-connection pool.
async fn read_user_id(pool: &PgPool) -> Option<String> {
    read_guc(pool, "app.current_user_id").await
}

/// Read `app.is_super_admin` from the given single-connection pool.
async fn read_super_admin(pool: &PgPool) -> Option<String> {
    read_guc(pool, "app.is_super_admin").await
}

// ---------------------------------------------------------------------------
// Test 1 — normal drop clears GUC
// ---------------------------------------------------------------------------

/// Acquire an RlsGuard, confirm the GUC is set, drop normally, then confirm
/// the GUC has been cleared before the connection re-enters the pool.
///
/// `RlsGuard::drop` spawns a Tokio cleanup task. We yield with a short sleep
/// so that task executes before we re-acquire.
#[sqlx::test]
async fn normal_drop_clears_org_id_guc(pool: PgPool) {
    let rls_pool = RlsPool::new(pool.clone());

    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Acquire and confirm GUC is set.
    {
        let mut guard = rls_pool
            .acquire_with_rls(org_id, user_id, false)
            .await
            .expect("acquire guard");

        // The set_request_context call is internal; verify via direct query.
        let val: Option<String> =
            sqlx::query_scalar("SELECT current_setting('app.current_org_id', true)")
                .fetch_one(&mut **guard.conn())
                .await
                .ok()
                .filter(|s: &String| !s.is_empty());

        assert_eq!(
            val.as_deref(),
            Some(org_id.to_string().as_str()),
            "GUC must be set while guard is live"
        );

        // guard drops here — spawns cleanup task
    }

    // Let the cleanup task run.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Re-acquire from the same pool and check the GUC is gone.
    let org_after = read_org_id(&pool).await;
    assert!(
        org_after.is_none(),
        "app.current_org_id must be cleared after normal guard drop, got: {org_after:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — panic drop clears GUC
// ---------------------------------------------------------------------------

/// Acquire a guard, set GUC, then panic inside `catch_unwind`.  After the
/// unwind the connection is returned to the pool with a cleanup task spawned
/// by `Drop`. Confirm the GUC has been cleared on the next acquire.
///
/// This is the "mid-request panic" scenario — the most dangerous path for
/// context bleeding.
#[sqlx::test]
async fn panic_drop_clears_org_id_guc(pool: PgPool) {
    let rls_pool = RlsPool::new(pool.clone());

    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Wrap in Arc so it's Send + Sync for catch_unwind.
    let rls_pool_arc = std::sync::Arc::new(rls_pool);
    let rls_pool_clone = rls_pool_arc.clone();

    // Acquire the guard inside a block we can panic in.
    let acquire_result = rls_pool_clone
        .acquire_with_rls(org_id, user_id, false)
        .await;
    let guard = acquire_result.expect("acquire guard");

    // Simulate a mid-request panic. `catch_unwind` requires the closure to
    // be `UnwindSafe`; we satisfy that by not capturing `&mut` refs.
    // The guard is moved into the closure so it drops during unwind.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        // guard is now owned by this closure; the panic causes it to drop.
        let _g = guard;
        panic!("simulated mid-request panic");
    }));

    assert!(result.is_err(), "should have caught the panic");

    // Yield so the Drop's cleanup task can execute.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Re-acquire and confirm the GUC is cleared.
    let org_after = read_org_id(&pool).await;
    assert!(
        org_after.is_none(),
        "app.current_org_id must be cleared after panic-triggered guard drop, got: {org_after:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — all three GUCs cleared (org_id, user_id, super-admin flag)
// ---------------------------------------------------------------------------

/// Acquire a guard with super-admin context. Confirm all three GUCs are set.
/// Drop normally. Confirm all three GUCs are cleared.
#[sqlx::test]
async fn normal_drop_clears_all_gucs(pool: PgPool) {
    let rls_pool = RlsPool::new(pool.clone());

    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    {
        let mut guard = rls_pool
            .acquire_with_rls(org_id, user_id, true) // is_super_admin = true
            .await
            .expect("acquire super-admin guard");

        // Confirm all three GUCs are live on the guard's connection.
        let org_val: String =
            sqlx::query_scalar("SELECT current_setting('app.current_org_id', true)")
                .fetch_one(&mut **guard.conn())
                .await
                .expect("read org");

        let user_val: String =
            sqlx::query_scalar("SELECT current_setting('app.current_user_id', true)")
                .fetch_one(&mut **guard.conn())
                .await
                .expect("read user");

        let admin_val: String =
            sqlx::query_scalar("SELECT current_setting('app.is_super_admin', true)")
                .fetch_one(&mut **guard.conn())
                .await
                .expect("read admin");

        assert!(!org_val.is_empty(), "org GUC must be set");
        assert!(!user_val.is_empty(), "user GUC must be set");
        assert!(
            admin_val == "true" || admin_val == "on" || admin_val == "1",
            "super-admin GUC must be truthy, got: {admin_val}"
        );

        // Drop the guard (normal drop path).
    }

    // Allow cleanup task to execute.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let org_after = read_org_id(&pool).await;
    let user_after = read_user_id(&pool).await;
    let admin_after = read_super_admin(&pool).await;

    assert!(
        org_after.is_none(),
        "app.current_org_id must be cleared after drop, got: {org_after:?}"
    );
    assert!(
        user_after.is_none(),
        "app.current_user_id must be cleared after drop, got: {user_after:?}"
    );
    // The super-admin flag is cleared to 'false' or '' by clear_request_context.
    let admin_not_truthy = admin_after
        .as_deref()
        .map(|v| v.is_empty() || v == "false" || v == "off" || v == "0")
        .unwrap_or(true);
    assert!(
        admin_not_truthy,
        "app.is_super_admin must be cleared/falsy after drop, got: {admin_after:?}"
    );
}

/// Same as `normal_drop_clears_all_gucs` but with a panic in the middle.
#[sqlx::test]
async fn panic_drop_clears_all_gucs(pool: PgPool) {
    let rls_pool = RlsPool::new(pool.clone());

    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let guard = rls_pool
        .acquire_with_rls(org_id, user_id, true)
        .await
        .expect("acquire guard");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _g = guard;
        panic!("simulated panic with super-admin guard");
    }));
    assert!(result.is_err(), "panic must be caught");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let org_after = read_org_id(&pool).await;
    let user_after = read_user_id(&pool).await;
    let admin_after = read_super_admin(&pool).await;

    assert!(
        org_after.is_none(),
        "app.current_org_id not cleared after panic, got: {org_after:?}"
    );
    assert!(
        user_after.is_none(),
        "app.current_user_id not cleared after panic, got: {user_after:?}"
    );
    let admin_not_truthy = admin_after
        .as_deref()
        .map(|v| v.is_empty() || v == "false" || v == "off" || v == "0")
        .unwrap_or(true);
    assert!(
        admin_not_truthy,
        "app.is_super_admin not cleared after panic, got: {admin_after:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 5 — failed context clear DISCARDS the connection (issue #3583)
// ---------------------------------------------------------------------------

/// Regression for #3583 (follow-up to #3577): when `clear_request_context`
/// FAILS on a connection that holds the super-admin SYSTEM context, that
/// connection must NOT be returned to the pool — otherwise the next request
/// checking it out would run with RLS bypassed. `clear_or_discard` detaches
/// and hard-closes the connection on clear failure.
///
/// We force a real clear failure by poisoning the connection with an aborted
/// transaction (`BEGIN` then a division-by-zero), after which every statement
/// on it — including `SELECT clear_request_context()` — errors with
/// "current transaction is aborted". On a `max_connections = 1` pool we then
/// assert (a) the pool has zero connections (the tainted one was discarded,
/// not recycled) and (b) a freshly opened connection carries no super-admin
/// context and is usable.
#[sqlx::test]
async fn failed_clear_discards_connection(pool: PgPool) {
    // Build a single-connection pool against the same (isolated) test DB so we
    // can observe whether a specific physical connection is recycled or not.
    let opts = (*pool.connect_options()).clone();
    let single = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("open single-connection pool");

    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Acquire the one connection and place it into super-admin SYSTEM context.
    let mut conn = single.acquire().await.expect("acquire tainted conn");
    set_request_context(&mut *conn, Some(org_id), Some(user_id), true)
        .await
        .expect("set super-admin context");

    // Poison the connection: open a transaction then run a failing statement so
    // the session enters the "current transaction is aborted" state. Every
    // subsequent query — including the context clear — now fails.
    (&mut *conn).execute("BEGIN").await.expect("begin txn");
    assert!(
        (&mut *conn).execute("SELECT 1 / 0").await.is_err(),
        "division by zero must error and abort the transaction"
    );

    // Release via the hardened primitive. The clear fails (aborted txn), so the
    // connection must be detached + closed, NOT returned to the pool.
    clear_or_discard(conn, "test: forced-clear-failure").await;

    // The tainted connection must have been discarded: the max=1 pool now holds
    // zero connections.
    assert_eq!(
        single.size(),
        0,
        "connection whose clear failed must be discarded, not returned to the pool"
    );

    // Next acquire opens a FRESH connection: no super-admin context, usable.
    let admin_after = read_super_admin(&single).await;
    let admin_not_truthy = admin_after
        .as_deref()
        .map(|v| v.is_empty() || v == "false" || v == "off" || v == "0")
        .unwrap_or(true);
    assert!(
        admin_not_truthy,
        "fresh connection after discard must not carry super-admin context, got: {admin_after:?}"
    );

    // And the fresh connection is not stuck in an aborted transaction.
    let ok: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&single)
        .await
        .expect("fresh connection must be usable");
    assert_eq!(ok, 1);
}

// ---------------------------------------------------------------------------
// Test 6 — failed clear ON ACQUIRE discards the connection (issue #3583)
// ---------------------------------------------------------------------------

/// Regression for #3583 (follow-up to #3577): the *acquire* side of the pool
/// has the same hazard as the *release* side. `RlsPool::acquire_public` clears
/// any stale RLS context left on a checked-out connection before handing it to
/// a public (unauthenticated) route. Before the fix, a FAILED clear was only
/// logged and the connection was handed back anyway — so a stale super-admin /
/// global-read context from a previous request could bleed into a public
/// request (RLS bypass). The fix makes `acquire_public` discard the connection
/// and return `Err` on a failed clear.
///
/// We force a real clear failure by poisoning the pool's single connection with
/// an aborted transaction, then dropping it back into a plain (hook-less) pool
/// so `acquire_public` checks out that very connection. On a `max_connections =
/// 1` pool we then assert: (a) `acquire_public()` returns `Err` (it did NOT
/// hand back the poisoned connection), (b) the pool holds zero connections (the
/// tainted one was discarded, not recycled), and (c) a freshly opened
/// connection carries no super-admin context and is usable.
#[sqlx::test]
async fn acquire_public_discards_connection_on_failed_clear(pool: PgPool) {
    // Plain single-connection pool (no `after_release` hook) so we can observe
    // whether the specific poisoned physical connection is reused or discarded,
    // and so `acquire_public` is guaranteed to check out that same connection.
    let opts = (*pool.connect_options()).clone();
    let single = PgPoolOptions::new()
        .max_connections(1)
        .test_before_acquire(false)
        .connect_with(opts)
        .await
        .expect("open single-connection pool");

    let org_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Place the one connection into super-admin SYSTEM context, then poison it
    // with an aborted transaction so the clear performed inside `acquire_public`
    // will fail. Dropping it returns the poisoned connection to the hook-less
    // pool, where `acquire_public` will check it back out.
    {
        let mut conn = single.acquire().await.expect("acquire tainted conn");
        set_request_context(&mut *conn, Some(org_id), Some(user_id), true)
            .await
            .expect("set super-admin context");
        (&mut *conn).execute("BEGIN").await.expect("begin txn");
        assert!(
            (&mut *conn).execute("SELECT 1 / 0").await.is_err(),
            "division by zero must error and abort the transaction"
        );
    }

    let rls_pool = RlsPool::new(single.clone());

    // acquire_public must FAIL rather than hand back a connection whose stale
    // (possibly super-admin) context it could not clear.
    let result = rls_pool.acquire_public().await;
    assert!(
        result.is_err(),
        "acquire_public must return Err when it cannot clear stale RLS context, \
         not hand back a connection that may still carry super-admin context"
    );
    drop(result);

    // The poisoned connection must have been discarded on acquire: the max=1
    // pool now holds zero connections.
    assert_eq!(
        single.size(),
        0,
        "connection whose stale-context clear failed must be discarded on acquire, \
         not returned to the pool"
    );

    // Next acquire opens a FRESH connection: no super-admin context, usable.
    let admin_after = read_super_admin(&single).await;
    let admin_not_truthy = admin_after
        .as_deref()
        .map(|v| v.is_empty() || v == "false" || v == "off" || v == "0")
        .unwrap_or(true);
    assert!(
        admin_not_truthy,
        "fresh connection after discard must not carry super-admin context, got: {admin_after:?}"
    );

    let ok: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&single)
        .await
        .expect("fresh connection must be usable");
    assert_eq!(ok, 1);
}

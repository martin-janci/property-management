//! Regression tests for the Epic 6 messaging RLS spot-check (issue #898).
//!
//! The api-server messaging handlers (`routes/messaging.rs`) enforce
//! participant + tenant isolation in the handler body, and treat the
//! PostgreSQL row-level-security policies on `message_threads` / `messages` /
//! `user_blocks` as defense-in-depth. Those policies (00017 + 00019, later
//! regenerated verbatim by 00140) restrict reads to threads the caller
//! participates in, within the caller's org.
//!
//! BUT 00017/00019 only `ENABLE`d RLS — they never `FORCE`d it. `ENABLE`
//! (without `FORCE`) is bypassed for the table owner, and the api-server
//! frequently connects as the owning role. Migration 00171 adds
//! `FORCE ROW LEVEL SECURITY`, closing that owner-bypass gap.
//!
//! ## Why these tests must SET ROLE to a non-superuser
//!
//! The `#[sqlx::test]` harness connects as the `DATABASE_URL` role, which in CI
//! (`backend.yml` `test` job) and locally is the `postgres` **superuser**.
//! PostgreSQL exempts superusers from RLS *entirely* — `FORCE ROW LEVEL
//! SECURITY` binds the table OWNER, but NOT superusers / `BYPASSRLS` roles.
//! Connecting as `postgres` therefore bypasses every policy and would make a
//! "must not see" assertion pass vacuously (the superuser sees all rows).
//!
//! The dedicated RLS suites (`rls_smoke_tests`, `rls_penetration_tests`,
//! `rls_listings_global_context_tests`) sidestep this by connecting through the
//! non-superuser `rls_test_runner` role via `TEST_DATABASE_URL`. Those suites
//! are `#[ignore]`d and only run in the `security-tests` / `rls-smoke-test` CI
//! jobs. To keep this regression in the always-on `test` job (with migrations
//! auto-applied by the `sqlx::test` migrator) we instead create a per-test
//! non-superuser role inside the test and `SET ROLE` to it for the assertion
//! queries — that makes FORCE RLS actually bind to the connection.
//!
//! Fixtures are seeded first, as the superuser (RLS bypassed), under
//! super-admin context. RLS context (`set_request_context`) is session-scoped,
//! so every test pins a single connection acquired from the pool.

use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures (all created under super-admin RLS context so setup bypasses RLS)
// ---------------------------------------------------------------------------

async fn seed_org(conn: &mut PoolConnection<Postgres>, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Msg RLS {slug}"))
    .bind(format!("msg-rls-{slug}"))
    .bind(format!("{slug}@msg-rls.test"))
    .fetch_one(&mut **conn)
    .await
    .expect("seed org")
}

async fn seed_user(conn: &mut PoolConnection<Postgres>, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Msg RLS User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(&mut **conn)
    .await
    .expect("seed user")
}

/// Insert a 2-participant thread directly. Returns the thread id.
async fn seed_thread(
    conn: &mut PoolConnection<Postgres>,
    org_id: Uuid,
    p1: Uuid,
    p2: Uuid,
) -> Uuid {
    let mut ids = [p1, p2];
    ids.sort();
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO message_threads (organization_id, participant_ids)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(ids.to_vec())
    .fetch_one(&mut **conn)
    .await
    .expect("seed thread")
}

async fn seed_message(
    conn: &mut PoolConnection<Postgres>,
    thread_id: Uuid,
    sender_id: Uuid,
    content: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO messages (thread_id, sender_id, content)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(thread_id)
    .bind(sender_id)
    .bind(content)
    .fetch_one(&mut **conn)
    .await
    .expect("seed message")
}

/// Switch the connection's RLS context to (org, user, super_admin).
async fn set_ctx(
    conn: &mut PoolConnection<Postgres>,
    org: Option<Uuid>,
    user: Option<Uuid>,
    super_admin: bool,
) {
    db::set_request_context(&mut **conn, org, user, super_admin)
        .await
        .expect("set_request_context");
}

/// Create a fresh, non-superuser DB role on this connection (the `sqlx::test`
/// connection runs as the `postgres` superuser, which bypasses RLS entirely).
/// Grants it the read access the messaging policies need, then `SET ROLE`s the
/// session to it so `FORCE ROW LEVEL SECURITY` actually binds.
///
/// `role_name` must be unique per test — roles are cluster-global, so parallel
/// `sqlx::test` cases would otherwise collide on `CREATE ROLE`.
async fn switch_to_rls_role(conn: &mut PoolConnection<Postgres>, role_name: &str) {
    // Each statement must be sent on its own: sqlx's `query(...).execute()` uses
    // the extended (prepared) protocol, which rejects multiple commands in one
    // string ("cannot insert multiple commands into a prepared statement").
    //
    // Idempotent create (no native CREATE ROLE IF NOT EXISTS), then mirror the
    // CI `rls_test_runner` grants: SELECT on every table so the policies' helper
    // subqueries (messages policy -> message_threads) can run under this role,
    // plus EXECUTE on the context-setter functions.
    //
    // `role_name` is a hard-coded per-test constant (no external input), so the
    // dynamic SQL is safe — assert it for sqlx 0.9's SqlSafeStr guard.
    let statements = [
        format!(
            r#"DO $$
            BEGIN
                IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{role_name}') THEN
                    CREATE ROLE "{role_name}" NOSUPERUSER NOBYPASSRLS;
                END IF;
            END $$"#
        ),
        format!(r#"GRANT USAGE ON SCHEMA public TO "{role_name}""#),
        format!(r#"GRANT SELECT ON ALL TABLES IN SCHEMA public TO "{role_name}""#),
        format!(r#"GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO "{role_name}""#),
        format!(r#"SET ROLE "{role_name}""#),
    ];
    for stmt in statements {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&mut **conn)
            .await
            .expect("create + grant + set rls role");
    }
}

async fn count_visible_threads(conn: &mut PoolConnection<Postgres>, thread_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM message_threads WHERE id = $1")
        .bind(thread_id)
        .fetch_one(&mut **conn)
        .await
        .expect("count threads")
}

async fn count_visible_messages(conn: &mut PoolConnection<Postgres>, thread_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM messages WHERE thread_id = $1")
        .bind(thread_id)
        .fetch_one(&mut **conn)
        .await
        .expect("count messages")
}

/// Cross-tenant read: user B in org B must NOT see user A's thread in org A,
/// even knowing the exact thread UUID. Under a non-superuser role with FORCE
/// RLS (00171), the participant policy hides the row.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn user_b_cannot_read_user_a_thread(pool: PgPool) {
    let mut conn = pool.acquire().await.expect("acquire connection");

    // --- setup as super-admin (bypasses RLS for fixtures) ---
    set_ctx(&mut conn, None, None, true).await;
    let org_a = seed_org(&mut conn, "a").await;
    let org_b = seed_org(&mut conn, "b").await;
    let user_a = seed_user(&mut conn, "a@msg-rls.test").await;
    let user_a2 = seed_user(&mut conn, "a2@msg-rls.test").await; // A's counterpart
    let user_b = seed_user(&mut conn, "b@msg-rls.test").await;

    let thread = seed_thread(&mut conn, org_a, user_a, user_a2).await;
    let _msg = seed_message(&mut conn, thread, user_a, "secret org-A message").await;

    // --- attacker: user B in org B, targeting org-A thread by UUID ---
    // SET ROLE to a non-superuser so FORCE RLS binds (postgres superuser would
    // bypass the policy and see the row regardless).
    switch_to_rls_role(&mut conn, "msg_rls_attacker_xtenant").await;
    set_ctx(&mut conn, Some(org_b), Some(user_b), false).await;
    assert_eq!(
        count_visible_threads(&mut conn, thread).await,
        0,
        "cross-tenant: user B must not see user A's thread (RLS must be FORCEd)"
    );
    assert_eq!(
        count_visible_messages(&mut conn, thread).await,
        0,
        "cross-tenant: user B must not see user A's messages (RLS must be FORCEd)"
    );
}

/// Same-org non-participant must NOT see the thread either: org membership is
/// not sufficient, participant membership is required.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn same_org_non_participant_cannot_read_thread(pool: PgPool) {
    let mut conn = pool.acquire().await.expect("acquire connection");

    set_ctx(&mut conn, None, None, true).await;
    let org_a = seed_org(&mut conn, "sa").await;
    let user_a = seed_user(&mut conn, "sa-a@msg-rls.test").await;
    let user_a2 = seed_user(&mut conn, "sa-a2@msg-rls.test").await;
    let outsider = seed_user(&mut conn, "sa-out@msg-rls.test").await; // same org, not a participant

    let thread = seed_thread(&mut conn, org_a, user_a, user_a2).await;
    let _msg = seed_message(&mut conn, thread, user_a, "private to A & A2").await;

    // outsider is in the SAME org but not a participant. SET ROLE to a
    // non-superuser so the participant branch of the policy is actually applied.
    switch_to_rls_role(&mut conn, "msg_rls_outsider_sameorg").await;
    set_ctx(&mut conn, Some(org_a), Some(outsider), false).await;
    assert_eq!(
        count_visible_threads(&mut conn, thread).await,
        0,
        "same-org non-participant must not see the thread (participant check)"
    );
    assert_eq!(
        count_visible_messages(&mut conn, thread).await,
        0,
        "same-org non-participant must not see the messages"
    );
}

/// Same-context SUCCESS: a participant (user A) reading their own thread and
/// messages in their own org sees them. Proves the FORCE policy does not
/// over-block the legitimate case — also under the non-superuser role, so the
/// positive result is the policy allowing access, not a superuser bypass.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn participant_can_read_own_thread_and_messages(pool: PgPool) {
    let mut conn = pool.acquire().await.expect("acquire connection");

    set_ctx(&mut conn, None, None, true).await;
    let org_a = seed_org(&mut conn, "ok").await;
    let user_a = seed_user(&mut conn, "ok-a@msg-rls.test").await;
    let user_a2 = seed_user(&mut conn, "ok-a2@msg-rls.test").await;

    let thread = seed_thread(&mut conn, org_a, user_a, user_a2).await;
    let _m1 = seed_message(&mut conn, thread, user_a, "hi from A").await;
    let _m2 = seed_message(&mut conn, thread, user_a2, "hi from A2").await;

    // user A reads in their own org context, under a non-superuser role so the
    // visibility is the policy's doing, not a superuser bypass.
    switch_to_rls_role(&mut conn, "msg_rls_participant_ok").await;
    set_ctx(&mut conn, Some(org_a), Some(user_a), false).await;
    assert_eq!(
        count_visible_threads(&mut conn, thread).await,
        1,
        "participant must see their own thread"
    );
    assert_eq!(
        count_visible_messages(&mut conn, thread).await,
        2,
        "participant must see both messages in their own thread"
    );
}

//! Regression tests for the Epic 6 messaging RLS spot-check (issue #898).
//!
//! The api-server messaging handlers (`routes/messaging.rs`) enforce
//! participant + tenant isolation in the handler body, and treat the
//! PostgreSQL row-level-security policies on `message_threads` / `messages` /
//! `user_blocks` as defense-in-depth. Those policies (00017 + 00019) restrict
//! reads to threads the caller participates in, within the caller's org.
//!
//! BUT 00017/00019 only `ENABLE`d RLS — they never `FORCE`d it. `ENABLE`
//! (without `FORCE`) is bypassed for the table owner, and the api-server / the
//! `#[sqlx::test]` harness both connect as the owning `postgres` role. So
//! before migration 00171 the DB-layer isolation was silently a no-op for the
//! owner: a connection that set another user's RLS context could still read
//! any thread by UUID. 00171 adds `FORCE ROW LEVEL SECURITY`, closing the gap.
//!
//! These tests run under the `#[sqlx::test]` harness, which connects as the
//! table owner — exactly the bypass condition. On `dev`/main (no FORCE) the
//! cross-tenant assertions FAIL because the owner bypasses the policy; after
//! 00171 they PASS.
//!
//! RLS context (`set_request_context`) is session-scoped, so every test pins a
//! single connection acquired from the pool and runs all queries on it.

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
/// even knowing the exact thread UUID. With FORCE RLS (00171) the owner-role
/// connection is subject to the policy and sees zero rows.
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

    // outsider is in the SAME org but not a participant.
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
/// over-block the legitimate case.
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

    // user A reads in their own org context.
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

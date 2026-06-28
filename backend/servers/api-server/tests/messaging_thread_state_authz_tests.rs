//! Authz + behavioral regression tests for the per-participant thread-state
//! routes added in BIT-182 (Epic 6 gaps #4/#6):
//!
//!   * `DELETE /api/v1/messages/threads/{id}`          (per-user soft hide)
//!   * `POST   /api/v1/messages/threads/{id}/archive`  (per-user archive)
//!   * `DELETE /api/v1/messages/threads/{id}/archive`  (per-user un-archive)
//!
//! All three are gated by the same participant + tenant check `get_thread`
//! applies: a member of the org who is NOT a participant in the thread must be
//! rejected with `403`, and one participant acting on their own copy must never
//! affect the other participant's view.
//!
//! These exercise the HTTP surface end-to-end with real HS256 JWTs. The CI test
//! pool runs as superuser (bypasses FORCE RLS), so the participant gate proven
//! here is the handler-layer check, and the per-user list filtering is the
//! repository SQL (`thread_participant_state` join) — both independent of RLS.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("MsgState Org {slug}"))
    .bind(format!("msgstate-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@msgstate.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, label: &str) -> (Uuid, String) {
    let email = format!("{label}-{}@msgstate.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'MsgState User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

/// Insert a direct-message thread between `a` and `b` in `org`, return its id.
async fn seed_thread(pool: &PgPool, org: Uuid, a: Uuid, b: Uuid) -> Uuid {
    let mut ids = [a, b];
    ids.sort();
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO message_threads (organization_id, participant_ids)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(&ids[..])
    .fetch_one(pool)
    .await
    .expect("seed thread")
}

/// Mint a real HS256 access token signed with the same secret the TestApp
/// configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "MsgState User", None, None)
        .expect("mint access token")
}

// ---------------------------------------------------------------------------
// T1 — non-participant DELETE thread is forbidden
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_thread_by_non_participant_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "del403").await;
    let (alice, _) = seed_user(&pool, "alice").await;
    let (bob, _) = seed_user(&pool, "bob").await;
    let (carol, carol_email) = seed_user(&pool, "carol").await;
    // All three are members of the org (so tenant validation passes); only
    // alice + bob are participants in the thread.
    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "org_admin").await;
    seed_membership(&pool, org, carol, "org_admin").await;
    let thread = seed_thread(&pool, org, alice, bob).await;

    let token_c = mint_token(carol, &carol_email);
    let uri = format!("/api/v1/messages/threads/{thread}");
    let resp = app
        .execute(
            app.delete(&uri)
                .bearer(&token_c)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a non-participant must not be able to delete someone else's thread"
    );

    // No per-user state row may have been written for carol.
    let rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM thread_participant_state WHERE thread_id = $1")
            .bind(thread)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0, "no thread_participant_state row should be created");
}

// ---------------------------------------------------------------------------
// T2 — non-participant ARCHIVE thread is forbidden
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn archive_thread_by_non_participant_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "arch403").await;
    let (alice, _) = seed_user(&pool, "alice").await;
    let (bob, _) = seed_user(&pool, "bob").await;
    let (carol, carol_email) = seed_user(&pool, "carol").await;
    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "org_admin").await;
    seed_membership(&pool, org, carol, "org_admin").await;
    let thread = seed_thread(&pool, org, alice, bob).await;

    let token_c = mint_token(carol, &carol_email);
    let uri = format!("/api/v1/messages/threads/{thread}/archive");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_c)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a non-participant must not be able to archive someone else's thread"
    );
}

// ---------------------------------------------------------------------------
// T3 — participant DELETE hides the thread for them only
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn participant_delete_hides_thread_for_self_only(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "del200").await;
    let (alice, alice_email) = seed_user(&pool, "alice").await;
    let (bob, bob_email) = seed_user(&pool, "bob").await;
    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "org_admin").await;
    let thread = seed_thread(&pool, org, alice, bob).await;

    let token_a = mint_token(alice, &alice_email);
    let token_b = mint_token(bob, &bob_email);

    // Alice deletes her copy.
    let del = app
        .execute(
            app.delete(&format!("/api/v1/messages/threads/{thread}"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        del.status,
        StatusCode::OK,
        "alice may delete her own thread"
    );

    // Alice's thread list no longer contains it.
    let alice_list = app
        .execute(
            app.get("/api/v1/messages/threads")
                .bearer(&token_a)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(alice_list.status, StatusCode::OK);
    assert_eq!(
        alice_list.json_value()["total"].as_i64(),
        Some(0),
        "alice's inbox must not contain the thread she deleted"
    );

    // Bob's copy is untouched.
    let bob_list = app
        .execute(
            app.get("/api/v1/messages/threads")
                .bearer(&token_b)
                .header("X-Tenant-ID", &org.to_string())
                .build(),
        )
        .await;
    assert_eq!(bob_list.status, StatusCode::OK);
    assert_eq!(
        bob_list.json_value()["total"].as_i64(),
        Some(1),
        "bob's copy of the thread must be unaffected by alice's per-user delete"
    );
}

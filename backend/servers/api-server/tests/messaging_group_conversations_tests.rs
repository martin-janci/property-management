//! Tests for N-party group conversations on the messaging start-thread
//! endpoint (`POST /api/v1/messages/threads`, UC-05.8 / [BIT-183]).
//!
//! Before [BIT-183], `start_thread` hardcoded exactly two participants
//! (`vec![user_id, body.recipient_id]`) and its "same organization" guard read
//! a non-existent `users.organization_id` column. This suite proves the
//! generalized behavior:
//!
//!   1. A caller can open a 3-party thread with `recipient_ids` (all
//!      same-tenant) — the thread carries all three participants.
//!   2. The legacy single `recipient_id` field still opens a 2-party thread.
//!   3. A recipient outside the caller's tenant is rejected (no cross-tenant
//!      IDOR) and no thread is created.
//!   4. A blocked recipient is rejected.
//!
//! Tenant membership is validated against `organization_members` (there is no
//! `users.organization_id` column), which is the same source
//! `ValidatedTenantExtractor` authorizes against. The CI test pool runs as
//! superuser (bypasses FORCE RLS), so these tests specifically exercise the
//! org-keyed SQL membership check, not the RLS policy.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Group Msg User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a block: `blocker` has blocked `blocked` within `org`.
async fn seed_block(pool: &PgPool, org: Uuid, blocker: Uuid, blocked: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO user_blocks (blocker_id, blocked_id, organization_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (blocker_id, blocked_id) DO NOTHING
        "#,
    )
    .bind(blocker)
    .bind(blocked)
    .bind(org)
    .execute(pool)
    .await
    .expect("seed block");
}

/// Mint a real HS256 access token for `user_id`, signed with the same secret
/// the TestApp configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "Group Msg User", None, None)
        .expect("mint access token")
}

/// Count threads stored for `org` (used to prove rejected requests create none).
async fn thread_count(pool: &PgPool, org: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM message_threads WHERE organization_id = $1")
        .bind(org)
        .fetch_one(pool)
        .await
        .expect("count threads")
}

// ---------------------------------------------------------------------------
// T1 — 3-party group thread creation
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_group_thread_creates_three_party_thread(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "grp3").await;
    let alice = seed_user(&pool, "grp3-alice@msg.test").await;
    let bob = seed_user(&pool, "grp3-bob@msg.test").await;
    let carol = seed_user(&pool, "grp3-carol@msg.test").await;
    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;
    seed_membership(&pool, org, carol, "resident").await;

    let token = mint_token(alice, "grp3-alice@msg.test");
    let body = json!({
        "recipientIds": [bob, carol],
        "initialMessage": "Hello group",
    });
    let resp = app
        .execute(
            app.post("/api/v1/messages/threads")
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(body)
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let v = resp.json_value();
    let participants = v["thread"]["participantIds"]
        .as_array()
        .expect("participantIds array");
    assert_eq!(
        participants.len(),
        3,
        "group thread should carry all three participants, got {v}"
    );
    for id in [alice, bob, carol] {
        assert!(
            participants.iter().any(|p| p == &json!(id)),
            "thread must include participant {id}: {v}"
        );
    }
}

// ---------------------------------------------------------------------------
// T2 — legacy single-recipient still works (2-party back-compat)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_thread_back_compat_single_recipient(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "compat").await;
    let alice = seed_user(&pool, "compat-alice@msg.test").await;
    let bob = seed_user(&pool, "compat-bob@msg.test").await;
    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;

    let token = mint_token(alice, "compat-alice@msg.test");
    let body = json!({ "recipientId": bob });
    let resp = app
        .execute(
            app.post("/api/v1/messages/threads")
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(body)
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let v = resp.json_value();
    let participants = v["thread"]["participantIds"]
        .as_array()
        .expect("participantIds array");
    assert_eq!(
        participants.len(),
        2,
        "single recipient = 2-party thread: {v}"
    );
}

// ---------------------------------------------------------------------------
// T3 — a cross-tenant recipient is rejected and creates no thread (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_group_thread_with_cross_org_recipient_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "xorg-a").await;
    let org_b = seed_org(&pool, "xorg-b").await;
    let alice = seed_user(&pool, "xorg-alice@msg.test").await;
    let bob = seed_user(&pool, "xorg-bob@msg.test").await;
    let mallory = seed_user(&pool, "xorg-mallory@msg.test").await;
    seed_membership(&pool, org_a, alice, "org_admin").await;
    seed_membership(&pool, org_a, bob, "resident").await;
    // Mallory belongs to a DIFFERENT org.
    seed_membership(&pool, org_b, mallory, "org_admin").await;

    let token = mint_token(alice, "xorg-alice@msg.test");
    // Alice (org A) tries to pull a foreign-tenant user (Mallory) into a thread.
    let body = json!({ "recipientIds": [bob, mallory] });
    let resp = app
        .execute(
            app.post("/api/v1/messages/threads")
                .bearer(&token)
                .header("X-Tenant-ID", &org_a.to_string())
                .json(body)
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
    assert_eq!(
        resp.json_value()["code"],
        json!("CROSS_ORG_DENIED"),
        "cross-tenant recipient must be denied: {}",
        resp.text()
    );
    assert_eq!(
        thread_count(&pool, org_a).await,
        0,
        "no thread may be created when a recipient is rejected"
    );
}

// ---------------------------------------------------------------------------
// T4 — a blocked recipient is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_group_thread_with_blocked_recipient_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "blk").await;
    let alice = seed_user(&pool, "blk-alice@msg.test").await;
    let bob = seed_user(&pool, "blk-bob@msg.test").await;
    let carol = seed_user(&pool, "blk-carol@msg.test").await;
    seed_membership(&pool, org, alice, "org_admin").await;
    seed_membership(&pool, org, bob, "resident").await;
    seed_membership(&pool, org, carol, "resident").await;
    // Carol has blocked Alice.
    seed_block(&pool, org, carol, alice).await;

    let token = mint_token(alice, "blk-alice@msg.test");
    let body = json!({ "recipientIds": [bob, carol] });
    let resp = app
        .execute(
            app.post("/api/v1/messages/threads")
                .bearer(&token)
                .header("X-Tenant-ID", &org.to_string())
                .json(body)
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
    assert_eq!(
        resp.json_value()["code"],
        json!("USER_BLOCKED"),
        "blocked recipient must be denied: {}",
        resp.text()
    );
}

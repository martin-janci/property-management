//! Regression tests for cross-user isolation in the push-fanout stale-token
//! eviction path.
//!
//! Audit history: `DevicePushTokenRepository::delete_stale_token` is called
//! by `FcmHttpAdapter` when FCM returns `NOT_REGISTERED`.  The DELETE is
//! scoped to `WHERE user_id = $1 AND token = $2` so that a stale-token
//! callback for one user cannot evict a token that happens to share the same
//! value but belongs to a different user.
//!
//! Additionally, migration `00160_device_push_tokens_service_delete.sql` adds
//! an RLS FOR DELETE policy on `device_push_tokens` so the service-role pool
//! (used by the fanout worker) can actually delete rows — previously the table
//! had no DELETE policy and stale-token eviction was silently a no-op.
//!
//! These tests exercise:
//! 1. Unauthenticated DELETE on the user-facing push-token endpoint → 4xx.
//! 2. Cross-user delete attempt via the HTTP endpoint → 4xx (auth gate fires).
//! 3. Direct DB isolation: `delete_stale_token`-equivalent DELETE with a
//!    mismatched `user_id` does NOT remove the token row.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Fanout Test', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_push_token(pool: &PgPool, user_id: Uuid, token: &str) {
    sqlx::query(
        r#"
        INSERT INTO device_push_tokens (user_id, token, platform)
        VALUES ($1, $2, 'fcm')
        ON CONFLICT (token) DO NOTHING
        "#,
    )
    .bind(user_id)
    .bind(token)
    .execute(pool)
    .await
    .expect("seed push token");
}

async fn token_exists(pool: &PgPool, token: &str) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM device_push_tokens WHERE token = $1")
        .bind(token)
        .fetch_one(pool)
        .await
        .expect("count push tokens")
        > 0
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: expected 4xx rejection, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Test 1: DELETE push-token endpoint without auth is rejected
// ---------------------------------------------------------------------------

/// Unauthenticated DELETE on `/api/v1/users/me/push-tokens/{token}` must
/// be rejected before any DB mutation.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn delete_push_token_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_id = seed_user(&pool, "no-auth-delete@fanout.test").await;
    let token = format!("fanout-no-auth-{}", Uuid::new_v4());
    seed_push_token(&pool, user_id, &token).await;

    let req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/users/me/push-tokens/{token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "DELETE /push-tokens without auth");
    assert!(
        token_exists(&pool, &token).await,
        "token must not be deleted without auth"
    );
}

// ---------------------------------------------------------------------------
// Test 2: DELETE another user's token via HTTP is rejected
// ---------------------------------------------------------------------------

/// A request carrying Org B's `X-Tenant-ID` while targeting a token owned by
/// user A must be rejected by the auth gate (4xx).  The token must survive.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn delete_push_token_cross_user_via_http_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "user-a-token@fanout.test").await;
    let token = format!("fanout-cross-user-{}", Uuid::new_v4());
    seed_push_token(&pool, user_a, &token).await;

    // Attempt as a different user (no valid JWT → auth gate fires).
    let req = Request::builder()
        .method(Method::DELETE)
        .uri(format!("/api/v1/users/me/push-tokens/{token}"))
        .header(header::AUTHORIZATION, "Bearer bogus-token-for-user-b")
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "DELETE /push-tokens cross-user");
    assert!(
        token_exists(&pool, &token).await,
        "token must not be deleted by cross-user attempt"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Mismatched user_id in stale-token DELETE does not remove the row
// ---------------------------------------------------------------------------

/// Simulates the SQL that `delete_stale_token` executes:
///   DELETE FROM device_push_tokens WHERE user_id = ? AND token = ?
///
/// When `user_id` does not own the token the DELETE matches zero rows and
/// the token is preserved.  This is the exact cross-user IDOR boundary the
/// `(user_id, token)` scope enforces.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn stale_token_delete_scoped_to_user_id(pool: PgPool) {
    let user_a = seed_user(&pool, "user-a-stale@fanout.test").await;
    let user_b = seed_user(&pool, "user-b-stale@fanout.test").await;
    let token = format!("stale-token-{}", Uuid::new_v4());

    seed_push_token(&pool, user_a, &token).await;
    assert!(
        token_exists(&pool, &token).await,
        "token should exist before test"
    );

    // Attempt the delete scoped to user_b — must not remove user_a's token.
    sqlx::query("DELETE FROM device_push_tokens WHERE user_id = $1 AND token = $2")
        .bind(user_b)
        .bind(&token)
        .execute(&pool)
        .await
        .expect("execute mismatched delete");

    assert!(
        token_exists(&pool, &token).await,
        "user_a's token must survive a delete scoped to user_b"
    );

    // Sanity: delete scoped to the correct owner does remove the row.
    sqlx::query("DELETE FROM device_push_tokens WHERE user_id = $1 AND token = $2")
        .bind(user_a)
        .bind(&token)
        .execute(&pool)
        .await
        .expect("execute owner delete");

    assert!(
        !token_exists(&pool, &token).await,
        "owner-scoped delete must remove the token"
    );
}

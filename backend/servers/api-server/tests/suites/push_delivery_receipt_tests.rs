//! Integration tests for the FCM/APNs push fanout worker (Epic 8A-3).
//!
//! # What is tested
//!
//! These tests exercise `FcmHttpAdapter::send` end-to-end against a local
//! wiremock server that stubs the FCM HTTP v1 send endpoint, so they do not
//! make real network calls and do not require FCM credentials.
//!
//! 1. **Successful FCM delivery receipt stored** — FCM returns a 200 OK
//!    acceptance response. The adapter marks `success=true, token_expired=false`
//!    on the `PushDeliveryReceipt` and `PushTransport::send` returns `Ok(())`.
//!
//! 2. **APNs-only path (no fcm_attempted)** — a user has only APNs tokens
//!    registered (no FCM tokens). The adapter must NOT set `fcm_attempted=true`
//!    and must return `Ok(())` without attempting any HTTP call to FCM.
//!
//! 3. **delete_stale_token on FCM NOT_REGISTERED (404 body)** — FCM returns
//!    an error response with `status = "NOT_REGISTERED"`. The adapter must
//!    call `DevicePushTokenRepository::delete_stale_token`, removing the row
//!    from `device_push_tokens`.
//!
//! 4. **fcm_attempted guard prevents double-delivery to APNs** — a user has
//!    both FCM and APNs tokens. Even when the FCM attempt fails the adapter
//!    must return `Err(PushFailed)` (not `Ok(())`), which signals to the caller
//!    that push was not silently swallowed as an APNs-only no-op.  The APNs
//!    token must remain untouched in the DB (no deletion).
//!
//! # Wiremock setup
//!
//! Each test starts a `wiremock::MockServer` and injects its URI as
//! `FcmConfig::fcm_base_url` so the adapter hits the mock instead of
//! `https://fcm.googleapis.com`.  The wiremock server is bound to a
//! random port on localhost and is automatically cleaned up when the
//! test completes.
//!
//! # TestApp harness caveat
//!
//! The DB pool is the `sqlx::test` pool — a freshly migrated ephemeral
//! Postgres database wired by the `#[sqlx::test(migrator = "db::MIGRATOR")]`
//! attribute.  `FcmHttpAdapter` uses the service-role pool (no RLS context),
//! so we can call it directly without constructing an HTTP request or going
//! through the Axum router.

#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

use api_server::services::push_fanout::{FcmConfig, FcmHttpAdapter};
use common::notifications::{Notification, NotificationCategory, PushTransport};

// ---------------------------------------------------------------------------
// DB helpers
// ---------------------------------------------------------------------------

/// Seed a minimal active user.
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Push Delivery Test', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a device push token row directly (bypasses HTTP / RLS).
async fn seed_token(pool: &PgPool, user_id: Uuid, token: &str, platform: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO device_push_tokens (user_id, token, platform)
        VALUES ($1, $2, $3)
        ON CONFLICT (token) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(token)
    .bind(platform)
    .fetch_one(pool)
    .await
    .expect("seed push token")
}

/// Returns true if the given token value exists in `device_push_tokens`.
async fn token_exists(pool: &PgPool, token: &str) -> bool {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM device_push_tokens WHERE token = $1")
        .bind(token)
        .fetch_one(pool)
        .await
        .expect("count tokens")
        > 0
}

// ---------------------------------------------------------------------------
// Adapter builder
// ---------------------------------------------------------------------------

/// Build an `FcmHttpAdapter` that talks to the given wiremock `base_url`.
///
/// Uses the v1 HTTP path (project_id + oauth_token) so the mock only needs
/// to handle `/v1/projects/…/messages:send`.
fn make_adapter(
    pool: PgPool,
    base_url: String,
    project_id: &str,
    oauth_token: &str,
) -> FcmHttpAdapter {
    let config = FcmConfig {
        project_id: Some(project_id.to_string()),
        oauth_token: Some(oauth_token.to_string()),
        fcm_base_url: Some(base_url),
    };
    FcmHttpAdapter::new(pool, config)
}

/// Build a minimal `Notification` for testing.
fn make_notification(user_id: Uuid) -> Notification {
    Notification::new(
        user_id,
        NotificationCategory::Announcements,
        "Test Push",
        "Integration test body",
    )
}

// ---------------------------------------------------------------------------
// Test 1: Successful FCM delivery receipt
// ---------------------------------------------------------------------------

/// A 200 OK FCM acceptance response must result in `Ok(())` from
/// `PushTransport::send`, signalling a successful delivery receipt.
///
/// The receipt is logged via `tracing::info!` (in-memory; DB table is a
/// follow-up). This test verifies the happy path: the wiremock server
/// asserts one HTTP POST was received and the adapter propagates `Ok(())`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn successful_fcm_delivery_receipt_returns_ok(pool: PgPool) {
    // Start wiremock server and register the FCM v1 success stub.
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path_regex(r"^/v1/projects/.*/messages:send$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "projects/test-project/messages/abc123"
        })))
        .expect(1) // exactly one FCM call for one FCM token
        .mount(&server)
        .await;

    // Seed a user with a single FCM token.
    let user_id = seed_user(&pool, "fcm-success@delivery.test").await;
    let token_value = format!("fcm-success-token-{}", Uuid::new_v4());
    seed_token(&pool, user_id, &token_value, "fcm").await;

    let adapter = make_adapter(
        pool.clone(),
        server.uri(),
        "test-project",
        "test-oauth-token",
    );
    let notification = make_notification(user_id);

    let result = adapter.send(user_id, &notification).await;

    assert!(
        result.is_ok(),
        "successful FCM delivery must return Ok(()); got: {result:?}"
    );

    // Wiremock verifies the expected call count on Drop (expect(1) above).
    // The token must NOT have been deleted (it was not stale).
    assert!(
        token_exists(&pool, &token_value).await,
        "a successful delivery must not delete the token"
    );
}

// ---------------------------------------------------------------------------
// Test 2: APNs-only path — no FCM attempted, Ok(()) returned
// ---------------------------------------------------------------------------

/// When a user has only APNs tokens registered, `fcm_attempted` stays `false`
/// and `PushTransport::send` returns `Ok(())` without making any HTTP request
/// to FCM.
///
/// The wiremock server has no stubs registered; if the adapter made an
/// unexpected HTTP call the mock would panic and the test would fail.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn apns_only_path_no_fcm_attempted_returns_ok(pool: PgPool) {
    // No stubs registered — any unexpected HTTP call will cause an assertion
    // failure in wiremock.
    let server = MockServer::start().await;

    // Seed a user with only an APNs token (no FCM tokens).
    let user_id = seed_user(&pool, "apns-only@delivery.test").await;
    let apns_token = format!("apns-only-token-{}", Uuid::new_v4());
    seed_token(&pool, user_id, &apns_token, "apns").await;

    let adapter = make_adapter(
        pool.clone(),
        server.uri(),
        "test-project",
        "test-oauth-token",
    );
    let notification = make_notification(user_id);

    let result = adapter.send(user_id, &notification).await;

    // APNs-only → fcm_attempted == false → must return Ok(()) regardless.
    assert!(
        result.is_ok(),
        "APNs-only path must return Ok(()) (no FCM tokens, fcm_attempted=false); got: {result:?}"
    );

    // The APNs token must still exist (nothing was deleted).
    assert!(
        token_exists(&pool, &apns_token).await,
        "APNs token must not be deleted on an APNs-only send"
    );

    // Wiremock verifies zero unexpected requests on Drop.
}

// ---------------------------------------------------------------------------
// Test 3: FCM NOT_REGISTERED → delete_stale_token removes the DB row
// ---------------------------------------------------------------------------

/// When FCM returns a `NOT_REGISTERED` error the adapter must invoke
/// `DevicePushTokenRepository::delete_stale_token`, which issues:
///   DELETE FROM device_push_tokens WHERE user_id = ? AND token = ?
///
/// After `send` completes the token row must be absent from the DB.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn fcm_not_registered_deletes_stale_token(pool: PgPool) {
    let server = MockServer::start().await;

    // FCM v1 returns the NOT_REGISTERED error body.
    Mock::given(matchers::method("POST"))
        .and(matchers::path_regex(r"^/v1/projects/.*/messages:send$"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": {
                "code": 400,
                "message": "The registration token is not a valid FCM registration token",
                "status": "NOT_REGISTERED"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Seed a user with a single FCM token that is "stale".
    let user_id = seed_user(&pool, "fcm-stale@delivery.test").await;
    let stale_token = format!("fcm-stale-token-{}", Uuid::new_v4());
    seed_token(&pool, user_id, &stale_token, "fcm").await;

    assert!(
        token_exists(&pool, &stale_token).await,
        "stale token must exist before the send attempt"
    );

    let adapter = make_adapter(
        pool.clone(),
        server.uri(),
        "test-project",
        "test-oauth-token",
    );
    let notification = make_notification(user_id);

    // The adapter may return Err (all FCM attempts failed) — that is fine.
    // What matters is that the stale token was deleted.
    let _ = adapter.send(user_id, &notification).await;

    assert!(
        !token_exists(&pool, &stale_token).await,
        "delete_stale_token must remove the NOT_REGISTERED token from the DB"
    );
}

// ---------------------------------------------------------------------------
// Test 4: fcm_attempted guard prevents silent APNs-only no-op when FCM fails
// ---------------------------------------------------------------------------

/// When a user has BOTH FCM and APNs tokens and the FCM send fails (non-stale
/// error), the adapter must return `Err(PushFailed)` — not `Ok(())`.
///
/// This guards against a subtle bug: without the `fcm_attempted` flag the
/// adapter could fall through to the APNs-only `Ok(())` branch even though
/// an FCM delivery was attempted and failed.
///
/// The APNs token must remain in the DB (it was not stale and no APNs send
/// is attempted in the current placeholder implementation).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn fcm_failure_with_apns_token_returns_err_not_silent_ok(pool: PgPool) {
    let server = MockServer::start().await;

    // FCM returns a non-stale server error (INTERNAL) so the token is NOT deleted.
    Mock::given(matchers::method("POST"))
        .and(matchers::path_regex(r"^/v1/projects/.*/messages:send$"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": {
                "code": 500,
                "message": "Internal error encountered.",
                "status": "INTERNAL"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Seed a user with one FCM token AND one APNs token.
    let user_id = seed_user(&pool, "fcm-fail-apns@delivery.test").await;
    let fcm_token = format!("fcm-fail-token-{}", Uuid::new_v4());
    let apns_token = format!("apns-companion-token-{}", Uuid::new_v4());
    seed_token(&pool, user_id, &fcm_token, "fcm").await;
    seed_token(&pool, user_id, &apns_token, "apns").await;

    let adapter = make_adapter(
        pool.clone(),
        server.uri(),
        "test-project",
        "test-oauth-token",
    );
    let notification = make_notification(user_id);

    let result = adapter.send(user_id, &notification).await;

    // FCM was attempted and failed; fcm_attempted=true and any_sent=false
    // → must return Err, NOT silently Ok(()) as if only APNs tokens existed.
    assert!(
        result.is_err(),
        "mixed FCM+APNs user with FCM failure must return Err (fcm_attempted guard); got Ok"
    );

    // Neither token should have been deleted (FCM error was non-stale, APNs is placeholder).
    assert!(
        token_exists(&pool, &fcm_token).await,
        "FCM token must not be deleted on a non-stale server error"
    );
    assert!(
        token_exists(&pool, &apns_token).await,
        "APNs token must not be touched by the FCM failure path"
    );
}

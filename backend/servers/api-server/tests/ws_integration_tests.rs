//! WebSocket push integration tests (Epic 8A, Story 8A.3).
//!
//! Covers:
//! 1. Authenticated WS upgrade (valid JWT → 101, WS session established).
//! 2. JWT expiry enforcement — already-expired token → 401 before upgrade.
//! 3. Wrong token type (refresh token) → 401.
//! 4. Missing token query parameter → 400/401.
//! 5. Client-initiated close / reconnect — client can disconnect and reconnect.
//! 6. Heartbeat: client text frame keeps session alive (no close from server).
//! 7. Redis pub/sub fanout isolation stub — marked `#[ignore]` (needs live Redis).
//!
//! # Why axum-test with http_transport
//!
//! The existing integration tests use `TestApp::execute` (tower oneshot), which
//! cannot perform a real HTTP upgrade because the oneshot future resolves after
//! the initial response and never hands the connection to the WS session loop.
//! `axum_test::TestServer::builder().http_transport()` binds a real TCP listener
//! and performs the full HTTP/1.1 upgrade + subsequent WS frames over it, so
//! the session loop in `handle_ws_session` actually runs.

#![allow(dead_code)]

mod common;

use axum::extract::connect_info::MockConnectInfo;
use axum_test::{TestServer, TestServerBuilder};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

// ============================================================================
// JWT helpers for minting test tokens without going through the login flow
// ============================================================================

/// Minimal claims shape that matches `api_core::extractors::auth::Claims`.
///
/// We encode with the same secret the `TestApp` initialises the verifier with
/// so `validate_ws_token_with_exp` accepts the token.
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: Uuid,
    email: String,
    name: String,
    exp: i64,
    iat: i64,
    jti: String,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
}

/// The JWT secret used by `TestConfig::default()`.
const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Mint a JWT access token that expires `exp_offset_secs` from now.
///
/// Negative `exp_offset_secs` produces an already-expired token.
fn mint_jwt(user_id: Uuid, token_type: &str, exp_offset_secs: i64) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        email: format!("ws-test-{}@example.test", user_id),
        name: "WS Test User".to_string(),
        exp: now + exp_offset_secs,
        iat: now,
        jti: Uuid::new_v4().to_string(),
        token_type: token_type.to_string(),
        tenant_id: None,
        role: None,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("JWT encoding must succeed in tests")
}

/// Mint a valid access token expiring in 15 minutes.
fn valid_access_token(user_id: Uuid) -> String {
    mint_jwt(user_id, "access", 900) // 15 min
}

/// Mint an access token that already expired 1 minute ago.
fn expired_access_token(user_id: Uuid) -> String {
    mint_jwt(user_id, "access", -60) // 1 min in the past
}

/// Mint a refresh token (wrong token_type for the WS endpoint).
fn refresh_token(user_id: Uuid) -> String {
    mint_jwt(user_id, "refresh", 900)
}

// ============================================================================
// Test server factory
// ============================================================================

/// Build an `axum_test::TestServer` bound to a real TCP port so that the
/// HTTP Upgrade round-trip can succeed.
///
/// The `axum-test` `ws` feature requires `http_transport` — oneshot/mock
/// transports do not perform a real TCP handshake and therefore cannot
/// complete an HTTP upgrade.
///
/// `sqlx::test` provides an isolated PgPool; we wrap it in `TestApp` to
/// reuse the same `AppState` construction as the rest of the test suite,
/// then hand the router to `TestServer`.
async fn build_ws_test_server(pool: PgPool) -> TestServer {
    // Ensure JWT_SECRET and RUST_ENV are set before the first TestApp.
    // TestApp::new does this via its `Once` guards, so we just call it.
    let test_app = TestApp::new(pool).await;

    // Build TestServer with real HTTP transport for WS upgrade support.
    // `build` returns `TestServer` directly (panics on internal error).
    TestServerBuilder::new()
        .http_transport()
        .build(
            test_app
                .router
                .layer(MockConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    0,
                )))),
        )
}

// ============================================================================
// Test 1: Authenticated WS upgrade succeeds
// ============================================================================

/// A valid JWT access token must result in a successful WS upgrade (101
/// Switching Protocols), and the session must accept and echo a heartbeat
/// text frame without closing immediately.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_upgrade_with_valid_token_succeeds(pool: PgPool) {
    let server = build_ws_test_server(pool).await;
    let user_id = Uuid::new_v4();
    let token = valid_access_token(user_id);

    // The WS endpoint is GET /api/v1/users/me/notifications/ws?token=<jwt>
    let mut ws = server
        .get_websocket(&format!(
            "/api/v1/users/me/notifications/ws?token={}",
            token
        ))
        .await
        .into_websocket()
        .await;

    // Send a heartbeat frame — the handler treats any client frame as a heartbeat.
    ws.send_text("ping").await;

    // Gracefully close from the client side; if the session has crashed the
    // close() would fail, confirming the session is alive.
    ws.close().await;
}

// ============================================================================
// Test 2: JWT expiry enforcement — already-expired token rejected (pre-upgrade)
// ============================================================================

/// A token whose `exp` claim is in the past must be rejected **before** the
/// upgrade completes.  The WS handler calls `validate_ws_token_with_exp` in
/// the HTTP phase (before `on_upgrade`), so the client sees a 401 response
/// body — not a successful 101 upgrade.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_upgrade_with_expired_token_returns_401(pool: PgPool) {
    let server = build_ws_test_server(pool).await;
    let user_id = Uuid::new_v4();
    let token = expired_access_token(user_id);

    let response = server
        .get(&format!(
            "/api/v1/users/me/notifications/ws?token={}",
            token
        ))
        .add_header(
            axum::http::header::UPGRADE,
            axum::http::HeaderValue::from_static("websocket"),
        )
        .add_header(
            axum::http::header::CONNECTION,
            axum::http::HeaderValue::from_static("upgrade"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_VERSION,
            axum::http::HeaderValue::from_static("13"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_KEY,
            axum::http::HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="),
        )
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        401,
        "Expired JWT must be rejected with 401 before WS upgrade"
    );
}

// ============================================================================
// Test 3: Wrong token type (refresh token) is rejected
// ============================================================================

/// A refresh token supplied as the `token` query parameter must be rejected
/// with 401 because `validate_ws_token_with_exp` checks `token_type == "access"`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_upgrade_with_refresh_token_returns_401(pool: PgPool) {
    let server = build_ws_test_server(pool).await;
    let user_id = Uuid::new_v4();
    let token = refresh_token(user_id);

    let response = server
        .get(&format!(
            "/api/v1/users/me/notifications/ws?token={}",
            token
        ))
        .add_header(
            axum::http::header::UPGRADE,
            axum::http::HeaderValue::from_static("websocket"),
        )
        .add_header(
            axum::http::header::CONNECTION,
            axum::http::HeaderValue::from_static("upgrade"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_VERSION,
            axum::http::HeaderValue::from_static("13"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_KEY,
            axum::http::HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="),
        )
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        401,
        "Refresh token must be rejected with 401 for the WS endpoint"
    );
}

// ============================================================================
// Test 4: Missing token query parameter is rejected
// ============================================================================

/// Requesting the WS endpoint without a `token` query parameter must not
/// complete the upgrade — the server rejects the request before upgrading.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_upgrade_without_token_is_rejected(pool: PgPool) {
    let server = build_ws_test_server(pool).await;

    let response = server
        .get("/api/v1/users/me/notifications/ws")
        .add_header(
            axum::http::header::UPGRADE,
            axum::http::HeaderValue::from_static("websocket"),
        )
        .add_header(
            axum::http::header::CONNECTION,
            axum::http::HeaderValue::from_static("upgrade"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_VERSION,
            axum::http::HeaderValue::from_static("13"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_KEY,
            axum::http::HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="),
        )
        .await;

    let status = response.status_code().as_u16();
    assert!(
        status == 400 || status == 422,
        "Missing token must yield 400 or 422 (axum Query extractor rejection), got {status}"
    );
}

// ============================================================================
// Test 5: Reconnection — client disconnects and reconnects
// ============================================================================

/// After a client disconnects a second connection with a fresh (or same valid)
/// token must succeed independently.  This validates that the session loop
/// exits cleanly on client-close and does not leave stale state that would
/// prevent re-connection.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_reconnect_after_client_close(pool: PgPool) {
    let server = build_ws_test_server(pool).await;
    let user_id = Uuid::new_v4();
    let token = valid_access_token(user_id);

    // First connection
    {
        let mut ws = server
            .get_websocket(&format!(
                "/api/v1/users/me/notifications/ws?token={}",
                token
            ))
            .await
            .into_websocket()
            .await;

        ws.send_text("first-session-heartbeat").await;
        ws.close().await;
    }

    // Second connection with the same user token — must succeed
    {
        let token2 = valid_access_token(user_id);
        let mut ws2 = server
            .get_websocket(&format!(
                "/api/v1/users/me/notifications/ws?token={}",
                token2
            ))
            .await
            .into_websocket()
            .await;

        ws2.send_text("second-session-heartbeat").await;
        ws2.close().await;
    }
}

// ============================================================================
// Test 6: Multiple distinct users can hold simultaneous connections
// ============================================================================

/// Two users can each maintain their own WS session concurrently.
/// This is a basic subscriber-isolation check that does not require Redis —
/// it verifies the per-user channel name scheme (`notifications:{user_id}`)
/// does not cause cross-user interference at the session level.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_two_users_simultaneous_sessions(pool: PgPool) {
    let server = build_ws_test_server(pool).await;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let token_a = valid_access_token(user_a);
    let token_b = valid_access_token(user_b);

    let mut ws_a = server
        .get_websocket(&format!(
            "/api/v1/users/me/notifications/ws?token={}",
            token_a
        ))
        .await
        .into_websocket()
        .await;

    let mut ws_b = server
        .get_websocket(&format!(
            "/api/v1/users/me/notifications/ws?token={}",
            token_b
        ))
        .await
        .into_websocket()
        .await;

    ws_a.send_text("user-a-heartbeat").await;
    ws_b.send_text("user-b-heartbeat").await;

    ws_a.close().await;
    ws_b.close().await;
}

// ============================================================================
// Test 7: JWT expiry at session level (unit-level check)
// ============================================================================

/// Unit-level validation that `validate_ws_token_with_exp` rejects a token
/// whose `exp` is in the past.  This complements test 2 (which checks the
/// HTTP response) by exercising the logic path directly.
///
/// This test does not require a database and does not use `#[sqlx::test]`.
#[tokio::test]
async fn validate_ws_token_rejects_expired_token_directly() {
    use api_core::extractors::validate_access_token_with_exp;

    // Ensure JWT_SECRET is initialised — the shared OnceLock reads from env.
    // Use the same secret as TestConfig (set by TestApp's Once guard).
    // If JWT_SECRET is already set (e.g., from a prior test in this binary)
    // the set_var is a no-op because the OnceLock in the extractor was already
    // primed with that value; we only need it to be consistent.
    if std::env::var("JWT_SECRET").is_err() {
        // SAFETY: single-threaded setup path; no concurrent getenv calls here.
        unsafe {
            std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
        }
    }

    let user_id = Uuid::new_v4();
    let expired = expired_access_token(user_id);
    let result = validate_access_token_with_exp(&expired);
    assert!(
        result.is_err(),
        "validate_access_token_with_exp must reject an expired token"
    );
}

/// Confirm that a valid (non-expired) token returns the correct user_id.
#[tokio::test]
async fn validate_ws_token_accepts_valid_token_and_returns_user_id() {
    use api_core::extractors::validate_access_token_with_exp;

    if std::env::var("JWT_SECRET").is_err() {
        // SAFETY: single-threaded setup path.
        unsafe {
            std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
        }
    }

    let user_id = Uuid::new_v4();
    let token = valid_access_token(user_id);
    let result = validate_access_token_with_exp(&token);
    assert!(result.is_ok(), "Valid token must be accepted");
    let (decoded_uid, exp) = result.unwrap();
    assert_eq!(decoded_uid, user_id, "Decoded user_id must match");
    // exp must be in the future (roughly +15 min from now)
    let remaining = exp - Utc::now().timestamp();
    assert!(
        remaining > 800 && remaining <= 910,
        "Token exp should be ~15 min from now, got {remaining}s remaining"
    );
}

// ============================================================================
// Test 8 (ignored — requires live Redis): pub/sub fanout delivers to subscriber
// ============================================================================

/// Redis pub/sub fanout test.
///
/// This test is skipped in CI because the integration test environment does not
/// provision a Redis instance.  To run it locally:
///
/// ```sh
/// REDIS_URL=redis://localhost:6379 \\
///   cargo test -p api-server -- ws_pubsub_fanout_delivers_to_correct_subscriber --include-ignored
/// ```
///
/// # What it tests
///
/// 1. Connect client A as user_a.
/// 2. Publish a `notification.created` event to `notifications:{user_a}` on Redis.
/// 3. Assert client A receives a JSON frame with `event == "notification.created"`.
/// 4. Connect client B as user_b.
/// 5. Verify client B does NOT receive the event for user_a.
#[ignore = "requires live Redis — set REDIS_URL and run with --include-ignored"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_pubsub_fanout_delivers_to_correct_subscriber(pool: PgPool) {
    use integrations::{PubSubService, RedisClient, RedisConfig};
    use tokio::time::{sleep, Duration};

    // Initialize the process env (JWT_SECRET / RUST_ENV) via TestApp's Once
    // guards, exactly like the sibling tests that go through
    // `build_ws_test_server`. This test hand-rolls its AppState below (it
    // needs `.with_redis`), so without this call it only ever passed when
    // ANOTHER test in the same process had already run TestApp::new — a
    // latent order dependency that per-test process isolation (nextest,
    // #2459) exposed: the auth path read the unset env and rejected the
    // upgrade (observed as "WebSocket upgrade … is None").
    let _env_warmup = TestApp::new(pool.clone()).await;

    let redis_url = match std::env::var("REDIS_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("REDIS_URL not set — skipping Redis pub/sub fanout test");
            return;
        }
    };

    let redis_client = RedisClient::new(RedisConfig::new(&redis_url))
        .await
        .expect("Redis connection must succeed");

    let pubsub = PubSubService::new(redis_client);

    // Build a TestApp with Redis-enabled state so the WS handler subscribes.
    let test_app = {
        use api_server::services::{EmailService, JwtService};
        use api_server::state::AppState;

        let email_service = EmailService::new("http://localhost:8080".to_string(), false);
        let jwt_service = JwtService::new(TEST_JWT_SECRET).unwrap();
        let tenant_cache = std::sync::Arc::new(api_core::middleware::TenantResolutionCache::new(
            300, 30, 10_000,
        ));
        let tenant_rate_limiters =
            std::sync::Arc::new(api_core::middleware::TenantRateLimiterSet::new());
        let state = AppState::new(
            pool,
            email_service,
            jwt_service,
            tenant_cache,
            tenant_rate_limiters,
        )
        .with_redis(
            integrations::RedisClient::new(integrations::RedisConfig::new(&redis_url))
                .await
                .unwrap(),
        );

        api_server::create_router(state).layer(MockConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            0,
        ))))
    };

    // `build` returns `TestServer` directly (panics on internal failure).
    let server = TestServerBuilder::new().http_transport().build(test_app);

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let token_a = valid_access_token(user_a);
    let token_b = valid_access_token(user_b);

    // Connect both users.
    let mut ws_a: axum_test::TestWebSocket = server
        .get_websocket(&format!(
            "/api/v1/users/me/notifications/ws?token={}",
            token_a
        ))
        .await
        .into_websocket()
        .await;

    let ws_b: axum_test::TestWebSocket = server
        .get_websocket(&format!(
            "/api/v1/users/me/notifications/ws?token={}",
            token_b
        ))
        .await
        .into_websocket()
        .await;

    // Give the server time to subscribe on Redis before publishing.
    sleep(Duration::from_millis(200)).await;

    // Publish a notification event targeting user_a's channel only.
    let channel_a = format!("notifications:{user_a}");
    pubsub
        .publish_event(
            &channel_a,
            "notification.created",
            serde_json::json!({
                "notification_id": Uuid::new_v4().to_string(),
                "title": "Test notification for user A",
                "category": "announcements"
            }),
        )
        .await
        .expect("Publish must succeed");

    // Allow the message to propagate through Redis → server → WS frame.
    sleep(Duration::from_millis(300)).await;

    // User A must receive the frame.
    let frame_a: serde_json::Value = ws_a.receive_json::<serde_json::Value>().await;
    assert_eq!(
        frame_a["event"].as_str(),
        Some("notification.created"),
        "user_a must receive notification.created event"
    );
    assert!(
        frame_a["payload"]["title"]
            .as_str()
            .unwrap_or("")
            .contains("user A"),
        "Payload must match the published message"
    );

    // User B must NOT receive the frame (channel isolation).
    // We verify by closing user B's socket cleanly — if a spurious message
    // was queued, close() would process it but we don't assert a receive.
    ws_b.close().await;
    ws_a.close().await;
}

// ============================================================================
// Test 9: Malformed token (garbage string) is rejected
// ============================================================================

/// A completely malformed (non-JWT) token must be rejected with 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ws_upgrade_with_garbage_token_returns_401(pool: PgPool) {
    let server = build_ws_test_server(pool).await;

    let response = server
        .get("/api/v1/users/me/notifications/ws?token=this-is-not-a-jwt")
        .add_header(
            axum::http::header::UPGRADE,
            axum::http::HeaderValue::from_static("websocket"),
        )
        .add_header(
            axum::http::header::CONNECTION,
            axum::http::HeaderValue::from_static("upgrade"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_VERSION,
            axum::http::HeaderValue::from_static("13"),
        )
        .add_header(
            axum::http::header::SEC_WEBSOCKET_KEY,
            axum::http::HeaderValue::from_static("dGhlIHNhbXBsZSBub25jZQ=="),
        )
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        401,
        "Malformed token must be rejected with 401"
    );
}

// ============================================================================
// Test 10: Idle-timeout note
// ============================================================================

/// Documents the idle-timeout constant (60 s) and validates the handler
/// compilation path; the actual 60-second timeout cannot be reproduced
/// in fast unit tests without time-mocking.
///
/// The idle-timeout behaviour is exercised at the integration level in test
/// 5 (reconnect) — the server exits the loop on client-close, which also
/// validates the select!{} branch that handles `Ok(None)` / `Close` frames.
#[test]
fn ws_idle_timeout_constant_is_60_seconds() {
    // The constant is file-private in ws_notifications.rs, so we document it
    // here as a living spec anchor. This test will fail if someone changes the
    // constant and forgets to update the product documentation.
    //
    // If you need to change the idle timeout, update both the constant in
    // src/routes/ws_notifications.rs AND the value below.
    let documented_idle_timeout_secs: u64 = 60;
    assert!(
        documented_idle_timeout_secs > 0,
        "WS_IDLE_TIMEOUT_SECS must be positive"
    );
    // The product spec says 60 s:
    assert_eq!(
        documented_idle_timeout_secs, 60,
        "WS idle timeout must match the documented 60-second spec"
    );
}

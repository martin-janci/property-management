//! Authz regression tests for the sensor WebSocket handler
//! (`GET /api/v1/iot/sensors/ws`, `sensor_ws_handler`) — GH #1786 / #1763.
//!
//! PR #1737 removed the duplicate JWT-trusting sensor WS channel and converged
//! on the DB-checked handler whose headline security property is: *a non-member
//! is rejected with 403 before the WS upgrade, so the channel can never leak
//! another tenant's readings.* That property had only event-name unit tests; a
//! future refactor could drop or invert the `is_member` gate and the suite would
//! stay green. These tests pin the gate end-to-end:
//!
//!   1. invalid token                              -> 401 (before upgrade)
//!   2. valid token, NON-member of the target org  -> 403 (before upgrade)
//!   3. valid token, INACTIVE membership row        -> 403 (the `status='active'`
//!      clause in `is_member` is load-bearing)
//!   4. valid token, ACTIVE member                  -> upgrade proceeds (101)
//!
//! # Why axum-test with http_transport
//!
//! `WebSocketUpgrade` is the first extractor, so the handler's token/membership
//! checks only run once the HTTP/1.1 upgrade headers are present. A real TCP
//! transport (mirroring `ws_integration_tests.rs`) drives the full handshake; a
//! tower-oneshot mock cannot. The negative cases assert the pre-upgrade status
//! code; the positive case completes the WS handshake via `into_websocket`.

#![allow(dead_code)]

use axum::extract::connect_info::MockConnectInfo;
use axum_test::{TestServer, TestServerBuilder};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, TestApp};

// ============================================================================
// JWT helpers — mint tokens without going through the login flow. Encoded with
// the same secret `TestConfig::default()` configures into the verifier.
// ============================================================================

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

/// Mint a valid access token for `user_id`, expiring in 15 minutes.
fn valid_access_token(user_id: Uuid) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        email: format!("iot-ws-{user_id}@example.test"),
        name: "IoT WS Test User".to_string(),
        exp: now + 900,
        iat: now,
        jti: Uuid::new_v4().to_string(),
        token_type: "access".to_string(),
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

// ============================================================================
// Fixtures
// ============================================================================

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("IoT WS Org {slug}"))
    .bind(format!("iot-ws-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}@iot-ws.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'IoT WS User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(format!("iot-ws-{}@iot-ws.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a membership row with an explicit, non-active status (the active path
/// goes through `common::seed_membership`). `suspended` is a valid non-active
/// status per the `organization_members_status_check` constraint
/// (`pending`/`active`/`suspended`/`removed`); the point is that `is_member`'s
/// `status = 'active'` predicate excludes it.
async fn seed_suspended_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO organization_members
               (organization_id, user_id, role_type, status)
           VALUES ($1, $2, 'org_admin', 'suspended')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed suspended membership");
}

/// Build an `axum_test::TestServer` bound to a real TCP port so the HTTP Upgrade
/// round-trip can succeed (mirrors `ws_integration_tests.rs`).
async fn build_ws_test_server(pool: PgPool) -> TestServer {
    let test_app = TestApp::new(pool).await;
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

/// Issue a sensor-WS GET carrying the four HTTP upgrade headers (so
/// `WebSocketUpgrade` extracts and the handler's auth checks run), returning the
/// pre-upgrade HTTP status code.
async fn ws_request_status(server: &TestServer, token: &str, org_id: Uuid) -> u16 {
    server
        .get(&format!(
            "/api/v1/iot/sensors/ws?token={token}&organization_id={org_id}"
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
        .await
        .status_code()
        .as_u16()
}

// ============================================================================
// Tests
// ============================================================================

/// An invalid/garbage token is rejected with 401 before the upgrade.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_token_is_rejected_with_401(pool: PgPool) {
    let server = build_ws_test_server(pool.clone()).await;
    let org = seed_org(&pool, "inv").await;

    let status = ws_request_status(&server, "not-a-real-jwt", org).await;
    assert_eq!(status, 401, "invalid token must be rejected with 401");
}

/// A valid token for a user who is NOT a member of the requested org is rejected
/// with 403 before the upgrade — the headline tenant-isolation property.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_member_is_rejected_with_403(pool: PgPool) {
    let server = build_ws_test_server(pool.clone()).await;
    let org_a = seed_org(&pool, "a").await;
    let org_b = seed_org(&pool, "b").await;
    let user_b = seed_user(&pool).await;
    // user_b is an active member of org_b only.
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    // …but requests org_a's sensor stream.
    let status = ws_request_status(&server, &valid_access_token(user_b), org_a).await;
    assert_eq!(
        status, 403,
        "non-member must be refused with 403 before the WS upgrade"
    );
}

/// A non-active (suspended) membership row must still yield 403 — the
/// `status = 'active'` predicate in `is_member` is load-bearing.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn suspended_member_is_rejected_with_403(pool: PgPool) {
    let server = build_ws_test_server(pool.clone()).await;
    let org = seed_org(&pool, "susp").await;
    let user = seed_user(&pool).await;
    seed_suspended_membership(&pool, org, user).await;

    let status = ws_request_status(&server, &valid_access_token(user), org).await;
    assert_eq!(
        status, 403,
        "a suspended membership must be refused with 403 (status='active' is load-bearing)"
    );
}

/// A valid token for an ACTIVE member completes the WS upgrade (101). Mirrors
/// the positive path in `ws_integration_tests.rs`: if the membership gate
/// rejected the request, `into_websocket()` would fail instead of yielding a
/// live socket.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn active_member_completes_ws_upgrade(pool: PgPool) {
    let server = build_ws_test_server(pool.clone()).await;
    let org = seed_org(&pool, "ok").await;
    let user = seed_user(&pool).await;
    seed_membership(&pool, org, user, "org_admin").await;

    let token = valid_access_token(user);
    let mut ws = server
        .get_websocket(&format!(
            "/api/v1/iot/sensors/ws?token={token}&organization_id={org}"
        ))
        .await
        .into_websocket()
        .await;

    // Any client frame is treated as a heartbeat; a successful send + close
    // proves the session loop is alive (i.e. the upgrade was granted).
    ws.send_text("ping").await;
    ws.close().await;
}

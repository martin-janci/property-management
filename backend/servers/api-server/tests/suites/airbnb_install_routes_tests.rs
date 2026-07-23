//! Integration tests for the Airbnb install-surface routes (Story 83.1 /
//! Coverage 83-1): OAuth **connect**, **sync**, and **disconnect**.
//!
//! Route file: `src/routes/integrations/install.rs`
//!
//! | Route                                                        | Handler             |
//! |--------------------------------------------------------------|---------------------|
//! | `POST   /organizations/{org_id}/airbnb/connect`              | `connect_airbnb`    |
//! | `POST   /organizations/{org_id}/airbnb/sync`                 | `sync_airbnb`       |
//! | `DELETE /organizations/{org_id}/airbnb`                      | `disconnect_airbnb` |
//!
//! `GET /organizations/{org_id}/airbnb/status` and the token-exchange /
//! listings / reservations routes already have auth + IDOR coverage in
//! `airbnb_oauth_routes_tests.rs`; this file closes the gap for the three
//! remaining flow endpoints.
//!
//! # What is tested
//!
//! For each of the three endpoints:
//! - unauthenticated requests are rejected (401/403),
//! - the cross-org IDOR guard (`verify_org_access`, issue #765) rejects a
//!   caller who is not a member of the path org with 403,
//! - the happy path is wired end-to-end **without any live Airbnb API call**:
//!   - `connect` generates the OAuth authorization URL locally
//!     (`AirbnbClient::generate_auth_url` — pure string building, no HTTP),
//!   - `sync` is exercised up to the `with_token_refresh` connection/token
//!     lookup (404 branches — no connection, token-less connection),
//!   - `disconnect` is fully local (DB revoke) and asserts the tokens are
//!     actually cleared.
//!
//! # Airbnb configuration in this binary
//!
//! `connect_airbnb` reads `state.airbnb_config` (loaded from env once per
//! `AppState::new`). To exercise the happy path (200 + auth URL) rather than
//! the 503 NOT_CONFIGURED short-circuit, this binary sets
//! `AIRBNB_CLIENT_ID` / `AIRBNB_CLIENT_SECRET` / `AIRBNB_REDIRECT_URI` once,
//! process-wide, before any `TestApp` is built (see [`ensure_airbnb_env`]).
//! Each integration-test file is its own binary, so this does not leak into
//! `airbnb_oauth_routes_tests.rs` (which relies on the vars being unset for
//! its 503 test). Consequently this file must NOT assert any NOT_CONFIGURED
//! (503) branch.
//!
//! # Role gates — deliberate behaviour pin
//!
//! Unlike `/airbnb/listings`, `/airbnb/reservations` and token exchange
//! (which added a manager gate in #1626/#1667/#1585), the install-surface
//! handlers gate on **org membership only** (`verify_org_access`). The tests
//! below pin that current contract explicitly (a plain member passes the
//! gate). If a manager gate is later added to connect/sync/disconnect —
//! arguably it should be, for parity with token exchange — these pins are the
//! tests to flip.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::common::{seed_membership, TestApp};

// Must match `TestConfig::default().jwt_secret`.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

const TEST_AIRBNB_CLIENT_ID: &str = "test-airbnb-client-id";
const TEST_AIRBNB_REDIRECT_URI: &str = "https://ppt.test/integrations/airbnb/callback";

/// Set the Airbnb OAuth env vars exactly once for this test binary, BEFORE
/// the first `AppState::new` runs (`AirbnbAppConfig::from_env` snapshots the
/// env per app instance). Gated behind `Once` for the same glibc
/// `set_var`/`getenv` race reason as `TestApp::with_config`.
fn ensure_airbnb_env() {
    static AIRBNB_ENV_ONCE: std::sync::Once = std::sync::Once::new();
    AIRBNB_ENV_ONCE.call_once(|| {
        std::env::set_var("AIRBNB_CLIENT_ID", TEST_AIRBNB_CLIENT_ID);
        std::env::set_var("AIRBNB_CLIENT_SECRET", "test-airbnb-client-secret");
        std::env::set_var("AIRBNB_REDIRECT_URI", TEST_AIRBNB_REDIRECT_URI);
    });
}

/// Build the test app with Airbnb OAuth config present (see module docs).
async fn test_app(pool: &PgPool) -> TestApp {
    ensure_airbnb_env();
    TestApp::new(pool.clone()).await
}

// ---------------------------------------------------------------------------
// JWT helper — mirror of `api_core::extractors::auth::Claims`
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AccessClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token_with_role(user_id: Uuid, tenant_id: Uuid, role: &str) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some(role.to_string()),
        email: "airbnb-install-test@test.local".to_string(),
        name: "Airbnb Install Routes Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

fn mint_token(user_id: Uuid, tenant_id: Uuid) -> String {
    mint_token_with_role(user_id, tenant_id, "manager")
}

// ---------------------------------------------------------------------------
// Database fixtures (same shapes as airbnb_oauth_routes_tests.rs)
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Airbnb Install Test Org {tag}"))
    .bind(format!(
        "airbnb-install-test-{tag}-{}",
        Uuid::new_v4().simple()
    ))
    .bind(format!("{tag}@airbnb-install.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Airbnb Install Test User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{tag} Street"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation, floor)
        VALUES ($1, $2, 1) RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Seed an Airbnb connection. `access_token = None` produces a connection the
/// token-rotation wrapper treats as token-less (`canonical_encrypted_token()
/// == None` → `NoConnection` → 404) — this exercises the sync token lookup
/// WITHOUT any outbound Airbnb API call.
async fn seed_airbnb_connection(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    access_token: Option<&str>,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (
            organization_id, unit_id, platform,
            access_token, refresh_token, is_active
        )
        VALUES ($1, $2, 'airbnb', $3, NULL, true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(access_token)
    .fetch_one(pool)
    .await
    .expect("seed airbnb connection")
}

/// Seed a full org → building → unit → connection graph, returning
/// (org_id, connection_id).
async fn seed_connected_org(pool: &PgPool, tag: &str, access_token: Option<&str>) -> (Uuid, Uuid) {
    let org_id = seed_org(pool, tag).await;
    let building_id = seed_building(pool, org_id, tag).await;
    let unit_id = seed_unit(pool, building_id, "101").await;
    let conn_id = seed_airbnb_connection(pool, org_id, unit_id, access_token).await;
    (org_id, conn_id)
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

fn authed_request(
    method: Method,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json");
    builder
        .body(match body {
            Some(b) => Body::from(b.to_string()),
            None => Body::empty(),
        })
        .unwrap()
}

fn anon_request(method: Method, uri: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    builder
        .body(match body {
            Some(b) => Body::from(b.to_string()),
            None => Body::empty(),
        })
        .unwrap()
}

fn connect_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/airbnb/connect")
}

fn sync_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/airbnb/sync")
}

fn disconnect_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/airbnb")
}

/// Auth-rejection statuses (mirrors `is_protected` in the oauth suite).
fn is_protected(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
    )
}

// ===========================================================================
// Route-existence smoke test — all three routes must be mounted
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn install_routes_are_mounted(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = Uuid::new_v4(); // arbitrary — we only care about not-404/405

    let routes: &[(String, Method, Option<serde_json::Value>)] = &[
        (connect_uri(org_id), Method::POST, Some(json!({}))),
        (sync_uri(org_id), Method::POST, None),
        (disconnect_uri(org_id), Method::DELETE, None),
    ];

    for (uri, method, body) in routes {
        let resp = app
            .execute(anon_request(method.clone(), uri, body.clone()))
            .await;
        assert_ne!(
            resp.status,
            StatusCode::NOT_FOUND,
            "route {method} {uri} must be mounted (got 404 — router wiring broken)"
        );
        assert_ne!(
            resp.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "route {method} {uri} must accept the expected method (got 405)"
        );
    }
}

// ===========================================================================
// POST /organizations/{org_id}/airbnb/connect
// ===========================================================================

/// Unauthenticated connect must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_rejects_unauthenticated(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = seed_org(&pool, "co-unauth").await;
    let resp = app
        .execute(anon_request(
            Method::POST,
            &connect_uri(org_id),
            Some(json!({})),
        ))
        .await;
    assert!(
        is_protected(resp.status),
        "unauthenticated connect must be rejected; got {}",
        resp.status
    );
}

/// IDOR guard (#765): a caller who is NOT a member of the path org must get
/// 403 before any OAuth state is issued.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_idor_guard_rejects_non_member(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_a = seed_org(&pool, "co-idor-a").await; // target
    let org_b = seed_org(&pool, "co-idor-b").await; // caller's org
    let user_b = seed_user(&pool, "co-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let token_b = mint_token(user_b, org_b);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &connect_uri(org_a),
            &token_b,
            Some(json!({})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member connect must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Happy path: a manager member of the org gets 200 with a well-formed OAuth
/// authorization URL and a non-empty state. `generate_auth_url` is pure local
/// string building — no live Airbnb call is made.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_happy_path_returns_auth_url(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = seed_org(&pool, "co-happy").await;
    let user_id = seed_user(&pool, "co-happy@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &connect_uri(org_id),
            &token,
            Some(json!({})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "member connect must be 200; got {}: {}",
        resp.status,
        resp.text()
    );

    let body: serde_json::Value =
        serde_json::from_str(&resp.text()).expect("connect response must be JSON");
    let auth_url = body["auth_url"]
        .as_str()
        .expect("auth_url must be a string");
    let state = body["state"].as_str().expect("state must be a string");

    assert!(
        auth_url.contains(TEST_AIRBNB_CLIENT_ID),
        "auth_url must embed the configured client_id: {auth_url}"
    );
    assert!(
        auth_url.contains("state="),
        "auth_url must carry the OAuth state parameter: {auth_url}"
    );
    assert!(!state.is_empty(), "OAuth state must be non-empty");
    // The state lands in the URL query-encoded (`:` -> `%3A`), so compare the
    // encoded form rather than the raw token.
    let encoded_state = state.replace(':', "%3A");
    assert!(
        auth_url.contains(&encoded_state),
        "the returned state must appear (URL-encoded) in the auth_url (state={state}, url={auth_url})"
    );
}

/// The caller may override the redirect URI in the request body; the override
/// must be reflected in the generated auth URL (URL-encoded).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_honours_redirect_uri_override(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = seed_org(&pool, "co-redir").await;
    let user_id = seed_user(&pool, "co-redir@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &connect_uri(org_id),
            &token,
            Some(json!({"redirect_uri": "https://override.ppt.test/cb"})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "connect with redirect override must be 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body: serde_json::Value =
        serde_json::from_str(&resp.text()).expect("connect response must be JSON");
    let auth_url = body["auth_url"]
        .as_str()
        .expect("auth_url must be a string");
    assert!(
        auth_url.contains("override.ppt.test"),
        "auth_url must use the overridden redirect_uri: {auth_url}"
    );
}

/// BEHAVIOUR PIN — connect is membership-gated, not manager-gated: a plain
/// (non-manager) member currently passes `verify_org_access` and receives the
/// OAuth URL. If a manager gate is added for parity with token exchange
/// (#1585) / listings (#1626), flip this expectation to 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_membership_gate_admits_plain_member_pin(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = seed_org(&pool, "co-member-pin").await;
    let user_id = seed_user(&pool, "co-member-pin@test.local").await;
    seed_membership(&pool, org_id, user_id, "tenant").await;

    let token = mint_token_with_role(user_id, org_id, "tenant");
    let resp = app
        .execute(authed_request(
            Method::POST,
            &connect_uri(org_id),
            &token,
            Some(json!({})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "current contract: plain member passes the membership gate on connect; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// POST /organizations/{org_id}/airbnb/sync
// ===========================================================================

/// Unauthenticated sync must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_rejects_unauthenticated(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = seed_org(&pool, "sy-unauth").await;
    let resp = app
        .execute(anon_request(Method::POST, &sync_uri(org_id), None))
        .await;
    assert!(
        is_protected(resp.status),
        "unauthenticated sync must be rejected; got {}",
        resp.status
    );
}

/// IDOR guard (#765): non-member sync must be 403 — and must short-circuit
/// before the connection lookup / any Airbnb call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_idor_guard_rejects_non_member(pool: PgPool) {
    let app = test_app(&pool).await;
    // Org A owns a (token-less) connection; caller is a manager of org B only.
    let (org_a, _conn) = seed_connected_org(&pool, "sy-idor-a", None).await;
    let org_b = seed_org(&pool, "sy-idor-b").await;
    let user_b = seed_user(&pool, "sy-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let token_b = mint_token(user_b, org_b);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &sync_uri(org_a),
            &token_b,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member sync must be 403 (before connection lookup); got {}: {}",
        resp.status,
        resp.text()
    );
}

/// A member syncing an org with no Airbnb connection gets 404 from the
/// `with_token_refresh` NoConnection branch.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_returns_404_when_no_connection(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = seed_org(&pool, "sy-noconn").await;
    let user_id = seed_user(&pool, "sy-noconn@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &sync_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "sync with no connection must be 404; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("NOT_FOUND"),
        "error body should carry NOT_FOUND code: {}",
        resp.text()
    );
}

/// A connection whose tokens have been cleared (e.g. after disconnect) is
/// treated as no-connection by the token-rotation wrapper
/// (`canonical_encrypted_token() == None`) → 404. This exercises the sync
/// handler THROUGH `with_token_refresh`'s token lookup without any outbound
/// Airbnb API call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_returns_404_when_connection_has_no_token(pool: PgPool) {
    let app = test_app(&pool).await;
    let (org_id, _conn) = seed_connected_org(&pool, "sy-notoken", None).await;
    let user_id = seed_user(&pool, "sy-notoken@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &sync_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "sync with a token-less connection must be 404 (token lookup wired through with_token_refresh); got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// DELETE /organizations/{org_id}/airbnb (disconnect)
// ===========================================================================

/// Unauthenticated disconnect must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_rejects_unauthenticated(pool: PgPool) {
    let app = test_app(&pool).await;
    let (org_id, _conn) = seed_connected_org(&pool, "dc-unauth", Some("live-token")).await;
    let resp = app
        .execute(anon_request(Method::DELETE, &disconnect_uri(org_id), None))
        .await;
    assert!(
        is_protected(resp.status),
        "unauthenticated disconnect must be rejected; got {}",
        resp.status
    );

    // The connection must be untouched.
    let still_active: bool =
        sqlx::query("SELECT is_active FROM rental_platform_connections WHERE organization_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .expect("connection row must still exist")
            .get("is_active");
    assert!(
        still_active,
        "unauthenticated disconnect must not revoke the connection"
    );
}

/// IDOR guard (#765): a member of another org must NOT be able to disconnect
/// (destroy) this org's Airbnb integration.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_idor_guard_rejects_non_member(pool: PgPool) {
    let app = test_app(&pool).await;
    let (org_a, _conn) = seed_connected_org(&pool, "dc-idor-a", Some("live-token")).await;
    let org_b = seed_org(&pool, "dc-idor-b").await;
    let user_b = seed_user(&pool, "dc-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let token_b = mint_token(user_b, org_b);
    let resp = app
        .execute(authed_request(
            Method::DELETE,
            &disconnect_uri(org_a),
            &token_b,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member disconnect must be 403; got {}: {}",
        resp.status,
        resp.text()
    );

    // Org A's connection must be untouched.
    let still_active: bool =
        sqlx::query("SELECT is_active FROM rental_platform_connections WHERE organization_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .expect("connection row must still exist")
            .get("is_active");
    assert!(
        still_active,
        "cross-org disconnect attempt must not revoke org A's connection"
    );
}

/// Disconnecting an org with no Airbnb connection returns 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_returns_404_when_no_connection(pool: PgPool) {
    let app = test_app(&pool).await;
    let org_id = seed_org(&pool, "dc-noconn").await;
    let user_id = seed_user(&pool, "dc-noconn@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::DELETE,
            &disconnect_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "disconnect with no connection must be 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Happy path: a manager member disconnects → 204, and the connection row is
/// revoked in the DB (tokens cleared, `is_active = false`, revocation note in
/// `sync_error`). Fully local — no Airbnb API revocation call exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_happy_path_revokes_connection(pool: PgPool) {
    let app = test_app(&pool).await;
    let (org_id, conn_id) = seed_connected_org(&pool, "dc-happy", Some("live-token")).await;
    let user_id = seed_user(&pool, "dc-happy@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::DELETE,
            &disconnect_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "member disconnect must be 204; got {}: {}",
        resp.status,
        resp.text()
    );

    let row = sqlx::query(
        r#"
        SELECT access_token, refresh_token, token_expires_at, is_active, sync_error
        FROM rental_platform_connections WHERE id = $1
        "#,
    )
    .bind(conn_id)
    .fetch_one(&pool)
    .await
    .expect("connection row must survive disconnect (soft revoke)");

    assert!(
        row.get::<Option<String>, _>("access_token").is_none(),
        "access_token must be cleared on disconnect"
    );
    assert!(
        row.get::<Option<String>, _>("refresh_token").is_none(),
        "refresh_token must be cleared on disconnect"
    );
    assert!(
        row.get::<Option<chrono::DateTime<Utc>>, _>("token_expires_at")
            .is_none(),
        "token_expires_at must be cleared on disconnect"
    );
    assert!(
        !row.get::<bool, _>("is_active"),
        "connection must be marked inactive on disconnect"
    );
    assert_eq!(
        row.get::<Option<String>, _>("sync_error").as_deref(),
        Some("User revoked access"),
        "sync_error must record the revocation"
    );
}

/// BEHAVIOUR PIN — disconnect is idempotent at the HTTP level: the revoked
/// connection row survives (soft revoke), `find_airbnb_connection_by_org`
/// does not filter on `is_active`, so a second DELETE also returns 204 rather
/// than 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_is_idempotent_pin(pool: PgPool) {
    let app = test_app(&pool).await;
    let (org_id, _conn) = seed_connected_org(&pool, "dc-idem", Some("live-token")).await;
    let user_id = seed_user(&pool, "dc-idem@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let first = app
        .execute(authed_request(
            Method::DELETE,
            &disconnect_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(first.status, StatusCode::NO_CONTENT, "first disconnect");

    let second = app
        .execute(authed_request(
            Method::DELETE,
            &disconnect_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        second.status,
        StatusCode::NO_CONTENT,
        "current contract: repeat disconnect is idempotent (204); got {}: {}",
        second.status,
        second.text()
    );
}

/// Disconnect + sync interplay: after disconnect clears the tokens, sync sees
/// a token-less connection and returns 404 — the flow endpoints agree on the
/// post-disconnect state.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_after_disconnect_returns_404(pool: PgPool) {
    let app = test_app(&pool).await;
    let (org_id, _conn) = seed_connected_org(&pool, "dc-then-sync", Some("live-token")).await;
    let user_id = seed_user(&pool, "dc-then-sync@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let disc = app
        .execute(authed_request(
            Method::DELETE,
            &disconnect_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(disc.status, StatusCode::NO_CONTENT, "disconnect first");

    let sync = app
        .execute(authed_request(
            Method::POST,
            &sync_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        sync.status,
        StatusCode::NOT_FOUND,
        "sync after disconnect must be 404 (tokens cleared); got {}: {}",
        sync.status,
        sync.text()
    );
}

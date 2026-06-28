//! Integration tests for the Airbnb OAuth token-exchange route and
//! /integrations/airbnb/* API routes (Story 83.1 / Coverage 83-1).
//!
//! # What is tested
//!
//! 1. **Token-exchange endpoint** — `POST /organizations/{org_id}/airbnb/token/exchange`
//!    - Rejects unauthenticated requests (401/403).
//!    - Rejects an empty authorization code (400).
//!    - IDOR guard: a caller who is NOT a member of the target org is rejected
//!      with 403 rather than forwarding the code to Airbnb.
//!
//! 2. **Listings endpoint** — `GET /organizations/{org_id}/airbnb/listings`
//!    - Rejects unauthenticated requests.
//!    - IDOR guard: non-member caller is rejected with 403.
//!    - Returns 404 when no Airbnb connection exists for the org.
//!
//! 3. **Reservations endpoint** — `GET /organizations/{org_id}/airbnb/reservations`
//!    - Rejects unauthenticated requests.
//!    - IDOR guard: non-member caller is rejected with 403.
//!    - Returns 404 when no Airbnb connection exists for the org.
//!
//! # Scope
//!
//! These tests are deliberately limited to auth/IDOR/shape validation: they do
//! not make live calls to the Airbnb API (no `AIRBNB_CLIENT_ID` is set in CI)
//! and do not require a real OAuth token to be stored.  The "not configured"
//! branch (503) and the "not found connection" branch (404) are both exercised
//! without external network access.
//!
//! Live end-to-end token exchange is covered by the manual QA checklist in
//! `docs/api/README.md#airbnb-oauth-flow`.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp};

// Must match `TestConfig::default().jwt_secret`.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Mirror of `api_core::extractors::auth::Claims`.
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

fn mint_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some("manager".to_string()),
        email: "airbnb-routes-test@test.local".to_string(),
        name: "Airbnb Routes Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

// ---------------------------------------------------------------------------
// Database fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Airbnb Routes Test Org {tag}"))
    .bind(format!(
        "airbnb-routes-test-{tag}-{}",
        Uuid::new_v4().simple()
    ))
    .bind(format!("{tag}@airbnb-routes.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Airbnb Test User', 'active', NOW())
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

/// Seed a minimal Airbnb connection so the listings/reservations handlers have
/// something to load (they will fail at the Airbnb API call, not at "no
/// connection found").
///
/// `token_expires_at` — pass `None` for a token with no expiry, or `Some(ts)`
/// to test the proactive-refresh path (set `ts` in the past or within the
/// 5-minute buffer window).  The access token is stored as plaintext (no
/// `enc:` prefix) so `decrypt_if_available` can return it without a live
/// encryption key.
async fn seed_airbnb_connection(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    token_expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (
            organization_id, unit_id, platform,
            access_token, token_expires_at, is_active
        )
        VALUES ($1, $2, 'airbnb', 'test-plaintext-token', $3, true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(token_expires_at)
    .fetch_one(pool)
    .await
    .expect("seed airbnb connection")
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

fn authed_get(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn authed_post_with_tenant(
    uri: &str,
    token: &str,
    tenant_id: Uuid,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Tenant-ID", tenant_id.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn anon_get(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn anon_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Helper: protected = 4xx in the auth/validation range
// ---------------------------------------------------------------------------

fn is_protected(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
    )
}

// ===========================================================================
// Token-exchange endpoint tests
// ===========================================================================

/// Unauthenticated POST to the token-exchange endpoint must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "te-unauth").await;
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/token/exchange");
    let resp = app
        .execute(anon_post(&uri, json!({"code": "abc123"})))
        .await;
    assert!(
        is_protected(resp.status),
        "unauthenticated token-exchange must be rejected; got {}",
        resp.status
    );
}

/// An empty authorization code must be rejected with 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_rejects_empty_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "te-empty").await;
    let user_id = seed_user(&pool, "te-empty@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/token/exchange");
    let resp = app
        .execute(authed_post_with_tenant(
            &uri,
            &token,
            org_id,
            json!({"code": ""}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "empty code must return 400; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.text();
    assert!(
        body.contains("MISSING_CODE") || body.contains("code"),
        "error body should mention missing code: {body}"
    );
}

/// A caller who is NOT a member of the target organisation must be rejected
/// with 403 — the IDOR guard must fire before any Airbnb API call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_idor_guard_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "te-idor-a").await; // owns the target
    let org_b = seed_org(&pool, "te-idor-b").await; // caller's org
    let user_b = seed_user(&pool, "te-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await; // member of B, not A

    let token_b = mint_token(user_b, org_b);
    let uri = format!("/api/v1/integrations/organizations/{org_a}/airbnb/token/exchange");
    let resp = app
        .execute(authed_post_with_tenant(
            &uri,
            &token_b,
            org_b,
            json!({"code": "some-code"}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member token exchange must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// #1585: the manager gate must read the caller's role for the PATH org, not
/// trust the JWT role / X-Tenant-ID. A user who is a manager in org A but only a
/// plain member of org B must NOT be able to bind org B's Airbnb integration.
/// Mirrors `token_exchange_rejects_manager_of_a_different_org` in the booking
/// suite — the gate is shared (`verify_manager_role_in_org`), but this pins that
/// the airbnb handler actually invokes it before exchanging.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_rejects_manager_of_a_different_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "te-mgr-desync-a").await;
    let org_b = seed_org(&pool, "te-mgr-desync-b").await;
    let user_id = seed_user(&pool, "te-mgr-desync@test.local").await;
    seed_membership(&pool, org_a, user_id, "manager").await; // manager in A
    seed_membership(&pool, org_b, user_id, "tenant").await; //  plain member in B

    // JWT carries role=manager + tenant_id = A (X-Tenant-ID = A), but the path
    // org being mutated is B, where the caller is only a plain member.
    let token = mint_token(user_id, org_a);
    let uri = format!("/api/v1/integrations/organizations/{org_b}/airbnb/token/exchange");
    let resp = app
        .execute(authed_post_with_tenant(
            &uri,
            &token,
            org_a, // X-Tenant-ID = A (where the caller is a manager)
            json!({"code": "valid-looking-code"}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "manager of org A must not bind org B's Airbnb integration as a plain member; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// When Airbnb credentials are not configured (empty `AIRBNB_CLIENT_ID`), the
/// endpoint must return 503 SERVICE_UNAVAILABLE rather than forwarding a bad
/// request to Airbnb.  This test relies on the fact that no `AIRBNB_CLIENT_ID`
/// env var is set in test runs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_returns_503_when_not_configured(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "te-nocfg").await;
    let user_id = seed_user(&pool, "te-nocfg@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/token/exchange");
    // Airbnb is not configured in the test environment (AIRBNB_CLIENT_ID is
    // empty/unset), so we expect 503.
    let resp = app
        .execute(authed_post_with_tenant(
            &uri,
            &token,
            org_id,
            json!({"code": "abc123"}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "unconfigured Airbnb must return 503; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// Listings endpoint tests
// ===========================================================================

/// Unauthenticated GET to the listings endpoint must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn listings_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ls-unauth").await;
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/listings");
    let resp = app.execute(anon_get(&uri)).await;
    assert!(
        is_protected(resp.status),
        "unauthenticated listings request must be rejected; got {}",
        resp.status
    );
}

/// Non-member caller must be rejected with 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn listings_idor_guard_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "ls-idor-a").await;
    let org_b = seed_org(&pool, "ls-idor-b").await;
    let user_b = seed_user(&pool, "ls-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let token_b = mint_token(user_b, org_b);
    let uri = format!("/api/v1/integrations/organizations/{org_a}/airbnb/listings");
    let resp = app.execute(authed_get(&uri, &token_b)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member listings request must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Manager gate (#1626): a member whose org role is `resident` must be rejected
/// with 403 even though they pass the membership IDOR check — live Airbnb
/// listings are manager-level operational data. The role is read from
/// `organization_members.role_type` for the PATH org, not the JWT (#1525/#1585):
/// the resident is rejected despite `mint_token` minting a `manager` claim, and
/// the 403 short-circuits before any Airbnb connection lookup or external call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn listings_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ls-resident").await;
    let user_id = seed_user(&pool, "ls-resident@test.local").await;
    seed_membership(&pool, org_id, user_id, "resident").await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/listings");
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident member must be 403 (manager-level read); got {}: {}",
        resp.status,
        resp.text()
    );
}

/// When no Airbnb connection exists for an org, the listings endpoint returns 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn listings_returns_404_when_no_connection(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "ls-noconn").await;
    let user_id = seed_user(&pool, "ls-noconn@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/listings");
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "missing connection must return 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// Reservations endpoint tests
// ===========================================================================

/// Unauthenticated GET to the reservations endpoint must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reservations_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "res-unauth").await;
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/reservations");
    let resp = app.execute(anon_get(&uri)).await;
    assert!(
        is_protected(resp.status),
        "unauthenticated reservations request must be rejected; got {}",
        resp.status
    );
}

/// Non-member caller must be rejected with 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reservations_idor_guard_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "res-idor-a").await;
    let org_b = seed_org(&pool, "res-idor-b").await;
    let user_b = seed_user(&pool, "res-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let token_b = mint_token(user_b, org_b);
    let uri = format!("/api/v1/integrations/organizations/{org_a}/airbnb/reservations");
    let resp = app.execute(authed_get(&uri, &token_b)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member reservations request must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Manager gate (#1667, follow-up to #1626/#1635): a member whose org role is
/// `resident` must be rejected with 403 even though they pass the membership
/// IDOR check — live Airbnb reservation data carries guest PII (names,
/// check-in/check-out dates, booking/listing IDs) and is manager-level
/// operational data, for parity with `/airbnb/listings`. The role is read from
/// `organization_members.role_type` for the PATH org, not the JWT (#1525/#1585):
/// the resident is rejected despite `mint_token` minting a `manager` claim, and
/// the 403 short-circuits before any Airbnb connection lookup or external call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reservations_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "res-resident").await;
    let user_id = seed_user(&pool, "res-resident@test.local").await;
    seed_membership(&pool, org_id, user_id, "resident").await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/reservations");
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident member must be 403 (manager-level read); got {}: {}",
        resp.status,
        resp.text()
    );
}

/// When no Airbnb connection exists for an org, the reservations endpoint
/// returns 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reservations_returns_404_when_no_connection(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "res-noconn").await;
    let user_id = seed_user(&pool, "res-noconn@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/reservations");
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "missing connection must return 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// listing_id query parameter is accepted — route parses it without error.
/// The request will fail at the Airbnb API call (no valid token), but the
/// route itself must not panic or return a parse error.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reservations_listing_id_filter_is_parsed(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "res-filter").await;
    let user_id = seed_user(&pool, "res-filter@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    // No connection — we expect 404, NOT a 400/500 from query parsing.
    let uri = format!(
        "/api/v1/integrations/organizations/{org_id}/airbnb/reservations?listing_id=listing-abc"
    );
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "listing_id filter with no connection must return 404 (not a parse error); got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// Route-existence smoke tests (unauthenticated → must not be 404/405)
// ===========================================================================
//
// These guard that the three new routes are actually mounted — if the router
// wiring is broken the endpoints return 404 or 405 instead of a 4xx auth
// error.  We accept any 4xx that is NOT 404/405.

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn new_routes_are_mounted(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = Uuid::new_v4(); // arbitrary — we just want 401/403, not 404

    let routes: &[(&str, Method)] = &[
        (
            &format!("/api/v1/integrations/organizations/{org_id}/airbnb/token/exchange"),
            Method::POST,
        ),
        (
            &format!("/api/v1/integrations/organizations/{org_id}/airbnb/listings"),
            Method::GET,
        ),
        (
            &format!("/api/v1/integrations/organizations/{org_id}/airbnb/reservations"),
            Method::GET,
        ),
    ];

    for (uri, method) in routes {
        let req = Request::builder()
            .method(method.clone())
            .uri(*uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(if *method == Method::POST {
                Body::from(r#"{"code":"x"}"#)
            } else {
                Body::empty()
            })
            .unwrap();

        let resp = app.execute(req).await;
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
// Token-refresh path coverage
// ===========================================================================

/// Verify that the `with_token_refresh` wrapper is exercised for the listings
/// route when an Airbnb connection exists with an already-expired token.
///
/// # What this proves
///
/// 1. The route does NOT return 404 (no-connection guard) — the connection is
///    found in the DB.
/// 2. The route reaches the `with_token_refresh` code path: the token is
///    decrypted, the proactive-refresh check fires (token is expired), a
///    refresh is attempted, and — because no real Airbnb credentials are
///    configured in CI — the Airbnb API call ultimately fails.
/// 3. The failure surfaces as 502 BAD_GATEWAY (`CallFailed` or
///    `RefreshFailed`), confirming the refresh wrapper was invoked rather than
///    the handler short-circuiting at the connection-lookup step.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn listings_with_token_refresh_wrapper_invoked_on_expired_token(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Seed org + member user.
    let org_id = seed_org(&pool, "tr-listings").await;
    let user_id = seed_user(&pool, "tr-listings@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    // Seed a building and unit — required FK for rental_platform_connections.
    let building_id = seed_building(&pool, org_id, "Token Refresh Test").await;
    let unit_id = seed_unit(&pool, building_id, "101").await;

    // Seed an Airbnb connection whose token expired 1 hour ago.
    // Storing a plaintext token (no `enc:` prefix) ensures `decrypt_if_available`
    // returns it as-is without needing a live encryption key.
    let expired_at = Utc::now() - Duration::hours(1);
    seed_airbnb_connection(&pool, org_id, unit_id, Some(expired_at)).await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/listings");
    let resp = app.execute(authed_get(&uri, &token)).await;

    // The response must NOT be 404 (connection was found) and NOT be 500
    // (decryption succeeded on the plaintext token).  The expected outcome is
    // 502 because the Airbnb API call (or refresh attempt) fails without real
    // credentials — this is the `CallFailed` / `RefreshFailed` branch of
    // `with_token_refresh`.
    assert_ne!(
        resp.status,
        StatusCode::NOT_FOUND,
        "with_token_refresh must not short-circuit at no-connection; got {}: {}",
        resp.status,
        resp.text()
    );
    assert_ne!(
        resp.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "plaintext token must decrypt without error; got {}: {}",
        resp.status,
        resp.text()
    );
    assert_eq!(
        resp.status,
        StatusCode::BAD_GATEWAY,
        "expired token path must reach Airbnb API call and return 502; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// Manager-role gate — BIT-85
// ===========================================================================

/// A non-manager org member (role = "tenant" in JWT) must be rejected with 403
/// when calling the Airbnb token-exchange endpoint.
/// Binding an org-wide OTA integration is a manager-level action.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_token_exchange_rejects_non_manager_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "mgr-gate-a").await;
    let user_id = seed_user(&pool, "non-manager-a@airbnb-routes.test").await;
    seed_membership(&pool, org_id, user_id, "tenant").await;

    // Mint a token with a non-manager role. Membership is valid but
    // verify_manager_role must still reject with 403.
    let now = chrono::Utc::now();
    #[derive(serde::Serialize)]
    struct Claims {
        sub: Uuid,
        exp: i64,
        iat: i64,
        token_type: String,
        tenant_id: Option<Uuid>,
        role: Option<String>,
        email: String,
        name: String,
    }
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("tenant".to_string()),
        email: "non-manager-a@airbnb-routes.test".to_string(),
        name: "Non-Manager Test".to_string(),
    };
    let non_manager_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint non-manager token");

    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/token/exchange");
    let resp = app
        .execute(authed_post_with_tenant(
            &uri,
            &non_manager_token,
            org_id,
            json!({"code": "valid-code"}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-manager member must be rejected with 403 on airbnb token exchange; got {}: {}",
        resp.status,
        resp.text()
    );
}

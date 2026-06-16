//! Handler-level + CSRF/secure-credential coverage for the Booking.com OAuth
//! token-exchange route (Coverage 83-2; follow-ups #1362 and #1374).
//!
//! # Why this file exists
//!
//! PR #1326 shipped `POST /organizations/{org_id}/booking/token/exchange`
//! (`booking_token_exchange`) as a faithful mirror of the Airbnb back-channel
//! token-exchange (Story 83.1), but its only tests were serde round-trips of
//! the request/response wire contract (`oauth.rs` `mod tests`). Issue #1374
//! flagged that the *behavioral* guards — the exact things the route exists to
//! enforce — were untested:
//!
//! - IDOR 403 for a caller who is not a member of the target org;
//! - `NOT_CONFIGURED` 503 when `BOOKING_CLIENT_ID` is empty (fail-closed);
//! - `MISSING_CODE` 400 for an empty authorization code;
//! - unauthenticated requests rejected before any provider call;
//! - the route is actually mounted (not 404/405).
//!
//! Issue #1362 separately flagged that PR #1285 removed the legacy
//! unauthenticated `/api/v1/rentals/oauth/{airbnb,booking}/callback` handlers
//! (which trusted a client-supplied connection id from the OAuth `state` with
//! no auth / org / CSRF-state check) and called out only the *Airbnb* secure
//! replacement. The Booking.com secure replacement is this back-channel
//! token-exchange endpoint. The `legacy_booking_callback_is_gone` regression
//! test below pins that the insecure CSRF-vulnerable callback stays removed,
//! and that the secure org-scoped endpoint is the only Booking connect surface.
//!
//! # Scope / known limitation
//!
//! These tests drive the real router via `TestApp` against an isolated test
//! database (the established convention in `airbnb_oauth_routes_tests.rs`).
//! They deliberately do NOT exercise the live-token-endpoint happy path
//! (200 + ciphertext-at-rest assertion requested in #1374), because
//! `BookingOAuthClient::exchange_code` posts to the compile-time constant
//! `integrations::BOOKING_OAUTH_TOKEN_URL`, which cannot be redirected to a
//! `wiremock` server without a production change to make the token URL
//! injectable. That refactor is a `pm-integration`/`rust-backend` change
//! (out of scope for this QA-coverage task) — tracked in #1374's "Better
//! approach" finding (shared `oauth_common` helper). Every guard reachable
//! without a live provider call is covered here.

#[allow(dead_code)]
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

use common::TestApp;

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
        email: "booking-routes-test@test.local".to_string(),
        name: "Booking Routes Test".to_string(),
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
    .bind(format!("Booking Routes Test Org {tag}"))
    .bind(format!(
        "booking-routes-test-{tag}-{}",
        Uuid::new_v4().simple()
    ))
    .bind(format!("{tag}@booking-routes.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Booking Test User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO organization_members
            (id, organization_id, user_id, role_type, status, created_at)
        VALUES ($1, $2, $3, 'manager', 'active', NOW())
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

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

fn anon_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// `protected` = a 4xx in the auth/validation range (not 404/405).
fn is_protected(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
    )
}

fn token_exchange_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking/token/exchange")
}

// ===========================================================================
// Token-exchange endpoint — behavioral guards (#1374)
// ===========================================================================

/// Unauthenticated POST to the token-exchange endpoint must be rejected before
/// any Booking.com provider call is made.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "te-unauth").await;
    let resp = app
        .execute(anon_post(
            &token_exchange_uri(org_id),
            json!({"code": "abc123"}),
        ))
        .await;
    assert!(
        is_protected(resp.status),
        "unauthenticated Booking token-exchange must be rejected; got {}",
        resp.status
    );
}

/// An empty authorization code must be rejected with 400 `MISSING_CODE` before
/// the IDOR check or any provider call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_rejects_empty_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "te-empty").await;
    let user_id = seed_user(&pool, "te-empty@booking-routes.test").await;
    seed_membership(&pool, org_id, user_id).await;
    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_post_with_tenant(
            &token_exchange_uri(org_id),
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

/// IDOR guard: a caller who is NOT a member of the target organisation must be
/// rejected with 403 — `verify_org_access` must fire before the token exchange,
/// so the forged `org_id` never reaches Booking.com or the token store.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_idor_guard_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "te-idor-a").await; // owns the target
    let org_b = seed_org(&pool, "te-idor-b").await; // caller's org
    let user_b = seed_user(&pool, "te-idor-b@booking-routes.test").await;
    seed_membership(&pool, org_b, user_b).await; // member of B, not A

    let token_b = mint_token(user_b, org_b);
    let resp = app
        .execute(authed_post_with_tenant(
            &token_exchange_uri(org_a),
            &token_b,
            org_b,
            json!({"code": "some-code"}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member Booking token exchange must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Fail-closed: when Booking.com credentials are not configured (empty
/// `BOOKING_CLIENT_ID`, the default in test/CI), a *member* caller with a valid
/// code must get 503 `NOT_CONFIGURED` rather than the endpoint attempting a
/// token exchange against an unconfigured client. This proves the
/// not-configured branch is reached only AFTER the IDOR check passes.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_returns_503_when_not_configured(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "te-nocfg").await;
    let user_id = seed_user(&pool, "te-nocfg@booking-routes.test").await;
    seed_membership(&pool, org_id, user_id).await;
    let token = mint_token(user_id, org_id);
    let resp = app
        .execute(authed_post_with_tenant(
            &token_exchange_uri(org_id),
            &token,
            org_id,
            json!({"code": "abc123"}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "unconfigured Booking.com must return 503; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("NOT_CONFIGURED"),
        "503 body should carry NOT_CONFIGURED: {}",
        resp.text()
    );
}

/// Route-existence smoke: the token-exchange route must be mounted. A broken
/// router would surface as 404 (not mounted) or 405 (wrong method) instead of
/// an auth/validation 4xx.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_route_is_mounted(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = Uuid::new_v4(); // arbitrary — we want 401/403, not 404
    let req = Request::builder()
        .method(Method::POST)
        .uri(token_exchange_uri(org_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"code":"x"}"#))
        .unwrap();
    let resp = app.execute(req).await;
    assert_ne!(
        resp.status,
        StatusCode::NOT_FOUND,
        "booking/token/exchange must be mounted (got 404 — router wiring broken)"
    );
    assert_ne!(
        resp.status,
        StatusCode::METHOD_NOT_ALLOWED,
        "booking/token/exchange must accept POST (got 405)"
    );
}

// ===========================================================================
// CSRF / secure-credential-replacement regression (#1362)
// ===========================================================================

/// Regression guard for #1362 / PAP-141 (#1285): the legacy *unauthenticated*
/// Booking.com OAuth callback `GET|POST /api/v1/rentals/oauth/booking/callback`
/// — which trusted a client-supplied connection id from the OAuth `state` with
/// no auth, no org check, and no CSRF-state validation — must stay removed.
///
/// Hitting it must return 404 (route gone), NOT a 2xx/4xx that would imply the
/// CSRF-vulnerable handler is still wired up. Both GET and POST are checked so
/// a re-introduction under either method is caught.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legacy_booking_callback_is_gone(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    for method in [Method::GET, Method::POST] {
        let req = Request::builder()
            .method(method.clone())
            .uri("/api/v1/rentals/oauth/booking/callback?code=x&state=forged-connection-id")
            .body(Body::empty())
            .unwrap();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "legacy unauthenticated booking callback ({method}) must be removed (404); \
             got {} — CSRF-vulnerable handler may have been re-introduced",
            resp.status
        );
    }
}

/// The secure replacement for the removed legacy callback is the org-scoped,
/// authenticated back-channel token-exchange. This pins that the secure
/// endpoint refuses an *anonymous* caller (defence-in-depth: it does not fall
/// back to the legacy unauthenticated behaviour). Combined with
/// `token_exchange_idor_guard_rejects_non_member`, this documents that the
/// only Booking.com connect surface is auth + org-scoped.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn secure_booking_connect_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "secure-anon").await;
    let resp = app
        .execute(anon_post(
            &token_exchange_uri(org_id),
            json!({"code": "abc123"}),
        ))
        .await;
    assert!(
        is_protected(resp.status),
        "the secure Booking connect endpoint must reject anonymous callers; got {}",
        resp.status
    );
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "anonymous Booking connect must never succeed (legacy unauthenticated behaviour)"
    );
}

// ===========================================================================
// Manager-role gate — BIT-85
// ===========================================================================

/// A non-manager org member (role = "tenant" in JWT) must be rejected with 403
/// when calling the Booking.com token-exchange endpoint.
/// Binding an org-wide OTA integration is a manager-level action.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_exchange_rejects_non_manager_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "mgr-gate-b").await;
    let user_id = seed_user(&pool, "non-manager-b@booking-routes.test").await;
    seed_membership(&pool, org_id, user_id).await;

    // Mint a token with a non-manager role (tenant). Org membership is valid,
    // but verify_manager_role must fire and return 403.
    let now = chrono::Utc::now();
    use chrono::Duration;
    use jsonwebtoken::{encode, EncodingKey, Header};
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
        email: "non-manager-b@booking-routes.test".to_string(),
        name: "Non-Manager Test".to_string(),
    };
    let non_manager_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint non-manager token");

    let resp = app
        .execute(authed_post_with_tenant(
            &token_exchange_uri(org_id),
            &non_manager_token,
            org_id,
            json!({"code": "valid-looking-code"}),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-manager member must be rejected with 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

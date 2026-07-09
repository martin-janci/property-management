//! Integration tests for the Booking.com OAuth token-exchange handler and the
//! Airbnb OAuth **CSRF-state callback** (GH #1374).
//!
//! # What is tested
//!
//! 1. **Booking.com token-exchange handler** —
//!    `POST /api/v1/integrations/organizations/{org_id}/booking/token/exchange`
//!    (previously had no handler-level coverage):
//!    - Rejects unauthenticated requests (401/403).
//!    - Rejects an empty authorization code (400 `MISSING_CODE`).
//!    - IDOR guard: a caller who is NOT a member of the target org is rejected
//!      with 403 *before* any Booking.com API call.
//!    - Fails closed with 503 when Booking.com is not configured.
//!
//! 2. **Airbnb OAuth CSRF-state callback** —
//!    `GET /api/v1/integrations/organizations/{org_id}/airbnb/callback`:
//!    the OAuth `state` parameter is a CSRF token, so the callback must enforce
//!    it. We assert the three security-relevant `state` cases —
//!    - **missing** state → 400 `INVALID_STATE`,
//!    - **invalid** state (malformed / org mismatch) → 400,
//!    - **valid** state → clears the CSRF gate and proceeds (surfacing 503 when
//!      Airbnb is not configured in test) —
//!      plus the IDOR guard that runs after the state gate.
//!
//! # Parity across the OAuth providers
//!
//! Both providers expose a back-channel/redirect entry point that must enforce
//! org membership: the Booking.com token-exchange (`token_exchange_*` here) and
//! the Airbnb callback (`airbnb_callback_idor_*`). The Airbnb back-channel
//! token-exchange variant has equivalent coverage in
//! `airbnb_oauth_routes_tests.rs`. The single-use + org-binding heart of the
//! `state` CSRF check is unit-tested in `routes::integrations::oauth_state`
//! (`decide_consume_*`).
//!
//! # Scope
//!
//! No live Airbnb/Booking.com calls (`AIRBNB_CLIENT_ID` / `BOOKING_CLIENT_ID`
//! are unset in CI) and no Redis is wired into `TestApp`, so the state store is
//! `StoreUnavailable` and the callback relies on the stateless org-prefix +
//! membership checks — exactly the path these tests pin.

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
        email: "booking-csrf-test@test.local".to_string(),
        name: "Booking CSRF Test".to_string(),
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
    .bind(format!("Booking CSRF Test Org {tag}"))
    .bind(format!(
        "booking-csrf-test-{tag}-{}",
        Uuid::new_v4().simple()
    ))
    .bind(format!("{tag}@booking-csrf.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Booking CSRF User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
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

fn anon_post(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// "Protected" = a 4xx in the auth/validation range (not 404/405/5xx).
fn is_protected(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
    )
}

fn booking_exchange_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking/token/exchange")
}

fn airbnb_callback_uri(org_id: Uuid, code: &str, state: &str) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/airbnb/callback?code={code}&state={state}")
}

// ===========================================================================
// Booking.com token-exchange handler tests
// ===========================================================================

/// Unauthenticated POST to the Booking.com token-exchange endpoint is rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_token_exchange_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "bk-unauth").await;
    let resp = app
        .execute(anon_post(
            &booking_exchange_uri(org_id),
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
/// any Booking.com API call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_token_exchange_rejects_empty_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "bk-empty").await;
    let user_id = seed_user(&pool, "bk-empty@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .execute(authed_post_with_tenant(
            &booking_exchange_uri(org_id),
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
        "error body should mention the missing code: {body}"
    );
}

/// IDOR guard: a caller who is NOT a member of the target org is rejected with
/// 403 — the membership check must fire before the code is forwarded.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_token_exchange_idor_guard_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "bk-idor-a").await; // owns the target
    let org_b = seed_org(&pool, "bk-idor-b").await; // caller's org
    let user_b = seed_user(&pool, "bk-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await; // member of B, not A
    let token_b = mint_token(user_b, org_b);

    let resp = app
        .execute(authed_post_with_tenant(
            &booking_exchange_uri(org_a),
            &token_b,
            org_b,
            json!({"code": "some-code"}),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member Booking token-exchange must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// When Booking.com credentials are not configured (empty `BOOKING_CLIENT_ID`),
/// the handler fails closed with 503 rather than forwarding a bad request.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_token_exchange_returns_503_when_not_configured(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "bk-nocfg").await;
    let user_id = seed_user(&pool, "bk-nocfg@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .execute(authed_post_with_tenant(
            &booking_exchange_uri(org_id),
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
}

/// Manager-role gate: a caller who *is* a member of the org but only holds a
/// `resident` role must still be rejected with 403. Binding a paid OTA
/// integration is a manager-level action, so `verify_org_access` (membership)
/// passing is necessary but not sufficient — `verify_manager_role_in_org` runs
/// after it and must fail closed for a non-manager member.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_token_exchange_rejects_non_manager_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "bk-resident").await;
    let user_id = seed_user(&pool, "bk-resident@test.local").await;
    // Member of the org, but only a `resident` — not a manager tier.
    seed_membership(&pool, org_id, user_id, "resident").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .execute(authed_post_with_tenant(
            &booking_exchange_uri(org_id),
            &token,
            org_id,
            json!({"code": "some-code"}),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident member Booking token-exchange must be 403 (manager gate); got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// Airbnb OAuth CSRF-state callback tests
// ===========================================================================

/// Unauthenticated GET to the Airbnb callback is rejected before any state
/// processing. Even a well-formed `state` must not let an anonymous caller
/// drive a token exchange — the `AuthUser` extractor gates the handler.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-anon").await;

    let well_formed_state = format!("{org_id}:{}", Uuid::new_v4());
    let req = Request::builder()
        .method(Method::GET)
        .uri(airbnb_callback_uri(org_id, "auth-code", &well_formed_state))
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;

    assert!(
        is_protected(resp.status),
        "unauthenticated Airbnb callback must be rejected; got {}",
        resp.status
    );
}

/// Missing (empty) `state` → 400 `INVALID_STATE`. An OAuth callback with no
/// CSRF token must never proceed to a token exchange.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_rejects_missing_state(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-missing").await;
    let user_id = seed_user(&pool, "cb-missing@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", ""),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "missing state must return 400; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("INVALID_STATE"),
        "missing state error must be INVALID_STATE: {}",
        resp.text()
    );
}

/// Malformed `state` (no `{org}:{nonce}` shape) → 400. A forged value that does
/// not even parse must be rejected before the org check.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_rejects_malformed_state(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-malformed").await;
    let user_id = seed_user(&pool, "cb-malformed@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    // No colon → a single segment → fails the `{org}:{nonce}` format check.
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", "not-a-valid-state"),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "malformed state must return 400; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("INVALID_STATE"),
        "malformed state error must be INVALID_STATE: {}",
        resp.text()
    );
}

/// Invalid `state` bound to a *different* org → 400 `STATE_MISMATCH`. This is
/// the cross-org CSRF/IDOR case: the state's embedded org must equal the path.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_rejects_org_mismatch_state(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-mismatch").await;
    let user_id = seed_user(&pool, "cb-mismatch@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    // state embeds a *different* org than the callback path.
    let other_org = Uuid::new_v4();
    let forged_state = format!("{other_org}:{}", Uuid::new_v4());
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &forged_state),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "org-mismatch state must return 400; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("STATE_MISMATCH"),
        "org-mismatch state error must be STATE_MISMATCH: {}",
        resp.text()
    );
}

/// `state` whose first segment is present (so the `{a}:{b}` format check passes)
/// but is NOT a parseable UUID → 400 `STATE_MISMATCH`. This exercises the
/// `Uuid::parse_str(...).ok() == None` branch, distinct from the valid-but-
/// different-org UUID case above: a non-UUID prefix can never equal the path
/// org, so the callback must reject it as a mismatch rather than 500.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_rejects_non_uuid_org_prefix(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-nonuuid").await;
    let user_id = seed_user(&pool, "cb-nonuuid@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    // Two segments (passes the `< 2` format check) but the first is not a UUID.
    let bad_prefix_state = format!("not-a-uuid:{}", Uuid::new_v4());
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &bad_prefix_state),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "non-UUID state org prefix must return 400; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("STATE_MISMATCH"),
        "non-UUID org prefix must be rejected as STATE_MISMATCH: {}",
        resp.text()
    );
}

/// A `state` with extra trailing `:`-segments (`{path_org}:{nonce}:extra`) still
/// clears the stateless org-prefix gate: `split(':')` yields ≥2 parts and
/// `parts[0]` equals the path org, so the callback proceeds past the CSRF gate
/// to the (unconfigured) token exchange → 503. Pins the `split(':')` len≥2
/// contract so a future stricter parse doesn't silently change acceptance.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_accepts_state_with_extra_segments(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-extra").await;
    let user_id = seed_user(&pool, "cb-extra@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let state = format!("{org_id}:{}:extra-segment", Uuid::new_v4());
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &state),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "state with extra segments but a matching org prefix must clear the gate \
         and reach the unconfigured token exchange → 503; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.text();
    assert!(
        !body.contains("INVALID_STATE") && !body.contains("STATE_MISMATCH"),
        "extra-segment state with a matching org prefix must not be CSRF-rejected: {body}"
    );
}

/// Valid `state` (correct `{path_org}:{nonce}` shape) clears the CSRF gate. With
/// no Redis the single-use store is `StoreUnavailable`, so the flow falls
/// through to the membership check and then to "Airbnb not configured" → 503.
/// A 503 here proves the state gate *accepted* the value (it is NOT a 400
/// `INVALID_STATE`/`STATE_MISMATCH` rejection and NOT a 403).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_valid_state_passes_csrf_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-valid").await;
    let user_id = seed_user(&pool, "cb-valid@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    let valid_state = format!("{org_id}:{}", Uuid::new_v4());
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &valid_state),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "a valid state must clear the CSRF gate and reach the (unconfigured) \
         token exchange → 503, not a state rejection; got {}: {}",
        resp.status,
        resp.text()
    );
    let body = resp.text();
    assert!(
        !body.contains("INVALID_STATE") && !body.contains("STATE_MISMATCH"),
        "a valid state must not be rejected by the CSRF gate: {body}"
    );
}

/// Parity with the Booking.com IDOR guard: even with a well-formed, org-matching
/// `state`, a caller who is NOT a member of the org is rejected with 403. The
/// state gate is necessary but not sufficient — membership is still enforced.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_idor_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "cb-idor-a").await; // callback target
    let org_b = seed_org(&pool, "cb-idor-b").await; // caller's org
    let user_b = seed_user(&pool, "cb-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await; // member of B, not A
    let token_b = mint_token(user_b, org_b);

    // state correctly embeds org_a (passes the stateless org-prefix check), so
    // the request reaches `verify_org_access`, which must reject the non-member.
    let state = format!("{org_a}:{}", Uuid::new_v4());
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_a, "auth-code", &state),
            &token_b,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member Airbnb callback must be 403 after the state gate; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Membership-gate boundary: unlike the token-exchange POST (which additionally
/// requires a manager role), the browser callback gates only on `verify_org_access`
/// (membership). A `resident` member therefore clears the state gate AND the
/// membership check and reaches the unconfigured token exchange → 503 (NOT 403).
/// Pins this deliberate asymmetry so a future gate change is caught.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_resident_member_passes_membership_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-resident").await;
    let user_id = seed_user(&pool, "cb-resident@test.local").await;
    // Only a `resident` — still a member, and the callback has no manager gate.
    seed_membership(&pool, org_id, user_id, "resident").await;
    let token = mint_token(user_id, org_id);

    let state = format!("{org_id}:{}", Uuid::new_v4());
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &state),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "resident member must clear the membership-only callback gate → 503, \
         not 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// Route-existence smoke test
// ===========================================================================

/// Guard that both surfaces are mounted — a broken router would surface as
/// 404/405 instead of an auth/validation 4xx.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oauth_routes_are_mounted(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = Uuid::new_v4();

    let booking = app
        .execute(anon_post(
            &booking_exchange_uri(org_id),
            json!({"code": "x"}),
        ))
        .await;
    assert_ne!(
        booking.status,
        StatusCode::NOT_FOUND,
        "booking token-exchange route must be mounted (got 404)"
    );
    assert_ne!(
        booking.status,
        StatusCode::METHOD_NOT_ALLOWED,
        "booking token-exchange route must accept POST (got 405)"
    );

    let callback = Request::builder()
        .method(Method::GET)
        .uri(airbnb_callback_uri(org_id, "x", "y"))
        .body(Body::empty())
        .unwrap();
    let callback = app.execute(callback).await;
    assert_ne!(
        callback.status,
        StatusCode::NOT_FOUND,
        "airbnb callback route must be mounted (got 404)"
    );
    assert_ne!(
        callback.status,
        StatusCode::METHOD_NOT_ALLOWED,
        "airbnb callback route must accept GET (got 405)"
    );
}

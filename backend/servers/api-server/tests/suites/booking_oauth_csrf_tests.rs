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
//! `airbnb_oauth_routes_tests.rs`.
//!
//! # Single-use / replay coverage (issue #2203)
//!
//! Most callback tests reach their expected status via the *stateless* fallback
//! (`ConsumeOutcome::StoreUnavailable`, because `TestApp` wires no Redis). To
//! also pin the stateful heart of the CSRF check — the single-use consume path
//! and its `Rejected → 400 INVALID_STATE` branch — `airbnb_callback_rejects_replayed_state`
//! and `airbnb_callback_rejects_unissued_state_when_store_active` install an
//! in-memory `OAuthStateStore` (via `TestApp::with_oauth_state_store`) whose
//! consume semantics mirror the Redis path. The pure decision (`decide_consume`)
//! and the store's single-use behaviour are additionally unit-tested in
//! `routes::integrations::oauth_state`.
//!
//! # Scope
//!
//! No live Airbnb/Booking.com calls (`AIRBNB_CLIENT_ID` / `BOOKING_CLIENT_ID`
//! are unset in CI). Tests that don't install a store rely on the stateless
//! org-prefix + membership checks (the `StoreUnavailable` fallback); the two
//! replay tests install the in-memory store to drive the real consume path.

#![allow(dead_code)]

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

use crate::common::{seed_membership, TestApp};

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

/// "Auth-rejected" = the request was stopped by the authentication layer
/// (`AuthUser` extractor) specifically — `401 UNAUTHORIZED` or `403 FORBIDDEN`.
///
/// Issue #2203 (test-quality): deliberately excludes `400 BAD_REQUEST`. The
/// earlier `is_protected` helper accepted 400 too, so an unauthenticated test
/// would still pass if the auth extractor were removed and the request instead
/// failed later on state/validation with a 400. Narrowing to 401/403 means
/// these tests can only pass when auth actually fired.
fn is_auth_rejected(status: StatusCode) -> bool {
    matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
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
        is_auth_rejected(resp.status),
        "unauthenticated Booking token-exchange must be rejected by auth (401/403); got {}",
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
        is_auth_rejected(resp.status),
        "unauthenticated Airbnb callback must be rejected by auth (401/403); got {}",
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

/// **Replay / single-use rejection at the handler level (issue #2203).**
///
/// The previous tests all reach their expected status via the *stateless*
/// fallback (`ConsumeOutcome::StoreUnavailable`, because `TestApp` wires no
/// Redis) — none exercises the stateful consume path. This test installs an
/// in-memory `OAuthStateStore` (same single-use semantics as the Redis path)
/// and drives the real `verify_and_consume` flow:
///
/// 1. A freshly-issued state is seeded, so the first callback → `Consumed` →
///    clears the CSRF gate → membership passes → unconfigured Airbnb → 503.
/// 2. The SAME state is replayed → the store already consumed it → `Rejected`
///    → the handler returns **400 `INVALID_STATE`**.
///
/// This is the branch (oauth.rs `ConsumeOutcome::Rejected → 400`) that had zero
/// coverage: if a regression made a consume failure fall through instead of
/// rejecting, every store-unavailable test would still pass but this one fails.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_rejects_replayed_state(pool: PgPool) {
    let (app, store) = TestApp::with_oauth_state_store(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-replay").await;
    let user_id = seed_user(&pool, "cb-replay@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    // A genuinely-issued, single-use state bound to this org.
    let valid_state = format!("{org_id}:{}", Uuid::new_v4());
    store.seed(&valid_state, org_id, user_id);

    // 1st callback: state is found + org-bound → Consumed → clears the CSRF
    // gate → membership passes → unconfigured Airbnb → 503 (NOT a state rejection).
    let first = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &valid_state),
            &token,
        ))
        .await;
    assert_eq!(
        first.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "first use of a freshly-issued state must be Consumed and clear the gate \
         → 503; got {}: {}",
        first.status,
        first.text()
    );
    assert!(
        !first.text().contains("INVALID_STATE") && !first.text().contains("STATE_MISMATCH"),
        "first use of a valid single-use state must not be CSRF-rejected: {}",
        first.text()
    );

    // 2nd callback with the SAME state: already consumed → Rejected → 400.
    let replay = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &valid_state),
            &token,
        ))
        .await;
    assert_eq!(
        replay.status,
        StatusCode::BAD_REQUEST,
        "a replayed (already-consumed) state must return 400; got {}: {}",
        replay.status,
        replay.text()
    );
    assert!(
        replay.text().contains("INVALID_STATE"),
        "replayed state rejection must be INVALID_STATE: {}",
        replay.text()
    );
}

/// A well-formed state that the server never issued (not present in the store)
/// is rejected on its FIRST use with 400 `INVALID_STATE` — proving the consume
/// path (not just the stateless org-prefix gate) is authoritative when a store
/// is wired. Complements the replay test: here the `None`/`Rejected` arm is hit
/// without any prior successful use, so a forged token cannot ride the
/// stateless fallback when single-use enforcement is active.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_rejects_unissued_state_when_store_active(pool: PgPool) {
    let (app, _store) = TestApp::with_oauth_state_store(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-forged").await;
    let user_id = seed_user(&pool, "cb-forged@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let token = mint_token(user_id, org_id);

    // Correct `{org}:{nonce}` shape and matching org prefix, but never seeded
    // into the store → not a genuine single-use token.
    let forged_state = format!("{org_id}:{}", Uuid::new_v4());
    let resp = app
        .execute(authed_get(
            &airbnb_callback_uri(org_id, "auth-code", &forged_state),
            &token,
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "an un-issued state must be rejected by the active consume path → 400; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("INVALID_STATE"),
        "un-issued state rejection must be INVALID_STATE: {}",
        resp.text()
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

/// Manager-gate boundary: the browser callback now mirrors the token-exchange
/// POST — after `verify_org_access` (membership) it runs `verify_manager_role_in_org`.
/// A `resident` member therefore clears the state gate AND the membership check
/// but is short-circuited by the manager gate → 403 (NOT the 503 it used to
/// reach at the unconfigured token exchange). Pins the manager gate so a
/// regression that drops it is caught.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_callback_resident_member_rejected_by_manager_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cb-resident").await;
    let user_id = seed_user(&pool, "cb-resident@test.local").await;
    // Only a `resident` — a member, but the callback now requires a manager.
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
        StatusCode::FORBIDDEN,
        "resident member must be blocked by the callback manager gate → 403; \
         got {}: {}",
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

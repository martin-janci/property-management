//! Integration tests for the inbound Airbnb realtime webhook route
//! (Coverage 83-1).
//!
//! The handler under test is
//! `routes/integrations/webhook.rs::handle_airbnb_webhook`, mounted at
//! `POST /api/v1/integrations/airbnb/webhook`. It is a *public* endpoint
//! (no bearer auth) — authentication is the HMAC-SHA256 signature over the
//! raw request body, keyed by `AIRBNB_WEBHOOK_SECRET`.
//!
//! # What is tested (end-to-end, through the real Axum router + DB)
//!
//! The pre-existing unit tests in `webhook.rs` exercise the dedup-key
//! derivation and the crypto helpers in isolation, and
//! `airbnb_webhook_dedup_tests.rs` exercises the raw ledger SQL. Neither
//! drives the actual HTTP handler. These tests close that gap:
//!
//! 1. **Signature verification** — an unsigned / mis-signed delivery is
//!    rejected with 401 before any parsing or persistence happens.
//! 2. **Payload parsing** — a correctly-signed but malformed body is rejected
//!    with 400.
//! 3. **Happy path + persistence** — a correctly-signed first delivery returns
//!    200 and records exactly one row in the `airbnb_webhook_events`
//!    idempotency ledger (migration 00169).
//! 4. **Idempotent redelivery** — a byte-identical redelivery (Airbnb's
//!    at-least-once guarantee) returns 200 and does NOT write a second ledger
//!    row, proving the dedup gate fires through the real handler.
//! 5. **Route wiring** — the endpoint is actually mounted (not 404/405).
//!
//! # Secret wiring
//!
//! `AppState` loads `AIRBNB_WEBHOOK_SECRET` from the environment once at
//! construction (`AirbnbAppConfig::from_env`). Env is process-global and the
//! `TestApp` harness builds state per-test, so we seed a deterministic secret
//! exactly once via a `Once` *before* the first `TestApp::new`. Every test in
//! this binary then signs with `WEBHOOK_SECRET`. (The "secret not configured"
//! 500 branch cannot be exercised in the same process once the env is set, so
//! it is intentionally left to the per-request unit-level reasoning; the
//! high-value signature / parse / dedup paths are all covered here.)

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::{PgPool, Row};

use common::TestApp;

type HmacSha256 = Hmac<Sha256>;

const WEBHOOK_SECRET: &str = "airbnb-webhook-test-secret-key";
const WEBHOOK_URI: &str = "/api/v1/integrations/airbnb/webhook";

/// Seed the webhook secret into the process env before the first `AppState`
/// is constructed. `set_var` is not thread-safe alongside concurrent
/// `getenv`, so we gate it behind a `Once` exactly like the `TestApp` harness
/// does for `JWT_SECRET` / `RUST_ENV`.
fn ensure_webhook_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("AIRBNB_WEBHOOK_SECRET", WEBHOOK_SECRET);
    });
}

/// Compute the hex HMAC-SHA256 signature Airbnb would send in
/// `X-Airbnb-Signature` for `body` under `WEBHOOK_SECRET`.
fn sign(body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(WEBHOOK_SECRET.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn signed_post(body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(WEBHOOK_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Airbnb-Signature", sign(body))
        .body(Body::from(body.to_owned()))
        .unwrap()
}

fn post_with_signature(body: &str, signature: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(WEBHOOK_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Airbnb-Signature", signature)
        .body(Body::from(body.to_owned()))
        .unwrap()
}

async fn ledger_count(pool: &PgPool, event_id: &str) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM airbnb_webhook_events WHERE event_id = $1")
        .bind(event_id)
        .fetch_one(pool)
        .await
        .expect("count query")
        .get::<i64, _>("n")
}

// A well-formed event that maps to NO connection/booking (so the handler runs
// signature → parse → dedup-insert → dispatch, where dispatch finds no
// matching connection and returns 200 without side effects). This keeps the
// happy-path assertions about signature + persistence deterministic without
// needing to seed a full rental connection graph.
const EVENT_CREATED: &str = r#"{"event_type":"reservation_created","event_id":"evt_routes_create_1","listing_id":"listing-no-such","confirmation_code":"HMNONE1","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;

// ===========================================================================
// 1. Signature verification
// ===========================================================================

/// A delivery with NO signature header must be rejected with 401 and must not
/// touch the ledger.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rejects_missing_signature(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(WEBHOOK_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(EVENT_CREATED))
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "missing signature must be 401; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("INVALID_SIGNATURE"),
        "expected INVALID_SIGNATURE error body"
    );
    assert_eq!(
        ledger_count(&pool, "evt_routes_create_1").await,
        0,
        "rejected delivery must not write to the dedup ledger"
    );
}

/// A delivery with a WRONG signature must be rejected with 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rejects_invalid_signature(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let resp = app
        .execute(post_with_signature(EVENT_CREATED, "deadbeefdeadbeef"))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "invalid signature must be 401; got {}: {}",
        resp.status,
        resp.text()
    );
    assert_eq!(
        ledger_count(&pool, "evt_routes_create_1").await,
        0,
        "rejected delivery must not write to the dedup ledger"
    );
}

/// A delivery signed with the WRONG secret (right algorithm, wrong key) must
/// also be rejected — guards against the handler accepting any well-formed hex.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rejects_signature_from_wrong_secret(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let mut mac = HmacSha256::new_from_slice(b"the-wrong-secret").unwrap();
    mac.update(EVENT_CREATED.as_bytes());
    let wrong_sig = hex::encode(mac.finalize().into_bytes());

    let resp = app
        .execute(post_with_signature(EVENT_CREATED, &wrong_sig))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "signature from wrong secret must be 401; got {}: {}",
        resp.status,
        resp.text()
    );
}

// ===========================================================================
// 2. Payload parsing (correctly signed, but malformed)
// ===========================================================================

/// A correctly-signed but non-JSON body must be rejected with 400 PARSE_ERROR
/// — proving signature verification runs over the raw bytes BEFORE parsing.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rejects_malformed_payload_after_valid_signature(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let body = "this is not json";
    let resp = app.execute(signed_post(body)).await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "malformed payload must be 400; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("PARSE_ERROR"),
        "expected PARSE_ERROR error body"
    );
}

// ===========================================================================
// 3 + 4. Happy path, persistence, and idempotent redelivery
// ===========================================================================

/// A correctly-signed first delivery returns 200 and records exactly one
/// ledger row; a byte-identical redelivery returns 200 WITHOUT writing a
/// second row (idempotency through the real handler).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn first_delivery_persists_and_redelivery_is_idempotent(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // First delivery — listing maps to no connection, so dispatch is a no-op,
    // but the dedup ledger row is still written and the handler returns 200.
    let resp1 = app.execute(signed_post(EVENT_CREATED)).await;
    assert_eq!(
        resp1.status,
        StatusCode::OK,
        "first signed delivery must be 200; got {}: {}",
        resp1.status,
        resp1.text()
    );
    assert_eq!(
        ledger_count(&pool, "evt_routes_create_1").await,
        1,
        "first delivery must record exactly one ledger row"
    );

    // Redelivery — same event_id => dedup gate suppresses it; still 200, but no
    // second ledger row.
    let resp2 = app.execute(signed_post(EVENT_CREATED)).await;
    assert_eq!(
        resp2.status,
        StatusCode::OK,
        "redelivery must still be acknowledged with 200; got {}: {}",
        resp2.status,
        resp2.text()
    );
    assert_eq!(
        ledger_count(&pool, "evt_routes_create_1").await,
        1,
        "redelivery must NOT write a second ledger row (idempotent)"
    );
}

/// A correctly-signed delivery that omits `event_id` (legacy schema) is still
/// accepted (200) and recorded under a derived `synthetic:` key.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legacy_delivery_without_event_id_is_recorded_under_synthetic_key(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let body = r#"{"event_type":"listing_updated","listing_id":"listing-legacy","timestamp":"2026-02-02T00:00:00Z","payload":{}}"#;
    let resp = app.execute(signed_post(body)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "legacy (no event_id) delivery must be 200; got {}: {}",
        resp.status,
        resp.text()
    );

    // Exactly one synthetic ledger row exists for this delivery.
    let synthetic_count: i64 = sqlx::query(
        "SELECT COUNT(*) AS n FROM airbnb_webhook_events WHERE event_id LIKE 'synthetic:%'",
    )
    .fetch_one(&pool)
    .await
    .expect("count query")
    .get::<i64, _>("n");
    assert_eq!(
        synthetic_count, 1,
        "legacy delivery must be recorded under exactly one synthetic key"
    );
}

// ===========================================================================
// 5. Route wiring
// ===========================================================================

/// The inbound webhook route must be mounted — a POST must not return
/// 404 (route missing) or 405 (wrong method).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn webhook_route_is_mounted(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // Unsigned POST — we just assert it is routed (401), not 404/405.
    let req = Request::builder()
        .method(Method::POST)
        .uri(WEBHOOK_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(EVENT_CREATED))
        .unwrap();
    let resp = app.execute(req).await;

    assert_ne!(
        resp.status,
        StatusCode::NOT_FOUND,
        "webhook route must be mounted (got 404 — router wiring broken)"
    );
    assert_ne!(
        resp.status,
        StatusCode::METHOD_NOT_ALLOWED,
        "webhook route must accept POST (got 405)"
    );
}

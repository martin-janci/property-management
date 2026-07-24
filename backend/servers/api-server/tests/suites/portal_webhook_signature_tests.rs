//! HTTP-level regression tests for the portal-webhook fail-closed fix (#833,
//! shipped in PR #874).
//!
//! PR #874 inverted the portal webhook signature contract so the receivers
//! fail **closed**: an unverified or invalid-signature webhook is rejected with
//! `401`, and only a correctly HMAC-SHA256-signed webhook is accepted. That PR
//! added unit tests for the `require_portal_webhook_verification` helper in
//! isolation, but no test exercised the contract end-to-end through the wired
//! Axum route stack (`POST /api/v1/webhooks/portals/...`). These tests close
//! that gap by driving the real router (`create_router` via `TestApp`) so the
//! route mount, the security gate ordering (verification runs *before* body
//! parsing and any DB access), and the `401` vs `200` outcomes are all pinned.
//!
//! Why this proves fail-closed:
//! - The rejection (`401`) happens in the security gate before the handler
//!   touches the listing repo, so a forged webhook never reaches the database.
//! - A valid signature gets *past* the gate; the handler then looks up the
//!   syndication. With no matching syndication seeded it returns `200` with
//!   `{"success": false, "message": "Syndication not found"}` — a `200` that is
//!   only reachable once verification has passed. That distinguishes "accepted
//!   the signature" (200, the fix working) from "rejected the signature"
//!   (401, the pre-fix forged path).
//!
//! These tests need a `PgPool` because `TestApp` builds the full `AppState`;
//! under `SQLX_OFFLINE=true` with no DB the harness skips them, and CI runs
//! them against an ephemeral migrated database.

#![allow(dead_code)]

use crate::common::TestApp;
use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::PgPool;

type HmacSha256 = Hmac<Sha256>;

/// The portal the `reality-portal` routes verify against. The route handler
/// passes the canonical `"reality_portal"` (underscore) to the verifier, so the
/// env var the verifier reads is `REALITY_PORTAL_WEBHOOK_SECRET`.
const PORTAL_SECRET_ENV: &str = "REALITY_PORTAL_WEBHOOK_SECRET";
const SIG_HEADER: &str = "X-Webhook-Signature";
const VIEWS_URI: &str = "/api/v1/webhooks/portals/reality-portal/views";

/// `std::env::{set_var, remove_var}` are not safe to interleave with `getenv`
/// on glibc, and integration tests in this binary may run concurrently. Every
/// test that mutates the webhook-secret env serializes behind this mutex.
/// Async-aware (`tokio::sync::Mutex`) because the guard must stay held across
/// the `.await`s of the whole test body — the env var has to remain stable
/// while `TestApp` boots and the request executes.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Compute the `sha256=<base64>` header value the verifier expects for `body`.
fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(body);
    format!("sha256={}", BASE64.encode(mac.finalize().into_bytes()))
}

/// Fail-closed: with NO secret configured the route rejects any webhook (even a
/// well-formed body) with `401` — never silently accepts an unverified payload.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unconfigured_secret_rejects_webhook(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    std::env::remove_var(PORTAL_SECRET_ENV);

    let app = TestApp::new(pool).await;

    let body = serde_json::json!({ "external_id": "ext-1", "views_count": 7 });
    let req = app
        .post(VIEWS_URI)
        .header(SIG_HEADER, "sha256=AAAA")
        .json(body)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "no secret configured must fail closed with 401, not accept the webhook. Body: {}",
        resp.text()
    );
}

/// Fail-closed: with a secret configured but an INVALID signature, the route
/// rejects with `401` before parsing the body or touching the database.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invalid_signature_is_rejected(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    std::env::set_var(PORTAL_SECRET_ENV, "super-secret-key");

    let app = TestApp::new(pool).await;

    let body = serde_json::json!({ "external_id": "ext-1", "views_count": 7 });
    let req = app
        .post(VIEWS_URI)
        .header(SIG_HEADER, "sha256=Zm9yZ2Vk") // "forged", not a valid HMAC
        .json(body)
        .build();
    let resp = app.execute(req).await;

    std::env::remove_var(PORTAL_SECRET_ENV);

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "an invalid signature must be rejected with 401. Body: {}",
        resp.text()
    );
}

/// Fail-closed: with a secret configured but the signature header MISSING
/// entirely, the route rejects with `401`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn missing_signature_header_is_rejected(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    std::env::set_var(PORTAL_SECRET_ENV, "super-secret-key");

    let app = TestApp::new(pool).await;

    let body = serde_json::json!({ "external_id": "ext-1", "views_count": 7 });
    let req = app.post(VIEWS_URI).json(body).build(); // no signature header
    let resp = app.execute(req).await;

    std::env::remove_var(PORTAL_SECRET_ENV);

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "a missing signature header must be rejected with 401. Body: {}",
        resp.text()
    );
}

/// Accept path: a correctly HMAC-SHA256-signed webhook gets PAST the security
/// gate. The body must be signed exactly as serialized on the wire, so we build
/// the JSON string once, sign those bytes, and send the same bytes as the body.
///
/// With no matching syndication seeded, the handler returns `200` with
/// `success: false` — a `200` that is only reachable once verification passed.
/// The assertion that matters for this regression is "not 401": the valid
/// signature was accepted rather than rejected as forged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn valid_signature_is_accepted(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    let secret = "super-secret-key";
    std::env::set_var(PORTAL_SECRET_ENV, secret);

    let app = TestApp::new(pool).await;

    // Serialize the body once and sign those exact bytes — the verifier HMACs
    // the raw request body, so signature and payload must be byte-identical.
    let body = serde_json::to_string(&serde_json::json!({
        "external_id": "ext-unknown",
        "views_count": 3,
    }))
    .unwrap();
    let signature = sign(secret, body.as_bytes());

    let req = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri(VIEWS_URI)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(SIG_HEADER, signature)
        .body(axum::body::Body::from(body))
        .expect("build signed request");
    let resp = app.execute(req).await;

    std::env::remove_var(PORTAL_SECRET_ENV);

    assert_ne!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "a correctly-signed webhook must be accepted (got past the signature \
         gate), not rejected as forged. Body: {}",
        resp.text()
    );
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "an accepted (but unmatched) webhook returns 200 with success=false. Body: {}",
        resp.text()
    );
    let json = resp.json_value();
    assert_eq!(
        json.get("success").and_then(|v| v.as_bool()),
        Some(false),
        "unmatched syndication is acknowledged with success=false: {json}"
    );
}

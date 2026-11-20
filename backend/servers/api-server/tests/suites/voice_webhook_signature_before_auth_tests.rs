//! Regression tests: the Alexa / Google voice webhook handlers must run their
//! platform signature / ID-token verification BEFORE any auth/RLS work.
//!
//! Audit history: `alexa_webhook` and `google_actions_webhook` each took a
//! `mut rls: RlsConnection` *extractor* as a handler argument. Axum resolves
//! extractors before the handler body runs, and `RlsConnection` resolves via
//! `ValidatedTenantExtractor` — full JWT + tenant-membership validation. A
//! genuine Alexa / Google webhook delivery is authenticated by the platform's
//! own request signature (Amazon certificate chain / Google-issued ID token),
//! NOT by a Property-Management bearer token, so it carries no PM JWT. The
//! extractor therefore rejected every legitimate delivery with **401 before the
//! webhook body was ever read** and before the signature was ever checked.
//!
//! The fix drops the `RlsConnection` extractor from both handlers: the
//! signature / ID-token check runs first, and the database connection is
//! acquired inside the handler body (a context-cleared public connection for
//! the non-RLS-bound `voice_assistant_devices` lookup, then the device owner's
//! org/user RLS context for command processing).
//!
//! These tests pin the ordering. A request with NO PM bearer token must reach
//! the handler's own signature-verification failure (`INVALID_SIGNATURE` for
//! Alexa, `INVALID_REQUEST` for Google) rather than being short-circuited by the
//! auth/RLS extractor. Both assert on the handler-produced error code in the
//! body — the pre-fix auth-extractor rejection never carries that code, so this
//! suite fails against the buggy handler and passes against the fixed one.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;

use crate::common::TestApp;

const ALEXA_URI: &str = "/api/v1/webhooks/voice/alexa";
const GOOGLE_URI: &str = "/api/v1/webhooks/voice/google";

// ---------------------------------------------------------------------------
// Alexa: a delivery with no PM bearer must reach signature verification (which
// fails here on the missing `SignatureCertChainUrl` header), NOT be rejected by
// the auth/RLS extractor. Proven by the `INVALID_SIGNATURE` error code, which
// only the handler's own signature path produces.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn alexa_webhook_runs_signature_check_before_auth(pool: PgPool) {
    let app = TestApp::new(pool).await;

    // No Authorization header (as a real Alexa delivery) and no Alexa signature
    // headers, so signature verification fails on the missing cert-chain header.
    let req = Request::builder()
        .method(Method::POST)
        .uri(ALEXA_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "request": {} }).to_string()))
        .unwrap();

    let resp = app.execute(req).await;
    let body = resp.text();

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "a signature-less Alexa delivery must be 401, got {}: {body}",
        resp.status,
    );
    assert!(
        body.contains("INVALID_SIGNATURE"),
        "the 401 must come from the handler's signature check (INVALID_SIGNATURE), \
         not the auth/RLS extractor — body was: {body}"
    );
}

// ---------------------------------------------------------------------------
// Google: a delivery with no PM bearer but a well-formed Actions request body
// must reach the ID-token verification (which fails here — no `Authorization`
// Bearer ID token), NOT be rejected by the auth/RLS extractor. Proven by the
// `INVALID_REQUEST` error code the handler's verification path produces.
// ---------------------------------------------------------------------------
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn google_actions_webhook_runs_verification_before_auth(pool: PgPool) {
    let app = TestApp::new(pool).await;

    // Minimal but structurally valid GoogleActionsRequest so the Json body
    // extractor succeeds and control reaches `verify_google_request`.
    let google_body = json!({
        "handler": { "name": "main" },
        "intent": { "name": "actions.intent.MAIN" },
        "session": { "id": "test-session" },
        "user": { "verificationStatus": "VERIFIED" }
    });

    let req = Request::builder()
        .method(Method::POST)
        .uri(GOOGLE_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(google_body.to_string()))
        .unwrap();

    let resp = app.execute(req).await;
    let body = resp.text();

    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "a Google delivery without a valid ID token must be 401, got {}: {body}",
        resp.status,
    );
    assert!(
        body.contains("INVALID_REQUEST"),
        "the 401 must come from the handler's Google ID-token verification \
         (INVALID_REQUEST), not the auth/RLS extractor — body was: {body}"
    );
}

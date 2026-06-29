//! Integration smoke tests for the baseline security-headers middleware
//! (#954 / PR #963 follow-up).
//!
//! # Why this file exists
//!
//! The `security_headers` middleware itself has unit coverage in
//! `api-core` (`middleware/security_headers.rs::tests`): those assert the
//! function attaches the right header *values* on a minimal router.
//!
//! What was missing — and what PR #963's main.rs wiring shipped without — is a
//! test proving the layer is actually wired into the served application. The
//! layer was originally attached ONLY in `main.rs::apply_middleware` (the
//! production-binary path), while every integration test builds its router via
//! `api_server::create_router` (the test path). The two chains had drifted
//! before for routes (#867 / #836) and for `host_tenant_middleware` (see the
//! `TestApp` caveat in `push_token_tests.rs`), so a security-critical edge
//! middleware living in only one of the two paths is exactly the failure mode
//! to guard against.
//!
//! These tests exercise the real `create_router` chain end-to-end and assert
//! the baseline headers appear on the response. A companion static guard in
//! `router_single_source_tests.rs` keeps the two chains from drifting again
//! without needing a database (so it runs in every `cargo test`, DB or not).

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt; // for `oneshot`

use common::TestApp;

/// `/health` is public, dependency-free, and returns 200 without auth — the
/// cleanest endpoint to prove the security-headers layer fires on a real
/// served response from `create_router`.
#[sqlx::test]
async fn security_headers_present_on_health_response(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let response = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/health must return 200 so we are asserting headers on a real handler response"
    );

    let headers = response.headers();

    // Assert the exact baseline shipped by api-core's `security_headers`. If the
    // layer is missing from `create_router` (the regression this file guards),
    // every one of these `.expect(...)` calls fails — proving the wiring, not
    // just the middleware function in isolation.
    assert_eq!(
        headers.get(header::STRICT_TRANSPORT_SECURITY).expect(
            "HSTS header must be present — security_headers layer not wired into create_router"
        ),
        "max-age=63072000; includeSubDomains",
    );
    assert_eq!(
        headers
            .get(header::X_CONTENT_TYPE_OPTIONS)
            .expect("X-Content-Type-Options header must be present"),
        "nosniff",
    );
    assert_eq!(
        headers
            .get(header::X_FRAME_OPTIONS)
            .expect("X-Frame-Options header must be present"),
        "DENY",
    );
    assert_eq!(
        headers
            .get(header::REFERRER_POLICY)
            .expect("Referrer-Policy header must be present"),
        "strict-origin-when-cross-origin",
    );
    assert_eq!(
        headers
            .get(header::CONTENT_SECURITY_POLICY)
            .expect("Content-Security-Policy header must be present"),
        "default-src 'none'",
    );
}

/// The headers must also be present on a 4xx response (e.g. an unauthenticated
/// request to a protected route). Tower layers execute outside-in, so the
/// security-headers layer wraps the whole stack and must fire regardless of the
/// inner status. This mirrors the api-core unit test's 5xx/404 cases but proves
/// it through the full `create_router` chain — security headers must never be
/// skipped on rejection paths, which is when they matter most.
#[sqlx::test]
async fn security_headers_present_on_unauthenticated_4xx(pool: PgPool) {
    let app = TestApp::new(pool).await;

    // A protected route with no auth headers — `RlsConnection` / auth extractors
    // reject with a 4xx before any handler logic runs.
    let response = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/users/me/push-tokens")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "unauthenticated request to a protected route must be rejected (got {})",
        response.status()
    );

    let headers = response.headers();
    assert!(
        headers.get(header::STRICT_TRANSPORT_SECURITY).is_some(),
        "HSTS must be set even on rejected (4xx) responses"
    );
    assert!(
        headers.get(header::X_CONTENT_TYPE_OPTIONS).is_some(),
        "X-Content-Type-Options must be set even on rejected (4xx) responses"
    );
    assert!(
        headers.get(header::CONTENT_SECURITY_POLICY).is_some(),
        "CSP must be set even on rejected (4xx) responses"
    );
}

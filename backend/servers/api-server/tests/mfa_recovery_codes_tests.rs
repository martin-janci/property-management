//! Integration tests for POST /api/v1/users/me/mfa/recovery-codes/verify
//! (Story 9.2 — single-use recovery code consumption).
//!
//! Covers:
//!   T1 — valid code accepted; codesRemaining decrements
//!   T2 — replay: same code returns 400 (used_at guard)
//!   T3 — invalid code returns 400 (not 410 after timing-oracle fix)
//!   T4 — MFA not enabled returns 404
//!   T5 — exhaustion: all 10 codes used → 11th attempt returns 400, not 410
//!   T6 — disable MFA invalidates all codes; verify returns 400 afterwards
//!
//! #2158: promoted out of the never-compiled `tests/integration/` subtree to a
//! real top-level test binary. It declares the shared `mod common;` harness
//! itself and reaches into it via `crate::common::*`. These T1–T6 functional
//! flows are NOT covered by the top-level `mfa_recovery_cross_user_idor_tests.rs`
//! (that file audits only cross-user IDOR scoping).

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::common::{cleanup_test_user, create_authenticated_user, TestApp, TestUser};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn current_totp_code(secret: &str) -> String {
    let secret_bytes = Secret::Encoded(secret.to_string())
        .to_bytes()
        .expect("valid base32 secret");
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some("Property Management".to_string()),
        "test".to_string(),
    )
    .expect("valid TOTP params");
    totp.generate_current().expect("valid system clock")
}

async fn do_mfa_setup(app: &TestApp, token: &str) -> Value {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/setup")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    resp.json_value()
}

async fn do_mfa_verify(app: &TestApp, token: &str, code: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/verify")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": code }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    (resp.status, resp.json_value())
}

/// Enable MFA and return the 10 plain-text recovery codes.
async fn enable_mfa_and_get_codes(app: &TestApp, token: &str) -> Vec<String> {
    let setup_body = do_mfa_setup(app, token).await;
    let secret = setup_body["secret"].as_str().expect("secret field");
    let code = current_totp_code(secret);
    let (status, body) = do_mfa_verify(app, token, &code).await;
    assert_eq!(status, StatusCode::OK, "MFA verify failed: {body}");
    body["recoveryCodes"]
        .as_array()
        .expect("recoveryCodes array")
        .iter()
        .map(|v| v.as_str().expect("string code").to_string())
        .collect()
}

async fn post_recovery_verify(app: &TestApp, token: &str, code: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users/me/mfa/recovery-codes/verify")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": code }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    (resp.status, resp.json_value())
}

async fn do_disable_mfa(app: &TestApp, token: &str, code: &str) -> StatusCode {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/disable")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": code }).to_string()))
        .unwrap();
    app.execute(req).await.status
}

// ─── T1: valid code accepted; codesRemaining decrements ──────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_valid_recovery_code_accepted_and_remaining_decrements(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (token, _) = create_authenticated_user(&app, &user).await;
    let codes = enable_mfa_and_get_codes(&app, &token).await;
    assert_eq!(codes.len(), 10, "must issue 10 recovery codes");

    let (status, body) = post_recovery_verify(&app, &token, &codes[0]).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "first recovery code must be accepted; body: {body}"
    );
    assert_eq!(
        body["codesRemaining"],
        json!(9),
        "codesRemaining should be 9 after consuming one code"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ─── T2: replay — second use of the same code is rejected ────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_replay_of_used_recovery_code_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (token, _) = create_authenticated_user(&app, &user).await;
    let codes = enable_mfa_and_get_codes(&app, &token).await;

    let (status1, _) = post_recovery_verify(&app, &token, &codes[0]).await;
    assert_eq!(status1, StatusCode::OK, "first use must succeed");

    let (status2, body2) = post_recovery_verify(&app, &token, &codes[0]).await;
    assert_eq!(
        status2,
        StatusCode::BAD_REQUEST,
        "replay must be rejected with 400; body: {body2}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ─── T3: invalid code returns 400 ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_invalid_recovery_code_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (token, _) = create_authenticated_user(&app, &user).await;
    let _codes = enable_mfa_and_get_codes(&app, &token).await;

    let (status, body) = post_recovery_verify(&app, &token, "XXXX-XXXX-XXXX-INVALID").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "invalid code must return 400; body: {body}"
    );
    assert_ne!(
        status.as_u16(),
        410,
        "must not reveal 'no codes remaining' via 410 GONE (timing oracle)"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ─── T4: MFA not enabled returns 404 ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_recovery_verify_without_mfa_enabled_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (token, _) = create_authenticated_user(&app, &user).await;

    let (status, body) = post_recovery_verify(&app, &token, "ANYCODE").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "must return 404 when MFA is not enabled; body: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ─── T5: exhaustion — all 10 codes used; 11th attempt returns 400, not 410 ───

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_exhausted_recovery_codes_return_400_not_410(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (token, _) = create_authenticated_user(&app, &user).await;
    let codes = enable_mfa_and_get_codes(&app, &token).await;
    assert_eq!(codes.len(), 10);

    // Consume all 10 codes.
    for (i, code) in codes.iter().enumerate() {
        let (status, body) = post_recovery_verify(&app, &token, code).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "code {i} should be accepted; body: {body}"
        );
    }

    // 11th attempt with an arbitrary string — must be 400, not 410.
    let (status, body) = post_recovery_verify(&app, &token, "AAAA-BBBB-CCCC-DDDD").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "exhausted recovery codes must return 400, not 410; body: {body}"
    );
    assert_ne!(
        status.as_u16(),
        410,
        "410 GONE must not be returned (timing oracle)"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ─── T6: disable MFA invalidates all codes ───────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_disable_mfa_invalidates_all_recovery_codes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (token, _) = create_authenticated_user(&app, &user).await;
    let codes = enable_mfa_and_get_codes(&app, &token).await;

    // Use the first code as the disable credential.
    let disable_status = do_disable_mfa(&app, &token, &codes[0]).await;
    assert_eq!(
        disable_status,
        StatusCode::OK,
        "MFA disable must succeed using a recovery code"
    );

    // Re-enable to have MFA active again, so the verify endpoint is reachable.
    let codes2 = enable_mfa_and_get_codes(&app, &token).await;

    // The first batch of codes (codes[1..]) must no longer be accepted.
    let (status, body) = post_recovery_verify(&app, &token, &codes[1]).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "old code from before disable must be rejected after re-enable; body: {body}"
    );
    // Sanity-check: new batch codes work.
    let (status2, _) = post_recovery_verify(&app, &token, &codes2[0]).await;
    assert_eq!(
        status2,
        StatusCode::OK,
        "fresh code from new batch must be accepted"
    );

    cleanup_test_user(&pool, &user.email).await;
}

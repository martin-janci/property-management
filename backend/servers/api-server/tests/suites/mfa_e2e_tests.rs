//! MFA end-to-end integration tests (gap-9-1-mfa-e2e-verify).
//!
//! Verifies the complete TOTP Two-Factor Authentication Story 9.1 flows:
//!
//! Flow 1 — Setup + Verify (QR scan → TOTP enable)
//! Flow 2 — Login 2FA prompt (mfa_required response when code omitted)
//! Flow 3 — Login with valid TOTP code after MFA is enabled
//! Flow 4 — Disable flow (requires current TOTP code)
//! Flow 5 — Backup-codes regeneration (requires current TOTP code)
//! Flow 6 — MFA status endpoint (idle / enabled states)
//!
//! Note: Tests that require a *valid* TOTP code derive it in-process using
//! `totp_rs` (same library used by api-server). This avoids clock-skew issues
//! and lets us exercise the real validation path.

// #2158: promoted out of the never-compiled `tests/integration/` subtree to a
// real top-level test binary. As the crate root of this binary, it declares the
// shared `mod common;` harness itself (the standard pattern used by every other
// top-level test file) and reaches into it via `crate::common::*`.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::common::{cleanup_test_user, create_authenticated_user_with_org, TestApp, TestUser};

// ─── helpers ─────────────────────────────────────────────────────────────────
//
// #2158: every MFA endpoint (setup/verify/disable/status/backup-codes) takes
// `RlsConnection`, whose extractor requires BOTH an `X-Tenant-ID` header and an
// active `organization_members` row — otherwise the request is rejected with
// 400/403 before the handler runs (see `tests/common/mod.rs:574-590`). So the
// helpers thread `org_id` through and stamp the tenant header, and the tests
// provision the user via `create_authenticated_user_with_org`. Login requests
// (`/api/v1/auth/login`) are NOT RLS-gated and need no tenant header.

/// Generate the current TOTP code from a base32-encoded secret.
fn current_totp_code(secret: &str) -> String {
    let secret_bytes = Secret::Encoded(secret.to_string())
        .to_bytes()
        .expect("valid base32 secret");
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,  // skew
        30, // step
        secret_bytes,
        Some("Property Management".to_string()),
        "test".to_string(),
    )
    .expect("valid TOTP params");
    totp.generate_current().expect("valid system clock")
}

/// Call POST /api/v1/auth/mfa/setup and return the parsed JSON body.
async fn do_mfa_setup(app: &TestApp, access_token: &str, org_id: Uuid) -> Value {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/setup")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    resp.json_value()
}

/// Call POST /api/v1/auth/mfa/verify with a generated code.
/// Returns (status, json_body).
async fn do_mfa_verify(
    app: &TestApp,
    access_token: &str,
    org_id: Uuid,
    code: &str,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/verify")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": code }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    (resp.status, resp.json_value())
}

/// Setup MFA and immediately verify with a valid TOTP code.
async fn setup_and_enable_mfa(app: &TestApp, access_token: &str, org_id: Uuid) {
    let setup_body = do_mfa_setup(app, access_token, org_id).await;
    let secret = setup_body["secret"].as_str().expect("secret field");
    let code = current_totp_code(secret);
    let (status, body) = do_mfa_verify(app, access_token, org_id, &code).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "MFA verify should succeed; body: {}",
        body
    );
    assert_eq!(body["enabled"], json!(true), "enabled should be true");
}

// ─── Flow 1: MFA setup response shape ───────────────────────────────────────

/// /api/v1/auth/mfa/setup returns `secret` and `qrUri` only (camelCase).
/// Recovery codes (10 single-use) are issued on verify, not setup (Story 9.2).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_setup_response_shape(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "shape").await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/setup")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();

    // Setup returns only secret + qrUri; backupCodes moved to verify (Story 9.2)
    assert!(body["secret"].is_string(), "secret must be a string");
    assert!(
        body["qrUri"].is_string(),
        "qrUri must be a string (camelCase)"
    );
    assert!(
        body["backupCodes"].is_null(),
        "backupCodes must NOT appear in setup response (issued at verify); got: {}",
        body
    );

    // QR URI should be a valid otpauth:// URI
    let qr_uri = body["qrUri"].as_str().unwrap();
    assert!(
        qr_uri.starts_with("otpauth://totp/"),
        "qrUri must be an otpauth:// URI, got: {}",
        qr_uri
    );

    // Verify → 10 single-use recovery codes are returned (not at setup)
    let secret = body["secret"].as_str().unwrap();
    let code = current_totp_code(secret);
    let (verify_status, verify_body) = do_mfa_verify(&app, &access_token, org_id, &code).await;
    assert_eq!(
        verify_status,
        StatusCode::OK,
        "verify must succeed; body: {}",
        verify_body
    );
    let recovery_codes = verify_body["recoveryCodes"]
        .as_array()
        .expect("recoveryCodes must be array in verify response");
    assert_eq!(
        recovery_codes.len(),
        10,
        "verify must return 10 recovery codes"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ─── Flow 1b: Setup → Verify with valid TOTP code ───────────────────────────

/// Full setup → verify path with a real TOTP code derived from the returned secret.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_full_setup_and_verify_with_valid_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "full").await;

    // Step 1: initiate setup
    let setup_body = do_mfa_setup(&app, &access_token, org_id).await;
    let secret = setup_body["secret"].as_str().expect("secret field");

    // Step 2: generate a valid code from the returned secret (same as scanning QR)
    let valid_code = current_totp_code(secret);

    // Step 3: verify → MFA should now be enabled
    let (status, body) = do_mfa_verify(&app, &access_token, org_id, &valid_code).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "verify should return 200; body: {}",
        body
    );
    assert_eq!(body["enabled"], json!(true));
    assert!(body["message"].is_string());

    cleanup_test_user(&pool, &user.email).await;
}

/// Verifying with an invalid code returns 400 with INVALID_CODE.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_verify_invalid_code_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "invcode").await;
    do_mfa_setup(&app, &access_token, org_id).await;

    let (status, body) = do_mfa_verify(&app, &access_token, org_id, "000000").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], json!("INVALID_CODE"));

    cleanup_test_user(&pool, &user.email).await;
}

// ─── Flow 6: MFA status endpoint ────────────────────────────────────────────

/// GET /api/v1/auth/mfa/status returns `enabled: false` before setup.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_status_disabled_before_setup(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "stdis").await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/mfa/status")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();

    assert_eq!(body["enabled"], json!(false));
    assert_eq!(body["backupCodesRemaining"], json!(0));

    cleanup_test_user(&pool, &user.email).await;
}

/// GET /api/v1/auth/mfa/status returns `enabled: true` after successful verify.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_status_enabled_after_verify(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "sten").await;
    setup_and_enable_mfa(&app, &access_token, org_id).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/mfa/status")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();

    assert_eq!(body["enabled"], json!(true), "status must be enabled");
    // backupCodesRemaining is 10 freshly after setup
    let remaining = body["backupCodesRemaining"].as_i64().unwrap_or(-1);
    assert!(remaining > 0, "should have some backup codes remaining");

    cleanup_test_user(&pool, &user.email).await;
}

// ─── Flow 2: Login 2FA prompt ────────────────────────────────────────────────

/// When MFA is enabled and `two_factor_code` is omitted from login, the server
/// returns `mfaRequired: true` with empty tokens (not a 401).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_login_returns_mfa_required_when_mfa_enabled_and_code_absent(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "mfareq").await;
    setup_and_enable_mfa(&app, &access_token, org_id).await;

    // Login without two_factor_code
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(user.login_body().to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    // Server returns 200 with mfaRequired rather than 401 to distinguish
    // "wrong password" from "need MFA code" at the client.
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();

    assert_eq!(
        body["mfaRequired"],
        json!(true),
        "mfaRequired must be true; body: {}",
        body
    );
    // Tokens are empty when MFA is required
    assert_eq!(body["accessToken"], json!(""), "accessToken must be empty");
    assert_eq!(
        body["refreshToken"],
        json!(""),
        "refreshToken must be empty"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// Login succeeds when valid `twoFactorCode` is included.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_login_succeeds_with_valid_mfa_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "loginok").await;

    // Setup & enable MFA, capture the secret for later use.
    let setup_body = do_mfa_setup(&app, &access_token, org_id).await;
    let secret = setup_body["secret"]
        .as_str()
        .expect("secret field")
        .to_string();
    let enable_code = current_totp_code(&secret);
    let (status, _) = do_mfa_verify(&app, &access_token, org_id, &enable_code).await;
    assert_eq!(status, StatusCode::OK);

    // Login again, this time providing the two_factor_code
    let login_code = current_totp_code(&secret);
    let login_body = json!({
        "email": user.email,
        "password": user.password,
        "twoFactorCode": login_code,
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();

    // Should NOT have mfaRequired set
    assert!(
        body["mfaRequired"].is_null(),
        "mfaRequired must not be present on success; body: {}",
        body
    );
    // Should have real tokens
    assert!(
        body["accessToken"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "accessToken must be non-empty; body: {}",
        body
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// Login with an *invalid* two_factor_code returns 401 INVALID_MFA_CODE.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_login_fails_with_invalid_mfa_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "loginbad").await;
    setup_and_enable_mfa(&app, &access_token, org_id).await;

    let login_body = json!({
        "email": user.email,
        "password": user.password,
        "twoFactorCode": "000000",
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
    let body = resp.json_value();
    assert_eq!(body["code"], json!("INVALID_MFA_CODE"));

    cleanup_test_user(&pool, &user.email).await;
}

// ─── Flow 4: Disable flow ────────────────────────────────────────────────────

/// POST /api/v1/auth/mfa/disable with valid TOTP code disables MFA.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_disable_with_valid_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "disok").await;

    // Setup MFA, capture secret
    let setup_body = do_mfa_setup(&app, &access_token, org_id).await;
    let secret = setup_body["secret"]
        .as_str()
        .expect("secret field")
        .to_string();
    let verify_code = current_totp_code(&secret);
    let (st, _) = do_mfa_verify(&app, &access_token, org_id, &verify_code).await;
    assert_eq!(st, StatusCode::OK, "enable must succeed");

    // Disable with a fresh TOTP code
    let disable_code = current_totp_code(&secret);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/disable")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": disable_code }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["message"].is_string());

    // Status should now report disabled
    let status_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/mfa/status")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let status_resp = app.execute(status_req).await;
    let status_body = status_resp.json_value();
    assert_eq!(
        status_body["enabled"],
        json!(false),
        "MFA must be disabled; body: {}",
        status_body
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// POST /api/v1/auth/mfa/disable with invalid code returns 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_disable_with_invalid_code_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "disbad").await;
    setup_and_enable_mfa(&app, &access_token, org_id).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/disable")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": "000000" }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body = resp.json_value();
    assert_eq!(body["code"], json!("INVALID_CODE"));

    cleanup_test_user(&pool, &user.email).await;
}

// ─── Flow 5: Backup-codes regeneration ──────────────────────────────────────

/// POST /api/v1/auth/mfa/backup-codes/regenerate returns fresh codes.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_backup_codes_regeneration(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "regen").await;

    // Enable MFA; recovery codes are issued by verify (not setup) in Story 9.2.
    let setup_body = do_mfa_setup(&app, &access_token, org_id).await;
    let secret = setup_body["secret"]
        .as_str()
        .expect("secret field")
        .to_string();

    let verify_code = current_totp_code(&secret);
    let (st, verify_body) = do_mfa_verify(&app, &access_token, org_id, &verify_code).await;
    assert_eq!(st, StatusCode::OK);
    let original_backup_codes = verify_body["recoveryCodes"]
        .as_array()
        .expect("recoveryCodes array in verify response")
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect::<Vec<_>>();

    // Regenerate backup codes
    let regen_code = current_totp_code(&secret);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/backup-codes/regenerate")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": regen_code }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();

    let new_codes = body["backupCodes"]
        .as_array()
        .expect("backupCodes in regenerate response");

    assert_eq!(new_codes.len(), 10, "should regenerate 10 backup codes");
    assert!(body["message"].is_string());

    // New codes should differ from originals
    let new_codes_str: Vec<String> = new_codes
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    let any_match = new_codes_str
        .iter()
        .any(|c| original_backup_codes.contains(c));
    assert!(
        !any_match,
        "regenerated codes should differ from original backup codes"
    );

    // Status should reflect 10 remaining backup codes
    let status_req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/auth/mfa/status")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let status_resp = app.execute(status_req).await;
    let status_body = status_resp.json_value();
    let remaining = status_body["backupCodesRemaining"].as_i64().unwrap_or(-1);
    assert_eq!(
        remaining, 10,
        "backupCodesRemaining should be 10 after regen"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// Regenerate with invalid code returns 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_backup_codes_regeneration_invalid_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "regenbad").await;
    setup_and_enable_mfa(&app, &access_token, org_id).await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/backup-codes/regenerate")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "code": "000000" }).to_string()))
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body = resp.json_value();
    assert_eq!(body["code"], json!("INVALID_CODE"));

    cleanup_test_user(&pool, &user.email).await;
}

// ─── Auth guard: unauthenticated requests ────────────────────────────────────

/// All MFA endpoints require a valid bearer token.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_endpoints_require_authentication(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let endpoints: &[(&str, Method)] = &[
        ("/api/v1/auth/mfa/setup", Method::POST),
        ("/api/v1/auth/mfa/verify", Method::POST),
        ("/api/v1/auth/mfa/disable", Method::POST),
        ("/api/v1/auth/mfa/status", Method::GET),
        ("/api/v1/auth/mfa/backup-codes/regenerate", Method::POST),
    ];

    for (uri, method) in endpoints {
        let req = Request::builder()
            .method(method.clone())
            .uri(*uri)
            .body(Body::empty())
            .unwrap();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "Expected 401 for unauthenticated {method} {uri}, got {}",
            resp.status
        );
    }
}

/// Setup endpoint returns 409 when MFA is already enabled.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_mfa_setup_conflicts_when_already_enabled(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "conflict").await;
    setup_and_enable_mfa(&app, &access_token, org_id).await;

    // Try to initiate setup again
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/mfa/setup")
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    resp.assert_status(StatusCode::CONFLICT);
    let body = resp.json_value();
    assert_eq!(body["code"], json!("MFA_ALREADY_ENABLED"));

    cleanup_test_user(&pool, &user.email).await;
}

/// Issue #487: brute-force protection on the MFA verify endpoint.
///
/// A 6-digit TOTP code has 10⁶ possibilities; without per-account rate
/// limiting an attacker can enumerate all of them in under a minute.
/// This test is `#[ignore]`d until rate-limiting middleware lands on
/// `/api/v1/auth/mfa/verify` — at which point flip the assertion to
/// expect 429 (or whatever the chosen lockout response is).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "Awaiting rate-limit middleware on /auth/mfa/verify — issue #487"]
async fn test_mfa_verify_rate_limited(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, org_id) = create_authenticated_user_with_org(&app, &user, "ratelim").await;
    setup_and_enable_mfa(&app, &access_token, org_id).await;

    // Hammer the verify endpoint with wrong codes.
    let mut final_status: StatusCode = StatusCode::OK;
    for _ in 0..11 {
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/mfa/verify")
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .header("X-Tenant-ID", org_id.to_string())
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({"code": "000000"}).to_string()))
            .unwrap();
        let resp = app.execute(req).await;
        final_status = resp.status;
    }

    assert_eq!(
        final_status,
        StatusCode::TOO_MANY_REQUESTS,
        "11th invalid TOTP code should trip rate limiting"
    );

    cleanup_test_user(&pool, &user.email).await;
}

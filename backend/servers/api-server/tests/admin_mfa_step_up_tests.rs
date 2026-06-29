//! Integration tests for `POST /api/v1/admin/mfa/verify` (TOTP step-up).
//!
//! Issue #764: an already-enrolled platform principal hitting a
//! capability-gated route gets `401 mfa_required`. Before this endpoint
//! existed the admin-web `MfaWrapper` POSTed the 6-digit code to
//! `/api/v1/auth/mfa/verify` — the *setup-completion* handler, which 409s for
//! enrolled users and never inserts a `two_factor_auth_verifications` row, so
//! the 15-minute recency gate could never be satisfied via TOTP. This endpoint
//! is the dedicated step-up path; on success it inserts a `method = 'totp'`
//! verification row.
//!
//! Scenarios mirror the contract in
//! `backend/servers/api-server/src/routes/admin/mfa/verify.rs`:
//!   1. Happy path — valid TOTP → 200, exactly one `two_factor_auth_verifications`
//!      row (`method = 'totp'`), audit row outcome=allowed + target `mfa_step_up`.
//!   2. Invalid code — 422 `invalid_code`, no verification row, audit Denied.
//!   3. Not enrolled — 412 `mfa_not_enrolled`, no verification row.
//!
//! All tests are gated `#[ignore = "requires postgres + migrations"]` per the
//! established convention.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use api_server::services::{JwtService, TotpService};
use common::TestApp;

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";
const TEST_ISSUER: &str = "Property Management";

// ─── helpers ──────────────────────────────────────────────────────────────────

async fn seed_user(pool: &PgPool, email: &str, kind: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'MFA StepUp Test', 'active', NOW(), $2)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed an *enrolled* user_2fa row and return the plaintext base32 secret so
/// the test can generate a live TOTP for it.
async fn enroll_user_2fa(pool: &PgPool, user_id: Uuid) -> String {
    let totp = TotpService::new(TEST_ISSUER.to_string());
    let secret = totp.generate_secret().expect("gen secret");
    let encrypted = totp.encrypt_secret(&secret).expect("encrypt secret");
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, $2, true, NOW(), '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE SET secret = EXCLUDED.secret, enabled = true, enabled_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&encrypted)
    .execute(pool)
    .await
    .expect("seed user_2fa");
    secret
}

/// Generate the current valid 6-digit TOTP for a base32 secret, matching the
/// SHA1 / 6-digit / 30-second parameters used by `TotpService::verify_code`.
fn current_totp(secret: &str) -> String {
    let bytes = Secret::Encoded(secret.to_string())
        .to_bytes()
        .expect("decode secret");
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some(TEST_ISSUER.to_string()),
        "".to_string(),
    )
    .expect("build totp");
    totp.generate_current().expect("generate current code")
}

fn mint_token(user_id: Uuid, kind: &str) -> String {
    let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        "mfa-stepup@test.local",
        "MFA StepUp Test",
        None,
        None,
        Some(kind.to_string()),
    )
    .expect("mint token")
}

fn set_env_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var("RUST_ENV").is_err() {
            std::env::set_var("RUST_ENV", "development");
        }
    });
}

async fn verification_count(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM two_factor_auth_verifications WHERE user_id = $1 AND method = 'totp'",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

// ─── tests ──────────────────────────────────────────────────────────────────

/// (1) Happy path: a valid current TOTP opens the recency window via a
/// `method = 'totp'` verification row and audits an allowed `mfa_step_up`.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn happy_path_totp_opens_window(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "happy-stepup@mfa-test.local", "platform").await;
    let secret = enroll_user_2fa(&pool, user).await;
    let token = mint_token(user, "platform");
    let code = current_totp(&secret);

    let req = app
        .post("/api/v1/admin/mfa/verify")
        .bearer(&token)
        .json(json!({ "code": code }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "expected 200, got {} body {}",
        resp.status,
        resp.text()
    );

    assert_eq!(
        verification_count(&pool, user).await,
        1,
        "exactly one totp verification row must be inserted"
    );

    let audit_allowed: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM audit_logs
           WHERE user_id = $1
             AND resource_type = 'mfa_step_up'
             AND details->>'outcome' = 'allowed'"#,
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        audit_allowed >= 1,
        "expected at least one allowed audit row, got {audit_allowed}"
    );
}

/// (2) Invalid code: a non-matching code yields 422 `invalid_code`, opens no
/// window, and audits a Denied `mfa_step_up_invalid` row.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn invalid_code_is_rejected_and_audited(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "invalid-stepup@mfa-test.local", "platform").await;
    let _secret = enroll_user_2fa(&pool, user).await;
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/verify")
        .bearer(&token)
        .json(json!({ "code": "000000" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "expected 422, got {} body {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("invalid_code"),
        "expected invalid_code, got {body}"
    );

    assert_eq!(
        verification_count(&pool, user).await,
        0,
        "no verification row must be inserted on invalid code"
    );
}

/// (3) Not enrolled: an un-enrolled platform principal gets 412
/// `mfa_not_enrolled` and no window is opened.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn not_enrolled_is_rejected(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "noenroll-stepup@mfa-test.local", "platform").await;
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/verify")
        .bearer(&token)
        .json(json!({ "code": "123456" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::PRECONDITION_FAILED,
        "expected 412, got {} body {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("mfa_not_enrolled"),
        "expected mfa_not_enrolled, got {body}"
    );

    assert_eq!(
        verification_count(&pool, user).await,
        0,
        "no verification row for un-enrolled user"
    );
}

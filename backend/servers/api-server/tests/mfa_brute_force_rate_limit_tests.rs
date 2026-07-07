//! MFA brute-force / rate-limit e2e tests (issue #487, follow-up to PR #473).
//!
//! A 6-digit TOTP code has only 10^6 possibilities; without per-account
//! throttling an attacker who knows (or has phished) the password can
//! enumerate the whole space in minutes. Issue #487 asked for e2e coverage
//! proving the code-entry paths are brute-force protected.
//!
//! # Why this lives at the top level of `tests/`
//!
//! PR #548 added `test_mfa_verify_rate_limited` to
//! `tests/integration/mfa_e2e_tests.rs` — but that subtree is **never
//! compiled**: nothing declares `mod integration;` and there is no
//! `tests/integration.rs` entry point, so Cargo's auto-discovery skips it
//! (see the note in `tests/auth_enumeration_tests.rs` and
//! `tests/mfa_recovery_cross_user_idor_tests.rs`). This file is a real
//! top-level test target that runs under `cargo test --workspace`.
//!
//! # What is covered
//!
//! 1. **Login with wrong TOTP codes** — each failed MFA code during login is
//!    recorded via `SessionRepository::record_login_attempt`, and
//!    `check_rate_limit` locks the email out with `429 RATE_LIMITED` after
//!    `RateLimitStatus::MAX_FAILED_ATTEMPTS` (5) failures within the
//!    15-minute window. Crucially, the lockout also rejects a *valid* code —
//!    the limiter cannot be raced/bypassed by eventually guessing right.
//! 2. **Lockout scoping** — the limiter is keyed on the email, so one user's
//!    brute-force lockout must not lock out anybody else.
//! 3. **`/api/v1/auth/mfa/verify`** — the setup-completion endpoint has no
//!    dedicated limiter yet; the `#[ignore]`d stub documents that gap (same
//!    contract as the dead-subtree stub from PR #548, now in a binary that
//!    actually compiles). Note the recovery-code path
//!    (`/api/v1/users/me/mfa/recovery-codes/verify`) *is* limited (GH #1523)
//!    and is covered by `mfa_recovery_cross_user_idor_tests.rs`.

// Each integration-test binary compiles `common` independently and uses only a
// subset of its helpers; without this the rest trip `-D dead_code` under CI's
// `RUSTFLAGS=-Dwarnings`. Matches every other test file in this directory.
#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use common::{cleanup_test_user, verify_user_email, TestApp, TestUser};
use serde_json::json;
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use api_server::services::TotpService;

/// Must match `RateLimitStatus::MAX_FAILED_ATTEMPTS` in
/// `backend/crates/db/src/models/refresh_token.rs`.
const MAX_FAILED_ATTEMPTS: usize = 5;

const TEST_ISSUER: &str = "Property Management";

// ─── helpers ─────────────────────────────────────────────────────────────────

async fn user_id_by_email(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("look up user id")
}

/// Register a user through the real endpoint, verify their email, and seed an
/// *enabled* `user_2fa` row. Returns the plaintext base32 TOTP secret so the
/// test can mint live codes. The secret is encrypted with the same
/// `TotpService` config the server uses (dev fallback key under
/// `RUST_ENV=development`, which `TestApp` guarantees), so the login handler
/// can decrypt it.
async fn register_user_with_mfa(app: &TestApp, pool: &PgPool, user: &TestUser) -> String {
    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await.assert_status(StatusCode::CREATED);
    verify_user_email(pool, &user.email).await;

    let totp = TotpService::new(TEST_ISSUER.to_string());
    let secret = totp.generate_secret().expect("generate TOTP secret");
    let encrypted = totp.encrypt_secret(&secret).expect("encrypt TOTP secret");

    let uid = user_id_by_email(pool, &user.email).await;
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, $2, true, NOW(), '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE
            SET secret = EXCLUDED.secret, enabled = true, enabled_at = NOW(),
                backup_codes = '[]'::jsonb, backup_codes_remaining = 0
        "#,
    )
    .bind(uid)
    .bind(&encrypted)
    .execute(pool)
    .await
    .expect("seed enabled user_2fa row");

    secret
}

fn totp_for(secret: &str) -> TOTP {
    let bytes = Secret::Encoded(secret.to_string())
        .to_bytes()
        .expect("decode base32 secret");
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some(TEST_ISSUER.to_string()),
        "".to_string(),
    )
    .expect("build totp")
}

/// The current valid 6-digit code (SHA1 / 6 digits / 30 s — the exact
/// parameters `TotpService::verify_code` uses).
fn current_totp(secret: &str) -> String {
    totp_for(secret)
        .generate_current()
        .expect("generate current code")
}

/// A 6-digit code guaranteed invalid for this secret, robust against the
/// server's ±1-step skew and against a window rollover mid-test: excludes the
/// codes for now ± 2 steps.
fn wrong_totp(secret: &str) -> String {
    let totp = totp_for(secret);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock before epoch")
        .as_secs();
    let live: Vec<String> = (-2i64..=2)
        .map(|offset| totp.generate((now as i64 + offset * 30) as u64))
        .collect();
    for candidate in ["000000", "111111", "222222", "333333", "444444", "555555"] {
        if !live.iter().any(|c| c == candidate) {
            return candidate.to_string();
        }
    }
    unreachable!("6 distinct candidates cannot all collide with 5 live codes")
}

fn login_body_with_code(user: &TestUser, code: &str) -> serde_json::Value {
    // `LoginRequest` is `#[serde(rename_all = "camelCase")]` — a snake_case
    // `two_factor_code` key would be silently ignored and the handler would
    // answer with the 200 `mfaRequired` gate instead of verifying the code.
    json!({
        "email": user.email,
        "password": user.password,
        "twoFactorCode": code,
    })
}

// ─── tests ───────────────────────────────────────────────────────────────────

/// Issue #487 core scenario: hammering login with the correct password but
/// wrong TOTP codes must trip the per-email rate limiter. The first
/// `MAX_FAILED_ATTEMPTS` guesses each get the generic `401 INVALID_MFA_CODE`;
/// the next attempt is rejected `429 RATE_LIMITED` *before* any verification —
/// even when it carries a perfectly valid code, so the lockout cannot be
/// bypassed by finally guessing right.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_with_wrong_totp_is_rate_limited_after_threshold(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    let secret = register_user_with_mfa(&app, &pool, &user).await;

    // Burn through the allowed window with wrong codes.
    for attempt in 1..=MAX_FAILED_ATTEMPTS {
        let resp = app
            .post("/api/v1/auth/login")
            .json(login_body_with_code(&user, &wrong_totp(&secret)))
            .send()
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "attempt {attempt}/{MAX_FAILED_ATTEMPTS} with a wrong TOTP code should be 401; body: {}",
            resp.text()
        );
        assert_eq!(
            resp.json_value()["code"],
            json!("INVALID_MFA_CODE"),
            "attempt {attempt} should surface the generic invalid-code error"
        );
    }

    // Attempt N+1 with yet another wrong code: throttled, not 401.
    let resp = app
        .post("/api/v1/auth/login")
        .json(login_body_with_code(&user, &wrong_totp(&secret)))
        .send()
        .await;
    resp.assert_status(StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        resp.json_value()["code"],
        json!("RATE_LIMITED"),
        "the {}th wrong-code login must be rate-limited",
        MAX_FAILED_ATTEMPTS + 1
    );

    // A VALID code is also refused while locked out — the limiter runs before
    // credential/TOTP verification, so brute force cannot "win" its final guess.
    let resp = app
        .post("/api/v1/auth/login")
        .json(login_body_with_code(&user, &current_totp(&secret)))
        .send()
        .await;
    resp.assert_status(StatusCode::TOO_MANY_REQUESTS);

    cleanup_test_user(&pool, &user.email).await;
}

/// The login rate limiter is keyed on the email: locking out user A by
/// brute-forcing their TOTP must not affect user B, who logs in fine with a
/// valid code while A is locked.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_totp_rate_limit_is_scoped_per_email(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let attacker_target = TestUser::new();
    let bystander = TestUser::new();
    cleanup_test_user(&pool, &attacker_target.email).await;
    cleanup_test_user(&pool, &bystander.email).await;

    let target_secret = register_user_with_mfa(&app, &pool, &attacker_target).await;
    let bystander_secret = register_user_with_mfa(&app, &pool, &bystander).await;

    // Lock out the target.
    for _ in 0..=MAX_FAILED_ATTEMPTS {
        app.post("/api/v1/auth/login")
            .json(login_body_with_code(
                &attacker_target,
                &wrong_totp(&target_secret),
            ))
            .send()
            .await;
    }
    let resp = app
        .post("/api/v1/auth/login")
        .json(login_body_with_code(
            &attacker_target,
            &wrong_totp(&target_secret),
        ))
        .send()
        .await;
    resp.assert_status(StatusCode::TOO_MANY_REQUESTS);

    // The bystander is unaffected and completes a full MFA login.
    let resp = app
        .post("/api/v1/auth/login")
        .json(login_body_with_code(
            &bystander,
            &current_totp(&bystander_secret),
        ))
        .send()
        .await;
    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["accessToken"].as_str().is_some_and(|t| !t.is_empty()),
        "bystander should receive a real access token; body: {body}"
    );

    cleanup_test_user(&pool, &attacker_target.email).await;
    cleanup_test_user(&pool, &bystander.email).await;
}

/// Sanity check for the happy path under the same fixtures: with no failed
/// history, a single login with a valid TOTP code succeeds (guards against the
/// limiter being over-eager on attempt #1).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_with_valid_totp_succeeds_without_prior_failures(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    let secret = register_user_with_mfa(&app, &pool, &user).await;

    let resp = app
        .post("/api/v1/auth/login")
        .json(login_body_with_code(&user, &current_totp(&secret)))
        .send()
        .await;
    resp.assert_status(StatusCode::OK);
    // A bare 200 is not enough: the "no code supplied" MFA gate is also a 200
    // (with empty tokens + mfaRequired=true). Require a real token so this
    // test cannot silently pass without the TOTP code being verified.
    let body = resp.json_value();
    assert!(
        body["accessToken"].as_str().is_some_and(|t| !t.is_empty()),
        "valid-TOTP login must issue a real access token; body: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// Issue #487 stub (ported from the never-compiled
/// `tests/integration/mfa_e2e_tests.rs` copy added in PR #548): the
/// setup-completion endpoint `/api/v1/auth/mfa/verify` has no dedicated
/// brute-force limiter yet. The exposure is smaller than login (the caller is
/// already authenticated and *received* the pending secret from `/setup`),
/// but a throttle is still defense-in-depth — e.g. against a confused-deputy
/// client replaying codes. Un-`#[ignore]` and keep the 429 assertion once a
/// limiter (mirroring `MFA_RECOVERY_RATE_LIMITER` in `routes/mfa.rs`, GH
/// #1523) lands on this route.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "Awaiting rate-limit on POST /api/v1/auth/mfa/verify — issue #487"]
async fn mfa_setup_verify_is_rate_limited(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    // Register + verify + login (MFA not yet enabled) to get a bearer token.
    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await.assert_status(StatusCode::CREATED);
    verify_user_email(&pool, &user.email).await;
    let login = app
        .post("/api/v1/auth/login")
        .json(user.login_body())
        .send()
        .await;
    login.assert_status(StatusCode::OK);
    let token = login.json_value()["accessToken"]
        .as_str()
        .expect("access token")
        .to_string();

    // Start MFA enrolment so there is a pending secret to verify against.
    app.post("/api/v1/auth/mfa/setup")
        .bearer(&token)
        .send()
        .await
        .assert_status(StatusCode::OK);

    // Hammer the verify endpoint with wrong codes.
    let mut final_status = StatusCode::OK;
    for _ in 0..11 {
        let resp = app
            .post("/api/v1/auth/mfa/verify")
            .bearer(&token)
            .json(json!({ "code": "000000" }))
            .send()
            .await;
        final_status = resp.status;
    }
    assert_eq!(
        final_status,
        StatusCode::TOO_MANY_REQUESTS,
        "the 11th invalid setup-verification code should trip rate limiting"
    );

    cleanup_test_user(&pool, &user.email).await;
}

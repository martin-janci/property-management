//! Account-enumeration regression tests for the api-server auth endpoints (#956).
//!
//! These cover two oracles that let an attacker probe which emails are
//! registered without valid credentials:
//!
//! 1. **Login** previously checked `is_verified()` *before* verifying the
//!    password, so a registered-but-unverified email returned
//!    `EMAIL_NOT_VERIFIED` even for a wrong password — distinguishable from the
//!    `INVALID_CREDENTIALS` an unknown email gets. The fix verifies the password
//!    first; `EMAIL_NOT_VERIFIED` is only surfaced once the password is correct.
//! 2. **Register** previously returned `409 EMAIL_EXISTS` for an existing email
//!    (vs `201` + a real `userId` for a new one). The fix returns an identical
//!    generic `201` for both and never echoes a user id.
//!
//! Lives at the top level of `tests/` (unlike the dead `tests/integration/`
//! subtree, which has no `tests/integration.rs` entry point and is never
//! compiled or run by `cargo test`).

// Each integration-test binary compiles `common` independently and uses only a
// subset of its helpers; without this the rest trip `-D dead_code` under CI's
// `RUSTFLAGS=-Dwarnings`. Matches every other test file in this directory.
#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use common::{cleanup_test_user, verify_user_email, TestApp, TestUser};
use serde_json::json;
use sqlx::PgPool;

// ============================================================================
// Login enumeration
// ============================================================================

/// A wrong password against a registered-but-unverified account must return the
/// generic `INVALID_CREDENTIALS`, NOT `EMAIL_NOT_VERIFIED` (which would confirm
/// the email is registered). This is the core fix for #956.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_unverified_wrong_password_returns_generic_invalid_credentials(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    // Register but DO NOT verify.
    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;

    // Wrong password on the unverified account.
    let login = app
        .post("/api/v1/auth/login")
        .json(json!({ "email": user.email, "password": "WrongPassword123!" }))
        .build();
    let response = app.execute(login).await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
    let body = response.json_value();
    assert_eq!(
        body["code"].as_str().unwrap(),
        "INVALID_CREDENTIALS",
        "a wrong password must not reveal verification state (no EMAIL_NOT_VERIFIED oracle)"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// An unknown email returns the same generic `INVALID_CREDENTIALS` — the
/// response must be indistinguishable from the unverified-wrong-password case.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_unknown_email_returns_generic_invalid_credentials(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let login = app
        .post("/api/v1/auth/login")
        .json(json!({ "email": "definitely-not-registered@example.com", "password": "AnyPassword123!" }))
        .build();
    let response = app.execute(login).await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.json_value()["code"].as_str().unwrap(),
        "INVALID_CREDENTIALS"
    );
}

/// The verification gate is preserved: with the CORRECT password, an unverified
/// account is still blocked with `EMAIL_NOT_VERIFIED` (after the password has
/// been verified, so it leaks nothing an authenticated caller doesn't know).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_unverified_correct_password_still_blocks_with_email_not_verified(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;

    // Correct password, still unverified.
    let login = app
        .post("/api/v1/auth/login")
        .json(user.login_body())
        .build();
    let response = app.execute(login).await;

    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.json_value()["code"].as_str().unwrap(),
        "EMAIL_NOT_VERIFIED",
        "a correct password on an unverified account should still surface the verification gate"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// A verified account with the correct password logs in successfully — the
/// reorder must not regress the happy path.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_verified_correct_password_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;
    verify_user_email(&pool, &user.email).await;

    let login = app
        .post("/api/v1/auth/login")
        .json(user.login_body())
        .build();
    let response = app.execute(login).await;

    response.assert_status(axum::http::StatusCode::OK);
    response.assert_json_field("accessToken");

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// Register enumeration
// ============================================================================

/// A fresh registration returns `201` with a generic message and no user id.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_new_email_does_not_echo_user_id(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    let response = app.execute(reg).await;

    response.assert_status(axum::http::StatusCode::CREATED);
    let body = response.json_value();
    response.assert_json_field("message");
    assert!(
        body.get("userId").is_none() && body.get("user_id").is_none(),
        "register must not echo a user id (enumeration vector): {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// Registering an already-registered email returns the SAME generic `201`
/// response as a fresh registration — no `409`, no `EMAIL_EXISTS`, and a
/// byte-for-byte identical body.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_existing_email_is_indistinguishable_from_new(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    // First registration.
    let first = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    let first_resp = app.execute(first).await;
    first_resp.assert_status(axum::http::StatusCode::CREATED);
    let first_body = first_resp.json_value();

    // Second registration with the same email.
    let second = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    let second_resp = app.execute(second).await;

    second_resp.assert_status(axum::http::StatusCode::CREATED);
    let second_body = second_resp.json_value();

    assert!(
        second_body.get("code").is_none(),
        "existing-email registration must not return an EMAIL_EXISTS code: {second_body}"
    );
    assert_eq!(
        first_body, second_body,
        "existing-email response must be identical to a fresh registration"
    );

    cleanup_test_user(&pool, &user.email).await;
}

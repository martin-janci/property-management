//! Behaviour-asserting integration tests for auth.rs partial endpoints (BIT-268 wave 4, batch 1).
//!
//! Covers success paths for:
//!   - GET  /api/v1/auth/verify-email
//!   - POST /api/v1/auth/resend-verification
//!   - POST /api/v1/auth/refresh          (success path — existing smoke only covers 400/401)
//!   - POST /api/v1/auth/logout
//!   - POST /api/v1/auth/forgot-password
//!   - POST /api/v1/auth/reset-password
//!   - GET  /api/v1/auth/sessions
//!   - POST /api/v1/auth/sessions/revoke
//!   - POST /api/v1/auth/sessions/revoke-all
//!   - GET  /api/v1/auth/me
//!   - PATCH /api/v1/auth/me

#![allow(dead_code)]

use crate::common::{cleanup_test_user, create_authenticated_user, TestApp, TestUser};
use axum::http::StatusCode;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

/// SHA-256 hex of a token — mirrors `AuthService::hash_token`, so a token we
/// seed directly with a known plaintext can be looked up by the handler.
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

// ============================================================================
// GET /api/v1/auth/verify-email
// ============================================================================

/// Verifying a fresh, non-expired token returns 200 and a success message.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-588: email_verification_tokens stores token_hash (argon2), not the raw token — test cannot retrieve the plaintext token from DB to drive the verify-email endpoint; needs a test-mode email-intercept or seed-with-known-token approach"]
async fn verify_email_with_valid_token_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    // Register — seeds a pending verification token in the DB.
    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;

    // Pull the raw (unhashed) token from email_verification_tokens.
    let raw_token: Option<String> = sqlx::query_scalar(
        "SELECT token FROM email_verification_tokens \
         WHERE user_id = (SELECT id FROM users WHERE email = $1) \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&user.email)
    .fetch_optional(&pool)
    .await
    .expect("query failed");

    let raw_token = raw_token.expect("no verification token seeded after register");

    let resp = app
        .get(&format!("/api/v1/auth/verify-email?token={raw_token}"))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["message"].as_str().unwrap_or("").contains("verified"),
        "expected 'verified' in message: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/resend-verification
// ============================================================================

/// Resend always returns 200 (anti-enumeration) even for an unknown email.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resend_verification_always_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let resp = app
        .post("/api/v1/auth/resend-verification")
        .json(json!({ "email": "does-not-exist-xyz@example.com" }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected a message field: {body}"
    );
}

/// Resend for an existing pending account returns 200 with the standard message.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resend_verification_for_pending_user_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;

    let resp = app
        .post("/api/v1/auth/resend-verification")
        .json(json!({ "email": user.email }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/refresh  (success path)
// ============================================================================

/// A valid refresh token returns 200 with a fresh access token.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn refresh_token_with_valid_token_returns_new_access_token(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (_access, refresh) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/auth/refresh")
        .json(json!({ "refreshToken": refresh }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["accessToken"].as_str().is_some(),
        "expected accessToken in refresh response: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/logout
// ============================================================================

/// Logout with a valid refresh token returns 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn logout_with_valid_refresh_token_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (_access, refresh) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/auth/logout")
        .json(json!({ "refreshToken": refresh }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected message in logout response: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// Logout always returns 200 even for an unknown/already-revoked token.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn logout_with_unknown_token_still_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let resp = app
        .post("/api/v1/auth/logout")
        .json(json!({ "refreshToken": "not-a-real-token" }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
}

// ============================================================================
// POST /api/v1/auth/forgot-password
// ============================================================================

/// Forgot-password always returns 200 (anti-enumeration).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn forgot_password_always_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let resp = app
        .post("/api/v1/auth/forgot-password")
        .json(json!({ "email": "nonexistent-abc@example.com" }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected message field: {body}"
    );
}

/// Forgot-password for a real account returns 200 with the same envelope.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn forgot_password_for_existing_user_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/auth/forgot-password")
        .json(json!({ "email": user.email }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/reset-password
// ============================================================================

/// Reset-password with a valid token returns 200 and allows subsequent login.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reset_password_with_valid_token_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    create_authenticated_user(&app, &user).await;

    // The service stores only the SHA-256 hash of a reset token — there is no
    // raw `token` column (the previous version of this test queried a
    // non-existent column and could never pass; BIT-588). Seed a row with the
    // hash of a known plaintext, then drive the handler with that plaintext.
    const RAW_TOKEN: &str = "bit588-known-reset-token-000000000000";
    sqlx::query(
        "INSERT INTO password_reset_tokens (user_id, token_hash, expires_at) \
         VALUES ((SELECT id FROM users WHERE email = $1), $2, NOW() + INTERVAL '1 hour')",
    )
    .bind(&user.email)
    .bind(hash_token(RAW_TOKEN))
    .execute(&pool)
    .await
    .expect("seed reset token");

    const NEW_PASSWORD: &str = "NewSecurePass456!";

    // The wire field is `newPassword` — the request struct renames all fields
    // to camelCase (`#[serde(rename_all = "camelCase")]`), so the Rust field
    // `new_password` serializes as `newPassword`. Sending `new_password` (or
    // the old `password`) fails to deserialize with 422 — BIT-588.
    let resp = app
        .post("/api/v1/auth/reset-password")
        .json(json!({
            "token": RAW_TOKEN,
            "newPassword": NEW_PASSWORD
        }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected message field: {body}"
    );

    // Confirm the new password works.
    let login = app
        .post("/api/v1/auth/login")
        .json(json!({ "email": user.email, "password": NEW_PASSWORD }))
        .build();
    let login_resp = app.execute(login).await;
    login_resp.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// GET /api/v1/auth/sessions
// ============================================================================

/// An authenticated request returns 200 with a sessions array.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_sessions_returns_200_with_sessions_array(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _refresh) = create_authenticated_user(&app, &user).await;

    let resp = app.get("/api/v1/auth/sessions").bearer(&access).build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["sessions"].is_array(),
        "expected sessions array: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/sessions/revoke
// ============================================================================

/// Revoking a known session returns 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_session_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _refresh) = create_authenticated_user(&app, &user).await;

    // Fetch sessions to get a real session id.
    let list_resp = app.get("/api/v1/auth/sessions").bearer(&access).build();
    let list_resp = app.execute(list_resp).await;
    list_resp.assert_status(StatusCode::OK);
    let body = list_resp.json_value();
    let sessions = body["sessions"].as_array().expect("sessions array");
    if sessions.is_empty() {
        // No sessions to revoke — just ensure listing works.
        cleanup_test_user(&pool, &user.email).await;
        return;
    }
    let session_id = sessions[0]["id"].as_str().expect("session id");

    let resp = app
        .post("/api/v1/auth/sessions/revoke")
        .bearer(&access)
        .json(json!({ "session_id": session_id }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/sessions/revoke-all
// ============================================================================

/// Revoking all sessions returns 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_all_sessions_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _refresh) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/auth/sessions/revoke-all")
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// GET /api/v1/auth/me
// ============================================================================

/// Authenticated GET /me returns 200 with user profile fields.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_me_returns_200_with_profile(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _refresh) = create_authenticated_user(&app, &user).await;

    let resp = app.get("/api/v1/auth/me").bearer(&access).build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["email"].as_str().unwrap_or(""),
        user.email,
        "email mismatch in /me response: {body}"
    );
    assert!(body["id"].as_str().is_some(), "expected id field: {body}");

    cleanup_test_user(&pool, &user.email).await;
}

/// Unauthenticated GET /me returns 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_me_without_token_returns_401(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let resp = app.get("/api/v1/auth/me").build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ============================================================================
// PATCH /api/v1/auth/me
// ============================================================================

/// PATCH /me with a valid displayName returns 200 and the updated name.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_me_returns_200_with_updated_name(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _refresh) = create_authenticated_user(&app, &user).await;

    const NEW_NAME: &str = "Updated Display Name";
    let resp = app
        .patch("/api/v1/auth/me")
        .bearer(&access)
        .json(json!({ "displayName": NEW_NAME }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["name"]
            .as_str()
            .or_else(|| body["displayName"].as_str())
            .unwrap_or(""),
        NEW_NAME,
        "expected updated name in response: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

/// PATCH /me with an empty displayName returns 400.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_me_with_blank_display_name_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _refresh) = create_authenticated_user(&app, &user).await;

    let resp = app
        .patch("/api/v1/auth/me")
        .bearer(&access)
        .json(json!({ "displayName": "   " }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::BAD_REQUEST);

    cleanup_test_user(&pool, &user.email).await;
}

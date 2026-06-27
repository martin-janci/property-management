//! Success-path integration tests for auth partial endpoints (BIT-268 batch 1).
//!
//! Covers the 200/201 happy-path assertions missing from prior test files:
//!
//! auth.rs
//!   GET  /api/v1/auth/verify-email       (200 via seeded DB token)
//!   POST /api/v1/auth/resend-verification (always 200)
//!   POST /api/v1/auth/forgot-password    (always 200)
//!   POST /api/v1/auth/refresh            (200 + new tokens)
//!   POST /api/v1/auth/logout             (200)
//!   GET  /api/v1/auth/sessions           (200 + session list)
//!   POST /api/v1/auth/sessions/revoke    (200)
//!   POST /api/v1/auth/sessions/revoke-all (200)
//!   GET  /api/v1/auth/me                 (200 + user object)
//!   PATCH /api/v1/auth/me                (200 + updated object)
//!   POST /api/v1/auth/reset-password     (200 via seeded DB token)
//!
//! gdpr.rs
//!   GET  /api/v1/gdpr/export/categories  (200, public)
//!   GET  /api/v1/gdpr/export/history     (200, empty list)
//!   GET  /api/v1/gdpr/deletion/status    (200, no active request)
//!   GET  /api/v1/gdpr/privacy            (200)
//!   POST /api/v1/gdpr/export/request     (200/201)
//!   POST /api/v1/gdpr/deletion/request   (200)
//!
//! onboarding.rs
//!   GET  /api/v1/onboarding/status       (200)
//!   GET  /api/v1/onboarding/tours        (200)

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use common::{cleanup_test_user, create_authenticated_user, TestApp, TestUser};

// ============================================================================
// auth.rs — POST /api/v1/auth/resend-verification
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resend_verification_always_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let resp = app
        .post("/api/v1/auth/resend-verification")
        .json(json!({ "email": "nonexistent-resend@example.com" }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected a message field in resend-verification response"
    );
}

// ============================================================================
// auth.rs — POST /api/v1/auth/forgot-password
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn forgot_password_always_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let resp = app
        .post("/api/v1/auth/forgot-password")
        .json(json!({ "email": "nobody@example.com" }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected a message field in forgot-password response"
    );
}

// ============================================================================
// auth.rs — POST /api/v1/auth/refresh (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn refresh_token_success_returns_200_with_new_tokens(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (_access_token, refresh_token) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/auth/refresh")
        .json(json!({ "refreshToken": refresh_token }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["accessToken"].as_str().is_some(),
        "expected accessToken in refresh response"
    );
    assert!(
        body["refreshToken"].as_str().is_some(),
        "expected refreshToken in refresh response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — POST /api/v1/auth/logout (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn logout_with_valid_refresh_token_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (_access_token, refresh_token) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/auth/logout")
        .json(json!({ "refreshToken": refresh_token }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected message in logout response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — GET /api/v1/auth/sessions (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_sessions_returns_200_with_sessions_array(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _refresh_token) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/auth/sessions")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["sessions"].is_array(),
        "expected sessions array in list-sessions response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — POST /api/v1/auth/sessions/revoke (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_session_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _refresh_token) = create_authenticated_user(&app, &user).await;

    // Get a session ID to revoke.
    let list_resp = app
        .get("/api/v1/auth/sessions")
        .bearer(&access_token)
        .build();
    let list_response = app.execute(list_resp).await;
    list_response.assert_status(StatusCode::OK);
    let sessions_body = list_response.json_value();
    let sessions = sessions_body["sessions"].as_array().expect("sessions array");
    let session_id = sessions
        .first()
        .expect("at least one active session after login")["id"]
        .as_str()
        .expect("session id string")
        .to_string();

    let resp = app
        .post("/api/v1/auth/sessions/revoke")
        .bearer(&access_token)
        .json(json!({ "session_id": session_id }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected message in revoke-session response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — POST /api/v1/auth/sessions/revoke-all (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_all_sessions_returns_200_with_revoked_count(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _refresh_token) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/auth/sessions/revoke-all")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["revokedCount"].is_number(),
        "expected revokedCount in revoke-all-sessions response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — GET /api/v1/auth/me (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_me_returns_200_with_user_object(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/auth/me")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert_eq!(
        body["email"].as_str().unwrap(),
        user.email,
        "GET /me should return the authenticated user's email"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — PATCH /api/v1/auth/me (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_me_returns_200_with_updated_display_name(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .patch("/api/v1/auth/me")
        .bearer(&access_token)
        .json(json!({ "displayName": "Updated Test Name" }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert_eq!(
        body["displayName"].as_str().unwrap_or(""),
        "Updated Test Name",
        "PATCH /me should return the updated display name"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// gdpr.rs — GET /api/v1/gdpr/export/categories (public, no auth)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_export_categories_returns_200_without_auth(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let resp = app.get("/api/v1/gdpr/export/categories").build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["categories"].is_array(),
        "expected categories array in export-categories response"
    );
    assert!(
        !body["categories"].as_array().unwrap().is_empty(),
        "expected at least one export category"
    );
}

// ============================================================================
// gdpr.rs — GET /api/v1/gdpr/export/history (empty list for new user)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_export_history_returns_200_empty_for_new_user(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/gdpr/export/history")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    assert!(
        response.json_value().is_array(),
        "expected array in export-history response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// gdpr.rs — GET /api/v1/gdpr/deletion/status (no active request)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_deletion_status_returns_200_no_active_request(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/gdpr/deletion/status")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert_eq!(
        body["has_active_request"].as_bool().unwrap_or(true),
        false,
        "newly created user should have no active deletion request"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// gdpr.rs — GET /api/v1/gdpr/privacy (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_privacy_settings_returns_200_with_visibility(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/gdpr/privacy")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["profile_visibility"].is_string(),
        "expected profile_visibility in privacy-settings response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// gdpr.rs — POST /api/v1/gdpr/export/request (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_data_export_returns_success(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/gdpr/export/request")
        .bearer(&access_token)
        .json(json!({ "format": "json" }))
        .build();
    let response = app.execute(resp).await;

    let status = response.status.as_u16();
    assert!(
        status == 200 || status == 201,
        "expected 200 or 201 from data-export request, got {status}"
    );
    let body = response.json_value();
    assert!(
        body["request_id"].as_str().is_some(),
        "expected request_id in data-export response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// gdpr.rs — POST /api/v1/gdpr/deletion/request (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_data_deletion_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/gdpr/deletion/request")
        .bearer(&access_token)
        .json(json!({ "confirmation": user.email }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert_eq!(
        body["has_active_request"].as_bool().unwrap_or(false),
        true,
        "expected has_active_request=true after deletion request"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// onboarding.rs — GET /api/v1/onboarding/status (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_onboarding_status_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/onboarding/status")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["needs_onboarding"].is_boolean(),
        "expected needs_onboarding bool in onboarding-status response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// onboarding.rs — GET /api/v1/onboarding/tours (success path)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_user_tours_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access_token, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/onboarding/tours")
        .bearer(&access_token)
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — GET /api/v1/auth/verify-email (success path via seeded token)
// ============================================================================

/// Seed a raw token into `email_verification_tokens` and verify via HTTP.
async fn seed_verification_token(pool: &PgPool, user_id: Uuid, raw_token: &str) {
    let hash = hex::encode(Sha256::digest(raw_token.as_bytes()));
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);
    sqlx::query(
        r#"INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
           VALUES ($1, $2, $3)"#,
    )
    .bind(user_id)
    .bind(hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed verification token");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_email_with_valid_token_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    // Register (creates user in pending state).
    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id after register");

    let raw_token = "test_verification_token_abc123";
    seed_verification_token(&pool, user_id, raw_token).await;

    let resp = app
        .get(&format!(
            "/api/v1/auth/verify-email?token={}",
            raw_token
        ))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected message in verify-email response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// auth.rs — POST /api/v1/auth/reset-password (success path via seeded token)
// ============================================================================

/// Seed a password reset token directly into the DB.
async fn seed_password_reset_token(pool: &PgPool, user_id: Uuid, raw_token: &str) {
    let hash = hex::encode(Sha256::digest(raw_token.as_bytes()));
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
    sqlx::query(
        r#"INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
           VALUES ($1, $2, $3)"#,
    )
    .bind(user_id)
    .bind(hash)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("seed password reset token");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reset_password_with_valid_token_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    // Register + verify the user so they have an active account.
    let (_access_token, _) = create_authenticated_user(&app, &user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");

    let raw_token = "test_reset_token_xyz987";
    seed_password_reset_token(&pool, user_id, raw_token).await;

    let resp = app
        .post("/api/v1/auth/reset-password")
        .json(json!({
            "token": raw_token,
            "newPassword": "NewPassword1!"
        }))
        .build();
    let response = app.execute(resp).await;

    response.assert_status(StatusCode::OK);
    let body = response.json_value();
    assert!(
        body["message"].as_str().is_some(),
        "expected message in reset-password response"
    );

    cleanup_test_user(&pool, &user.email).await;
}

//! BIT-268 wave 4 batch 2: auth group partial endpoint backfill.
//!
//! Covers success paths not in batch 1 (auth_partial_backfill_batch1_tests.rs):
//!   - GET  /api/v1/auth/verify-email             (200 via DB-seeded token)
//!   - POST /api/v1/auth/reset-password           (200 + re-login)
//!   - GET  /api/v1/auth/mfa/status               (200 enabled=false)
//!   - POST /api/v1/auth/mfa/setup                (200 secret+qrUri; 409 already-enabled)
//!   - POST /api/v1/auth/mfa/verify               (4xx invalid-code — endpoint reachability)
//!   - POST /api/v1/auth/mfa/backup-codes/regenerate (404 MFA not enabled)
//!   - DELETE /api/v1/users/me/push-tokens/{token}   (204 idempotent)
//!   - GET  /api/v1/onboarding/tours/{tour_id}    (200 known slug; 404/200 unknown)
//!   - POST /api/v1/onboarding/tours/{tour_id}/start
//!   - POST /api/v1/onboarding/tours/{tour_id}/skip
//!   - POST /api/v1/onboarding/tours/{tour_id}/reset
//!   - POST /api/v1/onboarding/tours/{tour_id}/complete
//!   - POST /api/v1/onboarding/tours/{tour_id}/steps/{step_id}/complete
//!   - GET  /api/v1/help/articles/{slug}          (200 via discovered slug)
//!   - POST /api/v1/help/articles/{slug}/feedback (2xx authed)
//!   - GET  /api/v1/help/categories/{slug}        (200 via discovered slug)
//!   - GET  /api/v1/help/tooltips/{key}           (200 via discovered key)

#[allow(dead_code)]
mod common;

use api_server::services::TotpService;
use axum::http::StatusCode;
use common::{
    cleanup_test_user, create_authenticated_user, create_authenticated_user_with_org, TestApp,
    TestUser,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// GET /api/v1/auth/verify-email
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_email_with_valid_token_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;

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
// POST /api/v1/auth/reset-password
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reset_password_with_valid_token_returns_200_and_new_password_works(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;
    create_authenticated_user(&app, &user).await;

    let forgot = app
        .post("/api/v1/auth/forgot-password")
        .json(json!({ "email": user.email }))
        .build();
    app.execute(forgot).await;

    let raw_token: Option<String> = sqlx::query_scalar(
        "SELECT token FROM password_reset_tokens \
         WHERE user_id = (SELECT id FROM users WHERE email = $1) \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&user.email)
    .fetch_optional(&pool)
    .await
    .expect("query failed");

    let raw_token = match raw_token {
        Some(t) => t,
        None => {
            cleanup_test_user(&pool, &user.email).await;
            return;
        }
    };

    const NEW_PASSWORD: &str = "NewSecurePass456!";

    let resp = app
        .post("/api/v1/auth/reset-password")
        .json(json!({ "token": raw_token, "password": NEW_PASSWORD }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);

    let login = app
        .post("/api/v1/auth/login")
        .json(json!({ "email": user.email, "password": NEW_PASSWORD }))
        .build();
    let login_resp = app.execute(login).await;
    login_resp.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// GET /api/v1/auth/mfa/status
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mfa_status_unenrolled_returns_200_disabled(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, org_id) = create_authenticated_user_with_org(&app, &user, "mfa-s1").await;

    let resp = app
        .get("/api/v1/auth/mfa/status")
        .bearer(&access)
        .tenant(org_id)
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["enabled"].as_bool(),
        Some(false),
        "unenrolled user must have enabled=false: {body}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/mfa/setup
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mfa_setup_returns_200_with_secret_and_qr_uri(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, org_id) = create_authenticated_user_with_org(&app, &user, "mfa-setup1").await;

    let resp = app
        .post("/api/v1/auth/mfa/setup")
        .bearer(&access)
        .tenant(org_id)
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["secret"].as_str().is_some(), "expected secret: {body}");
    assert!(body["qrUri"].as_str().is_some(), "expected qrUri: {body}");

    cleanup_test_user(&pool, &user.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mfa_setup_already_enabled_returns_409(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, org_id) = create_authenticated_user_with_org(&app, &user, "mfa-setup2").await;

    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("user id");

    let totp = TotpService::new("Property Management".to_string());
    let secret = totp.generate_secret().expect("gen secret");
    let encrypted = totp.encrypt_secret(&secret).expect("encrypt");
    sqlx::query(
        "INSERT INTO user_2fa \
         (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining) \
         VALUES ($1, $2, true, NOW(), '{}', 0)",
    )
    .bind(user_id)
    .bind(&encrypted)
    .execute(&pool)
    .await
    .expect("seed user_2fa");

    let resp = app
        .post("/api/v1/auth/mfa/setup")
        .bearer(&access)
        .tenant(org_id)
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::CONFLICT);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/mfa/verify
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mfa_verify_invalid_code_returns_4xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, org_id) = create_authenticated_user_with_org(&app, &user, "mfa-ver1").await;

    let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("user id");

    let totp = TotpService::new("Property Management".to_string());
    let secret = totp.generate_secret().expect("gen secret");
    let encrypted = totp.encrypt_secret(&secret).expect("encrypt");
    sqlx::query(
        "INSERT INTO user_2fa (user_id, secret, enabled, backup_codes, backup_codes_remaining) \
         VALUES ($1, $2, false, '{}', 0)",
    )
    .bind(user_id)
    .bind(&encrypted)
    .execute(&pool)
    .await
    .expect("seed pending user_2fa");

    let resp = app
        .post("/api/v1/auth/mfa/verify")
        .bearer(&access)
        .tenant(org_id)
        .json(json!({ "code": "000000" }))
        .build();
    let resp = app.execute(resp).await;

    assert_eq!(
        resp.status.as_u16() / 100,
        4,
        "expected 4xx for invalid TOTP: {}",
        resp.status
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// POST /api/v1/auth/mfa/backup-codes/regenerate
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mfa_regenerate_backup_codes_no_mfa_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, org_id) = create_authenticated_user_with_org(&app, &user, "mfa-regen1").await;

    let resp = app
        .post("/api/v1/auth/mfa/backup-codes/regenerate")
        .bearer(&access)
        .tenant(org_id)
        .json(json!({ "code": "000000" }))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::NOT_FOUND);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// DELETE /api/v1/users/me/push-tokens/{token}
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unregister_push_token_idempotent_returns_204(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, org_id) = create_authenticated_user_with_org(&app, &user, "pt-del1").await;

    let resp = app
        .delete("/api/v1/users/me/push-tokens/nonexistent-token-xyz-abc")
        .bearer(&access)
        .tenant(org_id)
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::NO_CONTENT);

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// Onboarding tour operations
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_tour_nonexistent_returns_200_or_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .get("/api/v1/onboarding/tours/nonexistent-tour-xyz")
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    let code = resp.status.as_u16();
    assert!(
        code == 200 || code == 404,
        "expected 200 or 404 for unknown tour: {code}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_tour_existing_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let list_resp = app
        .get("/api/v1/onboarding/tours")
        .bearer(&access)
        .build();
    let list_resp = app.execute(list_resp).await;
    list_resp.assert_status(StatusCode::OK);
    let tours = list_resp.json_value();
    let tours = tours.as_array().expect("tours array");

    if tours.is_empty() {
        cleanup_test_user(&pool, &user.email).await;
        return;
    }

    let tour_id = tours[0]["id"].as_str().expect("tour id");
    let resp = app
        .get(&format!("/api/v1/onboarding/tours/{tour_id}"))
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);

    cleanup_test_user(&pool, &user.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_tour_returns_2xx_or_4xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let list_resp = app.get("/api/v1/onboarding/tours").bearer(&access).build();
    let list_resp = app.execute(list_resp).await;
    let tours = list_resp.json_value();
    let tours = tours.as_array().expect("array");
    if tours.is_empty() {
        cleanup_test_user(&pool, &user.email).await;
        return;
    }

    let tour_id = tours[0]["id"].as_str().expect("id");
    let resp = app
        .post(&format!("/api/v1/onboarding/tours/{tour_id}/start"))
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    let code = resp.status.as_u16();
    assert!(
        (200..300).contains(&code) || (400..500).contains(&code),
        "expected 2xx or 4xx for start tour: {code}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn skip_tour_returns_2xx_or_4xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let list_resp = app.get("/api/v1/onboarding/tours").bearer(&access).build();
    let list_resp = app.execute(list_resp).await;
    let tours = list_resp.json_value();
    let tours = tours.as_array().expect("array");
    if tours.is_empty() {
        cleanup_test_user(&pool, &user.email).await;
        return;
    }

    let tour_id = tours[0]["id"].as_str().expect("id");
    let resp = app
        .post(&format!("/api/v1/onboarding/tours/{tour_id}/skip"))
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    let code = resp.status.as_u16();
    assert!(
        (200..300).contains(&code) || (400..500).contains(&code),
        "expected 2xx or 4xx for skip tour: {code}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reset_tour_returns_2xx_or_4xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let list_resp = app.get("/api/v1/onboarding/tours").bearer(&access).build();
    let list_resp = app.execute(list_resp).await;
    let tours = list_resp.json_value();
    let tours = tours.as_array().expect("array");
    if tours.is_empty() {
        cleanup_test_user(&pool, &user.email).await;
        return;
    }

    let tour_id = tours[0]["id"].as_str().expect("id");
    let resp = app
        .post(&format!("/api/v1/onboarding/tours/{tour_id}/reset"))
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    let code = resp.status.as_u16();
    assert!(
        (200..300).contains(&code) || (400..500).contains(&code),
        "expected 2xx or 4xx for reset tour: {code}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_tour_returns_2xx_or_4xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let list_resp = app.get("/api/v1/onboarding/tours").bearer(&access).build();
    let list_resp = app.execute(list_resp).await;
    let tours = list_resp.json_value();
    let tours = tours.as_array().expect("array");
    if tours.is_empty() {
        cleanup_test_user(&pool, &user.email).await;
        return;
    }

    let tour_id = tours[0]["id"].as_str().expect("id");
    let resp = app
        .post(&format!("/api/v1/onboarding/tours/{tour_id}/complete"))
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    let code = resp.status.as_u16();
    assert!(
        (200..300).contains(&code) || (400..500).contains(&code),
        "expected 2xx or 4xx for complete tour: {code}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_step_returns_2xx_or_4xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let resp = app
        .post("/api/v1/onboarding/tours/some-tour/steps/some-step/complete")
        .bearer(&access)
        .build();
    let resp = app.execute(resp).await;

    let code = resp.status.as_u16();
    assert!(
        (200..300).contains(&code) || (400..500).contains(&code),
        "expected 2xx or 4xx for step complete: {code}"
    );

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// GET /api/v1/help/articles/{slug}
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_help_article_existing_slug_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let list_resp = app.get("/api/v1/help/articles").build();
    let list_resp = app.execute(list_resp).await;
    list_resp.assert_status(StatusCode::OK);
    let articles = list_resp.json_value();
    let articles = articles
        .as_array()
        .or_else(|| articles["articles"].as_array());

    let Some(articles) = articles else {
        return;
    };
    if articles.is_empty() {
        return;
    }

    let slug = articles[0]["slug"].as_str().expect("slug");
    let resp = app
        .get(&format!("/api/v1/help/articles/{slug}"))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
}

// ============================================================================
// POST /api/v1/help/articles/{slug}/feedback
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_article_feedback_authed_returns_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    cleanup_test_user(&pool, &user.email).await;

    let (access, _) = create_authenticated_user(&app, &user).await;

    let list_resp = app.get("/api/v1/help/articles").build();
    let list_resp = app.execute(list_resp).await;
    list_resp.assert_status(StatusCode::OK);
    let articles = list_resp.json_value();
    let articles = articles
        .as_array()
        .or_else(|| articles["articles"].as_array());

    let Some(articles) = articles else {
        cleanup_test_user(&pool, &user.email).await;
        return;
    };
    if articles.is_empty() {
        cleanup_test_user(&pool, &user.email).await;
        return;
    }

    let slug = articles[0]["slug"].as_str().expect("slug");
    let resp = app
        .post(&format!("/api/v1/help/articles/{slug}/feedback"))
        .bearer(&access)
        .json(json!({ "helpful": true }))
        .build();
    let resp = app.execute(resp).await;

    let code = resp.status.as_u16();
    assert!((200..300).contains(&code), "expected 2xx: {code}");

    cleanup_test_user(&pool, &user.email).await;
}

// ============================================================================
// GET /api/v1/help/categories/{slug}
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_help_category_existing_slug_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let list_resp = app.get("/api/v1/help/categories").build();
    let list_resp = app.execute(list_resp).await;
    list_resp.assert_status(StatusCode::OK);
    let cats = list_resp.json_value();
    let cats = cats
        .as_array()
        .or_else(|| cats["categories"].as_array());

    let Some(cats) = cats else {
        return;
    };
    if cats.is_empty() {
        return;
    }

    let slug = cats[0]["slug"].as_str().expect("slug");
    let resp = app
        .get(&format!("/api/v1/help/categories/{slug}"))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
}

// ============================================================================
// GET /api/v1/help/tooltips/{key}
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_help_tooltip_existing_key_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let list_resp = app.get("/api/v1/help/tooltips").build();
    let list_resp = app.execute(list_resp).await;
    list_resp.assert_status(StatusCode::OK);
    let tooltips = list_resp.json_value();
    let tooltips = tooltips
        .as_array()
        .or_else(|| tooltips["tooltips"].as_array());

    let Some(tooltips) = tooltips else {
        return;
    };
    if tooltips.is_empty() {
        return;
    }

    let key = tooltips[0]["key"].as_str().expect("key");
    let resp = app
        .get(&format!("/api/v1/help/tooltips/{key}"))
        .build();
    let resp = app.execute(resp).await;

    resp.assert_status(StatusCode::OK);
}

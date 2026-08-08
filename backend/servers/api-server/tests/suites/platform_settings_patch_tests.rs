//! Regression test for `PATCH /api/v1/platform-admin/settings`.
//!
//! Guards the platform-settings write endpoint that unblocks the admin-web
//! `/admin/platform` Save button. Before this endpoint existed the route was
//! absent from the router, so a `PATCH` (and the sibling `GET`) returned
//! `404 Not Found` — this suite fails on that baseline (IG3) and passes once
//! the route + handler + repository land.
//!
//! Provisioning mirrors `admin_platform_happy_path_tests.rs`: a platform
//! principal, MFA-enrolled, holding a single non-MFA-gated capability grant.

#![allow(dead_code)]

use api_server::services::JwtService;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::TestApp;

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Seed a platform-principal user and return its id.
async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'Platform Settings Test', 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed platform user")
}

/// Mark the user as MFA-enrolled (the capability gate's step-2.5 wall).
async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("enroll mfa");
}

/// Grant a single named `capability` with `mfa_required = false` so the
/// extractor's recent-MFA recency check is skipped. `granted_by` must
/// reference a distinct real user (FK + app-layer no-self-grant rule).
async fn grant_capability(pool: &PgPool, user_id: Uuid, granted_by: Uuid, capability: &str) {
    sqlx::query(
        r#"
        INSERT INTO capability_grants
            (user_id, capability, granted_by, expires_at, mfa_required, note)
        VALUES ($1, $2, $3, NULL, false, 'platform-settings-patch-test')
        "#,
    )
    .bind(user_id)
    .bind(capability)
    .bind(granted_by)
    .execute(pool)
    .await
    .expect("grant capability");
}

/// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret.
fn mint_platform_token(user_id: Uuid, email: &str) -> String {
    let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        email,
        "Platform Settings Test",
        None,
        Some(vec!["super_admin".to_string()]),
        Some("platform".to_string()),
    )
    .expect("mint token")
}

/// Provision a fully-authorized platform admin holding `capability` and return
/// its bearer token.
async fn authorized_admin_token(pool: &PgPool, label: &str, capability: &str) -> String {
    let email = format!("psp-admin-{label}-{}@test.local", Uuid::new_v4());
    let granter_email = format!("psp-granter-{label}-{}@test.local", Uuid::new_v4());
    let admin = seed_platform_user(pool, &email).await;
    let granter = seed_platform_user(pool, &granter_email).await;
    enroll_mfa(pool, admin).await;
    grant_capability(pool, admin, granter, capability).await;
    mint_platform_token(admin, &email)
}

/// PATCH the platform settings, then read them back — asserting the write took
/// effect and persisted. Fails on the pre-endpoint baseline (404).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_platform_settings_persists(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let write_token = authorized_admin_token(&pool, "write", "site_settings_write").await;

    let resp = app
        .patch("/api/v1/platform-admin/settings")
        .bearer(&write_token)
        .json(json!({
            "platform.maintenance_mode": true,
            "platform.signup_enabled": false,
            "platform.support_email": "ops@example.com",
        }))
        .send()
        .await;

    assert!(
        resp.status.is_success(),
        "PATCH platform settings: expected 2xx, got {} — body: {}",
        resp.status,
        resp.text()
    );

    let body = resp.json_value();
    assert_eq!(body["platform.maintenance_mode"], json!(true), "body: {body}");
    assert_eq!(body["platform.signup_enabled"], json!(false), "body: {body}");
    assert_eq!(
        body["platform.support_email"],
        json!("ops@example.com"),
        "body: {body}"
    );

    // Read back via GET to confirm the write persisted.
    let read_token = authorized_admin_token(&pool, "read", "site_settings_read").await;
    let get_resp = app
        .get("/api/v1/platform-admin/settings")
        .bearer(&read_token)
        .send()
        .await;

    assert!(
        get_resp.status.is_success(),
        "GET platform settings: expected 2xx, got {} — body: {}",
        get_resp.status,
        get_resp.text()
    );
    let got = get_resp.json_value();
    assert_eq!(got["platform.maintenance_mode"], json!(true), "got: {got}");
    assert_eq!(got["platform.signup_enabled"], json!(false), "got: {got}");
    assert_eq!(
        got["platform.support_email"],
        json!("ops@example.com"),
        "got: {got}"
    );
}

/// A partial PATCH leaves omitted fields unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_platform_settings_partial_update(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let write_token = authorized_admin_token(&pool, "partial", "site_settings_write").await;

    // Seed a known state.
    app.patch("/api/v1/platform-admin/settings")
        .bearer(&write_token)
        .json(json!({
            "platform.maintenance_mode": true,
            "platform.signup_enabled": true,
            "platform.support_email": "first@example.com",
        }))
        .send()
        .await;

    // Patch only one field.
    let resp = app
        .patch("/api/v1/platform-admin/settings")
        .bearer(&write_token)
        .json(json!({ "platform.support_email": "second@example.com" }))
        .send()
        .await;

    assert!(
        resp.status.is_success(),
        "partial PATCH: expected 2xx, got {} — body: {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    // Untouched fields keep their prior values.
    assert_eq!(body["platform.maintenance_mode"], json!(true), "body: {body}");
    assert_eq!(body["platform.signup_enabled"], json!(true), "body: {body}");
    // The patched field changed.
    assert_eq!(
        body["platform.support_email"],
        json!("second@example.com"),
        "body: {body}"
    );
}

//! Admin / platform-admin happy-path (2xx) integration tests, batch 2 — BIT-416 (Wave 9).
//!
//! This is the additive sibling of `admin_platform_happy_path_tests.rs`
//! (BIT-357, Wave 7), which covered the read surface of the platform-admin
//! group. Batch 2 extends happy-path coverage to the **mutation** handlers
//! (create / update / toggle / delete) plus the remaining id-scoped reads that
//! Wave 7 left as `partial`:
//!
//!   - admin: `GET /principals/{id}`, `GET /users/{id}`
//!   - platform-admin reads: `GET /health/metrics/{name}/history`,
//!     `GET /support/users`
//!   - platform-admin feature flags: create, get-by-id, update, toggle,
//!     create override, delete override, delete
//!   - platform-admin announcements: create, get-by-id, update, delete
//!   - platform-admin maintenance: schedule, delete
//!   - platform-admin organizations: suspend, reactivate
//!
//! Provisioning mirrors the shipped recipe from BIT-357 / the
//! `oauth_client_registration_test.rs` pattern: a platform `principal_kind`
//! user, MFA enrolled, with `capability_grants` rows (`mfa_required = false`)
//! for exactly the capabilities each handler gates on. Every write asserts a
//! `2xx` and changes no production code.

#![allow(dead_code)]

mod common;

use api_server::services::JwtService;
use sqlx::PgPool;
use uuid::Uuid;

use common::{TestApp, TestResponse};

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

// ─── seeding helpers (mirror admin_platform_happy_path_tests.rs / BIT-357) ───

/// Seed a platform-principal user and return its id. The INSERT bypasses the
/// `BEFORE UPDATE` principal_kind guard, so `'platform'` can be set directly.
async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'BIT416 Admin Test', 'active', NOW(), 'platform')
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
/// reference a distinct real user (FK plus app-layer no-self-grant rule).
async fn grant_capability(pool: &PgPool, user_id: Uuid, granted_by: Uuid, capability: &str) {
    sqlx::query(
        r#"
        INSERT INTO capability_grants
            (user_id, capability, granted_by, expires_at, mfa_required, note)
        VALUES ($1, $2, $3, NULL, false, 'bit416-happy-path')
        "#,
    )
    .bind(user_id)
    .bind(capability)
    .bind(granted_by)
    .execute(pool)
    .await
    .expect("grant capability");
}

/// Seed an `organizations` row and return its id.
async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    let name = format!("BIT416 Org {slug}");
    let org_slug = format!("bit416-org-{slug}-{}", Uuid::new_v4());
    let email = format!("{slug}-{}@bit416.internal", Uuid::new_v4());
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(org_slug)
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Mint a platform-kind bearer JWT (super_admin role) validated by `TestApp`'s
/// JWT secret. The `super_admin` role satisfies `extract_super_admin_token`.
fn mint_platform_token(user_id: Uuid, email: &str) -> String {
    let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        email,
        "BIT416 Admin Test",
        None,
        Some(vec!["super_admin".to_string()]),
        Some("platform".to_string()),
    )
    .expect("mint token")
}

/// Provision a fully-authorized platform admin holding every capability in
/// `capabilities`, returning `(bearer_token, admin_id, granter_id)`.
/// `granter_id` is a second real platform user — usable both as the grant
/// author and as a known-existing target for id-scoped reads.
async fn authorized_admin_caps(
    pool: &PgPool,
    label: &str,
    capabilities: &[&str],
) -> (String, Uuid, Uuid) {
    let email = format!("bit416-admin-{label}-{}@test.local", Uuid::new_v4());
    let granter_email = format!("bit416-granter-{label}-{}@test.local", Uuid::new_v4());
    let admin = seed_platform_user(pool, &email).await;
    let granter = seed_platform_user(pool, &granter_email).await;
    enroll_mfa(pool, admin).await;
    for cap in capabilities {
        grant_capability(pool, admin, granter, cap).await;
    }
    (mint_platform_token(admin, &email), admin, granter)
}

/// Single-capability convenience over [`authorized_admin_caps`].
async fn authorized_admin(pool: &PgPool, label: &str, capability: &str) -> (String, Uuid, Uuid) {
    authorized_admin_caps(pool, label, &[capability]).await
}

/// Token-only convenience over [`authorized_admin`].
async fn authorized_admin_token(pool: &PgPool, label: &str, capability: &str) -> String {
    authorized_admin(pool, label, capability).await.0
}

/// Assert a response carried a `2xx`, surfacing status + body on failure.
fn assert_2xx(resp: &TestResponse, what: &str) {
    assert!(
        resp.status.is_success(),
        "{what}: expected 2xx, got {} — body: {}",
        resp.status,
        resp.text()
    );
}

/// GET `url` with `token` and assert a `2xx`.
async fn get_2xx(app: &TestApp, url: &str, token: &str, what: &str) {
    let resp = app.get(url).bearer(token).send().await;
    assert_2xx(&resp, what);
}

/// Create a feature flag via the handler and return its id (asserts 201).
async fn create_flag(app: &TestApp, token: &str, label: &str) -> String {
    let key = format!("bit416-flag-{label}-{}", Uuid::new_v4());
    let body = serde_json::json!({
        "key": key,
        "name": "BIT416 Flag",
        "description": "happy-path batch 2",
        "is_enabled": true,
    });
    let url = "/api/v1/platform-admin/feature-flags";
    let resp = app.post(url).bearer(token).json(body).send().await;
    assert_2xx(&resp, "feature_flag create");
    resp.json_value()["flag"]["id"]
        .as_str()
        .expect("flag id")
        .to_string()
}

/// Create a system announcement via the handler and return its id (asserts 201).
async fn create_announcement(app: &TestApp, token: &str) -> String {
    let body = serde_json::json!({
        "title": "BIT416 Announcement",
        "message": "happy-path batch 2",
        "severity": "info",
        "start_at": chrono::Utc::now(),
    });
    let url = "/api/v1/platform-admin/announcements";
    let resp = app.post(url).bearer(token).json(body).send().await;
    assert_2xx(&resp, "announcement create");
    resp.json_value()["announcement"]["id"]
        .as_str()
        .expect("announcement id")
        .to_string()
}

// ─── id-scoped admin reads (Wave 7 left these partial) ───────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_principals_get_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _admin, granter) = authorized_admin(&pool, "pget", "users_read").await;
    let url = format!("/api/v1/admin/principals/{granter}");
    get_2xx(&app, &url, &token, "principals_get").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_user_lifecycle_get_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _admin, granter) = authorized_admin(&pool, "uget", "users_read").await;
    let url = format!("/api/v1/admin/users/{granter}");
    get_2xx(&app, &url, &token, "user_get").await;
}

// ─── remaining platform-admin reads ──────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_metric_history_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "mh", "audit_read").await;
    let url = "/api/v1/platform-admin/health/metrics/api_latency/history?range=24h";
    get_2xx(&app, url, &token, "metric_history").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_support_users_search_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "sus", "users_read").await;
    let url = "/api/v1/platform-admin/support/users";
    get_2xx(&app, url, &token, "support_users_search").await;
}

// ─── platform-admin feature-flag mutations ───────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flag_create_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ffc", "feature_flags_write").await;
    // create_flag itself asserts the 201.
    let _id = create_flag(&app, &token, "create").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flag_get_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ffg", "feature_flags_write").await;
    let id = create_flag(&app, &token, "get").await;
    let url = format!("/api/v1/platform-admin/feature-flags/{id}");
    get_2xx(&app, &url, &token, "feature_flag_get").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flag_update_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ffu", "feature_flags_write").await;
    let id = create_flag(&app, &token, "update").await;
    let url = format!("/api/v1/platform-admin/feature-flags/{id}");
    let body = serde_json::json!({ "description": "updated by bit416", "is_enabled": false });
    let resp = app.put(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "feature_flag_update");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flag_toggle_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "fft", "feature_flags_write").await;
    let id = create_flag(&app, &token, "toggle").await;
    let url = format!("/api/v1/platform-admin/feature-flags/{id}/toggle");
    let resp = app.post(&url).bearer(&token).send().await;
    assert_2xx(&resp, "feature_flag_toggle");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flag_override_create_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ffoc", "feature_flags_write").await;
    let id = create_flag(&app, &token, "ovc").await;
    let org = seed_org(&pool, "ovc").await;
    let url = format!("/api/v1/platform-admin/feature-flags/{id}/overrides");
    let body = serde_json::json!({
        "scope_type": "organization",
        "scope_id": org,
        "is_enabled": true,
    });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "feature_flag_override_create");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flag_override_delete_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ffod", "feature_flags_write").await;
    let id = create_flag(&app, &token, "ovd").await;
    let org = seed_org(&pool, "ovd").await;
    let create_url = format!("/api/v1/platform-admin/feature-flags/{id}/overrides");
    let body = serde_json::json!({
        "scope_type": "organization",
        "scope_id": org,
        "is_enabled": true,
    });
    let create_resp = app.post(&create_url).bearer(&token).json(body).send().await;
    assert_2xx(&create_resp, "override create (setup)");
    let override_id = create_resp.json_value()["id"]
        .as_str()
        .expect("override id")
        .to_string();
    let del_url = format!("/api/v1/platform-admin/feature-flags/{id}/overrides/{override_id}");
    let resp = app.delete(&del_url).bearer(&token).send().await;
    assert_2xx(&resp, "feature_flag_override_delete");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flag_delete_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ffd", "feature_flags_write").await;
    let id = create_flag(&app, &token, "delete").await;
    let url = format!("/api/v1/platform-admin/feature-flags/{id}");
    let resp = app.delete(&url).bearer(&token).send().await;
    assert_2xx(&resp, "feature_flag_delete");
}

// ─── platform-admin announcement mutations ───────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_announcement_create_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "annc", "site_settings_write").await;
    let _id = create_announcement(&app, &token).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_announcement_get_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let caps = ["site_settings_write", "site_settings_read"];
    let (token, _admin, _granter) = authorized_admin_caps(&pool, "anng", &caps).await;
    let id = create_announcement(&app, &token).await;
    let url = format!("/api/v1/platform-admin/announcements/{id}");
    get_2xx(&app, &url, &token, "announcement_get").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_announcement_update_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "annu", "site_settings_write").await;
    let id = create_announcement(&app, &token).await;
    let url = format!("/api/v1/platform-admin/announcements/{id}");
    let body = serde_json::json!({ "title": "BIT416 Announcement (edited)" });
    let resp = app.put(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "announcement_update");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_announcement_delete_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "annd", "site_settings_write").await;
    let id = create_announcement(&app, &token).await;
    let url = format!("/api/v1/platform-admin/announcements/{id}");
    let resp = app.delete(&url).bearer(&token).send().await;
    assert_2xx(&resp, "announcement_delete");
}

// ─── platform-admin maintenance mutations ────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_maintenance_schedule_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "msc", "site_settings_write").await;
    let start = chrono::Utc::now() + chrono::Duration::hours(1);
    let end = start + chrono::Duration::hours(2);
    let body = serde_json::json!({
        "title": "BIT416 Maintenance",
        "description": "happy-path batch 2",
        "start_at": start,
        "end_at": end,
        "create_announcement": false,
    });
    let url = "/api/v1/platform-admin/maintenance";
    let resp = app.post(url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "maintenance_schedule");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_maintenance_delete_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "msd", "site_settings_write").await;
    let start = chrono::Utc::now() + chrono::Duration::hours(1);
    let end = start + chrono::Duration::hours(2);
    let body = serde_json::json!({
        "title": "BIT416 Maintenance",
        "start_at": start,
        "end_at": end,
        "create_announcement": false,
    });
    let create_url = "/api/v1/platform-admin/maintenance";
    let create_resp = app.post(create_url).bearer(&token).json(body).send().await;
    assert_2xx(&create_resp, "maintenance create (setup)");
    let maint_id = create_resp.json_value()["maintenance"]["id"]
        .as_str()
        .expect("maintenance id")
        .to_string();
    let url = format!("/api/v1/platform-admin/maintenance/{maint_id}");
    let resp = app.delete(&url).bearer(&token).send().await;
    assert_2xx(&resp, "maintenance_delete");
}

// ─── platform-admin organization lifecycle mutations ─────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_organization_suspend_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "osp", "agencies_suspend").await;
    let org = seed_org(&pool, "osp").await;
    let url = format!("/api/v1/platform-admin/organizations/{org}/suspend");
    let body = serde_json::json!({ "reason": "bit416 happy-path" });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "organization_suspend");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_organization_reactivate_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ora", "agencies_suspend").await;
    let org = seed_org(&pool, "ora").await;
    // Suspend first so the org is in the state reactivate expects.
    let susp = format!("/api/v1/platform-admin/organizations/{org}/suspend");
    let susp_body = serde_json::json!({ "reason": "bit416 setup" });
    let setup = app.post(&susp).bearer(&token).json(susp_body).send().await;
    assert_2xx(&setup, "organization suspend (setup)");
    let url = format!("/api/v1/platform-admin/organizations/{org}/reactivate");
    let body = serde_json::json!({ "note": "bit416 happy-path" });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "organization_reactivate");
}

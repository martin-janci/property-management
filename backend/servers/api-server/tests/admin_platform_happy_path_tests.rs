//! Admin / platform-admin happy-path (2xx) integration tests — BIT-357 (Wave 7).
//!
//! The platform-admin surface (`/api/v1/admin/*`, `/api/v1/platform-admin/*`)
//! was, until now, only exercised on its **authorization-denial** edge
//! (`platform_admin_authz_tests.rs` asserts 401/403 for anon / non-platform /
//! ungranted principals). This file is the additive **happy-path** half: it
//! drives a fully-provisioned platform principal (platform `principal_kind`,
//! MFA enrolled, an active `capability_grants` row) through each read
//! surface and asserts a `2xx`.
//!
//! The provisioning recipe mirrors the shipped, passing pattern in
//! `oauth_client_registration_test.rs`:
//!   1. INSERT a `users` row with `principal_kind = 'platform'` (the INSERT
//!      bypasses the BEFORE-UPDATE principal_kind guard).
//!   2. Enroll MFA (`user_2fa.enabled = true`) — the capability extractor's
//!      step-2.5 wall.
//!   3. INSERT a `capability_grants` row with `mfa_required = false` so the
//!      recent-MFA recency check is skipped (that wall is covered by the
//!      admin-mfa step-up tests; here we exercise the read handlers). The
//!      `granted_by` references a *distinct* platform user — the app layer
//!      forbids self-grant.
//!
//! These tests assert the SHIPPED behaviour against an empty (migrated) DB:
//! every targeted endpoint is a list / aggregate read that returns `2xx` with
//! an empty-but-well-formed payload. They change no production code.

#![allow(dead_code)]

mod common;

use api_server::services::JwtService;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

// ─── seeding helpers (mirror oauth_client_registration_test.rs) ─────────────

/// Seed a platform-principal user and return its id. The INSERT bypasses the
/// `BEFORE UPDATE` principal_kind guard, so `'platform'` can be set directly.
async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'BIT357 Admin Test', 'active', NOW(), 'platform')
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
        VALUES ($1, $2, $3, NULL, false, 'bit357-happy-path')
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
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("BIT357 Org {slug}"))
    .bind(format!("bit357-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@bit357.internal", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret.
fn mint_platform_token(user_id: Uuid, email: &str) -> String {
    let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        email,
        "BIT357 Admin Test",
        None,
        Some(vec!["super_admin".to_string()]),
        Some("platform".to_string()),
    )
    .expect("mint token")
}

/// Provision a fully-authorized platform admin holding `capability` and return
/// `(bearer_token, admin_id, granter_id)`. `granter_id` is a second real
/// platform user — usable as a known-existing target for id-scoped reads.
async fn authorized_admin(pool: &PgPool, label: &str, capability: &str) -> (String, Uuid, Uuid) {
    let email = format!("bit357-admin-{label}-{}@test.local", Uuid::new_v4());
    let granter_email = format!("bit357-granter-{label}-{}@test.local", Uuid::new_v4());
    let admin = seed_platform_user(pool, &email).await;
    let granter = seed_platform_user(pool, &granter_email).await;
    enroll_mfa(pool, admin).await;
    grant_capability(pool, admin, granter, capability).await;
    (mint_platform_token(admin, &email), admin, granter)
}

/// Token-only convenience over [`authorized_admin`].
async fn authorized_admin_token(pool: &PgPool, label: &str, capability: &str) -> String {
    authorized_admin(pool, label, capability).await.0
}

/// A bare platform-principal token (no capability grant) — for the bootstrap
/// `/capabilities/me` endpoint, which is gated by `RequestPrincipal` only.
async fn platform_principal_token(pool: &PgPool, label: &str) -> String {
    let email = format!("bit357-pp-{label}-{}@test.local", Uuid::new_v4());
    let id = seed_platform_user(pool, &email).await;
    mint_platform_token(id, &email)
}

/// GET `url` with `token` and assert a `2xx`, surfacing status + body on
/// failure for CI diagnostics.
async fn get_2xx(app: &TestApp, url: &str, token: &str, what: &str) {
    let resp = app.get(url).bearer(token).send().await;
    assert!(
        resp.status.is_success(),
        "{what}: expected 2xx, got {} — body: {}",
        resp.status,
        resp.text()
    );
}

// ─── /api/v1/admin/* read surface ───────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_capabilities_registry_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "reg", "audit_read").await;
    let url = "/api/v1/admin/capabilities/registry";
    get_2xx(&app, url, &token, "registry").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_capabilities_me_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = platform_principal_token(&pool, "me").await;
    let url = "/api/v1/admin/capabilities/me";
    get_2xx(&app, url, &token, "cap_me").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_capabilities_for_user_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _admin, granter) = authorized_admin(&pool, "capuser", "audit_read").await;
    let url = format!("/api/v1/admin/capabilities/users/{granter}");
    get_2xx(&app, &url, &token, "cap_user").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_audit_list_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "audit", "audit_read").await;
    let url = "/api/v1/admin/audit";
    get_2xx(&app, url, &token, "audit").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_audit_csv_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "auditcsv", "audit_read").await;
    let url = "/api/v1/admin/audit/csv";
    get_2xx(&app, url, &token, "audit_csv").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_metrics_summary_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "metrics", "audit_read").await;
    let url = "/api/v1/admin/metrics/summary";
    get_2xx(&app, url, &token, "metrics").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_notifications_analytics_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "notif", "audit_read").await;
    let url = "/api/v1/admin/notifications/analytics";
    get_2xx(&app, url, &token, "notif").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_agencies_list_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "agencies", "agencies_read").await;
    let url = "/api/v1/admin/agencies";
    get_2xx(&app, url, &token, "agencies").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_users_list_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "users", "users_read").await;
    let url = "/api/v1/admin/users";
    get_2xx(&app, url, &token, "users").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_principals_search_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "principals", "users_read").await;
    let url = "/api/v1/admin/principals";
    get_2xx(&app, url, &token, "principals").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_impersonation_active_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "imp", "users_impersonate").await;
    let url = "/api/v1/admin/impersonation/active";
    get_2xx(&app, url, &token, "imp_active").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_memberships_merge_collisions_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "merge", "memberships_grant").await;
    let url = "/api/v1/admin/memberships/merge-collisions";
    get_2xx(&app, url, &token, "merge").await;
}

// ─── /api/v1/platform-admin/* read surface ──────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_organizations_list_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "orgs", "agencies_read").await;
    let url = "/api/v1/platform-admin/organizations";
    get_2xx(&app, url, &token, "orgs").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_organization_get_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "org1", "agencies_read").await;
    let org = seed_org(&pool, "g1").await;
    let url = format!("/api/v1/platform-admin/organizations/{org}");
    get_2xx(&app, &url, &token, "org_get").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_stats_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "stats", "audit_read").await;
    let url = "/api/v1/platform-admin/stats";
    get_2xx(&app, url, &token, "stats").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_feature_flags_list_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ff", "feature_flags_write").await;
    let url = "/api/v1/platform-admin/feature-flags";
    get_2xx(&app, url, &token, "feat_flags").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_health_dashboard_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "hd", "audit_read").await;
    let url = "/api/v1/platform-admin/health/dashboard";
    get_2xx(&app, url, &token, "health_dash").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_health_alerts_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ha", "audit_read").await;
    let url = "/api/v1/platform-admin/health/alerts";
    get_2xx(&app, url, &token, "health_alerts").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_health_thresholds_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ht", "audit_read").await;
    let url = "/api/v1/platform-admin/health/thresholds";
    get_2xx(&app, url, &token, "thresholds").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_announcements_list_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ann", "site_settings_read").await;
    let url = "/api/v1/platform-admin/announcements";
    get_2xx(&app, url, &token, "announce").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_maintenance_list_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "maint", "site_settings_read").await;
    let url = "/api/v1/platform-admin/maintenance";
    get_2xx(&app, url, &token, "maint").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_support_data_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "sd", "audit_read").await;
    let url = "/api/v1/platform-admin/support-data";
    get_2xx(&app, url, &token, "support_data").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_onboarding_config_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "onb", "site_settings_read").await;
    let url = "/api/v1/platform-admin/onboarding-config";
    get_2xx(&app, url, &token, "onboarding").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_support_user_get_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _admin, granter) = authorized_admin(&pool, "su", "users_read").await;
    let url = format!("/api/v1/platform-admin/support/users/{granter}");
    get_2xx(&app, &url, &token, "su_get").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_support_user_memberships_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _admin, granter) = authorized_admin(&pool, "sm", "users_read").await;
    let url = format!("/api/v1/platform-admin/support/users/{granter}/memberships");
    get_2xx(&app, &url, &token, "su_members").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_support_user_sessions_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _admin, granter) = authorized_admin(&pool, "ss", "users_read").await;
    let url = format!("/api/v1/platform-admin/support/users/{granter}/sessions");
    get_2xx(&app, &url, &token, "su_sessions").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_support_user_activity_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _admin, granter) = authorized_admin(&pool, "sa", "audit_read").await;
    let url = format!("/api/v1/platform-admin/support/users/{granter}/activity");
    get_2xx(&app, &url, &token, "su_activity").await;
}

//! Admin / platform-admin happy-path (2xx) integration tests, batch 3 — BIT-557
//! (Wave 10).
//!
//! Additive sibling of `admin_platform_happy_path_tests.rs` (BIT-357, Wave 7 —
//! read surface) and `admin_platform_happy_path_batch2_tests.rs` (BIT-416,
//! Wave 9 — platform-admin mutations). Batch 3 promotes the remaining
//! `partial` admin mutation handlers that neither prior batch exercised:
//!
//!   - admin agencies: `GET /agencies/{id}`, `POST /agencies/{id}/suspend`,
//!     `POST /agencies/{id}/domains`
//!   - admin user lifecycle: `POST /users/{id}/suspend`, `.../reactivate`,
//!     `.../delete`
//!   - admin capabilities: `POST /capabilities/users/{id}/grant`,
//!     `DELETE /capabilities/users/{id}/grant/{grant_id}`
//!   - admin impersonation: `POST /impersonation/start`,
//!     `DELETE /impersonation/{token_id}`
//!   - admin memberships: `POST /memberships/invite`
//!   - platform-admin support: `POST /support/users/{id}/sessions/revoke`
//!   - platform-admin health: `PUT /health/thresholds/{name}`
//!
//! Provisioning mirrors the shipped BIT-357 / BIT-416 recipe: a platform
//! `principal_kind` user, MFA enrolled, with `capability_grants` rows
//! (`mfa_required = false`) for exactly the capabilities each handler gates on.
//! Every test asserts a `2xx` and changes no production code.

#![allow(dead_code)]

use api_server::services::JwtService;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{TestApp, TestResponse};

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

// ─── seeding helpers (mirror admin_platform_happy_path_batch2_tests.rs) ──────

/// Seed a platform-principal user and return its id. The INSERT bypasses the
/// `BEFORE UPDATE` principal_kind guard, so `'platform'` can be set directly.
/// Also used to seed arbitrary *target* users for id-scoped mutations.
async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'BIT557 Admin Test', 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed platform user")
}

/// Seed a plain (non-platform) active user — a valid target for suspend /
/// reactivate / delete / impersonate / session-revoke.
async fn seed_target_user(pool: &PgPool, label: &str) -> Uuid {
    let email = format!("bit557-target-{label}-{}@test.local", Uuid::new_v4());
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'h', 'BIT557 Target', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed target user")
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
        VALUES ($1, $2, $3, NULL, false, 'bit557-happy-path')
        "#,
    )
    .bind(user_id)
    .bind(capability)
    .bind(granted_by)
    .execute(pool)
    .await
    .expect("grant capability");
}

/// Seed an `organizations` row and return its id. Platform-admin treats an
/// organization as an "agency" for the `/admin/agencies/*` handlers.
async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    let name = format!("BIT557 Org {slug}");
    let org_slug = format!("bit557-org-{slug}-{}", Uuid::new_v4());
    let email = format!("{slug}-{}@bit557.internal", Uuid::new_v4());
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
        "BIT557 Admin Test",
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
    let email = format!("bit557-admin-{label}-{}@test.local", Uuid::new_v4());
    let granter_email = format!("bit557-granter-{label}-{}@test.local", Uuid::new_v4());
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

// ─── admin agencies (an "agency" is an organization row) ─────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_agency_get_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "aget", "agencies_read").await;
    let org = seed_org(&pool, "aget").await;
    let url = format!("/api/v1/admin/agencies/{org}");
    get_2xx(&app, &url, &token, "agency_get").await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_agency_suspend_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "asp", "agencies_suspend").await;
    let org = seed_org(&pool, "asp").await;
    let url = format!("/api/v1/admin/agencies/{org}/suspend");
    let body = serde_json::json!({ "reason": "bit557 happy-path" });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "agency_suspend");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_agency_add_domain_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "adom", "agencies_write").await;
    let org = seed_org(&pool, "adom").await;
    let url = format!("/api/v1/admin/agencies/{org}/domains");
    let host = format!("bit557-{}.example.com", Uuid::new_v4().simple());
    let body = serde_json::json!({ "host": host, "kind": "custom" });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "agency_add_domain");
}

// ─── admin user lifecycle mutations ──────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_user_suspend_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "usp", "users_write").await;
    let target = seed_target_user(&pool, "usp").await;
    let url = format!("/api/v1/admin/users/{target}/suspend");
    let body = serde_json::json!({ "reason": "bit557 happy-path" });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "user_suspend");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_user_reactivate_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "ureact", "users_write").await;
    let target = seed_target_user(&pool, "ureact").await;
    // Suspend first so reactivate has a suspended user to restore.
    let susp = format!("/api/v1/admin/users/{target}/suspend");
    let setup = app
        .post(&susp)
        .bearer(&token)
        .json(serde_json::json!({ "reason": "bit557 setup" }))
        .send()
        .await;
    assert_2xx(&setup, "user suspend (setup)");
    let url = format!("/api/v1/admin/users/{target}/reactivate");
    let body = serde_json::json!({ "reason": "bit557 happy-path" });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "user_reactivate");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_user_delete_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "udel", "users_write").await;
    let target = seed_target_user(&pool, "udel").await;
    let url = format!("/api/v1/admin/users/{target}/delete");
    let body = serde_json::json!({ "reason": "bit557 happy-path" });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "user_delete");
}

// ─── admin capability grant / revoke ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_capability_grant_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "cgrant", "memberships_grant").await;
    // Target has no memberships → the auth-policy grant check short-circuits Ok.
    let target = seed_target_user(&pool, "cgrant").await;
    let url = format!("/api/v1/admin/capabilities/users/{target}/grant");
    let body = serde_json::json!({ "capability": "audit_read", "mfa_required": false });
    let resp = app.post(&url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "capability_grant");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_capability_revoke_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let caps = ["memberships_grant", "memberships_revoke"];
    let token = authorized_admin_caps(&pool, "crev", &caps).await.0;
    let target = seed_target_user(&pool, "crev").await;
    // Grant via the handler so we own a real grant_id to revoke.
    let grant_url = format!("/api/v1/admin/capabilities/users/{target}/grant");
    let grant_body = serde_json::json!({ "capability": "audit_read", "mfa_required": false });
    let grant_resp = app
        .post(&grant_url)
        .bearer(&token)
        .json(grant_body)
        .send()
        .await;
    assert_2xx(&grant_resp, "capability grant (setup)");
    let grant_id = grant_resp.json_value()["id"]
        .as_str()
        .expect("grant id")
        .to_string();
    let url = format!("/api/v1/admin/capabilities/users/{target}/grant/{grant_id}");
    let resp = app.delete(&url).bearer(&token).send().await;
    assert_2xx(&resp, "capability_revoke");
}

// ─── admin impersonation start / stop ────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_impersonation_start_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "imps", "users_impersonate").await;
    let target = seed_target_user(&pool, "imps").await;
    let url = "/api/v1/admin/impersonation/start";
    let body = serde_json::json!({ "target_user_id": target, "reason": "bit557 happy-path" });
    let resp = app.post(url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "impersonation_start");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_impersonation_stop_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "impe", "users_impersonate").await;
    let target = seed_target_user(&pool, "impe").await;
    // Start a session so we have a token_id to end.
    let start = app
        .post("/api/v1/admin/impersonation/start")
        .bearer(&token)
        .json(serde_json::json!({ "target_user_id": target, "reason": "bit557 setup" }))
        .send()
        .await;
    assert_2xx(&start, "impersonation start (setup)");
    let token_id = start.json_value()["token_id"]
        .as_str()
        .expect("token id")
        .to_string();
    let url = format!("/api/v1/admin/impersonation/{token_id}");
    let resp = app.delete(&url).bearer(&token).send().await;
    assert_2xx(&resp, "impersonation_stop");
}

// ─── admin memberships invite ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn admin_membership_invite_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "inv", "memberships_grant").await;
    let org = seed_org(&pool, "inv").await;
    let email = format!("bit557-invitee-{}@test.local", Uuid::new_v4());
    let url = "/api/v1/admin/memberships/invite";
    let body = serde_json::json!({
        "email": email,
        "organization_id": org,
        "role": "member",
    });
    let resp = app.post(url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "membership_invite");
}

// ─── platform-admin support: revoke user sessions ────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_revoke_user_sessions_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "rvs", "users_write").await;
    let target = seed_target_user(&pool, "rvs").await;
    let url = format!("/api/v1/platform-admin/support/users/{target}/sessions/revoke");
    let resp = app.post(&url).bearer(&token).send().await;
    assert_2xx(&resp, "revoke_user_sessions");
}

// ─── platform-admin health: update a seeded metric threshold ─────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_update_threshold_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let token = authorized_admin_token(&pool, "uth", "site_settings_write").await;
    // `api_latency_p99` is seeded by migration 00031.
    let url = "/api/v1/platform-admin/health/thresholds/api_latency_p99";
    let body = serde_json::json!({ "warning_threshold": 600.0, "critical_threshold": 1200.0 });
    let resp = app.put(url).bearer(&token).json(body).send().await;
    assert_2xx(&resp, "update_threshold");
}

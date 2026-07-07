//! Outage (UC-12) happy-path integration tests — Wave 9, batch 3
//! (BIT-414, parent BIT-258).
//!
//! These exercise the success (`2xx`) path of every `/api/v1/outages/*`
//! endpoint over real HTTP. Prior coverage (`endpoints_smoke_tests`) only
//! asserted the unauthenticated reject (401) path for a handful of routes; this
//! file fills the create / read / mutate / lifecycle / stats gaps so each of the
//! 13 outage handlers has at least one green same-org happy-path assertion.
//!
//! # Harness
//!
//! The outage routes resolve the caller's org via `TenantExtractor` from the
//! `X-Tenant-ID` header and run under an `RlsConnection` whose
//! `get_current_org_id()` scopes every query. The mutating routes
//! (create/update/delete/start/resolve/cancel) are gated on
//! `TenantRole::is_manager()`, read from the JWT `role` claim (see the
//! "Authorization wiring" note below). `setup` seeds an active `org_admin`
//! membership — satisfying `RlsConnection` — and mints a raw token whose
//! `role = "org_admin"` (a manager-level role) satisfies every mutating gate.
//! Each request therefore carries `.bearer(token)` + `.tenant(org_id)`.
//!
//! Outages are created over HTTP in status `planned`; `start`/`resolve`/`cancel`
//! all admit a freshly-created `planned` outage, so each lifecycle test creates
//! its own outage to stay independent of transition ordering.
//!
//! # Authorization wiring (fixed — issue #2107)
//!
//! The mutating handlers now gate on `RlsConnection::role().is_manager()`, and
//! that role is the **DB-validated** `organization_members.role_type` resolved by
//! `ValidatedTenantExtractor` (which `RlsConnection` is built on) — not the JWT
//! `role` claim. Previously they gated on `TenantExtractor::role.is_manager()`,
//! read from the JWT `role` claim; but `JwtService::generate_access_token` (the
//! production login flow) emits `org_id`/`roles`, which the `AuthUser` extractor
//! never surfaces as `role`, so a real manager's login token resolved to `Guest`
//! and every mutating route 403'd in production while the raw-token tests below
//! stayed green (JWT-role vs DB-role mismatch).
//!
//! Two harnesses exercise the routes here:
//!
//! * The bulk of the tests mint a raw HS256 token (`mint_token`) carrying
//!   `tenant_id = org_id` and a `role` claim, alongside a seeded `org_admin`
//!   membership. This still authenticates and still passes after the fix
//!   (authorization now reads the DB `org_admin` membership, not the claim).
//! * `full_lifecycle_via_real_login_*` drives the **production** login flow via
//!   `create_authenticated_user_with_org` (register → verify → login →
//!   `org_admin` membership). It is the regression guard for #2107: before the
//!   fix these mutations 403'd on the login token; after it they succeed, and we
//!   assert the response-body `outage.status` transitions, not just HTTP 200.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{
    create_authenticated_user, create_authenticated_user_with_org, seed_membership, seed_org,
    TestApp, TestConfig, TestUser,
};

const BASE: &str = "/api/v1/outages";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Raw JWT claims matching the shape the `AuthUser` extractor decodes
/// (`tenant_id` + `role`), unlike the issuance service's `org_id`/`roles`.
#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

/// Mint a raw access token with a manager-level (`org_admin`) role claim scoped
/// to `org_id`, signed with the test JWT secret.
fn mint_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("org_admin".to_string()),
        email: "outages-hp@test.test".to_string(),
        name: "Outages HP User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

/// Seed an active user so the outage `created_by` FK and the membership row
/// resolve.
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Outages User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Boilerplate: app + `org_admin` (manager-level) principal with a raw JWT.
async fn setup(pool: PgPool, slug: &str) -> (TestApp, String, Uuid) {
    let app = TestApp::new(pool).await;
    let org_id = seed_org(&app.pool, slug).await;
    let email = format!("outages-{slug}-{}@test.test", Uuid::new_v4());
    let user_id = seed_user(&app.pool, &email).await;
    seed_membership(&app.pool, org_id, user_id, "org_admin").await;
    let token = mint_token(user_id, org_id);
    (app, token, org_id)
}

/// Create a `planned` outage over HTTP and return its id.
async fn create_outage(app: &TestApp, token: &str, org_id: Uuid) -> Uuid {
    let r = app
        .post(BASE)
        .bearer(token)
        .tenant(org_id)
        .json(json!({
            "title": "Planned water shutoff",
            "description": "Happy-path outage",
            "commodity": "water",
            "severity": "medium",
            "scheduled_start": "2026-06-01T08:00:00Z",
            "scheduled_end": "2026-06-01T12:00:00Z",
            "supplier_name": "City Water Co.",
        }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create outage: {}",
        resp.text()
    );
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id")).expect("uuid")
}

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_outage_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-create").await;
    let id = create_outage(&app, &token, org_id).await;
    assert!(!id.is_nil());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_outages_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-list").await;
    create_outage(&app, &token, org_id).await;

    let r = app.get(BASE).bearer(&token).tenant(org_id).build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "list: {}", resp.text());
    assert!(
        resp.json_value()["total"].as_i64().unwrap_or(0) >= 1,
        "listed outage should be counted: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_active_outages_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-active").await;
    create_outage(&app, &token, org_id).await;

    let r = app
        .get(&format!("{BASE}/active"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "active: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_outage_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-get").await;
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .get(&format!("{BASE}/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "get: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_outage_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-update").await;
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .put(&format!("{BASE}/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "title": "Updated outage title", "severity": "high" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "update: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_outage_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-delete").await;
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .delete(&format!("{BASE}/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Lifecycle (status changes)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_outage_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-start").await;
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/start"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({}))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "start: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_outage_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-resolve").await;
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/resolve"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "resolution_notes": "Service restored" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "resolve: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancel_outage_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-cancel").await;
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/cancel"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "reason": "Supplier rescheduled" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "cancel: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_read_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-read").await;
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/read"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "mark read: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Statistics & dashboard
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_statistics_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-stats").await;
    create_outage(&app, &token, org_id).await;

    let r = app
        .get(&format!("{BASE}/statistics"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "statistics: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_dashboard_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-dashboard").await;
    create_outage(&app, &token, org_id).await;

    let r = app
        .get(&format!("{BASE}/dashboard"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "dashboard: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_unread_count_succeeds(pool: PgPool) {
    let (app, token, org_id) = setup(pool, "outage-unread").await;
    create_outage(&app, &token, org_id).await;

    let r = app
        .get(&format!("{BASE}/unread-count"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "unread-count: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Regression: real login → X-Tenant-ID → mutate (issue #2107)
//
// The raw-token fixtures above passed even while production 403'd, because they
// hand-crafted a JWT carrying the `role` claim the handlers used to trust. These
// tests instead drive the *real* login flow (`create_authenticated_user_with_org`
// → register/verify/login + `org_admin` membership); the resulting access token
// has no `role` claim (`generate_access_token` emits `org_id`/`roles`), so it
// only authorizes the mutating routes once they derive the manager role from DB
// membership. They also assert the lifecycle response-body `status` transitions,
// not merely HTTP 200.
// ---------------------------------------------------------------------------

/// Read `outage.status` out of an `OutageActionResponse` body.
fn action_status(resp: &common::TestResponse) -> String {
    resp.json_value()["outage"]["status"]
        .as_str()
        .unwrap_or_else(|| panic!("missing outage.status in body: {}", resp.text()))
        .to_string()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_outage_via_real_login_succeeds(pool: PgPool) {
    // The narrowest regression: before #2107, the production login token
    // resolved to `Guest` and this create 403'd.
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "outage-login-create").await;

    let id = create_outage(&app, &token, org_id).await;
    assert!(!id.is_nil());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn full_lifecycle_via_real_login_transitions_status(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "outage-login-lifecycle").await;

    // planned → ongoing → resolved on one outage.
    let id = create_outage(&app, &token, org_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/start"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({}))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "start: {}", resp.text());
    assert_eq!(
        action_status(&resp),
        "ongoing",
        "start should set status=ongoing"
    );

    let r = app
        .post(&format!("{BASE}/{id}/resolve"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "resolution_notes": "Service restored" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "resolve: {}", resp.text());
    assert_eq!(
        action_status(&resp),
        "resolved",
        "resolve should set status=resolved"
    );

    // planned → cancelled on a second outage.
    let id2 = create_outage(&app, &token, org_id).await;
    let r = app
        .post(&format!("{BASE}/{id2}/cancel"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "reason": "Supplier rescheduled" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "cancel: {}", resp.text());
    assert_eq!(
        action_status(&resp),
        "cancelled",
        "cancel should set status=cancelled"
    );
}

// ---------------------------------------------------------------------------
// RBAC deny: real login, NON-manager membership → mutating route 403s (#2121)
//
// Follow-up to #2107 / PR #2120. The `*_via_real_login_*` tests above prove a
// real production-login token whose DB membership is `org_admin` is *allowed*
// through the `rls.role().is_manager()` gate. This is the negative complement:
// a real-login token whose DB-validated membership is a non-manager role
// (`resident`) must be *denied* on a mutating outage route.
//
// Unlike `create_authenticated_user_with_org` (which seeds an `org_admin`
// membership), we register/verify/login the user and then attach a `resident`
// membership by hand, so the caller passes `AuthUser` and
// `ValidatedTenantExtractor` (they ARE a member) but is rejected by the
// DB-role manager gate — asserting the exact `403 FORBIDDEN` real behavior so
// the test fails closed if a future change ever flips `is_manager()` open.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_outage_via_real_login_non_manager_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool).await;

    // Real login flow (register → verify → login), then attach the user to the
    // org as a NON-manager `resident` instead of the `org_admin` that
    // `create_authenticated_user_with_org` would seed.
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(&app, &user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let org_id = seed_org(&app.pool, "outage-login-deny").await;
    seed_membership(&app.pool, org_id, user_id, "resident").await;

    // POST /api/v1/outages is manager-gated on the DB-validated role. A
    // resident is a valid tenant member (passes the extractors) but must be
    // rejected by `rls.role().is_manager()` with `403 FORBIDDEN`.
    let r = app
        .post(BASE)
        .bearer(&token)
        .tenant(org_id)
        .json(json!({
            "title": "Unauthorized outage",
            "description": "Resident should not be able to create this",
            "commodity": "water",
            "severity": "medium",
            "scheduled_start": "2026-06-01T08:00:00Z",
            "scheduled_end": "2026-06-01T12:00:00Z",
            "supplier_name": "City Water Co.",
        }))
        .build();
    let resp = app.execute(r).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "#2121: a real-login resident must be denied the manager-gated create \
         outage route with 403, got {} body={}",
        resp.status,
        resp.text(),
    );

    // The handler tags this branch with the `FORBIDDEN` error code.
    let body = resp.json_value();
    let code = body
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    assert_eq!(
        code, "FORBIDDEN",
        "#2121: 403 response must carry the FORBIDDEN code, body={}",
        body
    );
}

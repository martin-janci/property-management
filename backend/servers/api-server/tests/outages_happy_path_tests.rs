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
//! # Authorization wiring
//!
//! The mutating handlers gate on `TenantExtractor::role.is_manager()`, and that
//! role is read from the **JWT `role` claim** — not the seeded DB membership
//! (`TenantExtractor` does no DB lookup). `JwtService::generate_access_token`
//! (used by the login flow) emits `org_id`/`roles` claims, which the `AuthUser`
//! extractor does *not* read, so a login token resolves to `Guest` and every
//! mutating route 403s. We therefore mint a raw HS256 token carrying
//! `tenant_id = org_id` and `role = "org_admin"`. The seeded `organization_members`
//! row is still required: `RlsConnection` validates DB membership via
//! `ValidatedTenantExtractor` and scopes RLS to `org_id`.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp, TestConfig};

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

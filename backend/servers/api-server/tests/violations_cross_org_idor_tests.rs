//! Regression tests for the cross-org IDOR fix on violation reads (#850).
//!
//! Audit history: `get_violation` in
//! `servers/api-server/src/routes/violations.rs` authenticated the caller but
//! called `get_violation(id)` whose repo query was
//! `SELECT * FROM violations WHERE id = $1` — no organization predicate. Any
//! authenticated user who knew (or guessed) another org's `violation_id` could
//! read that org's violation record (and likewise for `get_rule`, `get_action`,
//! `get_appeal`). `list_violations` correctly scoped by `organization_id`; the
//! by-id reads did not.
//!
//! The fix adds `_for_org` repository variants
//!   `SELECT * FROM violations WHERE id = $1 AND organization_id = $2`
//! and threads the verified `org_id` (JWT `tenant_id`) into them. A foreign-org
//! row therefore surfaces as `None` → HTTP 404, never a cross-tenant read.
//!
//! Tenancy here comes straight from the JWT `tenant_id` claim, which `AuthUser`
//! reads without a DB membership lookup. That lets these tests mint a real
//! access token per org and drive the handler end-to-end:
//!   - Org A's token reading Org A's violation -> 200 (same-org succeeds).
//!   - Org B's token reading Org A's violation -> 404 (cross-org blocked).

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT minting (matches api_core::extractors::auth::Claims)
// ---------------------------------------------------------------------------

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

/// Mint an access token bound to `org_id` (the JWT `tenant_id` claim), signed
/// with the same secret `TestApp` configures.
fn access_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "idor-test@violations.test".to_string(),
        name: "Violations IDOR User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("ViolationsIDOR Org {slug}"))
    .bind(format!("vio-idor-{slug}"))
    .bind(format!("{slug}@vio-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'ViolationsIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_violation(pool: &PgPool, org_id: Uuid, number: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO violations (
            organization_id, violation_number, category, title, description, occurred_at
        )
        VALUES ($1, $2, 'noise', 'Loud party', 'Excessive noise after 22:00', NOW())
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(number)
    .fetch_one(pool)
    .await
    .expect("seed violation")
}

// ---------------------------------------------------------------------------
// Test 1: same-org read succeeds
// ---------------------------------------------------------------------------

/// A user whose JWT `tenant_id` matches the violation's organization can read
/// it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_violation_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "same-org@vio-idor.test").await;
    let org_a = seed_org(&pool, "same-a").await;
    let violation_id = seed_violation(&pool, org_a, "VIO-SAME-0001").await;

    let token = access_token(user, org_a);
    let req = app
        .get(&format!("/api/v1/violations/{violation_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["id"].as_str(),
        Some(violation_id.to_string().as_str()),
        "same-org read must return the requested violation"
    );
}

// ---------------------------------------------------------------------------
// Test 2: cross-org read is blocked (404)
// ---------------------------------------------------------------------------

/// A user from Org B cannot read Org A's violation by id — the org-scoped
/// query finds no row and the handler returns 404 (the core #850 IDOR).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_violation_cross_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_b = seed_user(&pool, "attacker@vio-idor.test").await;
    let org_a = seed_org(&pool, "x-a").await;
    let org_b = seed_org(&pool, "x-b").await;
    let violation_id = seed_violation(&pool, org_a, "VIO-X-0001").await;

    // Org B token targeting Org A's violation.
    let token = access_token(user_b, org_b);
    let req = app
        .get(&format!("/api/v1/violations/{violation_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org violation read must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );
}

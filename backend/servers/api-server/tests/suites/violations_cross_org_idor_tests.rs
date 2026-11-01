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
//!
//! Coverage note: the #850 fix added `_for_org` variants for `get_rule`,
//! `get_action` and `get_appeal` as well as `get_violation`, but only
//! `get_violation` was pinned by a regression test. This suite now exercises
//! the other three sibling by-id reads with the same same-org-200 /
//! cross-org-404 contract, so a silent revert of any one of
//! `get_rule_for_org` / `get_enforcement_action_for_org` / `get_appeal_for_org`
//! back to an unscoped `WHERE id = $1` query is caught. (The *write* side of
//! the same contract lives in `violations_mutation_cross_org_idor_tests`; the
//! role gate lives in `violations_role_authz_tests` — this suite is read-only.)

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{TestApp, TestConfig};

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

async fn seed_rule(pool: &PgPool, org_id: Uuid, owner: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO community_rules (organization_id, rule_code, title, category, created_by)
        VALUES ($1, $2, 'No excessive noise', 'noise', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("NOISE-{}", &Uuid::new_v4().to_string()[..8]))
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed rule")
}

async fn seed_action(pool: &PgPool, org_id: Uuid, violation_id: Uuid, owner: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO enforcement_actions (
            violation_id, organization_id, action_type, status, fine_amount, issued_by
        )
        VALUES ($1, $2, 'first_fine', 'pending', 100.00, $3)
        RETURNING id
        "#,
    )
    .bind(violation_id)
    .bind(org_id)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed enforcement action")
}

async fn seed_appeal(
    pool: &PgPool,
    org_id: Uuid,
    violation_id: Uuid,
    action_id: Uuid,
    owner: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO violation_appeals (
            violation_id, enforcement_action_id, organization_id,
            appeal_number, status, reason, appellant_id
        )
        VALUES ($1, $2, $3, $4, 'submitted', 'I dispute this fine', $5)
        RETURNING id
        "#,
    )
    .bind(violation_id)
    .bind(action_id)
    .bind(org_id)
    .bind(format!("APP-{}", &Uuid::new_v4().to_string()[..8]))
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed appeal")
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

// ---------------------------------------------------------------------------
// get_rule — the #850 fix threads org_id via `get_rule_for_org`.
// ---------------------------------------------------------------------------

/// Same-org caller reads their own community rule by id -> 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_rule_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "rule-same@vio-idor.test").await;
    let org_a = seed_org(&pool, "rule-same-a").await;
    let rule_id = seed_rule(&pool, org_a, user).await;

    let token = access_token(user, org_a);
    let req = app
        .get(&format!("/api/v1/violations/rules/{rule_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    assert_eq!(
        resp.json_value()["id"].as_str(),
        Some(rule_id.to_string().as_str()),
        "same-org read must return the requested rule"
    );
}

/// Org B cannot read Org A's rule by id — org-scoped query finds no row (404).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_rule_cross_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let owner = seed_user(&pool, "rule-owner@vio-idor.test").await;
    let attacker = seed_user(&pool, "rule-attacker@vio-idor.test").await;
    let org_a = seed_org(&pool, "rule-x-a").await;
    let org_b = seed_org(&pool, "rule-x-b").await;
    let rule_id = seed_rule(&pool, org_a, owner).await;

    let token = access_token(attacker, org_b);
    let req = app
        .get(&format!("/api/v1/violations/rules/{rule_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org rule read must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// get_action — the #850 fix threads org_id via `get_enforcement_action_for_org`.
// ---------------------------------------------------------------------------

/// Same-org caller reads their own enforcement action by id -> 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_action_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "action-same@vio-idor.test").await;
    let org_a = seed_org(&pool, "action-same-a").await;
    let violation_id = seed_violation(&pool, org_a, "VIO-ACT-SAME-0001").await;
    let action_id = seed_action(&pool, org_a, violation_id, user).await;

    let token = access_token(user, org_a);
    let req = app
        .get(&format!(
            "/api/v1/violations/{violation_id}/actions/{action_id}"
        ))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    assert_eq!(
        resp.json_value()["id"].as_str(),
        Some(action_id.to_string().as_str()),
        "same-org read must return the requested action"
    );
}

/// Org B cannot read Org A's enforcement action by id (404). The handler
/// scopes solely by `action_id` + org (the path `violation_id` is ignored), so
/// this is the sharpest guard on `get_enforcement_action_for_org`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_action_cross_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let owner = seed_user(&pool, "action-owner@vio-idor.test").await;
    let attacker = seed_user(&pool, "action-attacker@vio-idor.test").await;
    let org_a = seed_org(&pool, "action-x-a").await;
    let org_b = seed_org(&pool, "action-x-b").await;
    let violation_id = seed_violation(&pool, org_a, "VIO-ACT-X-0001").await;
    let action_id = seed_action(&pool, org_a, violation_id, owner).await;

    let token = access_token(attacker, org_b);
    let req = app
        .get(&format!(
            "/api/v1/violations/{violation_id}/actions/{action_id}"
        ))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org action read must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// get_appeal — the #850 fix threads org_id via `get_appeal_for_org`.
// ---------------------------------------------------------------------------

/// Same-org caller reads their own appeal by id -> 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_appeal_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "appeal-same@vio-idor.test").await;
    let org_a = seed_org(&pool, "appeal-same-a").await;
    let violation_id = seed_violation(&pool, org_a, "VIO-APP-SAME-0001").await;
    let action_id = seed_action(&pool, org_a, violation_id, user).await;
    let appeal_id = seed_appeal(&pool, org_a, violation_id, action_id, user).await;

    let token = access_token(user, org_a);
    let req = app
        .get(&format!("/api/v1/violations/appeals/{appeal_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    assert_eq!(
        resp.json_value()["id"].as_str(),
        Some(appeal_id.to_string().as_str()),
        "same-org read must return the requested appeal"
    );
}

/// Org B cannot read Org A's appeal by id — org-scoped query finds no row (404).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_appeal_cross_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let owner = seed_user(&pool, "appeal-owner@vio-idor.test").await;
    let attacker = seed_user(&pool, "appeal-attacker@vio-idor.test").await;
    let org_a = seed_org(&pool, "appeal-x-a").await;
    let org_b = seed_org(&pool, "appeal-x-b").await;
    let violation_id = seed_violation(&pool, org_a, "VIO-APP-X-0001").await;
    let action_id = seed_action(&pool, org_a, violation_id, owner).await;
    let appeal_id = seed_appeal(&pool, org_a, violation_id, action_id, owner).await;

    let token = access_token(attacker, org_b);
    let req = app
        .get(&format!("/api/v1/violations/appeals/{appeal_id}"))
        .bearer(&token)
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org appeal read must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );
}

//! Cross-tenant IDOR regression tests for the **mutation** handlers on the
//! violations router (code-review finding
//! `code-review-api-handlers-violations-no-xtenant-idor-tests`).
//!
//! Audit history / gap this suite closes:
//!
//! Two sibling suites already guard the violations router, but each covers a
//! different axis and neither covers this one:
//!
//!   - `violations_cross_org_idor_tests` proves the *read* by-id handlers
//!     (`get_violation` / `get_rule` / `get_action` / `get_appeal`) are
//!     org-scoped (foreign org -> 404).
//!   - `violations_role_authz_tests` proves the privileged *mutations* reject
//!     an under-privileged role (Resident -> 403).
//!
//! What was untested: a caller who **passes** the role gate (a Manager) but
//! belongs to a *different* organization acting on another org's violation,
//! rule, enforcement action or appeal. Every mutation handler reads the org off
//! the JWT `tenant_id` and threads it into an `... WHERE id = $1 AND
//! organization_id = $2` repository call, so a foreign-org target affects no
//! rows. The contract is that this surfaces as **404** — identical to a
//! genuinely missing row — so a foreign id cannot be distinguished from a
//! non-existent one and cannot be probed, and so the mutation never lands on
//! another tenant's data (never a cross-tenant `200`, never a leaky `500`).
//!
//! Each case seeds a *real* resource in Org A (so the row genuinely exists) and
//! drives the handler with an Org A-absent **Manager** token for Org B. Using a
//! manager token is deliberate: it clears `require_manager`, isolating the
//! tenant check from the role check the sibling suite already covers. A 403
//! here would mean the role gate accidentally masks the tenancy bug; a 200/500
//! would mean the cross-tenant write leaked or errored instead of 404-ing.
//!
//! Tenancy + role both come straight from the JWT claims, which `AuthUser`
//! reads without a DB membership lookup, so these tests mint a real access
//! token per org and drive the handlers end-to-end.

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{TestApp, TestConfig, TestResponse};

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

/// Mint an access token bound to `org_id` (the JWT `tenant_id` claim) with a
/// `manager` role — enough to clear `require_manager` so the *tenant* check is
/// what each assertion exercises.
fn manager_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "xorg-manager@vio-idor.test".to_string(),
        name: "Violations XOrg Manager".to_string(),
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
// Fixtures — Org A owns every seeded resource; Org B is the attacker.
// ---------------------------------------------------------------------------

/// The seeded Org A resource graph the cross-org manager tries to reach.
struct OrgAFixture {
    violation_id: Uuid,
    action_id: Uuid,
    appeal_id: Uuid,
    rule_id: Uuid,
    evidence_id: Uuid,
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("ViolationsMutIDOR Org {slug}"))
    .bind(format!("vio-mut-idor-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@vio-mut-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Violations MutIDOR User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a complete violation graph (violation + enforcement action + appeal +
/// community rule + evidence) owned by `org_id`.
async fn seed_org_a(pool: &PgPool, org_id: Uuid, owner: Uuid) -> OrgAFixture {
    let violation_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO violations (
               organization_id, violation_number, category, title, description, occurred_at, reporter_id
           )
           VALUES ($1, $2, 'noise', 'Loud party', 'Excessive noise after 22:00', NOW(), $3)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(format!("VIO-MUT-{}", &Uuid::new_v4().to_string()[..8]))
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed violation");

    let action_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO enforcement_actions (
               violation_id, organization_id, action_type, status, fine_amount, issued_by
           )
           VALUES ($1, $2, 'first_fine', 'pending', 100.00, $3)
           RETURNING id"#,
    )
    .bind(violation_id)
    .bind(org_id)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed enforcement action");

    let appeal_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO violation_appeals (
               violation_id, enforcement_action_id, organization_id, appeal_number, status, reason, appellant_id
           )
           VALUES ($1, $2, $3, $4, 'submitted', 'I dispute this fine', $5)
           RETURNING id"#,
    )
    .bind(violation_id)
    .bind(action_id)
    .bind(org_id)
    .bind(format!("APP-MUT-{}", &Uuid::new_v4().to_string()[..8]))
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed appeal");

    let rule_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO community_rules (organization_id, rule_code, title, category, created_by)
           VALUES ($1, $2, 'No excessive noise', 'noise', $3)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(format!("NOISE-{}", &Uuid::new_v4().to_string()[..8]))
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed rule");

    let evidence_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO violation_evidence (violation_id, file_name, file_type, uploaded_by)
           VALUES ($1, 'proof.jpg', 'image/jpeg', $2)
           RETURNING id"#,
    )
    .bind(violation_id)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed evidence");

    OrgAFixture {
        violation_id,
        action_id,
        appeal_id,
        rule_id,
        evidence_id,
    }
}

/// Assert a cross-org mutation attempt is a clean 404 — never a cross-tenant
/// success (`2xx`), never a role-gate false positive (`403`), never a leaky
/// `500`.
fn assert_cross_org_404(label: &str, resp: &TestResponse) {
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "{label}: a foreign-org mutation must be 404 (not 2xx/403/500); got {} (body: {})",
        resp.status,
        resp.text()
    );
}

/// Build the two-org world: Org A owns a full violation graph, and we return an
/// Org B **manager** token to attack it with.
async fn arrange(app: &TestApp, slug: &str) -> (OrgAFixture, String) {
    let org_a = seed_org(&app.pool, &format!("a-{slug}")).await;
    let org_b = seed_org(&app.pool, &format!("b-{slug}")).await;
    let owner = seed_user(&app.pool, &format!("owner-{slug}@vio-mut-idor.test")).await;
    let attacker = seed_user(&app.pool, &format!("attacker-{slug}@vio-mut-idor.test")).await;

    let fx = seed_org_a(&app.pool, org_a, owner).await;
    let token = manager_token(attacker, org_b);
    (fx, token)
}

// ---------------------------------------------------------------------------
// Violation-level mutations
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_update_violation_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "update-vio").await;

    let resp = app
        .put(&format!("/api/v1/violations/{}", fx.violation_id))
        .bearer(&token)
        .json(json!({ "status": "under_review" }))
        .send()
        .await;

    assert_cross_org_404("update_violation", &resp);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_assign_violation_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "assign-vio").await;

    let resp = app
        .post(&format!("/api/v1/violations/{}/assign", fx.violation_id))
        .bearer(&token)
        .json(json!({ "assigned_to": Uuid::new_v4() }))
        .send()
        .await;

    assert_cross_org_404("assign_violation", &resp);
}

// ---------------------------------------------------------------------------
// Enforcement-action mutations — including the core "issue a fine on another
// org's violation" write-IDOR (must be 404, must NOT create a cross-org row).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_create_action_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (fx, token) = arrange(&app, "create-action").await;

    let resp = app
        .post(&format!("/api/v1/violations/{}/actions", fx.violation_id))
        .bearer(&token)
        .json(json!({ "action_type": "first_fine", "fine_amount": "250.00" }))
        .send()
        .await;

    assert_cross_org_404("create_action", &resp);

    // The write must not have landed: Org A's violation keeps exactly the one
    // action seeded in the fixture, with no foreign-org row injected.
    let action_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM enforcement_actions WHERE violation_id = $1")
            .bind(fx.violation_id)
            .fetch_one(&pool)
            .await
            .expect("count actions");
    assert_eq!(
        action_count, 1,
        "a cross-org fine must not be written onto Org A's violation"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_update_action_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "update-action").await;

    let resp = app
        .put(&format!(
            "/api/v1/violations/{}/actions/{}",
            fx.violation_id, fx.action_id
        ))
        .bearer(&token)
        .json(json!({ "notes": "tampered" }))
        .send()
        .await;

    assert_cross_org_404("update_action", &resp);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_mark_action_sent_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "send-action").await;

    let resp = app
        .post(&format!(
            "/api/v1/violations/{}/actions/{}/send",
            fx.violation_id, fx.action_id
        ))
        .bearer(&token)
        .json(json!({ "method": "email" }))
        .send()
        .await;

    assert_cross_org_404("mark_action_sent", &resp);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_record_payment_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "record-payment").await;

    let resp = app
        .post(&format!(
            "/api/v1/violations/{}/actions/{}/payments",
            fx.violation_id, fx.action_id
        ))
        .bearer(&token)
        .json(json!({ "amount": "50.00", "payment_method": "cash" }))
        .send()
        .await;

    assert_cross_org_404("record_payment", &resp);
}

// ---------------------------------------------------------------------------
// Appeal mutations — including the "decide another org's appeal" case.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_update_appeal_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "update-appeal").await;

    let resp = app
        .put(&format!("/api/v1/violations/appeals/{}", fx.appeal_id))
        .bearer(&token)
        .json(json!({ "decision": "tampered" }))
        .send()
        .await;

    assert_cross_org_404("update_appeal", &resp);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_decide_appeal_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (fx, token) = arrange(&app, "decide-appeal").await;

    let resp = app
        .post(&format!(
            "/api/v1/violations/appeals/{}/decide",
            fx.appeal_id
        ))
        .bearer(&token)
        .json(json!({ "approved": true, "decision": "Approved" }))
        .send()
        .await;

    assert_cross_org_404("decide_appeal", &resp);

    // The decision must not have cascaded onto Org A's appeal/violation.
    let appeal_status: String =
        sqlx::query_scalar("SELECT status::text FROM violation_appeals WHERE id = $1")
            .bind(fx.appeal_id)
            .fetch_one(&pool)
            .await
            .expect("read appeal status");
    assert_eq!(
        appeal_status, "submitted",
        "a cross-org decision must not mutate Org A's appeal"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_create_appeal_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "create-appeal").await;

    let resp = app
        .post(&format!("/api/v1/violations/{}/appeals", fx.violation_id))
        .bearer(&token)
        .json(json!({ "reason": "cross-org appeal" }))
        .send()
        .await;

    assert_cross_org_404("create_appeal", &resp);
}

// ---------------------------------------------------------------------------
// Community-rule mutations.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_update_rule_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "update-rule").await;

    let resp = app
        .put(&format!("/api/v1/violations/rules/{}", fx.rule_id))
        .bearer(&token)
        .json(json!({ "title": "tampered" }))
        .send()
        .await;

    assert_cross_org_404("update_rule", &resp);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_delete_rule_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "delete-rule").await;

    let resp = app
        .delete(&format!("/api/v1/violations/rules/{}", fx.rule_id))
        .bearer(&token)
        .send()
        .await;

    assert_cross_org_404("delete_rule", &resp);
}

// ---------------------------------------------------------------------------
// Evidence / comment mutations (open to any member — the tenancy check is the
// only gate, so these are the purest cross-org write tests).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_add_evidence_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "add-evidence").await;

    let resp = app
        .post(&format!("/api/v1/violations/{}/evidence", fx.violation_id))
        .bearer(&token)
        .json(json!({ "file_name": "forged.jpg", "file_type": "image/jpeg" }))
        .send()
        .await;

    assert_cross_org_404("add_evidence", &resp);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_delete_evidence_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "delete-evidence").await;

    let resp = app
        .delete(&format!(
            "/api/v1/violations/{}/evidence/{}",
            fx.violation_id, fx.evidence_id
        ))
        .bearer(&token)
        .send()
        .await;

    assert_cross_org_404("delete_evidence", &resp);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_add_comment_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (fx, token) = arrange(&app, "add-comment").await;

    let resp = app
        .post(&format!("/api/v1/violations/{}/comments", fx.violation_id))
        .bearer(&token)
        .json(json!({ "comment_type": "general", "content": "cross-org note" }))
        .send()
        .await;

    assert_cross_org_404("add_comment", &resp);
}

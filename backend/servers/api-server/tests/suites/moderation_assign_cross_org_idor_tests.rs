//! Cross-tenant IDOR regression tests for the **assign** mutation handler on
//! the AML/DSA content-moderation router
//! (`POST /api/v1/aml-dsa/moderation/cases/{id}/assign`).
//!
//! Audit history / gap this suite closes:
//!
//! Issue #3187 found that the violations router's `assign_violation` handler ran
//! its in-org *assignee-membership* gate (`403`) before the org-scoped existence
//! check of the path resource, so a cross-org manager supplying a foreign-org
//! assignee got `403` for another org's violation instead of the uniform
//! cross-tenant `404` the IDOR suites enforce — leaking that the foreign id was
//! distinguishable from an unknown one. PR #3188 fixed it by doing the
//! org-scoped existence check first.
//!
//! `assign_moderation_case` carried the identical anti-pattern: a
//! `require_moderator_role` role gate, then the assignee `is_member(org, …)`
//! `403` check, and only then the org-scoped `assign_moderation_case` repo call
//! (which 404s a foreign/unknown case via `RowNotFound`). A cross-org moderator
//! passing a foreign case id *and* a non-member assignee therefore short-circuited
//! to `403` rather than `404`. Its siblings (`unassign`, `take_action`,
//! `decide_appeal`) have no resource-dependent pre-check and already 404; and
//! `file_appeal` already does the existence check first — assign was the odd one
//! out, matching this sweep's remit.
//!
//! The repo-level `aml_dsa_cross_org_idor_tests` suite proves the repo method is
//! org-scoped but cannot catch the *handler's* check ordering — that only
//! manifests end-to-end, which is what this suite drives. Tenancy + role come
//! straight from the JWT claims (`AuthUser` reads them without a DB membership
//! lookup), so each case mints a real access token per org and drives the
//! handler through `TestApp` end-to-end.

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, TestApp, TestConfig, TestResponse};

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
/// `manager` role — enough to clear `require_moderator_role` so the *tenant*
/// check is what each assertion exercises.
fn manager_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "xorg-moderator@mod-idor.test".to_string(),
        name: "Moderation XOrg Manager".to_string(),
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
// Fixtures — Org A owns the moderation case; Org B is the attacker.
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("ModAssignIDOR Org {slug}"))
    .bind(format!("mod-assign-idor-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@mod-assign-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'ModAssignIDOR User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a moderation case owned by `org_id`.
async fn seed_moderation_case(pool: &PgPool, org_id: Uuid, content_owner_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO moderation_cases
               (content_type, content_id, content_owner_id, organization_id, report_source)
           VALUES ('listing', $1, $2, $3, 'user')
           RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(content_owner_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed moderation case")
}

/// Assert a cross-org mutation attempt is a clean 404 — never a cross-tenant
/// success (`2xx`), never a role/membership-gate false positive (`403`), never a
/// leaky `500`.
fn assert_cross_org_404(label: &str, resp: &TestResponse) {
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "{label}: a foreign-org mutation must be 404 (not 2xx/403/500); got {} (body: {})",
        resp.status,
        resp.text()
    );
}

/// Build the two-org world: Org A owns a moderation case, and we return an Org B
/// **manager** token to attack it with.
async fn arrange(app: &TestApp, slug: &str) -> (Uuid, Uuid, String) {
    let org_a = seed_org(&app.pool, &format!("a-{slug}")).await;
    let org_b = seed_org(&app.pool, &format!("b-{slug}")).await;
    let owner = seed_user(&app.pool, &format!("owner-{slug}@mod-assign-idor.test")).await;
    let attacker = seed_user(&app.pool, &format!("attacker-{slug}@mod-assign-idor.test")).await;

    let case_id = seed_moderation_case(&app.pool, org_a, owner).await;
    seed_membership(&app.pool, org_b, attacker, "manager").await;
    let token = manager_token(attacker, org_b);
    (case_id, attacker, token)
}

// ---------------------------------------------------------------------------
// Assign — the regression this suite guards.
// ---------------------------------------------------------------------------

/// Core regression (gh-issue-3187 sibling): a cross-org moderator passing a
/// **foreign-org / non-member** assignee for another org's case must get 404 —
/// NOT the assignee-membership 403 that fired before the fix (the existence
/// check now runs first).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_assign_moderation_case_foreign_assignee_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (case_id, _attacker, token) = arrange(&app, "assign-foreign").await;

    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{case_id}/assign"
        ))
        .bearer(&token)
        // A UUID that belongs to no org (certainly not the attacker's): pre-fix
        // this tripped the `is_member` 403 before the case was ever looked up.
        .json(json!({ "moderator_id": Uuid::new_v4() }))
        .send()
        .await;

    assert_cross_org_404("assign_moderation_case (foreign assignee)", &resp);
}

/// Positive control: self-assign (`moderator_id` omitted) skips the membership
/// gate, so it already 404'd on a foreign case before the fix. It must stay 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_assign_moderation_case_self_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let (case_id, _attacker, token) = arrange(&app, "assign-self").await;

    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{case_id}/assign"
        ))
        .bearer(&token)
        .json(json!({}))
        .send()
        .await;

    assert_cross_org_404("assign_moderation_case (self)", &resp);
}

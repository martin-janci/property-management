//! Compliance audit-logging regression for `routes/aml_dsa.rs` (PAP-42, parent
//! PAP-35 security review; FR115 production-readiness).
//!
//! The AML/EDD/DSA module previously emitted only `tracing::*` logs and **zero**
//! durable audit records, failing the PAP-35 audit-logging requirement and the
//! substantive obligations FR115 rests on (AML record-keeping; EU DSA Art. 17
//! statement-of-reasons / decision records). Each decision/state-change handler
//! now writes a `CreateAuditLog` row (actor + tenant + action + decision
//! payload) best-effort.
//!
//! These tests drive the real handlers through the mounted router with a minted
//! JWT carrying the required role (same approach as
//! `listings_global_publish_authz_tests.rs`), then assert a matching row landed
//! in `audit_logs`. They cover the two handlers named in the PAP-42 acceptance
//! criteria: `review_aml_assessment` and `take_moderation_action`.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

/// Same secret `TestConfig::default()` stamps into `JWT_SECRET`, so tokens we
/// mint here validate against the `AuthUser` extractor's cached verifier.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Minimal mirror of `api_core::extractors::auth::Claims` (the access-token
/// wire shape the extractor decodes). `role` is snake_case to match
/// `TenantRole`'s serde representation.
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

/// Mint a real access JWT for `user_id` in `org_id` with `role`
/// (snake_case, e.g. "platform_admin" / "manager").
fn mint_token(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role.to_string()),
        email: format!("{user_id}@aml-audit.test"),
        name: "AML Audit Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("AmlAudit {slug}"))
    .bind(format!("aml-audit-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@aml-audit.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'x', 'AML Audit User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_assessment(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO aml_risk_assessments (organization_id, party_id, party_type)
           VALUES ($1, $2, 'individual') RETURNING id"#,
    )
    .bind(org_id)
    .bind(Uuid::new_v4())
    .fetch_one(pool)
    .await
    .expect("seed assessment")
}

/// Seed a pending moderation case. Mirrors `compliance_repo.create_moderation_case`'s
/// required columns; remaining columns fall back to their schema defaults.
async fn seed_moderation_case(pool: &PgPool, org_id: Uuid, reported_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO moderation_cases (
               content_type, content_id, content_owner_id, organization_id,
               report_source, reported_by, violation_type, report_reason, priority
           )
           VALUES ('listing', $1, $2, $3, 'user', $4, 'spam', 'test report', 3)
           RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(reported_by)
    .fetch_one(pool)
    .await
    .expect("seed moderation case")
}

fn authed_json(method: Method, uri: &str, org: Uuid, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

/// Count audit rows matching the given resource for the actor + tenant.
async fn audit_row(pool: &PgPool, resource_type: &str, resource_id: Uuid) -> Option<Value> {
    sqlx::query_scalar::<_, Value>(
        r#"SELECT details FROM audit_logs
           WHERE resource_type = $1 AND resource_id = $2
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .bind(resource_type)
    .bind(resource_id)
    .fetch_optional(pool)
    .await
    .expect("query audit_logs")
}

// ---------------------------------------------------------------------------
// review_aml_assessment — AML approve/reject decision must be audited.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_aml_assessment_writes_audit_row(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "review").await;
    let user = seed_user(&pool, &format!("rev-{}@aml-audit.test", Uuid::new_v4())).await;
    let assessment = seed_assessment(&pool, org).await;

    // Compliance gate requires SuperAdmin/PlatformAdmin.
    let token = mint_token(user, org, "platform_admin");
    let resp = app
        .execute(authed_json(
            Method::POST,
            &format!("/api/v1/aml-dsa/aml/assessments/{assessment}/review"),
            org,
            &token,
            json!({ "decision": "approved", "notes": "verified ok" }),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "review must succeed, got {} (body: {})",
        resp.status,
        String::from_utf8_lossy(&resp.body)
    );

    let details = audit_row(&pool, "aml_assessment", assessment)
        .await
        .expect("an audit row must exist for the AML review decision");
    assert_eq!(
        details["operation"], "review_aml_assessment",
        "audit details must capture the operation, got {details}"
    );
    assert_eq!(
        details["decision"], "approved",
        "audit details must record the approve/reject decision, got {details}"
    );
}

// ---------------------------------------------------------------------------
// take_moderation_action — DSA statement-of-reasons must be audited
// (action + rationale).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn take_moderation_action_writes_audit_row(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "moderate").await;
    let user = seed_user(&pool, &format!("mod-{}@aml-audit.test", Uuid::new_v4())).await;
    let case = seed_moderation_case(&pool, org, user).await;

    // Moderation gate requires Manager or higher.
    let token = mint_token(user, org, "manager");
    let resp = app
        .execute(authed_json(
            Method::POST,
            &format!("/api/v1/aml-dsa/moderation/cases/{case}/action"),
            org,
            &token,
            // `ModerationActionType` derives serde with no `rename_all`, so its
            // wire form is the variant name verbatim ("Remove"), not snake_case.
            json!({ "action": "Remove", "rationale": "violates content policy" }),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "moderation action must succeed, got {} (body: {})",
        resp.status,
        String::from_utf8_lossy(&resp.body)
    );

    let details = audit_row(&pool, "moderation_case", case)
        .await
        .expect("an audit row must exist for the moderation decision");
    assert_eq!(
        details["operation"], "take_moderation_action",
        "audit details must capture the operation, got {details}"
    );
    // EU DSA Art. 17 statement of reasons: the action + rationale are the
    // load-bearing fields and must be persisted.
    assert_eq!(
        details["action"], "Remove",
        "audit details must record the moderation action, got {details}"
    );
    assert_eq!(
        details["rationale"], "violates content policy",
        "audit details must record the rationale (statement of reasons), got {details}"
    );
}

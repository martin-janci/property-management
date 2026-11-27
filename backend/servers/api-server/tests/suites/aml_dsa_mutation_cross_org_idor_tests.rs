//! Cross-tenant IDOR regression tests for the **mutation** handlers on the
//! AML/DSA compliance router (`/api/v1/aml-dsa/*`).
//!
//! Audit history / gap this suite closes:
//!
//! Sibling suites already guard adjacent slices of this router:
//!
//!   - `aml_dsa_cross_org_idor_tests` proves the *repository* methods
//!     (`get_aml_assessment` / `review_aml_assessment` / `get_moderation_case`
//!     / `assign_moderation_case` / `take_moderation_action` / `decide_appeal`)
//!     are org-scoped at the SQL `WHERE ... AND organization_id = $n` layer.
//!   - `moderation_assign_cross_org_idor_tests` drives the moderation *assign*
//!     handler end-to-end and asserts a foreign-org case 404s (gh-issue-3187
//!     sibling: the assignee-membership 403 must not mask the tenancy 404).
//!
//! What was untested at the **handler** layer: the AML *review* mutation
//! (`POST /api/v1/aml-dsa/aml/assessments/{id}/review`). The repo method is
//! org-scoped (so a cross-org `UPDATE ... RETURNING` matches no row and sqlx
//! yields `RowNotFound`), but the handler mapped *every* repo error — including
//! that routine `RowNotFound` — to a blanket `500 Internal Server Error`. That
//! diverges from the 403-vs-404 cross-tenant contract the rest of this router
//! (and the violations router, per gh-issue-3187) now enforces: a mutation on a
//! resource belonging to another org must be indistinguishable from a genuinely
//! missing one — **404**, never a leaky `500` and never a `2xx` that lands the
//! write. This suite locks that contract for `review_aml_assessment`.
//!
//! The caller seeds a *real* AML assessment in Org A (so the row genuinely
//! exists) and drives the handler with an Org A-absent **platform-admin** token
//! for Org B. A platform-admin role is deliberate: it clears
//! `require_compliance_role` (which admits only SuperAdmin / PlatformAdmin), so
//! the *tenant* check is what the assertion exercises, not the role gate.
//!
//! Tenancy + role both come straight from the JWT claims, which `AuthUser`
//! reads without a DB membership lookup, so these tests mint a real access
//! token per org and drive the handler end-to-end (matching the sibling
//! `moderation_assign_cross_org_idor_tests` harness).

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
/// `platform_admin` role — enough to clear `require_compliance_role` (which
/// admits only SuperAdmin / PlatformAdmin) so the *tenant* check is what each
/// assertion exercises.
fn platform_admin_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("platform_admin".to_string()),
        email: "xorg-compliance@aml-mut-idor.test".to_string(),
        name: "AML XOrg Compliance".to_string(),
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
// Fixtures — Org A owns the assessment; Org B is the attacker.
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("AmlMutIDOR Org {slug}"))
    .bind(format!("aml-mut-idor-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@aml-mut-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'AmlMutIDOR User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed an AML risk assessment owned by `org_id`.
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

/// Build the two-org world: Org A owns an AML assessment, and we return an Org B
/// **platform-admin** token to attack it with.
async fn arrange(app: &TestApp, slug: &str) -> (Uuid, String) {
    let org_a = seed_org(&app.pool, &format!("a-{slug}")).await;
    let org_b = seed_org(&app.pool, &format!("b-{slug}")).await;
    let attacker = seed_user(&app.pool, &format!("attacker-{slug}@aml-mut-idor.test")).await;

    let assessment_id = seed_assessment(&app.pool, org_a).await;
    seed_membership(&app.pool, org_b, attacker, "platform_admin").await;
    let token = platform_admin_token(attacker, org_b);
    (assessment_id, token)
}

// ---------------------------------------------------------------------------
// review_aml_assessment — the mutation this suite guards.
// ---------------------------------------------------------------------------

/// Core regression: a compliance officer in Org B reviewing an AML assessment
/// that belongs to Org A must get **404**, not the blanket `500` the handler
/// returned before it mapped the org-scoped `RowNotFound` to not-found. A 500
/// here would leak the cross-tenant write as a distinguishable server error
/// (and diverge from the router's 403-vs-404 contract); a 2xx would mean the
/// cross-org review actually landed.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_review_aml_assessment_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (assessment_id, token) = arrange(&app, "review").await;

    let resp = app
        .post(&format!(
            "/api/v1/aml-dsa/aml/assessments/{assessment_id}/review"
        ))
        .bearer(&token)
        .json(json!({ "decision": "approved" }))
        .send()
        .await;

    assert_cross_org_404("review_aml_assessment", &resp);

    // The review must not have landed: Org A's assessment keeps its original
    // (pending) status, with no cross-org approval written onto it.
    let status: String =
        sqlx::query_scalar("SELECT status::text FROM aml_risk_assessments WHERE id = $1")
            .bind(assessment_id)
            .fetch_one(&pool)
            .await
            .expect("read assessment status");
    assert_ne!(
        status, "approved",
        "a cross-org review must not mutate Org A's AML assessment"
    );
}

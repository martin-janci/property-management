//! Regression tests for the cross-tenant IDOR fix on insurance endpoints
//! (issue #826).
//!
//! Audit history: every handler on `/api/v1/insurance/*` derived the owning
//! organization from client-supplied data instead of the authenticated
//! principal:
//!
//! - `GET /policies` / `GET /claims` used
//!   `query.building_id.unwrap_or_default()` as the org id (a placeholder that
//!   resolves to `Uuid::nil()` when absent) — listed the wrong org's data, or
//!   nothing.
//! - `GET/PUT/DELETE /policies/{id}`, `GET/PUT/DELETE /claims/{id}`,
//!   `POST /claims`, `POST /claims/{id}/{submit,review,payment}`,
//!   `GET /policies/expiring`, `GET /statistics` accepted an
//!   `?organization_id=` query param / body `organization_id` field that was
//!   passed straight into the (correctly org-scoped) repository — so any
//!   caller could read or mutate another org's rows by naming a foreign org.
//!
//! The fix removes every client-supplied org id and derives the org from the
//! verified `RequestPrincipal` via `require_org_id(&principal)`.
//!
//! These tests exercise two layers:
//!
//!   1. HTTP surface — a request that claims to be from Org B (via a forged
//!      `X-Tenant-Context` header, replicating the attack) is rejected with a
//!      4xx and Org A's rows are neither read nor mutated.
//!   2. Repository layer — proves the org-scoping contract directly: Org A
//!      sees only its own policies/claims, never Org B's, and a lookup with
//!      the wrong org id returns `None`/empty (the positive "legitimate org
//!      sees its own data" case the issue asks for).
//!
//! TestApp wiring caveat (same as `equipment_cross_tenant_idor_tests`):
//! `TestApp` mounts the router without `host_tenant_middleware`, so
//! `RequestPrincipal` cannot derive a tenant from the Host header. A request
//! carrying a forged `X-Tenant-Context` header (and no bearer JWT that
//! `RequestPrincipal` would require) is rejected at the auth gate with
//! 401/403 rather than the 404 the handler returns in production. Both are
//! 4xx — the security contract (a cross-tenant request is never applied to the
//! row) holds either way. The repo-layer tests below pin the positive path
//! that the HTTP harness cannot reach.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::NaiveDate;
use db::models::{
    CreateInsuranceClaim, CreateInsurancePolicy, InsuranceClaimQuery, InsurancePolicyQuery,
};
use db::repositories::InsuranceRepository;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

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
    .bind(format!("InsIDOR Org {slug}"))
    .bind(format!("ins-idor-org-{slug}"))
    .bind(format!("{slug}@ins-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'InsIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn sample_policy(suffix: &str) -> CreateInsurancePolicy {
    CreateInsurancePolicy {
        policy_number: format!("POL-{suffix}"),
        policy_name: format!("Policy {suffix}"),
        provider_name: "Acme Insure".to_string(),
        provider_contact: None,
        provider_phone: None,
        provider_email: None,
        policy_type: "property".to_string(),
        coverage_amount: None,
        deductible: None,
        premium_amount: None,
        premium_frequency: None,
        building_id: None,
        unit_id: None,
        coverage_description: None,
        effective_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        expiration_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        renewal_date: None,
        auto_renew: None,
        terms: None,
        metadata: None,
    }
}

fn sample_claim(policy_id: Uuid, suffix: &str) -> CreateInsuranceClaim {
    CreateInsuranceClaim {
        policy_id,
        claim_number: Some(format!("CLM-{suffix}")),
        incident_date: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
        incident_description: format!("Incident {suffix}"),
        incident_location: None,
        building_id: None,
        unit_id: None,
        fault_id: None,
        claimed_amount: None,
        currency: None,
        metadata: None,
    }
}

/// Forge an `X-Tenant-Context` header claiming membership in `org_id`.
fn tenant_context_header(org_id: Uuid, user_id: Uuid) -> String {
    json!({
        "tenant_id": org_id,
        "user_id": user_id,
        "role": "OrgAdmin",
    })
    .to_string()
}

fn req(method: Method, uri: &str, ctx: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Tenant-Context", ctx);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let payload = body
        .map(|v| Body::from(v.to_string()))
        .unwrap_or_else(Body::empty);
    b.body(payload).unwrap()
}

/// Assert that the response is a rejection (any 4xx).
fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: cross-tenant request must be rejected with 4xx, got {status}"
    );
}

// ---------------------------------------------------------------------------
// H1 — HTTP: list_policies cross-tenant (the P0 finding)
// ---------------------------------------------------------------------------

/// GET /policies with a forged Org-B context targeting Org A's data → rejected.
/// The pre-fix handler used `building_id.unwrap_or_default()` as the org, so it
/// never consulted the principal at all.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_policies_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "lst-pol-a").await;
    let org_b = seed_org(&pool, "lst-pol-b").await;
    let user_b = seed_user(&pool, "lst-pol-b@ins-idor.test").await;

    let repo = InsuranceRepository::new(pool.clone());
    repo.create_policy(&pool, org_a, sample_policy("a1"))
        .await
        .expect("seed policy in org A");

    let ctx_b = tenant_context_header(org_b, user_b);
    let response = app
        .execute(req(Method::GET, "/api/v1/insurance/policies", &ctx_b, None))
        .await;

    assert_rejected(response.status, "list_policies cross-tenant");
}

// ---------------------------------------------------------------------------
// H2 — HTTP: get_policy cross-tenant (information disclosure / IDOR)
// ---------------------------------------------------------------------------

/// GET /policies/{id} naming Org A's policy from an Org-B context → rejected,
/// Org A's policy is not disclosed.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_policy_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "get-pol-a").await;
    let org_b = seed_org(&pool, "get-pol-b").await;
    let user_b = seed_user(&pool, "get-pol-b@ins-idor.test").await;

    let repo = InsuranceRepository::new(pool.clone());
    let policy_in_a = repo
        .create_policy(&pool, org_a, sample_policy("a2"))
        .await
        .expect("seed policy in org A");

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/insurance/policies/{}", policy_in_a.id);
    let response = app.execute(req(Method::GET, &uri, &ctx_b, None)).await;

    assert_rejected(response.status, "get_policy cross-tenant");
}

// ---------------------------------------------------------------------------
// H3 — HTTP: delete_policy cross-tenant (mutation must not apply)
// ---------------------------------------------------------------------------

/// DELETE /policies/{id} from an Org-B context targeting Org A's policy →
/// rejected, and the row still exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_policy_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "del-pol-a").await;
    let org_b = seed_org(&pool, "del-pol-b").await;
    let user_b = seed_user(&pool, "del-pol-b@ins-idor.test").await;

    let repo = InsuranceRepository::new(pool.clone());
    let policy_in_a = repo
        .create_policy(&pool, org_a, sample_policy("a3"))
        .await
        .expect("seed policy in org A");

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/insurance/policies/{}", policy_in_a.id);
    let response = app.execute(req(Method::DELETE, &uri, &ctx_b, None)).await;

    assert_rejected(response.status, "delete_policy cross-tenant");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM insurance_policies WHERE id = $1")
        .bind(policy_in_a.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "policy must survive cross-tenant DELETE");
}

// ---------------------------------------------------------------------------
// H4 — HTTP: list_claims cross-tenant (the P0 finding)
// ---------------------------------------------------------------------------

/// GET /claims with a forged Org-B context → rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_claims_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "lst-clm-a").await;
    let org_b = seed_org(&pool, "lst-clm-b").await;
    let user_a = seed_user(&pool, "lst-clm-a@ins-idor.test").await;
    let user_b = seed_user(&pool, "lst-clm-b@ins-idor.test").await;

    let repo = InsuranceRepository::new(pool.clone());
    let policy_in_a = repo
        .create_policy(&pool, org_a, sample_policy("a4"))
        .await
        .expect("seed policy in org A");
    repo.create_claim(&pool, org_a, user_a, sample_claim(policy_in_a.id, "a4"))
        .await
        .expect("seed claim in org A");

    let ctx_b = tenant_context_header(org_b, user_b);
    let response = app
        .execute(req(Method::GET, "/api/v1/insurance/claims", &ctx_b, None))
        .await;

    assert_rejected(response.status, "list_claims cross-tenant");
}

// ---------------------------------------------------------------------------
// H5 — Repository: org A sees only its own policies (positive + negative)
// ---------------------------------------------------------------------------

/// The repository is the authority the handler now scopes to. This pins the
/// contract directly so the "legitimate org sees its own data, and only its
/// own" guarantee is regression-tested independent of the HTTP harness.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repo_policies_are_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "repo-pol-a").await;
    let org_b = seed_org(&pool, "repo-pol-b").await;

    let repo = InsuranceRepository::new(pool.clone());
    let policy_a = repo
        .create_policy(&pool, org_a, sample_policy("repo-a"))
        .await
        .expect("policy A");
    let _policy_b = repo
        .create_policy(&pool, org_b, sample_policy("repo-b"))
        .await
        .expect("policy B");

    // Org A's list contains only org A's policy.
    let listed_a = repo
        .list_policies(&pool, org_a, InsurancePolicyQuery::default())
        .await
        .expect("list A");
    assert_eq!(listed_a.len(), 1, "org A must see exactly its own policy");
    assert_eq!(listed_a[0].id, policy_a.id);

    // Org A's list never contains org B's policy.
    assert!(
        listed_a.iter().all(|p| p.organization_id == org_a),
        "org A's policy list must not leak org B rows"
    );

    // Looking up org A's policy with org B's id returns nothing (closed IDOR).
    let cross = repo
        .find_policy_by_id(&pool, org_b, policy_a.id)
        .await
        .expect("find with wrong org");
    assert!(
        cross.is_none(),
        "org B must not be able to read org A's policy by id"
    );

    // The legitimate owner can read it.
    let own = repo
        .find_policy_by_id(&pool, org_a, policy_a.id)
        .await
        .expect("find with right org");
    assert!(own.is_some(), "org A must read its own policy by id");
}

// ---------------------------------------------------------------------------
// H6 — Repository: org A sees only its own claims (positive + negative)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repo_claims_are_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "repo-clm-a").await;
    let org_b = seed_org(&pool, "repo-clm-b").await;
    let user_a = seed_user(&pool, "repo-clm-a@ins-idor.test").await;
    let user_b = seed_user(&pool, "repo-clm-b@ins-idor.test").await;

    let repo = InsuranceRepository::new(pool.clone());
    let policy_a = repo
        .create_policy(&pool, org_a, sample_policy("repo-clm-a"))
        .await
        .expect("policy A");
    let policy_b = repo
        .create_policy(&pool, org_b, sample_policy("repo-clm-b"))
        .await
        .expect("policy B");
    let claim_a = repo
        .create_claim(&pool, org_a, user_a, sample_claim(policy_a.id, "repo-a"))
        .await
        .expect("claim A");
    let _claim_b = repo
        .create_claim(&pool, org_b, user_b, sample_claim(policy_b.id, "repo-b"))
        .await
        .expect("claim B");

    // Org A's claim list contains only org A's claim.
    let listed_a = repo
        .list_claims(&pool, org_a, InsuranceClaimQuery::default())
        .await
        .expect("list A");
    assert_eq!(listed_a.len(), 1, "org A must see exactly its own claim");
    assert!(
        listed_a.iter().all(|c| c.organization_id == org_a),
        "org A's claim list must not leak org B rows"
    );

    // Org B cannot read org A's claim by id.
    let cross = repo
        .find_claim_with_policy(&pool, org_b, claim_a.id)
        .await
        .expect("find with wrong org");
    assert!(
        cross.is_none(),
        "org B must not be able to read org A's claim by id"
    );

    // The legitimate owner can.
    let own = repo
        .find_claim_with_policy(&pool, org_a, claim_a.id)
        .await
        .expect("find with right org");
    assert!(own.is_some(), "org A must read its own claim by id");
}

// ---------------------------------------------------------------------------
// H7 — HTTP positive path: a legitimately tenant-resolved caller sees its
//      OWN org's policies (the IG3 fail-on-main case).
// ---------------------------------------------------------------------------
//
// This is the end-to-end proof that the handler now derives the org from the
// authenticated principal via `RlsConnection`. We mint a real JWT for a user
// who is an active `organization_members` member of org A (the membership table
// `ValidatedTenantExtractor` checks), pass `X-Tenant-ID: org_a` (the canonical
// tenant-resolution path under `TestApp`, which mounts no `host_tenant_middleware`),
// and seed one policy in org A.
//
// On the FIXED code, `list_policies` resolves `org_id = rls.tenant_id() = org_a`
// and returns that policy → `policies.len() == 1`.
//
// On `main`, `list_policies` ignored the principal and used
// `query.building_id.unwrap_or_default()` = `Uuid::nil()`, so the org-scoped
// repository query matched nothing → `policies.len() == 0`. This assertion is
// what fails on main and passes on the fix.

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_policies_returns_only_callers_own_org(pool: PgPool) {
    // Register + verify + login a user and make them an active `org_admin`
    // member of a fresh org A. `create_authenticated_user_with_org` seeds the
    // membership into `organization_members` — the table `RlsConnection`'s
    // `ValidatedTenantExtractor` validates against — and returns org A's id.
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::with_email(&format!("pos-pol-{}@ins-idor.test", Uuid::new_v4()));
    let (access_token, org_a) = create_authenticated_user_with_org(&app, &user, "pos-pol-a").await;

    // Seed a policy owned by org A.
    let repo = InsuranceRepository::new(pool.clone());
    let policy_a = repo
        .create_policy(&pool, org_a, sample_policy("positive-a"))
        .await
        .expect("seed policy in org A");

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/insurance/policies")
        .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
        .header("X-Tenant-ID", org_a.to_string())
        .body(Body::empty())
        .unwrap();

    let response = app.router.clone().oneshot(request).await.expect("request");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "legitimate tenant-resolved caller must get 200"
    );

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("parse body");
    let policies = json["policies"].as_array().expect("policies array");

    // FIX: principal-derived org_a → sees its own policy.
    // MAIN: nil-org placeholder → sees nothing (this assertion fails on main).
    assert_eq!(
        policies.len(),
        1,
        "org A must see exactly its own policy via the authenticated principal"
    );
    assert_eq!(
        policies[0]["id"].as_str().unwrap(),
        policy_a.id.to_string(),
        "the returned policy must be org A's own"
    );
}

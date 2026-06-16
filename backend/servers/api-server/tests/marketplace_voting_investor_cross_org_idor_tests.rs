//! Regression tests for the cross-tenant IDOR fixes on the marketplace,
//! voting, and investor-portal endpoints (issues #778 and #830).
//!
//! Audit history:
//!   * `GET /api/v1/marketplace/rfqs/{id}` looked an RFQ up by primary key
//!     alone (`find_rfq_by_id`) and discarded the auth context, so a member of
//!     org B could read org A's RFQ by UUID (cross-org IDOR).
//!   * `GET /api/v1/voting/{id}` called the raw-pool
//!     `find_by_id_with_details` with no org scope, leaking another tenant's
//!     vote.
//!   * `GET /api/v1/investor-portal/portfolios/{id}/properties` keyed on
//!     `portfolio_id` only — no check that the portfolio belongs to the
//!     caller's org — so any authenticated user could enumerate another org's
//!     portfolio properties.
//!
//! The fix threads the authenticated org through each handler: RFQ/review
//! reads re-derive the org from the fetched row and run a `verify_org_access`
//! membership check; `get_vote` calls the new org-scoped
//! `find_by_id_with_details_for_org`; the portfolio-property handlers gate on
//! `verify_portfolio_org`. Cross-tenant probes resolve to 404; same-org access
//! still succeeds.
//!
//! These tests exercise the HTTP surface end-to-end with real HS256 JWTs.
//!
//! Per-request org/tenant context wiring (the reason the first cut of these
//! tests passed *trivially* — the same-org positive cases 500/400'd for lack of
//! context, so the cross-org cases were "rejected" only because *every* request
//! was rejected):
//!
//!   * `marketplace::get_rfq` reads `AuthUser` and calls
//!     `verify_org_access(user.user_id, rfq.organization_id)` — i.e. it derives
//!     the org from the *fetched row* and checks `organization_members`
//!     membership. No per-request tenant header is needed; the bearer JWT's
//!     `sub` plus a seeded membership is enough. Cross-org rejection is a real
//!     `is_member == false` test.
//!   * `voting::get_vote` takes an `RlsConnection`, whose
//!     `ValidatedTenantExtractor` resolves the tenant from the `X-Tenant-ID`
//!     header (no `host_tenant_middleware` is mounted under `TestApp`, so the
//!     `ResolvedTenant` extension is absent and the header is the only source).
//!     We set `X-Tenant-ID: <org>` so the extractor resolves + membership-checks
//!     the caller's org.
//!   * `investor_portal::list_portfolio_properties` reads `AuthUser.tenant_id`
//!     (the JWT `tenant_id` claim) and gates on `verify_portfolio_org`. The JWT
//!     must therefore carry the `tenant_id` claim — note `JwtService` emits the
//!     claim as `org_id`, which does NOT deserialize into
//!     `api_core::extractors::auth::Claims::tenant_id`; that mismatch is exactly
//!     why the original `mint_token` left `auth.tenant_id == None` and the
//!     handler returned `400 "No organization context"`. We mint the token here
//!     with the `tenant_id` claim directly (same approach as
//!     `reserve_funds_cross_org_idor_tests`).

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{TestApp, TestConfig};

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
    .bind(format!("MVI Org {slug}"))
    .bind(format!("mvi-org-{slug}"))
    .bind(format!("{slug}@mvi-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'MVI User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Make `user_id` an active member of `org_id`.
async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role_type, status, joined_at)
        VALUES ($1, $2, 'org_admin', 'active', NOW())
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed an RFQ owned by `org_id` and return its id.
async fn seed_rfq(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rfqs (organization_id, created_by, title, description, service_category, status)
        VALUES ($1, $2, 'Roof repair', 'Fix the roof', 'roofing', 'draft')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed rfq")
}

/// Seed a vote owned by `org_id` (in `building_id`) and return its id.
async fn seed_vote(pool: &PgPool, org_id: Uuid, building_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO votes (organization_id, building_id, title, end_at, created_by)
        VALUES ($1, $2, 'Annual budget vote', NOW() + interval '7 days', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed vote")
}

/// Seed an investor profile + portfolio owned by `org_id`; return portfolio id.
async fn seed_portfolio(pool: &PgPool, org_id: Uuid) -> Uuid {
    let investor_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO investor_profiles (organization_id, display_name, investor_type)
        VALUES ($1, 'Acme Capital', 'individual')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed investor profile");

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO investment_portfolios
            (organization_id, investor_id, name, initial_investment, investment_date)
        VALUES ($1, $2, 'Growth Fund', 100000.00, CURRENT_DATE)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(investor_id)
    .fetch_one(pool)
    .await
    .expect("seed portfolio")
}

/// Seed a service-provider profile owned by `user_id`; return its id.
/// Provider profiles are intentionally cross-org/public (PAP-140 keeps them so);
/// ownership is by `user_id`, not org.
async fn seed_provider(pool: &PgPool, user_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO service_provider_profiles
            (user_id, company_name, contact_name, contact_email, status)
        VALUES ($1, $2, 'Contact', $3, 'active')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(format!("Provider {slug}"))
    .bind(format!("{slug}@provider-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed provider")
}

/// Seed an RFQ invitation addressed to `provider_id` and return its id.
async fn seed_invitation(pool: &PgPool, rfq_id: Uuid, provider_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rfq_invitations (rfq_id, provider_id)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(rfq_id)
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed invitation")
}

/// Seed a verification document for `provider_id` and return its id.
async fn seed_verification(pool: &PgPool, provider_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO provider_verifications
            (provider_id, verification_type, document_name, document_number, status)
        VALUES ($1, 'license', 'License.pdf', 'SECRET-DOC-123', 'pending')
        RETURNING id
        "#,
    )
    .bind(provider_id)
    .fetch_one(pool)
    .await
    .expect("seed verification")
}

/// Claims shape that matches `api_core::extractors::auth::Claims`. We mint with
/// this (rather than `JwtService`, which serializes the org as `org_id`) so the
/// `tenant_id` claim deserializes into `AuthUser.tenant_id` for the
/// `investor_portal` handlers.
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

/// Mint an HS256 access token for `user_id`, scoped to `org_id` via the JWT
/// `tenant_id` claim, signed with the same secret `TestApp` configures.
fn mint_token(user_id: Uuid, email: &str, org_id: Option<Uuid>) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: Some("manager".to_string()),
        email: email.to_string(),
        name: "MVI User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

/// Mint a token carrying an explicit `role` claim (e.g. `super_admin`), used to
/// exercise the platform-admin gates on the verification/badge endpoints.
fn mint_token_with_role(user_id: Uuid, email: &str, org_id: Option<Uuid>, role: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: Some(role.to_string()),
        email: email.to_string(),
        name: "MVI User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

fn assert_not_ok(status: StatusCode, ctx: &str) {
    assert_ne!(
        status,
        StatusCode::OK,
        "{ctx}: cross-tenant resource must not be readable (got 200)"
    );
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: cross-tenant request must be rejected with 4xx, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Marketplace — RFQ read
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_rfq_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "rfq-a").await;
    let org_b = seed_org(&pool, "rfq-b").await;
    let user_a = seed_user(&pool, "rfq-a@mvi-idor.test").await;
    let user_b = seed_user(&pool, "rfq-b@mvi-idor.test").await;
    seed_membership(&pool, org_a, user_a).await;
    seed_membership(&pool, org_b, user_b).await;
    let rfq_a = seed_rfq(&pool, org_a, user_a).await;

    let token_b = mint_token(user_b, "rfq-b@mvi-idor.test", Some(org_b));
    let uri = format!("/api/v1/marketplace/rfqs/{rfq_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_not_ok(resp.status, "get_rfq cross-tenant");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_rfq_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "rfq-own-a").await;
    let user_a = seed_user(&pool, "rfq-own-a@mvi-idor.test").await;
    seed_membership(&pool, org_a, user_a).await;
    let rfq_a = seed_rfq(&pool, org_a, user_a).await;

    let token_a = mint_token(user_a, "rfq-own-a@mvi-idor.test", Some(org_a));
    let uri = format!("/api/v1/marketplace/rfqs/{rfq_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_a).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to read its own RFQ: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Voting — vote read
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_vote_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "vote-a").await;
    let org_b = seed_org(&pool, "vote-b").await;
    let user_a = seed_user(&pool, "vote-a@mvi-idor.test").await;
    let user_b = seed_user(&pool, "vote-b@mvi-idor.test").await;
    seed_membership(&pool, org_a, user_a).await;
    seed_membership(&pool, org_b, user_b).await;
    let building_a = seed_building(&pool, org_a, "VoteA").await;
    let vote_a = seed_vote(&pool, org_a, building_a, user_a).await;

    // Org B's caller authenticates as org B (X-Tenant-ID = org_b, where they
    // are a member) but targets org A's vote. The org-scoped lookup finds no
    // row under org B → 404. (Without the header the request would 400 at the
    // tenant extractor, masking whether the IDOR scope actually works.)
    let token_b = mint_token(user_b, "vote-b@mvi-idor.test", Some(org_b));
    let uri = format!("/api/v1/voting/{vote_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_not_ok(resp.status, "get_vote cross-tenant");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_vote_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "vote-own-a").await;
    let user_a = seed_user(&pool, "vote-own-a@mvi-idor.test").await;
    seed_membership(&pool, org_a, user_a).await;
    let building_a = seed_building(&pool, org_a, "VoteOwnA").await;
    let vote_a = seed_vote(&pool, org_a, building_a, user_a).await;

    // Org A's caller authenticates as org A (X-Tenant-ID = org_a) and reads its
    // own vote → 200.
    let token_a = mint_token(user_a, "vote-own-a@mvi-idor.test", Some(org_a));
    let uri = format!("/api/v1/voting/{vote_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_a)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to read its own vote: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Investor portal — portfolio properties read
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_portfolio_properties_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pf-a").await;
    let org_b = seed_org(&pool, "pf-b").await;
    let user_b = seed_user(&pool, "pf-b@mvi-idor.test").await;
    seed_membership(&pool, org_b, user_b).await;
    let portfolio_a = seed_portfolio(&pool, org_a).await;

    let token_b = mint_token(user_b, "pf-b@mvi-idor.test", Some(org_b));
    let uri = format!("/api/v1/investor-portal/portfolios/{portfolio_a}/properties");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_not_ok(resp.status, "list_portfolio_properties cross-tenant");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_portfolio_properties_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pf-own-a").await;
    let user_a = seed_user(&pool, "pf-own-a@mvi-idor.test").await;
    seed_membership(&pool, org_a, user_a).await;
    let portfolio_a = seed_portfolio(&pool, org_a).await;

    let token_a = mint_token(user_a, "pf-own-a@mvi-idor.test", Some(org_a));
    let uri = format!("/api/v1/investor-portal/portfolios/{portfolio_a}/properties");
    let resp = app.execute(app.get(&uri).bearer(&token_a).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to list its own portfolio properties: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Marketplace — RFQ invitations (provider-owned; PAP-140)
//
// `mark_invitation_viewed` / `decline_invitation` previously discarded the auth
// context entirely (zero pre-check), so any authenticated provider could flip
// another provider's invitation by guessing its UUID. The repo methods now key
// on the caller's verified provider id.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn decline_invitation_for_other_provider_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "inv-org").await;
    let user_a = seed_user(&pool, "inv-prov-a@provider-idor.test").await;
    let user_b = seed_user(&pool, "inv-prov-b@provider-idor.test").await;
    let provider_a = seed_provider(&pool, user_a, "inv-a").await;
    let _provider_b = seed_provider(&pool, user_b, "inv-b").await;
    let rfq = seed_rfq(&pool, org, user_a).await;
    let invitation = seed_invitation(&pool, rfq, provider_a).await;

    // Provider B (own profile) tries to decline provider A's invitation → 404.
    let token_b = mint_token(user_b, "inv-prov-b@provider-idor.test", None);
    let uri = format!("/api/v1/marketplace/invitations/{invitation}/decline");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_b)
                .json(serde_json::json!({ "reason": "not interested" }))
                .build(),
        )
        .await;

    assert_not_ok(resp.status, "decline_invitation cross-provider");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn decline_invitation_for_own_provider_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "inv-own-org").await;
    let user_a = seed_user(&pool, "inv-own-a@provider-idor.test").await;
    let provider_a = seed_provider(&pool, user_a, "inv-own-a").await;
    let rfq = seed_rfq(&pool, org, user_a).await;
    let invitation = seed_invitation(&pool, rfq, provider_a).await;

    let token_a = mint_token(user_a, "inv-own-a@provider-idor.test", None);
    let uri = format!("/api/v1/marketplace/invitations/{invitation}/decline");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_a)
                .json(serde_json::json!({ "reason": "busy" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Provider must be able to decline its own invitation: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_invitation_viewed_for_other_provider_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "inv-view-org").await;
    let user_a = seed_user(&pool, "inv-view-a@provider-idor.test").await;
    let user_b = seed_user(&pool, "inv-view-b@provider-idor.test").await;
    let provider_a = seed_provider(&pool, user_a, "inv-view-a").await;
    let _provider_b = seed_provider(&pool, user_b, "inv-view-b").await;
    let rfq = seed_rfq(&pool, org, user_a).await;
    let invitation = seed_invitation(&pool, rfq, provider_a).await;

    let token_b = mint_token(user_b, "inv-view-b@provider-idor.test", None);
    let uri = format!("/api/v1/marketplace/invitations/{invitation}/view");
    let resp = app.execute(app.post(&uri).bearer(&token_b).build()).await;

    assert_not_ok(resp.status, "mark_invitation_viewed cross-provider");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_invitation_viewed_is_idempotent(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "inv-idem-org").await;
    let user_a = seed_user(&pool, "inv-idem-a@provider-idor.test").await;
    let provider_a = seed_provider(&pool, user_a, "inv-idem-a").await;
    let rfq = seed_rfq(&pool, org, user_a).await;
    let invitation = seed_invitation(&pool, rfq, provider_a).await;

    let token_a = mint_token(user_a, "inv-idem-a@provider-idor.test", None);
    let uri = format!("/api/v1/marketplace/invitations/{invitation}/view");

    // First view — marks viewed_at
    let resp1 = app.execute(app.post(&uri).bearer(&token_a).build()).await;
    assert_eq!(resp1.status, StatusCode::OK, "first view must succeed: {}", resp1.text());

    // Second view — already viewed, must still return 200 (idempotent)
    let resp2 = app.execute(app.post(&uri).bearer(&token_a).build()).await;
    assert_eq!(
        resp2.status,
        StatusCode::OK,
        "repeat view by owner must be idempotent (GH #1301): {}",
        resp2.text()
    );
}

// ---------------------------------------------------------------------------
// Marketplace — verification read (owner-or-platform-admin; PAP-140)
//
// `GET /verifications/{id}` returned sensitive document data (numbers, URLs)
// keyed on id alone. It now requires the owning provider or a platform admin.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_verification_for_other_provider_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user_a = seed_user(&pool, "ver-a@provider-idor.test").await;
    let user_b = seed_user(&pool, "ver-b@provider-idor.test").await;
    let provider_a = seed_provider(&pool, user_a, "ver-a").await;
    let _provider_b = seed_provider(&pool, user_b, "ver-b").await;
    let verification = seed_verification(&pool, provider_a).await;

    let token_b = mint_token(user_b, "ver-b@provider-idor.test", None);
    let uri = format!("/api/v1/marketplace/verifications/{verification}");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_not_ok(resp.status, "get_verification cross-provider");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_verification_for_owner_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user_a = seed_user(&pool, "ver-own-a@provider-idor.test").await;
    let provider_a = seed_provider(&pool, user_a, "ver-own-a").await;
    let verification = seed_verification(&pool, provider_a).await;

    let token_a = mint_token(user_a, "ver-own-a@provider-idor.test", None);
    let uri = format!("/api/v1/marketplace/verifications/{verification}");
    let resp = app.execute(app.get(&uri).bearer(&token_a).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Owning provider must be able to read its own verification: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_verification_as_platform_admin_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user_a = seed_user(&pool, "ver-adm-a@provider-idor.test").await;
    let admin = seed_user(&pool, "ver-admin@provider-idor.test").await;
    let provider_a = seed_provider(&pool, user_a, "ver-adm-a").await;
    let verification = seed_verification(&pool, provider_a).await;

    let token = mint_token_with_role(admin, "ver-admin@provider-idor.test", None, "super_admin");
    let uri = format!("/api/v1/marketplace/verifications/{verification}");
    let resp = app.execute(app.get(&uri).bearer(&token).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Platform admin must be able to read any verification: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Marketplace — platform-moderation gates (PAP-140)
//
// Verification review and badge revoke were callable by any authenticated user.
// They now require the platform-admin role; the gate runs before any DB work.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_verification_as_non_admin_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "rev-nonadmin@provider-idor.test").await;

    // Non-admin (manager) token; random verification id — the role gate fires first.
    let token = mint_token(user, "rev-nonadmin@provider-idor.test", None);
    let uri = format!(
        "/api/v1/marketplace/verifications/{}/review",
        Uuid::new_v4()
    );
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token)
                .json(serde_json::json!({ "status": "verified" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Non-admin must not be able to review verifications: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_badge_as_non_admin_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = seed_user(&pool, "badge-nonadmin@provider-idor.test").await;

    let token = mint_token(user, "badge-nonadmin@provider-idor.test", None);
    let uri = format!("/api/v1/marketplace/badges/{}", Uuid::new_v4());
    let resp = app.execute(app.delete(&uri).bearer(&token).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Non-admin must not be able to revoke badges: {}",
        resp.text()
    );
}

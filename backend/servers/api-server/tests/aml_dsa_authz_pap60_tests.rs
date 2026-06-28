//! Regression tests for the AML/DSA authz & data-integrity gaps (PAP-60,
//! re-landing the PAP-38/PAP-43 remediation that never reached `dev`; child of
//! the PAP-35 security review of `routes/aml_dsa.rs`).
//!
//! Three gaps are closed here:
//!
//! 1. `create_aml_assessment` had NO role check while every other AML/EDD
//!    handler calls `require_compliance_role` — any authenticated tenant user
//!    could create AML risk assessments. Now gated: a non-compliance caller
//!    gets 403.
//!
//! 2. `file_appeal` had no authz/ownership check — it called
//!    `file_appeal(id, reason)` on a global id, so any authenticated user could
//!    appeal any case in any tenant. Now the handler loads the case and
//!    enforces tenant isolation (case org == caller org) + ownership (case
//!    owner == caller); a cross-tenant case is refused with 404.
//!
//! 3. `report_content` stored `content_owner_id = Uuid::new_v4()` — a random
//!    bogus owner on every reported case, corrupting violation-history and
//!    owner-notification. The owner is now resolved from the real content via
//!    `ComplianceRepository::resolve_content_owner` (org-scoped), or the report
//!    is rejected when unresolvable. These repo-layer tests assert the resolver
//!    returns the *real* owner, refuses cross-org content, and refuses content
//!    types with no backing owner table.
//!
//! HTTP-layer caveat (see `agency_authz_idor_tests.rs`): `TestApp` mounts the
//! router without `host_tenant_middleware`, and `create_authenticated_user`
//! mints a token whose claims carry `role = None` / `tenant_id = None` (the
//! user has no org membership at login time). That is exactly the
//! "non-privileged / cross-tenant caller" the role and ownership gates must
//! reject, so the HTTP tests assert the rejection direction.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, TestApp, TestUser};
use db::models::compliance::ModeratedContentType;
use db::repositories::ComplianceRepository;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("PAP43 Org {slug}"))
    .bind(format!("pap43-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@pap43.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, label: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'PAP43 User', 'active', NOW()) RETURNING id"#,
    )
    .bind(format!("{label}-{}@pap43.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a listing owned by `owner` in `org`; returns the listing id.
async fn seed_listing(pool: &PgPool, org: Uuid, owner: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO listings
               (organization_id, created_by, transaction_type, title,
                street, city, postal_code, price)
           VALUES ($1, $2, 'sale', 'PAP43 Listing',
                   '1 Test St', 'Testville', '00000', 100000.00)
           RETURNING id"#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

/// Seed a moderation case in `org` owned by `owner`; returns the case id.
async fn seed_moderation_case(pool: &PgPool, org: Uuid, owner: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO moderation_cases
               (content_type, content_id, content_owner_id, organization_id,
                report_source, status, priority)
           VALUES ('listing', $1, $2, $3, 'user', 'pending', 3)
           RETURNING id"#,
    )
    .bind(Uuid::new_v4())
    .bind(owner)
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed moderation case")
}

// ===========================================================================
// (3) report_content: resolve_content_owner returns the REAL owner, org-scoped,
//     and refuses unresolvable content instead of inventing a random owner.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_content_owner_returns_real_listing_owner(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());
    let org = seed_org(&pool, "list").await;
    let owner = seed_user(&pool, "owner").await;
    let listing = seed_listing(&pool, org, owner).await;

    let resolved = repo
        .resolve_content_owner(ModeratedContentType::Listing, listing, Some(org))
        .await
        .expect("query ok");

    assert_eq!(
        resolved,
        Some(owner),
        "report_content must persist the listing's real owner, not a placeholder"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_content_owner_is_org_scoped(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());
    let org_a = seed_org(&pool, "a").await;
    let org_b = seed_org(&pool, "b").await;
    let owner = seed_user(&pool, "owner").await;
    let listing_in_a = seed_listing(&pool, org_a, owner).await;

    // Caller scoped to org B cannot resolve (and therefore cannot report)
    // content that lives in org A — returns None, so the handler 404s.
    let cross_org = repo
        .resolve_content_owner(ModeratedContentType::Listing, listing_in_a, Some(org_b))
        .await
        .expect("query ok");
    assert_eq!(
        cross_org, None,
        "content in another org must not be resolvable by a cross-tenant reporter"
    );

    // No org context at all is likewise unresolvable for org-scoped content.
    let no_org = repo
        .resolve_content_owner(ModeratedContentType::Listing, listing_in_a, None)
        .await
        .expect("query ok");
    assert_eq!(
        no_org, None,
        "org-scoped content needs an org context to resolve"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_content_owner_user_profile_is_self(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());
    let org = seed_org(&pool, "prof").await;
    let user = seed_user(&pool, "profile").await;

    // A user profile *is* the user; the content id is the owner.
    let resolved = repo
        .resolve_content_owner(ModeratedContentType::UserProfile, user, Some(org))
        .await
        .expect("query ok");
    assert_eq!(resolved, Some(user));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_content_owner_unknown_content_is_none(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());
    let org = seed_org(&pool, "none").await;

    // Missing listing → None (handler rejects with 404 rather than a placeholder).
    let missing = repo
        .resolve_content_owner(ModeratedContentType::Listing, Uuid::new_v4(), Some(org))
        .await
        .expect("query ok");
    assert_eq!(missing, None);

    // Content types with no backing owner table also resolve to None.
    for ct in [ModeratedContentType::Review, ModeratedContentType::Comment] {
        let r = repo
            .resolve_content_owner(ct, Uuid::new_v4(), Some(org))
            .await
            .expect("query ok");
        assert_eq!(
            r, None,
            "{ct:?} has no owner table and must not invent an owner"
        );
    }
}

// ===========================================================================
// (1) create_aml_assessment is role-gated: a non-compliance caller gets 403.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_aml_assessment_requires_compliance_role(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(&app, &user).await;

    let req = app
        .post("/api/v1/aml-dsa/aml/assess")
        .bearer(&token)
        .json(serde_json::json!({
            "party_id": Uuid::new_v4(),
            "party_type": "individual"
        }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a non-compliance caller must be refused (403) before any assessment is created"
    );
}

// ===========================================================================
// (2) file_appeal enforces tenant isolation: a caller outside the case's org
//     (here, a caller with no org context) is refused, not allowed to mutate.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn file_appeal_rejects_cross_tenant_case(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // A case that belongs to some org and is owned by some other user.
    let org = seed_org(&pool, "case").await;
    let owner = seed_user(&pool, "case-owner").await;
    let case_id = seed_moderation_case(&pool, org, owner).await;

    // An unrelated authenticated caller (no org context / not the owner).
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(&app, &user).await;

    let req = app
        .post(&format!(
            "/api/v1/aml-dsa/moderation/cases/{case_id}/appeal"
        ))
        .bearer(&token)
        .json(serde_json::json!({ "reason": "I disagree" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "a caller outside the case's tenant must not be able to file (or even see) the appeal"
    );

    // And the case must NOT have been mutated into the appealed state.
    let appealed: bool =
        sqlx::query_scalar("SELECT appeal_filed FROM moderation_cases WHERE id = $1")
            .bind(case_id)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert!(
        !appealed,
        "the cross-tenant appeal attempt must not have mutated the case"
    );
}

/// Repo-layer proof of the defense-in-depth `organization_id` predicate on
/// `ComplianceRepository::file_appeal` (PAP-60 acceptance: "Repo UPDATE carries
/// organization_id"). Even calling the repo directly with a *wrong* org id must
/// not mutate a case that lives in another org.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repo_file_appeal_is_org_scoped(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());

    let org_a = seed_org(&pool, "a").await;
    let org_b = seed_org(&pool, "b").await;
    let owner = seed_user(&pool, "owner").await;
    let case_in_a = seed_moderation_case(&pool, org_a, owner).await;

    // Calling with org B (not the case's org) must affect zero rows: fetch_one
    // therefore yields RowNotFound rather than mutating the case.
    let res = repo.file_appeal(case_in_a, org_b, "cross-org appeal").await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "file_appeal with the wrong org must not match (and must not mutate) the case"
    );

    let appealed: bool =
        sqlx::query_scalar("SELECT appeal_filed FROM moderation_cases WHERE id = $1")
            .bind(case_in_a)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert!(
        !appealed,
        "the org-scoped UPDATE must leave a cross-org case untouched"
    );

    // Sanity: with the correct org the same call succeeds and flips the flag.
    let ok = repo
        .file_appeal(case_in_a, org_a, "rightful appeal")
        .await
        .expect("owner-org appeal succeeds");
    assert!(ok.appeal_filed, "in-org appeal must mutate the case");
}

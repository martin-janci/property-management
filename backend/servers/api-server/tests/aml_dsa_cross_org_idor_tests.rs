//! Regression tests for the cross-org IDOR fix on the AML/DSA read-by-id
//! paths (issue #897, Epics 67/97/147 — AML/DSA & GDPR role-gated paths).
//!
//! Audit history: several handlers on `/api/v1/aml/*` and `/api/v1/edd/*`
//! gated on a compliance/moderator role (`require_compliance_role`) but then
//! called repository methods that took only the row id and applied NO
//! `organization_id` predicate, so a privileged caller scoped to org B could
//! read (and, for the EDD sub-resources, mutate) org A's rows by guessing or
//! enumerating the UUID:
//!
//! - `get_aml_assessment` → `get_aml_assessment(id)`  (assessment / PEP / sanctions leak)
//! - `get_edd_record`     → `get_edd(id)`             (EDD record / source-of-funds leak)
//! - `upload_edd_document`/`verify_edd_document`/`add_edd_note`/`complete_edd`
//!       all gate on `get_edd(id)` → cross-org mutate of another org's EDD subtree
//!
//! The `aml_risk_assessments` / `edd_records` tables DO ship FORCE-RLS policies
//! (migration `00117_rls_edd.sql`) scoped to `get_current_org_id()`, but these
//! handlers run on the raw pool (`state.edd_repo`) where the RLS session GUCs
//! are never set — so the policy cannot protect a global-id lookup. The fix
//! threads the caller's `tenant_id` into the SQL WHERE clause at the repo
//! layer (`get_aml_assessment(id, org_id)` / `get_edd(id, org_id)`).
//!
//! Why repo-layer and not HTTP-layer: as documented in the sibling
//! `ai_llm_cross_tenant_idor_tests.rs`, `TestApp` mounts the router without the
//! tenant middleware, so the security contract is asserted at the repository's
//! WHERE clause directly: the scoped method must NOT return another org's row
//! even though the raw (pre-fix) query would. Each test also runs the unscoped
//! query to demonstrate the leak the org predicate closes.
//!
//! These tests use tables that ship migrations (`00078_create_edd.sql`) so they
//! run against `db::MIGRATOR` deterministically.

use db::repositories::EddRepository;
use sqlx::PgPool;
use uuid::Uuid;

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
    .bind(format!("AmlIDOR Org {slug}"))
    .bind(format!("aml-idor-org-{slug}"))
    .bind(format!("{slug}@aml-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'AmlIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_assessment(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO aml_risk_assessments (organization_id, party_id, party_type)
        VALUES ($1, $2, 'individual') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(Uuid::new_v4())
    .fetch_one(pool)
    .await
    .expect("seed assessment")
}

async fn seed_edd(pool: &PgPool, org_id: Uuid, assessment_id: Uuid, initiated_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO edd_records (aml_assessment_id, organization_id, party_id, initiated_by)
        VALUES ($1, $2, $3, $4) RETURNING id
        "#,
    )
    .bind(assessment_id)
    .bind(org_id)
    .bind(Uuid::new_v4())
    .bind(initiated_by)
    .fetch_one(pool)
    .await
    .expect("seed edd")
}

// ---------------------------------------------------------------------------
// (1) AML assessment read is org-scoped — org B cannot read org A's assessment.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_aml_assessment_blocks_cross_org_read(pool: PgPool) {
    let repo = EddRepository::new(pool.clone());

    let org_a = seed_org(&pool, "aml-a").await;
    let org_b = seed_org(&pool, "aml-b").await;
    let assessment_in_a = seed_assessment(&pool, org_a).await;

    // Same-org read succeeds.
    let same_org = repo
        .get_aml_assessment(assessment_in_a, org_a)
        .await
        .expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must be able to read its own AML assessment"
    );

    // Cross-org read returns None (the IDOR is blocked).
    let cross_org = repo
        .get_aml_assessment(assessment_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross_org.is_none(),
        "org B must NOT be able to read org A's AML assessment"
    );

    // Demonstrate the leak the org predicate closes: the unscoped lookup the
    // pre-fix handler effectively performed returns the row regardless of org.
    let unscoped: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM aml_risk_assessments WHERE id = $1")
            .bind(assessment_in_a)
            .fetch_optional(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        unscoped,
        Some(assessment_in_a),
        "sanity: the unscoped query (the vulnerable pre-fix path) does leak the row"
    );
}

// ---------------------------------------------------------------------------
// (2) EDD record read is org-scoped — org B cannot read org A's EDD record.
//     Every EDD sub-resource handler (documents, notes, completion) gates on
//     this lookup, so scoping it closes the whole subtree against cross-org
//     read AND mutate.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_edd_blocks_cross_org_access(pool: PgPool) {
    let repo = EddRepository::new(pool.clone());

    let org_a = seed_org(&pool, "edd-a").await;
    let org_b = seed_org(&pool, "edd-b").await;
    let user_a = seed_user(&pool, "edd-a@aml-idor.test").await;
    let assessment_in_a = seed_assessment(&pool, org_a).await;
    let edd_in_a = seed_edd(&pool, org_a, assessment_in_a, user_a).await;

    // Same-org read succeeds.
    let same_org = repo.get_edd(edd_in_a, org_a).await.expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must be able to read its own EDD record"
    );

    // Cross-org read returns None — this is the gate every EDD mutation
    // (upload/verify document, add note, complete) checks before acting, so a
    // None here means org B is refused with 404 on all of them.
    let cross_org = repo.get_edd(edd_in_a, org_b).await.expect("query ok");
    assert!(
        cross_org.is_none(),
        "org B must NOT be able to read or mutate org A's EDD record"
    );

    // Demonstrate the leak the org predicate closes.
    let unscoped: Option<Uuid> = sqlx::query_scalar("SELECT id FROM edd_records WHERE id = $1")
        .bind(edd_in_a)
        .fetch_optional(&pool)
        .await
        .expect("query ok");
    assert_eq!(
        unscoped,
        Some(edd_in_a),
        "sanity: the unscoped query (the vulnerable pre-fix path) does leak the row"
    );
}

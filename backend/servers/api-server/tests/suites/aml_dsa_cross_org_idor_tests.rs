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
//!   all gate on `get_edd(id)` → cross-org mutate of another org's EDD subtree
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

#![allow(dead_code)]

use db::repositories::{ComplianceRepository, EddRepository};
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

// ---------------------------------------------------------------------------
// Fixtures — moderation cases
// ---------------------------------------------------------------------------

async fn seed_moderation_case(pool: &PgPool, org_id: Uuid, content_owner_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO moderation_cases
            (content_type, content_id, content_owner_id, organization_id, report_source)
        VALUES ('listing', $1, $2, $3, 'user')
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(content_owner_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed moderation case")
}

// ---------------------------------------------------------------------------
// (3) review_aml_assessment is org-scoped — org B cannot mutate org A's row.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_aml_assessment_blocks_cross_org_mutate(pool: PgPool) {
    use db::models::compliance::AmlAssessmentStatus;

    let repo = EddRepository::new(pool.clone());

    let org_a = seed_org(&pool, "review-a").await;
    let org_b = seed_org(&pool, "review-b").await;
    let user = seed_user(&pool, "reviewer@aml-idor.test").await;
    let assessment = seed_assessment(&pool, org_a).await;

    // Cross-org review must fail (0 rows updated → fetch_one returns RowNotFound).
    let cross_org_result = repo
        .review_aml_assessment(assessment, org_b, user, AmlAssessmentStatus::Approved, None)
        .await;
    assert!(
        cross_org_result.is_err(),
        "org B must NOT be able to review org A's AML assessment"
    );

    // Sanity: same-org review succeeds.
    let same_org_result = repo
        .review_aml_assessment(assessment, org_a, user, AmlAssessmentStatus::Approved, None)
        .await;
    assert!(
        same_org_result.is_ok(),
        "org A must be able to review its own AML assessment"
    );
}

// ---------------------------------------------------------------------------
// (4) get_moderation_case is org-scoped — org B cannot read org A's case.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_moderation_case_blocks_cross_org_read(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());

    let org_a = seed_org(&pool, "mod-case-a").await;
    let org_b = seed_org(&pool, "mod-case-b").await;
    let owner = seed_user(&pool, "owner@mod-case.test").await;
    let case_in_a = seed_moderation_case(&pool, org_a, owner).await;

    // Same-org read succeeds.
    let same_org = repo
        .get_moderation_case(case_in_a, org_a)
        .await
        .expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must be able to read its own moderation case"
    );

    // Cross-org read returns None.
    let cross_org = repo
        .get_moderation_case(case_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross_org.is_none(),
        "org B must NOT be able to read org A's moderation case"
    );

    // Demonstrate the pre-fix leak.
    let unscoped: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM moderation_cases WHERE id = $1")
            .bind(case_in_a)
            .fetch_optional(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        unscoped,
        Some(case_in_a),
        "sanity: the unscoped (pre-fix) query does leak the row"
    );
}

// ---------------------------------------------------------------------------
// (5) list_moderation_cases is org-scoped — each org only sees its own cases.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_moderation_cases_returns_only_own_org(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());

    let org_a = seed_org(&pool, "list-a").await;
    let org_b = seed_org(&pool, "list-b").await;
    let owner = seed_user(&pool, "list-owner@aml-idor.test").await;

    seed_moderation_case(&pool, org_a, owner).await;
    seed_moderation_case(&pool, org_a, owner).await;
    seed_moderation_case(&pool, org_b, owner).await;

    let (a_cases, a_total) = repo
        .list_moderation_cases(
            org_a, None, None, None, None, None, false, false, None, None, 50, 0,
        )
        .await
        .expect("query ok");
    assert_eq!(a_total, 2, "org A should see exactly its 2 cases");
    assert!(
        a_cases.iter().all(|c| c.organization_id == Some(org_a)),
        "all returned cases must belong to org A"
    );

    let (b_cases, b_total) = repo
        .list_moderation_cases(
            org_b, None, None, None, None, None, false, false, None, None, 50, 0,
        )
        .await
        .expect("query ok");
    assert_eq!(b_total, 1, "org B should see exactly its 1 case");
    assert!(
        b_cases.iter().all(|c| c.organization_id == Some(org_b)),
        "all returned cases must belong to org B"
    );
}

// ---------------------------------------------------------------------------
// (5b) overdue_only filter matches the queue-stats badge (issue #2859, PR #2856
//      follow-up). The `overdue_only` predicate in `list_moderation_cases` must
//      select *exactly* the rows the `overdue_count` predicate in
//      `get_moderation_queue_stats` counts — both are
//      `status IN ('pending','under_review') AND created_at < NOW() - 24h`.
//      This is the IG3 failing-on-main test that locks the list<->stat
//      invariant #2853 fixed, so a future edit to either predicate that drifts
//      them apart (re-introducing the badge-vs-list mismatch) fails here.
//
//      Seeds within ONE org: an old+open case (overdue), a fresh+open case
//      (not overdue — too young), and an old+closed(appealed) case (not
//      overdue — wrong status). Asserts the filtered list returns only the
//      overdue row AND that its `total` equals the org's `overdue_count`.
// ---------------------------------------------------------------------------

async fn seed_moderation_case_aged(
    pool: &PgPool,
    org_id: Uuid,
    content_owner_id: Uuid,
    status: &str,
    age_hours: i64,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO moderation_cases
            (content_type, content_id, content_owner_id, organization_id,
             report_source, status, created_at)
        VALUES ('listing', $1, $2, $3, 'user', $4::moderation_status,
                NOW() - (INTERVAL '1 hour' * $5))
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(content_owner_id)
    .bind(org_id)
    .bind(status)
    .bind(age_hours)
    .fetch_one(pool)
    .await
    .expect("seed aged moderation case")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_moderation_cases_overdue_only_matches_queue_stats(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());

    let org = seed_org(&pool, "overdue").await;
    let owner = seed_user(&pool, "overdue-owner@aml-idor.test").await;

    // (a) old + open  -> OVERDUE  (pending, 48h old — past the 24h SLA).
    let overdue = seed_moderation_case_aged(&pool, org, owner, "pending", 48).await;
    // (b) fresh + open -> NOT overdue (pending, 1h old — inside the SLA window).
    seed_moderation_case_aged(&pool, org, owner, "pending", 1).await;
    // (c) old + closed -> NOT overdue (appealed, 72h old — status is not one of
    //     pending/under_review, so it is excluded regardless of age).
    seed_moderation_case_aged(&pool, org, owner, "appealed", 72).await;

    // The overdue-only list must return exactly the one old+open row.
    let (cases, total) = repo
        .list_moderation_cases(
            org, None, None, None, None, None, false, /* overdue_only */ true, None, None, 50,
            0,
        )
        .await
        .expect("query ok");

    assert_eq!(total, 1, "exactly one case is overdue for this org");
    assert_eq!(cases.len(), 1, "only the overdue row is returned");
    assert_eq!(
        cases[0].id, overdue,
        "the returned row must be the old+open (overdue) case"
    );

    // Invariant: list(overdue_only=true).total == queue-stats overdue_count.
    // The badge (`overdue_count`) is org-wide and unbounded; if the two
    // predicates ever drift, the list truncates below the badge (#2853) and
    // this assertion catches it.
    let stats = repo
        .get_moderation_queue_stats(org)
        .await
        .expect("queue stats ok");
    assert_eq!(
        total, stats.overdue_count,
        "list(overdue_only=true).total must equal get_moderation_queue_stats.overdue_count"
    );
    assert_eq!(
        stats.overdue_count, 1,
        "queue stats must count exactly the one overdue case"
    );
}

// ---------------------------------------------------------------------------
// (6) assign_moderation_case is org-scoped — org B cannot assign org A's case.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn assign_moderation_case_blocks_cross_org_mutate(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());

    let org_a = seed_org(&pool, "assign-a").await;
    let org_b = seed_org(&pool, "assign-b").await;
    let owner = seed_user(&pool, "assign-owner@aml-idor.test").await;
    let case_in_a = seed_moderation_case(&pool, org_a, owner).await;

    // Cross-org assign must fail.
    let cross_org = repo.assign_moderation_case(case_in_a, org_b, owner).await;
    assert!(
        cross_org.is_err(),
        "org B must NOT be able to assign org A's moderation case"
    );

    // Same-org assign succeeds.
    let same_org = repo.assign_moderation_case(case_in_a, org_a, owner).await;
    assert!(
        same_org.is_ok(),
        "org A must be able to assign its own moderation case"
    );
}

// ---------------------------------------------------------------------------
// (7) take_moderation_action is org-scoped — org B cannot act on org A's case.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn take_moderation_action_blocks_cross_org_mutate(pool: PgPool) {
    use db::models::compliance::{ModerationActionType, TakeModerationAction};

    let repo = ComplianceRepository::new(pool.clone());

    let org_a = seed_org(&pool, "action-a").await;
    let org_b = seed_org(&pool, "action-b").await;
    let owner = seed_user(&pool, "action-owner@aml-idor.test").await;
    let actor = seed_user(&pool, "actor@aml-idor.test").await;
    let case_in_a = seed_moderation_case(&pool, org_a, owner).await;

    let action = TakeModerationAction {
        action: ModerationActionType::Warn,
        rationale: "test".to_string(),
        template_id: None,
    };

    // Cross-org action must fail.
    let cross_org = repo
        .take_moderation_action(case_in_a, org_b, action.clone(), actor)
        .await;
    assert!(
        cross_org.is_err(),
        "org B must NOT be able to act on org A's moderation case"
    );

    // Same-org action succeeds.
    let same_org = repo
        .take_moderation_action(case_in_a, org_a, action, actor)
        .await;
    assert!(
        same_org.is_ok(),
        "org A must be able to act on its own moderation case"
    );
}

// ---------------------------------------------------------------------------
// (8) decide_appeal is org-scoped — org B cannot decide org A's appeal.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn decide_appeal_blocks_cross_org_mutate(pool: PgPool) {
    let repo = ComplianceRepository::new(pool.clone());

    let org_a = seed_org(&pool, "appeal-a").await;
    let org_b = seed_org(&pool, "appeal-b").await;
    let owner = seed_user(&pool, "appeal-owner@aml-idor.test").await;
    let decider = seed_user(&pool, "decider@aml-idor.test").await;
    let case_in_a = seed_moderation_case(&pool, org_a, owner).await;

    // Mark the case as appealed so decide_appeal has something to act on.
    sqlx::query(
        "UPDATE moderation_cases SET appeal_filed = TRUE, status = 'appealed' WHERE id = $1",
    )
    .bind(case_in_a)
    .execute(&pool)
    .await
    .expect("mark appealed");

    // Cross-org decide must fail.
    let cross_org = repo
        .decide_appeal(case_in_a, org_b, "upheld", "test", decider)
        .await;
    assert!(
        cross_org.is_err(),
        "org B must NOT be able to decide org A's appeal"
    );

    // Same-org decide succeeds.
    let same_org = repo
        .decide_appeal(case_in_a, org_a, "upheld", "test", decider)
        .await;
    assert!(
        same_org.is_ok(),
        "org A must be able to decide its own appeal"
    );
}

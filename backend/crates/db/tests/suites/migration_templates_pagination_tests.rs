//! Round-trip coverage for `MigrationRepository::list_templates` /
//! `count_templates` SQL pagination (#1997, follow-up to PR #1994).
//!
//! PR #1994 pushed the template listing pagination down into SQL
//! (`ORDER BY updated_at DESC[, id DESC] LIMIT/OFFSET`) with a companion
//! `count_templates`. Before this file there was no test exercising that path.
//!
//! What this locks in
//! ------------------
//! 1. `count_templates` returns the true total for the same filter
//!    `list_templates` paginates (org-scoped, and org + system).
//! 2. Successive pages are disjoint and together cover the full set exactly
//!    once — i.e. `LIMIT/OFFSET` walks the ordering without gaps or overlap.
//! 3. The ordering is *total*: with system templates seeded in a single INSERT
//!    (identical `updated_at`), the `, id DESC` tiebreak added in #1997 makes
//!    the order deterministic. Seeding a page's worth of rows with an identical
//!    `updated_at` and asserting strict `id DESC` fails on `updated_at DESC`
//!    alone (undefined tie order) — this is the regression guard.
//! 4. A cross-tenant RLS negative: an org-A caller bound by FORCE RLS cannot
//!    see org-B's templates even when the query's WHERE targets org-B's id.
//!
//! CI runs each `#[sqlx::test]` against its own freshly-migrated Postgres
//! (see `.github/workflows/backend.yml`), connecting as the `postgres`
//! superuser — which bypasses RLS. The pagination tests exercise the repo
//! directly under that superuser; the RLS test drops to a dedicated
//! `NOSUPERUSER NOBYPASSRLS` role so `FORCE ROW LEVEL SECURITY` actually binds.

use crate::common::seed_org;
use db::models::ImportDataType;
use db::repositories::MigrationRepository;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Create `n` org-scoped templates, then flatten their `updated_at` to a single
/// fixed instant so ordering within the page depends solely on the `id DESC`
/// tiebreak. Returns the created ids.
async fn seed_org_templates(
    pool: &PgPool,
    repo: &MigrationRepository,
    org_id: Uuid,
    n: usize,
) -> Vec<Uuid> {
    let mut ids = Vec::with_capacity(n);
    for i in 0..n {
        let tmpl = repo
            .create_template(
                pool,
                Some(org_id),
                format!("Template {i}"),
                Some(format!("desc {i}")),
                ImportDataType::Buildings,
                json!([]),
                None,
                false,
            )
            .await
            .expect("create template");
        ids.push(tmpl.id);
    }
    // Collapse updated_at to one instant so the only remaining sort key is id.
    sqlx::query(
        "UPDATE import_templates SET updated_at = TIMESTAMPTZ '2026-01-01 00:00:00+00' \
         WHERE organization_id = $1",
    )
    .bind(org_id)
    .execute(pool)
    .await
    .expect("flatten updated_at");
    ids
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn count_and_pages_are_consistent_and_disjoint(pool: PgPool) {
    // Superuser context — inserts bypass RLS; is_super_admin GUC harmless.
    db::set_request_context(&pool, None, None, true)
        .await
        .expect("set super context");

    let org = seed_org(&pool, "mig-pag-a").await;
    let repo = MigrationRepository::new(pool.clone());

    let total = 5usize;
    let created = seed_org_templates(&pool, &repo, org, total).await;

    // --- count_templates matches the org-scoped total (no system rows). ---
    let org_count = repo
        .count_templates(&pool, org, None, false)
        .await
        .expect("count org templates");
    assert_eq!(
        org_count, total as i64,
        "count_templates must equal the number of org-scoped templates"
    );

    // count including system == org count + live system-template count (robust
    // against changes to the exact number of seeded system templates).
    let system_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM import_templates WHERE organization_id IS NULL")
            .fetch_one(&pool)
            .await
            .expect("count system templates");
    let with_system = repo
        .count_templates(&pool, org, None, true)
        .await
        .expect("count incl system");
    assert_eq!(
        with_system,
        total as i64 + system_count,
        "include_system count must add the system (organization_id IS NULL) rows"
    );

    // --- Page 1 vs Page 2 (per_page = 2) are disjoint. ---
    let per_page = 2i64;
    let page1: Vec<Uuid> = repo
        .list_templates(&pool, org, None, false, per_page, 0)
        .await
        .expect("page 1")
        .into_iter()
        .map(|t| t.id)
        .collect();
    let page2: Vec<Uuid> = repo
        .list_templates(&pool, org, None, false, per_page, per_page)
        .await
        .expect("page 2")
        .into_iter()
        .map(|t| t.id)
        .collect();
    let page3: Vec<Uuid> = repo
        .list_templates(&pool, org, None, false, per_page, per_page * 2)
        .await
        .expect("page 3")
        .into_iter()
        .map(|t| t.id)
        .collect();

    assert_eq!(page1.len(), 2, "page 1 must be full");
    assert_eq!(page2.len(), 2, "page 2 must be full");
    assert_eq!(page3.len(), 1, "page 3 holds the remainder");

    for id in &page1 {
        assert!(!page2.contains(id), "pages must be disjoint (p1 vs p2)");
        assert!(!page3.contains(id), "pages must be disjoint (p1 vs p3)");
    }
    for id in &page2 {
        assert!(!page3.contains(id), "pages must be disjoint (p2 vs p3)");
    }

    // Union of all pages == the full created set, each exactly once.
    let mut walked: Vec<Uuid> = page1.iter().chain(&page2).chain(&page3).copied().collect();
    walked.sort();
    let mut expected = created.clone();
    expected.sort();
    assert_eq!(
        walked, expected,
        "paging must cover every template exactly once"
    );

    // --- Total order: identical updated_at ⇒ strict id DESC (the #1997 tiebreak).
    //     On `ORDER BY updated_at DESC` alone this tie order is undefined. ---
    let full: Vec<Uuid> = repo
        .list_templates(&pool, org, None, false, 100, 0)
        .await
        .expect("full list")
        .into_iter()
        .map(|t| t.id)
        .collect();
    let mut sorted_desc = full.clone();
    sorted_desc.sort_by(|a, b| b.cmp(a));
    assert_eq!(
        full, sorted_desc,
        "with equal updated_at the order must be a stable, strict id DESC"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_templates_is_cross_tenant_isolated_under_force_rls(pool: PgPool) {
    // Seed both orgs + an org-B template under superuser context.
    db::set_request_context(&pool, None, None, true)
        .await
        .expect("set super context");

    let org_a = seed_org(&pool, "mig-rls-a").await;
    let org_b = seed_org(&pool, "mig-rls-b").await;
    let repo = MigrationRepository::new(pool.clone());

    let tmpl_b = repo
        .create_template(
            &pool,
            Some(org_b),
            "Org B Secret".to_string(),
            None,
            ImportDataType::Buildings,
            json!([]),
            None,
            false,
        )
        .await
        .expect("create org-B template");

    // Dedicated NOSUPERUSER NOBYPASSRLS role so FORCE RLS actually binds.
    let role = format!("ppt_mig_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON import_templates, organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
         get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute on rls helpers");

    {
        let mut conn = pool.acquire().await.expect("acquire connection");

        // Org-A context, then drop to the FORCE-applicable role.
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(Option::<Uuid>::None)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A context");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // Even though the query's WHERE targets org-B's id, FORCE RLS hides
        // org-B's rows from an org-A caller.
        let leaked = repo
            .list_templates(&mut *conn, org_b, None, false, 100, 0)
            .await
            .expect("list under org-A");
        assert!(
            leaked.is_empty(),
            "org-A caller must not see org-B templates (got {} rows incl {:?})",
            leaked.len(),
            tmpl_b.id,
        );

        let leaked_count = repo
            .count_templates(&mut *conn, org_b, None, false)
            .await
            .expect("count under org-A");
        assert_eq!(
            leaked_count, 0,
            "count_templates must not leak org-B's total to an org-A caller"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }
}

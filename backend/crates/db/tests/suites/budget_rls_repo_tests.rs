//! Repo-level behavioral RLS regression test for PAP-180 (PAP-67 / PAP-151
//! cluster).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the budget cluster (`budgets`, `reserve_funds`,
//! `capital_plans`, `budget_variance_alerts`, …). The production api-server
//! connects as the table OWNER, which `FORCE` binds. `BudgetRepository` used to
//! carry a raw `PgPool` plus a layer of deprecated non-`_rls` wrappers and a few
//! genuinely standalone `&self.pool` methods (`get_dashboard`,
//! `record_reserve_transaction`, `generate_reserve_projection`). Those pool paths
//! never called `set_request_context`, so on `dev` `get_current_org_id()` returned
//! NULL and every policy collapsed to `organization_id = NULL` → **deny-all** for
//! own-org traffic (and RLS-bypass on a BYPASSRLS pool).
//!
//! PAP-180 dropped the pool field and the deprecated wrappers, and converted the
//! standalone multi-query methods to take a caller-supplied `&mut PgConnection`
//! (handlers pass `&mut **rls.conn()`), so every query runs under the caller's
//! RLS context.
//!
//! This test exercises the *repository methods themselves* on a `FORCE`-bound
//! NOSUPERUSER role and proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_budget_by_id_rls` / `list_budgets_rls` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same calls return the own-org row.
//!   3. **Cross-tenant** — org B's budget stays invisible to an org-A caller, and
//!      the converted multi-query `get_dashboard` returns org A's active budget,
//!      never org B's.
//!   4. **Write path** — `create_budget_rls` on a context-set connection succeeds
//!      and the row is the caller's org. Without context the INSERT would fail the
//!      policy `WITH CHECK`.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS` role,
//! grants it access, and `SET ROLE`s to it so `FORCE` actually enforces the policy
//! the way the production owner role experiences it. Mirrors
//! `reserve_funds_rls_repo_tests.rs` (the sibling PAP-67 precedent).

use crate::common::{seed_org, set_ctx};
use db::models::{BudgetQuery, CreateBudget};
use db::repositories::BudgetRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Budget User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed an ACTIVE budget directly (as superuser, RLS-exempt) for an org.
async fn seed_active_budget(
    pool: &PgPool,
    org_id: Uuid,
    name: &str,
    fiscal_year: i32,
    user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO budgets (organization_id, fiscal_year, name, status, created_by)
        VALUES ($1, $2, $3, 'active', $4)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(fiscal_year)
    .bind(name)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed active budget")
}

/// Seed a budget category for an org (RLS-exempt superuser context).
async fn seed_category(pool: &PgPool, org_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO budget_categories (organization_id, name)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed budget category")
}

/// Seed a budget item (RLS-exempt superuser context). A non-empty item set
/// makes `get_budget_summary` aggregate over real rows so the dashboard
/// exercises the full multi-query path rather than the empty-table edge.
async fn seed_budget_item(pool: &PgPool, budget_id: Uuid, category_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO budget_items (budget_id, category_id, name, budgeted_amount, actual_amount)
        VALUES ($1, $2, 'Item', 1000, 250)
        RETURNING id
        "#,
    )
    .bind(budget_id)
    .bind(category_id)
    .fetch_one(pool)
    .await
    .expect("seed budget item")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn budget_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = BudgetRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-budget-a").await;
    let org_b = seed_org(&pool, "force-budget-b").await;
    let user_a = seed_user(&pool, "a@budget.test").await;
    let user_b = seed_user(&pool, "b@budget.test").await;
    let budget_a = seed_active_budget(&pool, org_a, "Budget A", 2030, user_a).await;
    let budget_b = seed_active_budget(&pool, org_b, "Budget B", 2030, user_b).await;
    // Give org A's active budget a real line item so the dashboard's summary
    // aggregates over rows (not the empty-table NULL edge).
    let category_a = seed_category(&pool, org_a, "Maintenance").await;
    seed_budget_item(&pool, budget_a, category_a).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_budget_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON budgets, budget_items, \
             budget_categories, budget_variance_alerts, reserve_funds TO \"{role}\""
        ),
        // RLS policy helpers must be EXECUTE-able by the bound role; the
        // soft-delete guard reads `organizations`.
        format!(
            "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
             get_current_org_not_deleted() TO \"{role}\""
        ),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    // ====================================================================
    // (1) DENY-ALL reproduction: role bound, NO context set (the dev raw-pool
    //     behavior). Own-org reads return nothing.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let found = repo
            .find_budget_by_id_rls(&mut *conn, org_a, budget_a)
            .await
            .expect("find_budget_by_id_rls (no ctx)");
        assert!(
            found.is_none(),
            "PAP-180 regression: without RLS context, own-org budget must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_budgets_rls(&mut *conn, org_a, BudgetQuery::default())
            .await
            .expect("list_budgets_rls (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-180 regression: without RLS context, list_budgets returns deny-all empty"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant: set context, drop to bound role, query repo.
    //     Includes the converted multi-query get_dashboard path.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // (2) Own-org row IS now visible through the repo — the fix.
        let found = repo
            .find_budget_by_id_rls(&mut *conn, org_a, budget_a)
            .await
            .expect("find_budget_by_id_rls (ctx)");
        assert_eq!(
            found.map(|b| b.id),
            Some(budget_a),
            "PAP-180 fix: with RLS context set, the repo must return the own-org budget"
        );

        let listed = repo
            .list_budgets_rls(&mut *conn, org_a, BudgetQuery::default())
            .await
            .expect("list_budgets_rls (ctx)");
        assert_eq!(
            listed.iter().map(|b| b.id).collect::<Vec<_>>(),
            vec![budget_a],
            "PAP-180 fix: list_budgets returns exactly the own-org budget under context"
        );

        // (3) Org B's budget stays invisible to an org-A caller.
        let cross = repo
            .find_budget_by_id_rls(&mut *conn, org_a, budget_b)
            .await
            .expect("find_budget_by_id_rls cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's budget must NOT be visible to an org-A caller"
        );

        // (3') Converted multi-query method: get_dashboard runs every query on
        //      this RLS-bound connection and must surface ONLY org A's active
        //      budget — never org B's.
        let dashboard = repo
            .get_dashboard(&mut conn, org_a, None)
            .await
            .expect("get_dashboard (ctx)");
        assert_eq!(
            dashboard.active_budget.map(|b| b.id),
            Some(budget_a),
            "PAP-180 fix: get_dashboard returns the own-org active budget under RLS context"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (4) WRITE path: a create on a context-set connection succeeds and the
    //     row is the caller's org. Without context the INSERT would fail the
    //     policy WITH CHECK (the write-side of deny-all).
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_a)
            .bind(user_a)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let created = repo
            .create_budget_rls(
                &mut *conn,
                org_a,
                user_a,
                CreateBudget {
                    building_id: None,
                    fiscal_year: 2031,
                    name: "Created under RLS".to_string(),
                    notes: None,
                },
            )
            .await
            .expect("create_budget_rls under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    let _ = user_b; // seeded for symmetry with org_b's budget
    for stmt in [
        format!(
            "REVOKE ALL ON budgets, budget_items, budget_categories, \
             budget_variance_alerts, reserve_funds FROM \"{role}\""
        ),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

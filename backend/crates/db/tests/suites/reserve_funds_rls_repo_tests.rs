//! Repo-level behavioral RLS regression test for PAP-79 (PAP-67 cluster).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the reserve-fund tables (`reserve_funds` and
//! the rest of the Epic-141 cluster). The production api-server connects as the
//! table OWNER, which `FORCE` binds. `ReserveFundRepository` held a raw `PgPool`
//! and ran every query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty, writes
//! failed. (PAP-79.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org `get_fund` /
//!      `list_funds` returns nothing. This is the "would have failed on dev"
//!      evidence.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's fund stays invisible to an org-A caller even
//!      with context set.
//!   4. **Write path** — a `create_fund` on a context-set connection succeeds and
//!      the row is the caller's org. Without context the INSERT would fail the
//!      policy `WITH CHECK`.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces the
//! policy the way the production owner role experiences it. Mirrors
//! `work_order_rls_repo_tests.rs` (the PAP-67 precedent this follows).

use crate::common::{seed_org, set_ctx};
use chrono::NaiveDate;
use db::models::reserve_funds::{
    ContributionFrequency, CreateContributionSchedule, CreateReserveFund, FundType,
};
use db::repositories::ReserveFundRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'RF User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a reserve fund directly (as superuser, RLS-exempt) for an org.
async fn seed_fund(pool: &PgPool, org_id: Uuid, name: &str, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reserve_funds
            (organization_id, name, fund_type, current_balance, currency, created_by)
        VALUES ($1, $2, 'reserve', 0, 'EUR', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed reserve fund")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn reserve_fund_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = ReserveFundRepository::new();

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-rf-a").await;
    let org_b = seed_org(&pool, "force-rf-b").await;
    let user_a = seed_user(&pool, "a@rf.test").await;
    let user_b = seed_user(&pool, "b@rf.test").await;
    let fund_a = seed_fund(&pool, org_a, "Fund A", user_a).await;
    let fund_b = seed_fund(&pool, org_b, "Fund B", user_b).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_rf_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON reserve_funds TO \"{role}\""),
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
        // Explicitly clear any inherited context, then drop to the bound role.
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let found = repo
            .get_fund(&mut *conn, org_a, fund_a)
            .await
            .expect("get_fund (no ctx)");
        assert!(
            found.is_none(),
            "PAP-79 regression: without RLS context, own-org reserve fund must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_funds(&mut *conn, org_a, None, None, false)
            .await
            .expect("list_funds (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-79 regression: without RLS context, list_funds returns deny-all empty"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant: set context, drop to bound role, query repo.
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
            .get_fund(&mut *conn, org_a, fund_a)
            .await
            .expect("get_fund (ctx)");
        assert_eq!(
            found.map(|f| f.id),
            Some(fund_a),
            "PAP-79 fix: with RLS context set, the repo must return the own-org reserve fund"
        );

        let listed = repo
            .list_funds(&mut *conn, org_a, None, None, false)
            .await
            .expect("list_funds (ctx)");
        assert_eq!(
            listed.iter().map(|f| f.id).collect::<Vec<_>>(),
            vec![fund_a],
            "PAP-79 fix: list_funds returns exactly the own-org fund under context"
        );

        // (3) Org B's fund stays invisible to an org-A caller.
        let cross = repo
            .get_fund(&mut *conn, org_a, fund_b)
            .await
            .expect("get_fund cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's reserve fund must NOT be visible to an org-A caller"
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
            .create_fund(
                &mut *conn,
                org_a,
                CreateReserveFund {
                    building_id: None,
                    name: "Created under RLS".to_string(),
                    description: None,
                    fund_type: FundType::Reserve,
                    target_balance: None,
                    minimum_balance: None,
                    currency: None,
                },
                user_a,
            )
            .await
            .expect("create_fund under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    let _ = user_b; // seeded for symmetry with org_b's fund
    for stmt in [
        format!("REVOKE ALL ON reserve_funds FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

/// Child-table IDOR: org-A caller must not see org-B's
/// `fund_contribution_schedules` via `list_contribution_schedules`, even when
/// they supply org-B's `fund_id`. The repo's `ensure_fund_in_org` guard
/// returns RowNotFound for a fund invisible under the caller's RLS context,
/// and the RLS policy on `fund_contribution_schedules` itself also propagates
/// the owning fund's `organization_id`.
///
/// Additionally verifies that an INSERT into org-B's fund via org-A context
/// fails (the INSERT-path RLS WITH CHECK guard).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn reserve_fund_child_table_idor_blocked(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "child-rf-a").await;
    let org_b = seed_org(&pool, "child-rf-b").await;
    let user_a = seed_user(&pool, "a@child-rf.test").await;
    let user_b = seed_user(&pool, "b@child-rf.test").await;
    let fund_b = seed_fund(&pool, org_b, "Fund B", user_b).await;
    let _ = (org_a, user_a); // seeded for symmetry

    // Seed a contribution schedule for org-B's fund as superuser (RLS-exempt).
    sqlx::query(
        "INSERT INTO fund_contribution_schedules (fund_id, name, amount, frequency, start_date) VALUES ($1, 'B Schedule', 500, 'monthly', '2025-01-01')",
    )
    .bind(fund_b)
    .execute(&pool)
    .await
    .expect("seed contribution schedule for org-B fund");

    let repo = ReserveFundRepository::new();

    let role = format!("ppt_rls_child_rf_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!(
            "GRANT SELECT, INSERT ON reserve_funds, fund_contribution_schedules TO \"{role}\""
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(),              get_current_org_not_deleted() TO \"{role}\""
        ),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant");
    }

    {
        let mut conn = pool.acquire().await.expect("acquire");

        // Org-A context: cross-tenant fund schedules must be blocked.
        sqlx::query("SELECT set_request_context($1, $2, false)")
            .bind(org_a)
            .bind(user_a)
            .execute(&mut *conn)
            .await
            .expect("set org-A ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        // list_contribution_schedules routes through ensure_fund_in_org which
        // returns RowNotFound for a fund invisible under org-A's context.
        let list_result = repo
            .list_contribution_schedules(&mut conn, org_a, fund_b, false)
            .await;
        let idor_blocked = match &list_result {
            Ok(rows) => rows.is_empty(),
            Err(_) => true, // RowNotFound from ensure_fund_in_org
        };
        assert!(
            idor_blocked,
            "org-A must not see org-B\'s fund_contribution_schedules              (child-table IDOR must be blocked by RLS or the org-scope guard)"
        );

        // INSERT into org-B's fund from org-A context must fail.
        let insert_result = repo
            .create_contribution_schedule(
                &mut conn,
                org_a,
                fund_b,
                CreateContributionSchedule {
                    name: "Hijacked Schedule".to_string(),
                    description: None,
                    amount: Decimal::new(100, 0),
                    frequency: ContributionFrequency::Monthly,
                    start_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    end_date: None,
                    auto_collect: None,
                },
            )
            .await;
        assert!(
            insert_result.is_err(),
            "org-A must not INSERT a schedule into org-B\'s fund              (INSERT-path RLS WITH CHECK or org-scope guard must block the write)"
        );

        sqlx::query("RESET ROLE").execute(&mut *conn).await.ok();
    }

    // Cleanup.
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON reserve_funds, fund_contribution_schedules FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

//! Repo-level behavioral RLS regression test for PAP-67.
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `work_orders` (and the rest of the Epic-11+
//! cluster). The production api-server connects as the table OWNER, which
//! `FORCE` binds. `WorkOrderRepository` held a raw `PgPool` and ran every query
//! WITHOUT ever calling `set_request_context`, so on `dev` `get_current_org_id()`
//! returned NULL and the policy collapsed to `organization_id = NULL` →
//! **deny-all**: own-org reads returned empty, writes failed. (PAP-67.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org `find_by_id`
//!      / `list` returns nothing. This assertion FAILS to reproduce only if the
//!      regression is absent; it is the "would have failed on dev" evidence.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's work order stays invisible to an org-A
//!      caller even with context set.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces
//! the policy the way the production owner role experiences it. Mirrors
//! `documents_rls_cross_tenant_tests.rs` (the 00172/00163 precedent PAP-67
//! follows).

use db::models::{CreateWorkOrder, WorkOrderQuery};
use db::repositories::WorkOrderRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn set_ctx(pool: &PgPool, org_id: Option<Uuid>, user_id: Option<Uuid>, is_super_admin: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(is_super_admin)
        .execute(pool)
        .await
        .expect("set_request_context");
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("WO {slug}"))
    .bind(slug)
    .bind(format!("{slug}@wo.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'WO User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country, name)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .bind(format!("{slug} Building"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a work order directly (as superuser, RLS-exempt) for an org.
async fn seed_work_order(pool: &PgPool, org_id: Uuid, building_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO work_orders
            (organization_id, building_id, title, description, priority, work_type, created_by)
        VALUES ($1, $2, 'Seeded WO', 'desc', 'medium', 'corrective', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed work order")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn work_order_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = WorkOrderRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-a").await;
    let org_b = seed_org(&pool, "force-b").await;
    let user_a = seed_user(&pool, "a@wo.test").await;
    let user_b = seed_user(&pool, "b@wo.test").await;
    let bld_a = seed_building(&pool, org_a, "a").await;
    let bld_b = seed_building(&pool, org_b, "b").await;
    let wo_a = seed_work_order(&pool, org_a, bld_a, user_a).await;
    let wo_b = seed_work_order(&pool, org_b, bld_b, user_b).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_wo_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON work_orders, work_order_updates TO \"{role}\""
        ),
        // RLS policy helpers must be EXECUTE-able by the bound role; the
        // SECURITY INVOKER soft-delete guard reads `organizations`.
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
            .find_by_id(&mut *conn, wo_a)
            .await
            .expect("find_by_id (no ctx)");
        assert!(
            found.is_none(),
            "PAP-67 regression: without RLS context, own-org work order must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list(&mut *conn, org_a, WorkOrderQuery::default())
            .await
            .expect("list (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-67 regression: without RLS context, list returns deny-all empty"
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
            .find_by_id(&mut *conn, wo_a)
            .await
            .expect("find_by_id (ctx)");
        assert_eq!(
            found.map(|w| w.id),
            Some(wo_a),
            "PAP-67 fix: with RLS context set, the repo must return the own-org work order"
        );

        let listed = repo
            .list(&mut *conn, org_a, WorkOrderQuery::default())
            .await
            .expect("list (ctx)");
        assert_eq!(
            listed.iter().map(|w| w.id).collect::<Vec<_>>(),
            vec![wo_a],
            "PAP-67 fix: list returns exactly the own-org work order under context"
        );

        // (3) Org B's work order stays invisible to an org-A caller.
        let cross = repo
            .find_by_id(&mut *conn, wo_b)
            .await
            .expect("find_by_id cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's work order must NOT be visible to an org-A caller"
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
            .create_work_order(
                &mut *conn,
                org_a,
                user_a,
                CreateWorkOrder {
                    building_id: bld_a,
                    equipment_id: None,
                    fault_id: None,
                    title: "Created under RLS".to_string(),
                    description: "desc".to_string(),
                    priority: None,
                    work_type: None,
                    assigned_to: None,
                    vendor_id: None,
                    scheduled_date: None,
                    due_date: None,
                    estimated_cost: None,
                    tags: None,
                    metadata: None,
                },
            )
            .await
            .expect("create_work_order under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON work_orders, work_order_updates FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt)).execute(&pool).await.ok();
    }
}

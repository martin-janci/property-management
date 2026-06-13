//! Repo-level behavioral test for the work-order RLS routing (PAP-67 / PAP-179).
//!
//! Background
//! ----------
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on every work-order table (`work_orders`,
//! `work_order_updates`, `maintenance_schedules`, `schedule_executions`). Under
//! `FORCE` the api-server's owner connection is no longer exempt, so a query
//! issued on a connection WITHOUT `app.current_org_id` set collapses to
//! deny-all. The `WorkOrderRepository` was reworked (PAP-179) to hold no pool
//! and to take an RLS-context-bearing executor on every method (supplied by
//! `RlsConnection` in handlers). This test exercises that contract directly
//! against the repo.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser. To exercise the policy
//! the way the production owner role does, the test creates a plain
//! `NOSUPERUSER NOBYPASSRLS` role, grants it table access, and `SET ROLE`s to
//! it. `FORCE` binds that role, so the org-scoped policy is enforced. The org
//! context (`app.current_org_id`) is a session GUC and survives `SET ROLE`.

use db::repositories::WorkOrderRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("WorkOrders {slug}"))
    .bind(slug)
    .bind(format!("{slug}@work-orders.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'WorkOrders User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, street: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code)
        VALUES ($1, $2, 'Bratislava', '81101')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(street)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_work_order(
    pool: &PgPool,
    org_id: Uuid,
    building_id: Uuid,
    created_by: Uuid,
    title: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO work_orders (organization_id, building_id, title, description, created_by)
        VALUES ($1, $2, $3, 'seeded', $4)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(title)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed work order")
}

/// FORCE RLS on `work_orders` must:
///   1. deny ALL reads on a connection that has NO org context set
///      (the deny-all regression that 00179's FORCE introduces for any code
///      path that forgot to route through `RlsConnection`); and
///   2. expose the caller's OWN org's work order while hiding another org's
///      work order once the org context IS set.
///
/// Both assertions go through `WorkOrderRepository::find_by_id` — the exact repo
/// method the work-order handlers call — so this is a behavioral test of the RLS
/// routing, not just of the raw SQL policy.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn work_orders_force_rls_requires_context_and_blocks_cross_tenant(pool: PgPool) {
    // --- Seed as superuser (super-admin context satisfies the roles-trigger
    //     RLS WITH CHECK fired by INSERT INTO organizations). ---
    sqlx::query("SELECT set_request_context(NULL, NULL, true)")
        .execute(&pool)
        .await
        .expect("set super-admin context");

    let org_a = seed_org(&pool, "wo-force-a").await;
    let org_b = seed_org(&pool, "wo-force-b").await;
    let user_a = seed_user(&pool, "a@work-orders.test").await;
    let user_b = seed_user(&pool, "b@work-orders.test").await;
    let bldg_a = seed_building(&pool, org_a, "1 Alpha St").await;
    let bldg_b = seed_building(&pool, org_b, "1 Bravo St").await;

    let wo_a = seed_work_order(&pool, org_a, bldg_a, user_a, "alpha-job").await;
    let wo_b = seed_work_order(&pool, org_b, bldg_b, user_b, "bravo-job").await;

    let repo = WorkOrderRepository::new(pool.clone());

    // --- Create a NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    // Random suffix keeps the role name unique across parallel test DBs.
    let role = format!("ppt_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON work_orders TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on work_orders");
    // The policy calls helper functions (get_current_org_id, is_super_admin,
    // get_current_org_not_deleted); the role must be able to EXECUTE them.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
         get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute on rls helpers");
    // `get_current_org_not_deleted()` is SECURITY INVOKER; its body runs
    // `SELECT 1 FROM organizations` with the CALLER's privileges, so the bound
    // role must be able to read `organizations` or the soft-delete guard raises
    // `permission denied for table organizations`. `organizations` has no RLS
    // of its own, so a plain table-level SELECT grant is sufficient.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on organizations");

    // --- All role-bound work runs on ONE dedicated connection. `SET ROLE` and
    //     the org-context GUC are session state, so they must not be spread
    //     across pool acquisitions. ---
    {
        let mut conn = pool.acquire().await.expect("acquire connection");

        // ---- Phase 1: NO org context → deny-all under FORCE. ----
        // This is the deny-all regression a raw-pool repo would hit: without
        // `app.current_org_id`, even the OWN-org row is invisible.
        sqlx::query("SELECT set_request_context(NULL, NULL, false)")
            .execute(&mut *conn)
            .await
            .expect("clear org context");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let no_ctx = repo
            .find_by_id(&mut *conn, wo_a)
            .await
            .expect("find_by_id (no context)");
        assert!(
            no_ctx.is_none(),
            "without org context a FORCE-RLS work order read must return None (deny-all)"
        );

        // ---- Phase 2: org-A context → own visible, cross-tenant hidden. ----
        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role before re-setting context");
        sqlx::query("SELECT set_request_context($1, $2, false)")
            .bind(org_a)
            .bind(user_a)
            .execute(&mut *conn)
            .await
            .expect("set org-A context");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let own = repo
            .find_by_id(&mut *conn, wo_a)
            .await
            .expect("find_by_id (org-A own)");
        assert_eq!(
            own.map(|w| w.id),
            Some(wo_a),
            "org A's own work order must be visible under its own context"
        );

        let cross = repo
            .find_by_id(&mut *conn, wo_b)
            .await
            .expect("find_by_id (org-B cross-tenant)");
        assert!(
            cross.is_none(),
            "org B's work order must NOT be visible to an org-A caller (cross-tenant IDOR)"
        );

        // Drop back to superuser on this connection before it returns to the pool.
        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    sqlx::query("SELECT set_request_context(NULL, NULL, true)")
        .execute(&pool)
        .await
        .expect("restore super-admin context");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "REVOKE ALL ON work_orders FROM \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "REVOKE ALL ON organizations FROM \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP ROLE IF EXISTS \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
}

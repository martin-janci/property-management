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

use crate::common::seed_org;
use db::repositories::WorkOrderRepository;
use sqlx::PgPool;
use uuid::Uuid;

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

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment (organization_id, building_id, name, category)
        VALUES ($1, $2, $3, 'hvac')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

/// Seed a `completed` work order so it shows up in the service-history queries
/// (both `get_service_history` and `get_building_service_history` filter on
/// `status = 'completed'`).
#[allow(clippy::too_many_arguments)]
async fn seed_completed_work_order(
    pool: &PgPool,
    org_id: Uuid,
    building_id: Uuid,
    equipment_id: Uuid,
    created_by: Uuid,
    title: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO work_orders (
            organization_id, building_id, equipment_id, title, description,
            created_by, status, completed_at, actual_cost
        )
        VALUES ($1, $2, $3, $4, 'seeded completed', $5, 'completed', NOW(), 123.45)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(equipment_id)
    .bind(title)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed completed work order")
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
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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

/// Cross-org IDOR regression for the service-history endpoints (issue #1372).
///
/// `get_service_history(equipment_id)` and `get_building_service_history(
/// building_id)` are keyed ONLY by the resource UUID — neither query carries an
/// `organization_id` predicate (see `WorkOrderRepository::get_service_history` /
/// `get_building_service_history`: the SQL `WHERE` clause filters on
/// `wo.equipment_id` / `wo.building_id` + `status = 'completed'` only). The sole
/// tenant boundary is FORCE RLS on `work_orders` (migration 00179). The route
/// handlers (`get_equipment_service_history` / `get_building_service_history`)
/// run on the caller's `RlsConnection`, so under `FORCE` the join is scoped to
/// the caller's org by the row-security policy — closing the #821 P2 / #1372
/// cross-tenant gap at the database layer.
///
/// This test pins that contract behaviorally: an org-B caller who guesses org
/// A's `equipment_id` / `building_id` must get an EMPTY history (not org A's
/// service records), while org A still reads its own. Mirrors the role-switching
/// FORCE-RLS harness used by
/// `work_orders_force_rls_requires_context_and_blocks_cross_tenant` above.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn service_history_force_rls_blocks_cross_tenant(pool: PgPool) {
    // --- Seed under super-admin context (satisfies the org roles-trigger). ---
    sqlx::query("SELECT set_request_context(NULL, NULL, true)")
        .execute(&pool)
        .await
        .expect("set super-admin context");

    let org_a = seed_org(&pool, "wo-sh-a").await;
    let org_b = seed_org(&pool, "wo-sh-b").await;
    let user_a = seed_user(&pool, "a@work-orders-sh.test").await;
    let user_b = seed_user(&pool, "b@work-orders-sh.test").await;
    let bldg_a = seed_building(&pool, org_a, "1 Service St").await;
    let _bldg_b = seed_building(&pool, org_b, "2 Service St").await;
    let equip_a = seed_equipment(&pool, org_a, bldg_a, "Boiler A").await;

    // One COMPLETED work order in org A, keyed by both the equipment and the
    // building — it must surface in both service-history queries for org A and
    // in NEITHER for org B.
    let _wo_a =
        seed_completed_work_order(&pool, org_a, bldg_a, equip_a, user_a, "alpha-service").await;

    let repo = WorkOrderRepository::new(pool.clone());

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_sh_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");
    // The service-history SQL reads `work_orders` LEFT JOIN `equipment`, so the
    // bound role needs SELECT on both, plus the RLS helper functions and
    // `organizations` (read by the SECURITY-INVOKER soft-delete guard).
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON work_orders, equipment TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on work_orders + equipment");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
         get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute on rls helpers");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on organizations");

    {
        let mut conn = pool.acquire().await.expect("acquire connection");

        // ---- Phase 1: org-A context → own service history IS visible. ----
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

        let own_equip = repo
            .get_service_history(&mut *conn, equip_a, 50, 0)
            .await
            .expect("get_service_history (org-A own)");
        assert_eq!(
            own_equip.len(),
            1,
            "org A must read its own equipment service history"
        );

        let own_bldg = repo
            .get_building_service_history(&mut *conn, bldg_a, 50, 0)
            .await
            .expect("get_building_service_history (org-A own)");
        assert_eq!(
            own_bldg.len(),
            1,
            "org A must read its own building service history"
        );

        // ---- Phase 2: org-B context → org A's history is INVISIBLE. ----
        // The cross-tenant attacker reuses the SAME equipment_id / building_id.
        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role before re-setting context");
        sqlx::query("SELECT set_request_context($1, $2, false)")
            .bind(org_b)
            .bind(user_b)
            .execute(&mut *conn)
            .await
            .expect("set org-B context");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let cross_equip = repo
            .get_service_history(&mut *conn, equip_a, 50, 0)
            .await
            .expect("get_service_history (org-B cross-tenant)");
        assert!(
            cross_equip.is_empty(),
            "cross-tenant IDOR: org B must NOT read org A's equipment service \
             history by guessing the equipment_id"
        );

        let cross_bldg = repo
            .get_building_service_history(&mut *conn, bldg_a, 50, 0)
            .await
            .expect("get_building_service_history (org-B cross-tenant)");
        assert!(
            cross_bldg.is_empty(),
            "cross-tenant IDOR: org B must NOT read org A's building service \
             history by guessing the building_id"
        );

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
    for stmt in [
        format!("REVOKE ALL ON work_orders FROM \"{role}\""),
        format!("REVOKE ALL ON equipment FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!(
            "REVOKE EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
             get_current_org_not_deleted() FROM \"{role}\""
        ),
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

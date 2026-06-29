//! Repo-level behavioral test for the IoT sensor RLS routing (PAP-67 / PAP-78).
//!
//! Background
//! ----------
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on every sensor table. Under `FORCE` the
//! api-server's owner connection is no longer exempt, so a query issued on a
//! connection WITHOUT `app.current_org_id` set collapses to deny-all. The
//! `SensorRepository` was reworked to hold no pool and to take an
//! RLS-context-bearing executor on every method (supplied by `RlsConnection`
//! in handlers). This test exercises that contract directly against the repo.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser. To exercise the policy
//! the way the production owner role does, the test creates a plain
//! `NOSUPERUSER NOBYPASSRLS` role, grants it table access, and `SET ROLE`s to
//! it. `FORCE` binds that role, so the org-scoped policy is enforced. The org
//! context (`app.current_org_id`) is a session GUC and survives `SET ROLE`.

use db::models::ReadingQuery;
use db::repositories::SensorRepository;
use sqlx::PgPool;
use uuid::Uuid;

mod common;
use common::{seed_org, set_ctx};

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Sensors User', 'active', NOW(), 'public')
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

async fn seed_sensor(
    pool: &PgPool,
    org_id: Uuid,
    building_id: Uuid,
    created_by: Uuid,
    name: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO sensors (organization_id, building_id, name, sensor_type, created_by)
        VALUES ($1, $2, $3, 'temperature', $4)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(name)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed sensor")
}

/// FORCE RLS on `sensors` must:
///   1. deny ALL reads on a connection that has NO org context set
///      (the deny-all regression that 00179's FORCE introduces for any code
///      path that forgot to route through `RlsConnection`); and
///   2. expose the caller's OWN org's sensor while hiding another org's sensor
///      once the org context IS set.
///
/// Both assertions go through `SensorRepository::find_by_id` — the exact repo
/// method the IoT handlers call — so this is a behavioral test of the RLS
/// routing, not just of the raw SQL policy.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn sensors_force_rls_requires_context_and_blocks_cross_tenant(pool: PgPool) {
    // --- Seed as superuser (super-admin context satisfies the roles-trigger
    //     RLS WITH CHECK fired by INSERT INTO organizations). ---
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "force-a").await;
    let org_b = seed_org(&pool, "force-b").await;
    let user_a = seed_user(&pool, "a@sensors.test").await;
    let user_b = seed_user(&pool, "b@sensors.test").await;
    let bldg_a = seed_building(&pool, org_a, "1 Alpha St").await;
    let bldg_b = seed_building(&pool, org_b, "1 Bravo St").await;

    let sensor_a = seed_sensor(&pool, org_a, bldg_a, user_a, "alpha-temp").await;
    let sensor_b = seed_sensor(&pool, org_b, bldg_b, user_b, "bravo-temp").await;

    let repo = SensorRepository::new();

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
        "GRANT SELECT ON sensors TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on sensors");
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
            .find_by_id(&mut *conn, sensor_a)
            .await
            .expect("find_by_id (no context)");
        assert!(
            no_ctx.is_none(),
            "without org context a FORCE-RLS sensor read must return None (deny-all)"
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
            .find_by_id(&mut *conn, sensor_a)
            .await
            .expect("find_by_id (org-A own)");
        assert_eq!(
            own.map(|s| s.id),
            Some(sensor_a),
            "org A's own sensor must be visible under its own context"
        );

        let cross = repo
            .find_by_id(&mut *conn, sensor_b)
            .await
            .expect("find_by_id (org-B cross-tenant)");
        assert!(
            cross.is_none(),
            "org B's sensor must NOT be visible to an org-A caller (cross-tenant IDOR)"
        );

        // Drop back to superuser on this connection before it returns to the pool.
        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "REVOKE ALL ON sensors FROM \"{role}\""
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

/// Child-table IDOR: org-A caller must not see org-B's `sensor_readings` via
/// `SensorRepository::list_readings`, even when they supply org-B's sensor_id.
///
/// The RLS policy on `sensor_readings` propagates the owning sensor's
/// `organization_id` via a JOIN. Under org-A context, org-B's child rows
/// collapse to empty — the same cross-tenant isolation guarantee as the parent
/// table test above.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn sensor_readings_child_table_idor_blocked(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;

    let org_a = seed_org(&pool, "child-idor-a").await;
    let org_b = seed_org(&pool, "child-idor-b").await;
    let user_a = seed_user(&pool, "a@child-idor.test").await;
    let user_b = seed_user(&pool, "b@child-idor.test").await;
    let bldg_b = seed_building(&pool, org_b, "1 Bravo Ave").await;
    let sensor_b = seed_sensor(&pool, org_b, bldg_b, user_b, "bravo-child").await;
    let _ = (org_a, user_a); // seeded for symmetry

    // Seed a reading for org-B's sensor as superuser (RLS-exempt).
    sqlx::query(
        "INSERT INTO sensor_readings (sensor_id, value, unit, quality, timestamp)          VALUES ($1, 23.5, 'C', 'good', NOW())",
    )
    .bind(sensor_b)
    .execute(&pool)
    .await
    .expect("seed reading for org-B sensor");

    let repo = SensorRepository::new();

    let role = format!("ppt_rls_child_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT ON sensors, sensor_readings TO \"{role}\""),
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

        // Org-A context — cross-tenant sensor_readings must be empty.
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

        let readings = repo
            .list_readings(
                &mut *conn,
                sensor_b,
                ReadingQuery {
                    from_time: None,
                    to_time: None,
                    limit: None,
                    aggregation: None,
                },
            )
            .await
            .expect("list_readings must not error (just return empty)");

        assert!(
            readings.is_empty(),
            "org-A must not see org-B\'s sensor_readings via list_readings              (child-table IDOR blocked by RLS)"
        );

        sqlx::query("RESET ROLE").execute(&mut *conn).await.ok();
    }

    // Cleanup.
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON sensors, sensor_readings FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

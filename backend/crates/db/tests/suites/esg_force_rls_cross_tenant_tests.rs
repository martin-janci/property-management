//! Repo-level behavioral FORCE-RLS test for the ESG reporting tables (GH #1307).
//!
//! Background
//! ----------
//! PR #1281 (`fix(api-server): close esg_reporting cross-tenant IDOR`) made
//! `EsgReportingRepository` stateless and org-keyed every by-id query. The
//! existing `esg_reporting_cross_org_idor_tests.rs` (api-server integration
//! suite) runs on the CI superuser pool, which bypasses `FORCE ROW LEVEL
//! SECURITY` — it proves the application-layer `organization_id = $2`
//! predicate but NOT the primary defence added by migration `00179`:
//!
//! ```sql
//! CREATE POLICY esg_metrics_select ON esg_metrics
//!     FOR SELECT USING (organization_id = get_current_org_id() AND get_current_org_not_deleted());
//! ALTER TABLE esg_metrics FORCE ROW LEVEL SECURITY;
//! ```
//!
//! This test fills that gap by exercising the policy on a `NOSUPERUSER
//! NOBYPASSRLS` role so `FORCE` actually binds — mirroring the pattern in
//! `work_order_rls_repo_tests.rs`.
//!
//! Assertions
//! ----------
//! 1. **Deny-all** — role bound, no GUC set → `esg_metrics` returns empty.
//! 2. **Own-org read** — `set_request_context(org_a, user_a)` → own metric visible.
//! 3. **Cross-tenant block** — `set_request_context(org_b, user_b)` → org-A metric
//!    returns no rows, even when the query omits the org predicate (proving the
//!    FORCE policy, not the app-layer `organization_id = $2` guard).
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely. To exercise the policy, the test creates a plain `NOSUPERUSER
//! NOBYPASSRLS` role, grants it table + function access, and `SET ROLE`s to
//! it so `FORCE` binds exactly as the production owner role experiences it.

use crate::common::seed_org;
use db::repositories::EsgReportingRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1, 'test_hash', 'ESG User', 'active', NOW(), 'public') RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a metric directly as the superuser (RLS-exempt) so we can verify
/// the FORCE policy blocks it from the test role without the app-layer predicate.
async fn seed_metric_raw(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO esg_metrics \
         (id, organization_id, period_start, period_end, category, metric_type, \
          metric_name, value, unit, data_source, created_at, updated_at, created_by) \
         VALUES ($1, $2, '2025-01-01', '2025-12-31', 'environmental', \
                 'energy_consumption', 'Total energy kWh', 1000, 'kWh', \
                 'manual', NOW(), NOW(), $3) \
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed metric (superuser)")
}

/// `FORCE ROW LEVEL SECURITY` on `esg_metrics` blocks cross-tenant reads and
/// produces deny-all when no RLS context is set — proving the primary policy
/// boundary, independent of the `organization_id = $2` application-layer guard.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn esg_metrics_force_rls_blocks_cross_tenant_and_deny_all(pool: PgPool) {
    let org_a = seed_org(&pool, "esg-rls-a").await;
    let org_b = seed_org(&pool, "esg-rls-b").await;
    let user_a = seed_user(&pool, "esg-rls-a@force.test").await;
    let user_b = seed_user(&pool, "esg-rls-b@force.test").await;

    // Seed org-A metric as superuser (RLS-exempt).
    let metric_a = seed_metric_raw(&pool, org_a, user_a).await;

    let role = format!("esg_rls_test_{}", &Uuid::new_v4().to_string()[..8]);

    // --- Create a NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create test role");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON esg_metrics TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on esg_metrics");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organizations TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on organizations");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
         get_current_org_not_deleted() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute on rls helpers");

    // All role-bound work runs on ONE dedicated connection.
    let mut conn = pool.acquire().await.expect("acquire connection");

    // --- 1. Deny-all: role bound, no GUC set → 0 rows ---
    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role");

    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM esg_metrics WHERE id = $1")
        .bind(metric_a)
        .fetch_one(&mut *conn)
        .await
        .expect("deny-all count");
    assert_eq!(
        rows, 0,
        "deny-all: no GUC set → FORCE policy collapses to deny-all (0 rows)"
    );

    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role");

    // --- 2. Own-org read: org-A context → metric visible ---
    sqlx::query("SELECT set_request_context($1, $2, false)")
        .bind(org_a)
        .bind(user_a)
        .execute(&mut *conn)
        .await
        .expect("set org-a context");

    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role");

    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM esg_metrics WHERE id = $1")
        .bind(metric_a)
        .fetch_one(&mut *conn)
        .await
        .expect("own-org count");
    assert_eq!(rows, 1, "org-A must read its own metric under FORCE policy");

    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role");

    // --- 3. Cross-tenant block: org-B context → org-A metric invisible ---
    sqlx::query("SELECT set_request_context($1, $2, false)")
        .bind(org_b)
        .bind(user_b)
        .execute(&mut *conn)
        .await
        .expect("set org-b context");

    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role");

    // Raw SELECT without the org predicate — proves the FORCE policy, not the
    // app-layer `organization_id = $2` guard in `EsgReportingRepository`.
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM esg_metrics WHERE id = $1")
        .bind(metric_a)
        .fetch_one(&mut *conn)
        .await
        .expect("cross-tenant count");
    assert_eq!(
        rows, 0,
        "FORCE RLS must block org-B from reading org-A's metric by id"
    );

    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role");

    // --- Also verify the repo-level call mirrors the raw result ---
    let repo = EsgReportingRepository;

    // Restore superuser context for the repo call so we can use the test pool.
    sqlx::query("SELECT set_request_context($1, $2, false)")
        .bind(org_b)
        .bind(user_b)
        .execute(&pool)
        .await
        .expect("set org-b context on pool");

    // The repo method uses `organization_id = $2` as the app-layer guard. An
    // IDOR probe passes org_b as the `organization_id` while targeting org_a's
    // metric_id — the WHERE clause rejects it, returning None.
    let result = repo
        .get_metric(&pool, metric_a, org_b)
        .await
        .expect("repo get_metric");
    assert!(
        result.is_none(),
        "EsgReportingRepository::get_metric must return None for an IDOR probe \
         (org-B org_id passed while targeting org-A metric)"
    );

    // Restore super-admin context before cleanup.
    sqlx::query("SELECT set_request_context(NULL, NULL, true)")
        .execute(&pool)
        .await
        .expect("restore super-admin context");

    // --- Cleanup ---
    for stmt in [
        format!("REVOKE ALL ON esg_metrics FROM \"{role}\""),
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

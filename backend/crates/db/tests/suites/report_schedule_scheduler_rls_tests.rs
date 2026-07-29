//! RLS regression test for the report-schedule due-work consumer (issue #2318,
//! follow-up to PR #2313).
//!
//! Background
//! ----------
//! `report_schedules` / `report_executions` (migration 00162) carry FORCE ROW
//! LEVEL SECURITY with an org-scoped policy, and the production api-server
//! connects as the table OWNER — bound by FORCE, no BYPASSRLS (00179). The
//! scheduler's `fire_due_report_schedules` loop originally ran on the bare
//! pool with no GUC set, so:
//!
//!   * `get_due_schedules()` silently returned 0 rows (policy false),
//!   * `record_execution()`'s INSERT failed its WITH CHECK,
//!   * `advance_after_run()`'s UPDATE matched 0 rows → RowNotFound.
//!
//! Migration 00218 adds an `is_global_read_context()` SELECT-only leg to both
//! tables, and the scheduler now reads due work under the global-read context
//! and writes under each schedule's own tenant context.
//!
//! This test exercises the full select → record → advance loop through the
//! repository methods on a `NOSUPERUSER NOBYPASSRLS` role (FORCE does not bind
//! the `#[sqlx::test]` superuser, so a behavioral assertion would otherwise
//! pass vacuously — same role-switch discipline as `budget_rls_repo_tests.rs`):
//!
//!   1. **Deny-all reproduction** — role bound, NO context set (the exact
//!      pre-fix scheduler behavior): `get_due_schedules` returns nothing even
//!      though a due schedule exists.
//!   2. **Global-read fix** — with `set_global_read_context(TRUE)` the due
//!      schedule is visible cross-org, and the global-read context cannot
//!      WRITE (INSERT into `report_executions` is rejected).
//!   3. **Tenant-context writes** — under
//!      `set_tenant_context(schedule.organization_id)`, `record_execution`
//!      creates a `pending` row and `advance_after_run(id, None)` parks the
//!      schedule (NULL `next_run_at`), after which it stops appearing as due.

use crate::common::{seed_org, set_ctx};
use db::repositories::ReportScheduleRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed a due schedule directly (as superuser, RLS-exempt): active, with a
/// `next_run_at` one hour in the past so `get_due_schedules` should return it.
async fn seed_due_schedule(pool: &PgPool, org_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO report_schedules
            (report_id, organization_id, name, frequency, is_active, status, next_run_at)
        VALUES ($1, $2, $3, 'daily', TRUE, 'active', NOW() - INTERVAL '1 hour')
        RETURNING id
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed due schedule")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn scheduler_due_work_loop_under_force_rls(pool: PgPool) {
    let repo = ReportScheduleRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context. ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(
        &pool,
        format!("rls-reports-a-{}", Uuid::new_v4().simple()).as_str(),
    )
    .await;
    let org_b = seed_org(
        &pool,
        format!("rls-reports-b-{}", Uuid::new_v4().simple()).as_str(),
    )
    .await;
    let schedule_a = seed_due_schedule(&pool, org_a, "Due schedule A").await;
    let schedule_b = seed_due_schedule(&pool, org_b, "Due schedule B").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds (the production
    //     owner role experiences the policies the same way). ---
    let role = format!("ppt_rls_reports_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON report_schedules, report_executions TO \"{role}\""
        ),
        format!(
            "GRANT EXECUTE ON FUNCTION get_current_org_id(), is_super_admin(), \
             is_global_read_context(), set_global_read_context(BOOLEAN), \
             set_tenant_context(UUID), clear_request_context() TO \"{role}\""
        ),
        format!("GRANT SELECT ON organizations TO \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .expect("grant setup");
    }

    let mut conn = pool.acquire().await.expect("acquire");
    sqlx::query("SELECT clear_request_context()")
        .execute(&mut *conn)
        .await
        .expect("clear ctx");
    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role");

    // ====================================================================
    // (1) DENY-ALL reproduction: no context set — the pre-fix scheduler
    //     behavior. Both due schedules exist but the read returns nothing.
    // ====================================================================
    let due = repo
        .get_due_schedules(&mut *conn)
        .await
        .expect("due query without context should not error");
    assert!(
        due.is_empty(),
        "FORCE RLS with no context must silently hide due schedules (pre-#2318 no-op repro)"
    );

    // ====================================================================
    // (2) Global-read context: the cross-org due-work read now works.
    // ====================================================================
    db::tenant_context::set_global_read_context(&mut *conn, true)
        .await
        .expect("set global-read context");

    let due = repo
        .get_due_schedules(&mut *conn)
        .await
        .expect("due query under global-read context");
    let due_ids: Vec<Uuid> = due.iter().map(|s| s.id).collect();
    assert!(
        due_ids.contains(&schedule_a) && due_ids.contains(&schedule_b),
        "global-read context must see due schedules across orgs, got {due_ids:?}"
    );
    let sched_a = due
        .iter()
        .find(|s| s.id == schedule_a)
        .expect("schedule A present");
    assert_eq!(sched_a.organization_id, org_a, "row carries its own org id");

    // Global-read is SELECT-only: recording an execution from this context
    // must be rejected by the org-scoped WITH CHECK.
    let write_from_global = repo.record_execution(&mut *conn, schedule_a).await;
    assert!(
        write_from_global.is_err(),
        "global-read context must not be able to INSERT report_executions"
    );

    // ====================================================================
    // (3) Tenant-context writes: record + advance under the schedule's org.
    // ====================================================================
    db::tenant_context::set_global_read_context(&mut *conn, false)
        .await
        .expect("drop global-read context");
    db::tenant_context::set_tenant_context(&mut *conn, org_a)
        .await
        .expect("set tenant context for org A");

    let execution = repo
        .record_execution(&mut *conn, schedule_a)
        .await
        .expect("record_execution under tenant context");
    assert_eq!(execution.schedule_id, schedule_a);
    assert_eq!(execution.status, "pending");

    // advance_after_run(id, None) parks the schedule (NULL next_run_at).
    let advanced = repo
        .advance_after_run(&mut *conn, schedule_a, None)
        .await
        .expect("advance_after_run under tenant context");
    assert_eq!(advanced.id, schedule_a);
    assert!(
        advanced.next_run_at.is_none(),
        "None next_run_at must park the schedule"
    );
    assert!(advanced.last_run_at.is_some(), "advance stamps last_run_at");

    // The parked schedule stops appearing as due; org B's untouched schedule
    // is still due.
    db::tenant_context::set_global_read_context(&mut *conn, true)
        .await
        .expect("re-enter global-read context");
    let due_after = repo
        .get_due_schedules(&mut *conn)
        .await
        .expect("due query after advance");
    let due_after_ids: Vec<Uuid> = due_after.iter().map(|s| s.id).collect();
    assert!(
        !due_after_ids.contains(&schedule_a),
        "parked schedule must not re-fire"
    );
    assert!(
        due_after_ids.contains(&schedule_b),
        "unrelated due schedule must remain due"
    );
}

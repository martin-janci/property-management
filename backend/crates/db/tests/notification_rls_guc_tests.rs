//! Migration-effect (catalog metadata) regression test for the Epic 8A
//! notification RLS GUC-mismatch (issue #755, findings #4 & #5).
//!
//! 00021 (`notification_preferences`) and 00022 (`critical_notifications` /
//! `critical_notification_acknowledgments`) wrote their RLS policies against the
//! legacy `request.user_id` / `request.org_id` GUCs and only `ENABLE`d RLS.
//! Nothing sets `request.*`; the project convention (00159, 00171) is
//! `app.current_user_id` / `app.current_org_id` set by the `RlsConnection`
//! extractor. The result:
//!
//!   * `notification_preferences` is reached via `RlsConnection`, so the
//!     unset-`request.user_id` policy collapsed to the all-zeros UUID and
//!     matched nothing — the RLS path was silently broken.
//!   * the critical-notification tables are reached via a raw service-role pool
//!     and were `ENABLE`-only, so owner-bypass meant the broken policies were
//!     never exercised — non-functional defense-in-depth.
//!
//! Migration 00177 rewrites the policies against the `app.*` GUCs (plus
//! `is_super_admin()`) and adds `FORCE ROW LEVEL SECURITY`.
//!
//! ## Why this is a metadata test, not a behavioral one
//!
//! `#[sqlx::test]` connects as the `DATABASE_URL` superuser, which PostgreSQL
//! exempts from RLS entirely — a behavioral "must not see" assertion passes
//! vacuously here. Behavioral cross-tenant isolation is covered by CI's
//! dedicated non-superuser `rls-smoke-test` / `security-tests` jobs. This suite
//! instead asserts on the Postgres catalog what 00177 changes, mirroring the
//! 00171 messaging precedent (`messaging_rls_cross_tenant_tests.rs`):
//!   1. FORCE RLS (`relforcerowsecurity`) is set on each Epic 8A table.
//!   2. Each table still carries at least one policy (FORCE is real enforcement,
//!      not implicit deny-all).
//!   3. No surviving policy references the legacy `request.` GUC.
//!
//! Assertions (1) and (3) FAIL on `origin/dev` (pre-00177:
//! `relforcerowsecurity = false`, policies still on `request.*`) — the IG3
//! regression intent — and PASS after 00177 is applied.

use sqlx::PgPool;

/// The three Epic 8A notification tables that 00177 brings under FORCE RLS with
/// corrected `app.*` GUC policies.
const NOTIFICATION_TABLES: [&str; 3] = [
    "notification_preferences",
    "critical_notifications",
    "critical_notification_acknowledgments",
];

/// 00177 must set `relforcerowsecurity = true` (FORCE) on every Epic 8A
/// notification table, with `relrowsecurity = true` (ENABLE, from 00021/00022)
/// still in place. Pre-migration these are `(true, false)`, so this fails on
/// origin/dev.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn notification_tables_have_force_rls_enabled(pool: PgPool) {
    let rows: Vec<(String, bool, bool)> = sqlx::query_as(
        r#"
        SELECT relname, relrowsecurity, relforcerowsecurity
        FROM pg_class
        WHERE relname = ANY($1)
          AND relnamespace = 'public'::regnamespace
        ORDER BY relname
        "#,
    )
    .bind(&NOTIFICATION_TABLES[..])
    .fetch_all(&pool)
    .await
    .expect("query pg_class RLS flags");

    assert_eq!(
        rows.len(),
        NOTIFICATION_TABLES.len(),
        "expected all Epic 8A notification tables present in pg_class (public schema); got {rows:?}"
    );

    for (relname, relrowsecurity, relforcerowsecurity) in &rows {
        assert!(
            *relrowsecurity,
            "{relname}: ENABLE ROW LEVEL SECURITY (relrowsecurity) must be set (from 00021/00022)"
        );
        assert!(
            *relforcerowsecurity,
            "{relname}: FORCE ROW LEVEL SECURITY (relforcerowsecurity) must be set by 00177 — \
             without FORCE, the table owner bypasses the notification RLS policies (#755)"
        );
    }
}

/// FORCE RLS with no policy is an implicit deny-all; FORCE RLS with policies is
/// real enforcement. Assert each table still carries at least one RLS policy.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn notification_tables_have_rls_policies(pool: PgPool) {
    for table in NOTIFICATION_TABLES {
        let policy_count: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)
            FROM pg_policies
            WHERE schemaname = 'public'
              AND tablename = $1
            "#,
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .expect("query pg_policies");

        assert!(
            policy_count > 0,
            "{table}: expected at least one RLS policy so FORCE ROW LEVEL SECURITY enforces a \
             real policy rather than implicit deny-all; found {policy_count}"
        );
    }
}

/// No surviving policy on the Epic 8A tables may reference the legacy
/// `request.` GUC — every policy must use the `app.current_user_id` /
/// `app.current_org_id` convention. The `qual` (USING) and `with_check`
/// expressions are inspected as text. Fails on origin/dev (policies still on
/// `request.user_id` / `request.org_id`).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn notification_policies_use_app_guc_not_request(pool: PgPool) {
    let offenders: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT tablename, policyname
        FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = ANY($1)
          AND (
                COALESCE(qual, '')       LIKE '%request.%'
             OR COALESCE(with_check, '') LIKE '%request.%'
          )
        ORDER BY tablename, policyname
        "#,
    )
    .bind(&NOTIFICATION_TABLES[..])
    .fetch_all(&pool)
    .await
    .expect("query pg_policies for legacy request.* GUC");

    assert!(
        offenders.is_empty(),
        "Epic 8A notification policies must use the app.* GUC convention (00177); \
         these still reference the never-set legacy request.* GUC: {offenders:?}"
    );
}

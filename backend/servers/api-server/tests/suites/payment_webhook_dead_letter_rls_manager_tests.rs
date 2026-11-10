//! RLS regression test for the `payment_webhook_dead_letters` "manage" policy
//! (#3341, follow-up to PR #3300 / #3284).
//!
//! # The bug
//!
//! The create migration (00240) shipped a `payment_webhook_dead_letters_manage`
//! policy whose comment promised only "Admins/managers of the org (and super
//! admins)" could resolve dead letters, but whose USING / WITH CHECK predicate
//! checked only `organization_members` *membership* — identical to the read
//! policy. So ANY active org member (a plain `member`/`resident`) could resolve
//! a dead-letter row. Resolving a dead letter is a privileged financial
//! reconciliation action and must be manager-gated, exactly like every
//! comparable `*_manage` policy in the financial schema (`invoices_manage`,
//! `payments_manage`, …, all carry `AND om.role_type IN ('org_admin',
//! 'manager')`). Migration 00244 adds that role check to the manage policy.
//!
//! # Why this test switches roles
//!
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — a behavioral "must be denied" assertion would pass vacuously. To
//! actually exercise the policy the way a real (non-owner) caller does, the
//! test creates a plain `NOSUPERUSER NOBYPASSRLS` role and `SET ROLE`s to it.
//! The table is `ENABLE` RLS, so the policy binds to any non-owner role
//! (`FORCE` is only needed to also bind the table owner). The per-request org /
//! user context is carried in the `app.current_user_id` session GUC via
//! `set_request_context`, which survives `SET ROLE`.
//!
//! # What it proves (IG3)
//!
//! A correctly-seeded org with a `manager` and a plain `member`:
//!   * the **member** attempting to resolve a dead letter affects **0 rows**
//!     (denied by the manage policy's role check), and
//!   * the **manager** attempting the same resolve affects **1 row** (allowed).
//!
//! On `origin/dev` (pre-00244, membership-only manage policy) the member's
//! UPDATE affects 1 row, so the `assert_eq!(member_rows, 0, …)` FAILS — the
//! regression intent. After 00244 it PASSES.

#![allow(dead_code)]

use crate::common::{seed_membership, seed_org};
use sqlx::PgPool;
use uuid::Uuid;

/// Insert a minimal active, verified public user and return its id.
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'DL RLS User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert an unresolved dead-letter row for `org_id` and return its id.
async fn seed_dead_letter(pool: &PgPool, org_id: Uuid, provider_session_id: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO payment_webhook_dead_letters
            (organization_id, provider, provider_session_id, failure_reason, raw_event)
        VALUES ($1, 'stripe', $2, 'invoice_missing', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(provider_session_id)
    .bind(b"{}".as_slice())
    .fetch_one(pool)
    .await
    .expect("seed dead letter")
}

/// `UPDATE … SET resolved_at = NOW()` the dead letter under the currently
/// SET-ROLE'd (RLS-bound) connection, returning the number of rows the policy
/// let through. A denied caller sees 0 rows; an allowed caller sees 1.
async fn try_resolve(conn: &mut sqlx::PgConnection, dead_letter_id: Uuid) -> u64 {
    sqlx::query(
        r#"
        UPDATE payment_webhook_dead_letters
        SET resolved_at = NOW(), resolution = 'reconciled by test'
        WHERE id = $1
        "#,
    )
    .bind(dead_letter_id)
    .execute(conn)
    .await
    .expect("run resolve update")
    .rows_affected()
}

/// The manage policy must deny a plain org member and allow a manager.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manage_policy_denies_non_manager_allows_manager(pool: PgPool) {
    // --- Seed as superuser. ---
    let org = seed_org(&pool, "dlrls").await;
    let manager = seed_user(&pool, "mgr@dlrls.test").await;
    let member = seed_user(&pool, "member@dlrls.test").await;
    seed_membership(&pool, org, manager, "manager").await;
    seed_membership(&pool, org, member, "member").await;
    let dead_letter = seed_dead_letter(&pool, org, "cs_dlrls_1").await;

    // --- A NOSUPERUSER NOBYPASSRLS role so the ENABLE policy actually binds. ---
    let role = format!("ppt_rls_test_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create role");
    // The manage policy reads payment_webhook_dead_letters (SELECT for the WHERE
    // scan, UPDATE for the write) and its EXISTS subquery reads
    // organization_members; both policies call is_super_admin().
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT, UPDATE ON payment_webhook_dead_letters TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant on dead letters");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON organization_members TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select on organization_members");
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION is_super_admin() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute on is_super_admin");

    {
        // One dedicated connection: SET ROLE and the context GUC are session
        // state and must not be spread across pool acquisitions.
        let mut conn = pool.acquire().await.expect("acquire connection");

        // --- Non-manager member: resolve must be DENIED. ---
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org)
            .bind(member)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set member context");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role (member)");

        let member_rows = try_resolve(&mut conn, dead_letter).await;
        assert_eq!(
            member_rows, 0,
            "a plain org member must NOT be able to resolve a dead letter \
             (manage policy is manager-only — #3341)"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role after member");

        // The row is still unresolved after the denied attempt.
        let resolved_after_member: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT resolved_at FROM payment_webhook_dead_letters WHERE id = $1",
        )
        .bind(dead_letter)
        .fetch_one(&mut *conn)
        .await
        .expect("read resolved_at after member attempt");
        assert!(
            resolved_after_member.is_none(),
            "the dead letter must remain unresolved after a non-manager's denied attempt"
        );

        // --- Manager: resolve must be ALLOWED. ---
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org)
            .bind(manager)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set manager context");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role (manager)");

        let manager_rows = try_resolve(&mut conn, dead_letter).await;
        assert_eq!(
            manager_rows, 1,
            "a manager must be able to resolve a dead letter"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role after manager");
    }

    // --- Cleanup the cluster-global role. ---
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "REVOKE ALL ON payment_webhook_dead_letters FROM \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "REVOKE ALL ON organization_members FROM \"{role}\""
    )))
    .execute(&pool)
    .await
    .ok();
    // DROP OWNED severs every remaining privilege (incl. the function EXECUTE
    // grant that plain REVOKEs miss) so DROP ROLE does not fail with "objects
    // depend on it" and leak the cluster-global role.
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP OWNED BY \"{role}\"")))
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

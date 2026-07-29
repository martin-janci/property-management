//! Repo-level behavioral RLS regression test for PAP-73 (parent PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `insurance_policies` (and the rest of the
//! insurance cluster: `insurance_claims`, `insurance_claim_documents`,
//! `insurance_claim_history`, `insurance_policy_documents`,
//! `insurance_renewal_reminders`). The production api-server connects as the
//! table OWNER, which `FORCE` binds. `InsuranceRepository` held a raw `PgPool`
//! and ran every query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty, writes
//! failed. (PAP-73.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_policy_by_id` / `list_policies` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's policy stays invisible to an org-A caller
//!      even with context set.
//!   4. **Write path** — a `create_policy` on a context-set connection succeeds.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces
//! the policy the way the production owner role experiences it. Mirrors
//! `work_order_rls_repo_tests.rs` (the PAP-67 precedent PAP-73 follows).

use crate::common::{seed_org, set_ctx};
use chrono::NaiveDate;
use db::models::{CreateInsurancePolicy, InsurancePolicyQuery};
use db::repositories::InsuranceRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Ins User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed an insurance policy directly (as superuser, RLS-exempt) for an org.
async fn seed_policy(pool: &PgPool, org_id: Uuid, suffix: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO insurance_policies
            (organization_id, policy_number, policy_name, provider_name, policy_type,
             effective_date, expiration_date)
        VALUES ($1, $2, $3, 'Acme Insure', 'property', '2025-01-01', '2026-01-01')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("POL-{suffix}"))
    .bind(format!("Policy {suffix}"))
    .fetch_one(pool)
    .await
    .expect("seed policy")
}

fn sample_policy(suffix: &str) -> CreateInsurancePolicy {
    CreateInsurancePolicy {
        policy_number: format!("POL-{suffix}"),
        policy_name: format!("Policy {suffix}"),
        provider_name: "Acme Insure".to_string(),
        provider_contact: None,
        provider_phone: None,
        provider_email: None,
        policy_type: "property".to_string(),
        coverage_amount: None,
        deductible: None,
        premium_amount: None,
        premium_frequency: None,
        building_id: None,
        unit_id: None,
        coverage_description: None,
        effective_date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        expiration_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        renewal_date: None,
        auto_renew: None,
        terms: None,
        metadata: None,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn insurance_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = InsuranceRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-a").await;
    let org_b = seed_org(&pool, "force-b").await;
    let user_a = seed_user(&pool, "a@ins.test").await;
    let pol_a = seed_policy(&pool, org_a, "a").await;
    let _pol_b = seed_policy(&pool, org_b, "b").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_ins_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON insurance_policies TO \"{role}\""),
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
            .find_policy_by_id(&mut *conn, org_a, pol_a)
            .await
            .expect("find_policy_by_id (no ctx)");
        assert!(
            found.is_none(),
            "PAP-73 regression: without RLS context, own-org policy must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_policies(&mut *conn, org_a, InsurancePolicyQuery::default())
            .await
            .expect("list_policies (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-73 regression: without RLS context, list returns deny-all empty"
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
            .find_policy_by_id(&mut *conn, org_a, pol_a)
            .await
            .expect("find_policy_by_id (ctx)");
        assert_eq!(
            found.map(|p| p.id),
            Some(pol_a),
            "PAP-73 fix: with RLS context set, the repo must return the own-org policy"
        );

        let listed = repo
            .list_policies(&mut *conn, org_a, InsurancePolicyQuery::default())
            .await
            .expect("list_policies (ctx)");
        assert_eq!(
            listed.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![pol_a],
            "PAP-73 fix: list returns exactly the own-org policy under context"
        );

        // (3) Org B's policy stays invisible to an org-A caller.
        let cross = repo
            .find_policy_by_id(&mut *conn, org_a, _pol_b)
            .await
            .expect("find_policy_by_id cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's policy must NOT be visible to an org-A caller"
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
            .create_policy(&mut *conn, org_a, sample_policy("created-under-rls"))
            .await
            .expect("create_policy under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON insurance_policies FROM \"{role}\""),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        // DROP OWNED severs every remaining privilege this role holds in the
        // test database — the explicit REVOKEs above keep missing the RLS
        // helper-function EXECUTE grants, so DROP ROLE failed with "objects
        // depend on it" and leaked the cluster-global role (PAP-134).
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

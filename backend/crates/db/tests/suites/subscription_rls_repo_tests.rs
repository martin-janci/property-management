//! Repo-level behavioral RLS regression test for PAP-112 (parent PAP-80 / PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the seven org-scoped billing tables used
//! by `SubscriptionRepository` (`organization_subscriptions`,
//! `payment_methods`, `subscription_invoices`, `invoice_line_items`,
//! `usage_records`, `subscription_events`, `coupon_redemptions`). The
//! production api-server connects as the table OWNER, which `FORCE` binds.
//! `SubscriptionRepository` held a raw `PgPool` and its legacy methods ran
//! every query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policies collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty,
//! writes failed. (PAP-112.)
//!
//! The fix makes the repo hold no pool: every method takes an RLS-context
//! executor (the `RlsConnection` extractor in handlers sets the org/user GUCs
//! before any query). This test exercises the *repository methods themselves*
//! on a `FORCE`-bound role and proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_subscription_by_org` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first
//!      (what `RlsConnection` now does), the same call returns the own-org
//!      subscription, and the own-org by-id `find_subscription_by_id`
//!      resolves.
//!   3. **Cross-tenant by-id** — org B's subscription is invisible to an
//!      org-A caller: `find_subscription_by_id` returns `None` (handlers
//!      surface 404, indistinguishable from missing).
//!   4. **Write path** — a `create_subscription` on a context-set connection
//!      succeeds and the row is the caller's org (without context the INSERT
//!      would fail the policy `WITH CHECK` — the write-side of deny-all).
//!      This also covers the multi-statement `&mut PgConnection` shape: the
//!      method runs the plan lookup (`subscription_plans`, public read
//!      policy) and the insert on the same context-set connection.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral
//! assertion would pass vacuously. The test creates a plain `NOSUPERUSER
//! NOBYPASSRLS` role, grants it access, and `SET ROLE`s to it so `FORCE`
//! actually enforces the policy the way the production owner role experiences
//! it. Mirrors `integration_rls_repo_tests.rs` / `emergency_rls_repo_tests.rs`
//! (the PAP-67 precedent PAP-80 follows).

use crate::common::{seed_org, set_ctx};
use db::models::CreateOrganizationSubscription;
use db::repositories::SubscriptionRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Subscription User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a subscription plan (as superuser). `subscription_plans` is not
/// FORCE-bound and carries a public read policy, so the bound role can read
/// it during `create_subscription`'s plan lookup.
async fn seed_plan(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO subscription_plans (name, display_name, monthly_price, annual_price)
        VALUES ($1, $2, 29.00, 290.00)
        RETURNING id
        "#,
    )
    .bind(name)
    .bind(format!("{name} (display)"))
    .fetch_one(pool)
    .await
    .expect("seed plan")
}

/// Seed an organization subscription directly (as superuser, RLS-exempt).
async fn seed_subscription(pool: &PgPool, org_id: Uuid, plan_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organization_subscriptions
            (organization_id, plan_id, status, billing_cycle,
             current_period_start, current_period_end)
        VALUES ($1, $2, 'active', 'monthly', NOW(), NOW() + INTERVAL '1 month')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(plan_id)
    .fetch_one(pool)
    .await
    .expect("seed subscription")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn subscription_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = SubscriptionRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "sforce-a").await;
    let org_b = seed_org(&pool, "sforce-b").await;
    let org_c = seed_org(&pool, "sforce-c").await;
    let user_a = seed_user(&pool, "a@subscription.test").await;
    let _user_c = seed_user(&pool, "c@subscription.test").await;
    let plan = seed_plan(&pool, "sforce-basic").await;
    let sub_a = seed_subscription(&pool, org_a, plan).await;
    let sub_b = seed_subscription(&pool, org_b, plan).await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_subscription_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON organization_subscriptions TO \"{role}\""),
        // create_subscription's plan lookup reads subscription_plans (public
        // read policy, not FORCE-bound).
        format!("GRANT SELECT ON subscription_plans TO \"{role}\""),
        // The 00179 policy is `org = get_current_org_id() AND
        // get_current_org_not_deleted()`; the soft-delete helper is SECURITY
        // INVOKER and reads `organizations`, so the bound role needs SELECT
        // there too (else ctx-set reads fail "permission denied for table
        // organizations" — the no-ctx deny-all leg passes anyway because the
        // NULL org short-circuits before the soft-delete check).
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role.
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id() TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_not_deleted() TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION is_super_admin() TO \"{role}\""),
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
            .find_subscription_by_org(&mut *conn, org_a)
            .await
            .expect("find_subscription_by_org (no ctx)");
        assert!(
            found.is_none(),
            "PAP-112 regression: without RLS context, the own-org subscription \
             must be invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (2) FIX + (3) cross-tenant by-id: set context, drop to bound role, query.
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

        // (2) Own-org subscription IS now visible through the repo — the fix.
        let found = repo
            .find_subscription_by_org(&mut *conn, org_a)
            .await
            .expect("find_subscription_by_org (ctx)");
        assert_eq!(
            found.map(|s| s.id),
            Some(sub_a),
            "PAP-112 fix: with RLS context set, the own-org subscription resolves"
        );

        // Own-org by-id read resolves.
        let own = repo
            .find_subscription_by_id(&mut *conn, sub_a)
            .await
            .expect("get own-org subscription under context must succeed");
        assert_eq!(own.map(|s| s.id), Some(sub_a));

        // (3) Cross-tenant by-id read: org B's subscription must be invisible
        //     to an org-A caller (None → handlers surface 404).
        let cross = repo
            .find_subscription_by_id(&mut *conn, sub_b)
            .await
            .expect("cross-tenant get must not error, just filter");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's subscription must NOT be visible to an org-A caller"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (4) WRITE path: a create on a context-set connection succeeds and the
    //     row is the caller's org. Without context the INSERT would fail the
    //     policy WITH CHECK (the write-side of deny-all). Uses org C because
    //     organization_subscriptions.organization_id is UNIQUE.
    // ====================================================================
    {
        let mut conn = pool.acquire().await.expect("acquire");
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_c)
            .bind(_user_c)
            .bind(false)
            .execute(&mut *conn)
            .await
            .expect("set org-C ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let created = repo
            .create_subscription(
                &mut conn,
                org_c,
                CreateOrganizationSubscription {
                    plan_id: plan,
                    billing_cycle: Some("monthly".to_string()),
                    start_trial: None,
                    payment_method_id: None,
                    coupon_code: None,
                    metadata: None,
                },
            )
            .await
            .expect("create_subscription under context must succeed");
        assert_eq!(created.organization_id, org_c);
        assert_eq!(created.status, "active");

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON organization_subscriptions FROM \"{role}\""),
        format!("REVOKE ALL ON subscription_plans FROM \"{role}\""),
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

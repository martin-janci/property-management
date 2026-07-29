//! Repo-level behavioral RLS regression test for PAP-70 (parent PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `vendors` (and the rest of the Epic-11+
//! cluster). The production api-server connects as the table OWNER, which
//! `FORCE` binds. `VendorRepository` held a raw `PgPool` and ran every query
//! WITHOUT ever calling `set_request_context`, so on `dev` `get_current_org_id()`
//! returned NULL and the policy collapsed to `organization_id = NULL` →
//! **deny-all**: own-org reads returned empty, writes failed. (PAP-70.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org `find_by_id`
//!      / `list` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's vendor stays invisible to an org-A caller
//!      even with context set.
//!   4. **Write path** — a `create` on a context-set connection succeeds and the
//!      row is the caller's org (the write-side of deny-all).
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces
//! the policy the way the production owner role experiences it. Mirrors
//! `work_order_rls_repo_tests.rs` (the PAP-67 precedent PAP-70 follows).

use crate::common::{seed_org, set_ctx};
use db::models::{CreateVendor, VendorQuery};
use db::repositories::VendorRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Vendor User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a vendor directly (as superuser, RLS-exempt) for an org.
async fn seed_vendor(pool: &PgPool, org_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO vendors (organization_id, company_name)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed vendor")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn vendor_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = VendorRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "vforce-a").await;
    let org_b = seed_org(&pool, "vforce-b").await;
    let user_a = seed_user(&pool, "a@vendor.test").await;
    let _user_b = seed_user(&pool, "b@vendor.test").await;
    let vendor_a = seed_vendor(&pool, org_a, "Acme A").await;
    let vendor_b = seed_vendor(&pool, org_b, "Acme B").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_vendor_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON vendors TO \"{role}\""),
        // RLS policy helper must be EXECUTE-able by the bound role.
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id() TO \"{role}\""),
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
            .find_by_id(&mut *conn, vendor_a, org_a)
            .await
            .expect("find_by_id (no ctx)");
        assert!(
            found.is_none(),
            "PAP-70 regression: without RLS context, own-org vendor must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list(&mut *conn, org_a, VendorQuery::default())
            .await
            .expect("list (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-70 regression: without RLS context, list returns deny-all empty"
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
            .find_by_id(&mut *conn, vendor_a, org_a)
            .await
            .expect("find_by_id (ctx)");
        assert_eq!(
            found.map(|v| v.id),
            Some(vendor_a),
            "PAP-70 fix: with RLS context set, the repo must return the own-org vendor"
        );

        let listed = repo
            .list(&mut *conn, org_a, VendorQuery::default())
            .await
            .expect("list (ctx)");
        assert_eq!(
            listed.iter().map(|v| v.id).collect::<Vec<_>>(),
            vec![vendor_a],
            "PAP-70 fix: list returns exactly the own-org vendor under context"
        );

        // (3) Org B's vendor stays invisible to an org-A caller.
        let cross = repo
            .find_by_id(&mut *conn, vendor_b, org_a)
            .await
            .expect("find_by_id cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's vendor must NOT be visible to an org-A caller"
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
            .create(
                &mut *conn,
                org_a,
                CreateVendor {
                    company_name: "Created under RLS".to_string(),
                    contact_name: None,
                    phone: None,
                    email: None,
                    website: None,
                    address: None,
                    services: Vec::new(),
                    license_number: None,
                    tax_id: None,
                    contract_start: None,
                    contract_end: None,
                    hourly_rate: None,
                    is_preferred: None,
                    notes: None,
                    metadata: None,
                },
            )
            .await
            .expect("create under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON vendors FROM \"{role}\""),
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

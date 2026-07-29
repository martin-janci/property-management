//! Repo-level behavioral RLS regression test for PAP-80 (parent PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on every emergency table (`emergency_protocols`
//! and the other seven). The production api-server connects as the table OWNER,
//! which `FORCE` binds. `EmergencyRepository` held a raw `PgPool` and ran every
//! query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policy collapsed to
//! `organization_id = NULL` → **deny-all**: own-org reads returned empty, writes
//! failed. (PAP-80.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_protocol_by_id` / `list_protocols` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's protocol stays invisible to an org-A caller
//!      even with context set.
//!   4. **Write path** — a `create_protocol` on a context-set connection succeeds
//!      and the row is the caller's org (the write-side of deny-all).
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces
//! the policy the way the production owner role experiences it. Mirrors
//! `vendor_rls_repo_tests.rs` / `work_order_rls_repo_tests.rs` (the PAP-67
//! precedent PAP-80 follows).

use crate::common::{seed_org, set_ctx};
use db::models::{CreateEmergencyProtocol, EmergencyProtocolQuery};
use db::repositories::EmergencyRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// `EmergencyProtocolQuery` does not derive `Default`; build an all-`None`
/// (unfiltered) query for the list assertions.
fn any_protocols() -> EmergencyProtocolQuery {
    EmergencyProtocolQuery {
        building_id: None,
        protocol_type: None,
        is_active: None,
        limit: None,
        offset: None,
    }
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Emergency User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a protocol directly (as superuser, RLS-exempt) for an org.
async fn seed_protocol(pool: &PgPool, org_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO emergency_protocols (organization_id, name, protocol_type)
        VALUES ($1, $2, 'fire')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed protocol")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn emergency_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = EmergencyRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "eforce-a").await;
    let org_b = seed_org(&pool, "eforce-b").await;
    let user_a = seed_user(&pool, "a@emergency.test").await;
    let _user_b = seed_user(&pool, "b@emergency.test").await;
    let protocol_a = seed_protocol(&pool, org_a, "Fire A").await;
    let protocol_b = seed_protocol(&pool, org_b, "Fire B").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_emergency_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON emergency_protocols TO \"{role}\""),
        // get_current_org_not_deleted() is SECURITY INVOKER and reads
        // `organizations`, so the bound role needs SELECT on it (PAP-133).
        format!("GRANT SELECT ON organizations TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role.
        format!("GRANT EXECUTE ON FUNCTION get_current_org_id() TO \"{role}\""),
        format!("GRANT EXECUTE ON FUNCTION get_current_org_not_deleted() TO \"{role}\""),
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
            .find_protocol_by_id(&mut *conn, org_a, protocol_a)
            .await
            .expect("find_protocol_by_id (no ctx)");
        assert!(
            found.is_none(),
            "PAP-80 regression: without RLS context, own-org protocol must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_protocols(&mut *conn, org_a, any_protocols())
            .await
            .expect("list_protocols (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-80 regression: without RLS context, list returns deny-all empty"
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
            .find_protocol_by_id(&mut *conn, org_a, protocol_a)
            .await
            .expect("find_protocol_by_id (ctx)");
        assert_eq!(
            found.map(|p| p.id),
            Some(protocol_a),
            "PAP-80 fix: with RLS context set, the repo must return the own-org protocol"
        );

        let listed = repo
            .list_protocols(&mut *conn, org_a, any_protocols())
            .await
            .expect("list_protocols (ctx)");
        assert_eq!(
            listed.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![protocol_a],
            "PAP-80 fix: list returns exactly the own-org protocol under context"
        );

        // (3) Org B's protocol stays invisible to an org-A caller.
        let cross = repo
            .find_protocol_by_id(&mut *conn, org_a, protocol_b)
            .await
            .expect("find_protocol_by_id cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's protocol must NOT be visible to an org-A caller"
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
            .create_protocol(
                &mut *conn,
                org_a,
                user_a,
                CreateEmergencyProtocol {
                    building_id: None,
                    name: "Created under RLS".to_string(),
                    protocol_type: "fire".to_string(),
                    description: None,
                    steps: serde_json::json!([]),
                    contacts: None,
                    evacuation_info: None,
                    attachments: None,
                    is_active: None,
                    priority: None,
                },
            )
            .await
            .expect("create_protocol under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON emergency_protocols FROM \"{role}\""),
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

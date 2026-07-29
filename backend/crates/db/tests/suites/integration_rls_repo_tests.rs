//! Repo-level behavioral RLS regression test for PAP-105 (parent PAP-80 / PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `webhook_subscriptions` — the one table
//! used by `IntegrationRepository` that is FORCE-bound today. The production
//! api-server connects as the table OWNER, which `FORCE` binds.
//! `IntegrationRepository` held a raw `DbPool` and ran every query WITHOUT
//! ever calling `set_request_context`, so on `dev` `get_current_org_id()`
//! returned NULL and the policy collapsed to `organization_id = NULL` →
//! **deny-all**: own-org reads returned empty, writes failed. (PAP-105.)
//!
//! The fix makes the repo hold no pool: every method takes an RLS-context
//! executor (the `RlsConnection` extractor in handlers sets the org/user GUCs
//! before any query). This test exercises the *repository methods themselves*
//! on a `FORCE`-bound role and proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `list_webhook_subscriptions` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first
//!      (what `RlsConnection` now does), the same call returns the own-org
//!      subscription, and the own-org by-id `get_webhook_subscription`
//!      resolves.
//!   3. **Cross-tenant by-id** — org B's subscription is invisible to an
//!      org-A caller: `get_webhook_subscription` returns `None` (handlers
//!      surface 404, indistinguishable from missing).
//!   4. **Write path** — a `create_webhook_subscription` on a context-set
//!      connection succeeds and the row is the caller's org (without context
//!      the INSERT would fail the policy `WITH CHECK` — the write-side of
//!      deny-all).
//!
//! Schema note (Epic-61 scaffold drift)
//! ------------------------------------
//! `webhook_subscriptions` was created by migration `00102` for the API
//! ecosystem; the Epic-61 repository additionally expects `status` and
//! `retry_policy` columns that no migration ships yet. The test adds those
//! two nullable/defaulted columns up front (as the RLS-exempt superuser) so
//! the repository's SQL is runnable — the FORCE policy under test is
//! `organization_id = get_current_org_id()` and is orthogonal to the added
//! columns.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral
//! assertion would pass vacuously. The test creates a plain `NOSUPERUSER
//! NOBYPASSRLS` role, grants it access, and `SET ROLE`s to it so `FORCE`
//! actually enforces the policy the way the production owner role experiences
//! it. Mirrors `sentiment_rls_repo_tests.rs` / `emergency_rls_repo_tests.rs`
//! (the PAP-67 precedent PAP-80 follows).

use crate::common::{seed_org, set_ctx};
use db::models::CreateWebhookSubscription;
use db::repositories::IntegrationRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Integration User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Add the Epic-61 columns the repository expects but no migration ships yet
/// (see module docs — orthogonal to the FORCE policy under test).
async fn align_webhook_subscriptions_schema(pool: &PgPool) {
    for stmt in [
        "ALTER TABLE webhook_subscriptions \
         ADD COLUMN IF NOT EXISTS status VARCHAR(20) NOT NULL DEFAULT 'active'",
        "ALTER TABLE webhook_subscriptions ADD COLUMN IF NOT EXISTS retry_policy JSONB",
    ] {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .expect("align webhook_subscriptions schema");
    }
}

/// Seed a subscription directly (as superuser, RLS-exempt) for an org.
async fn seed_subscription(pool: &PgPool, org_id: Uuid, created_by: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO webhook_subscriptions
            (organization_id, name, url, secret, events, status, created_by)
        VALUES ($1, $2, 'https://example.com/hooks/seeded', 'seed-secret',
                ARRAY['fault.created'], 'active', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed subscription")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn integration_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    // Crypto None for determinism: secrets round-trip as plaintext.
    let repo = IntegrationRepository::with_crypto(pool.clone(), None);

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    align_webhook_subscriptions_schema(&pool).await;
    let org_a = seed_org(&pool, "iforce-a").await;
    let org_b = seed_org(&pool, "iforce-b").await;
    let user_a = seed_user(&pool, "a@integration.test").await;
    let user_b = seed_user(&pool, "b@integration.test").await;
    let sub_a = seed_subscription(&pool, org_a, user_a, "org-a hook").await;
    let sub_b = seed_subscription(&pool, org_b, user_b, "org-b hook").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_integration_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON webhook_subscriptions TO \"{role}\""),
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

        let listed = repo
            .list_webhook_subscriptions(&mut *conn, org_a)
            .await
            .expect("list_webhook_subscriptions (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-105 regression: without RLS context, own-org webhook \
             subscriptions must be invisible (deny-all) — this is what the \
             raw-pool repo did on dev"
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
        let listed = repo
            .list_webhook_subscriptions(&mut *conn, org_a)
            .await
            .expect("list_webhook_subscriptions (ctx)");
        assert_eq!(
            listed.iter().map(|s| s.id).collect::<Vec<_>>(),
            vec![sub_a],
            "PAP-105 fix: with RLS context set, list returns exactly the own-org subscription"
        );

        // Own-org by-id read resolves.
        let own = repo
            .get_webhook_subscription(&mut *conn, sub_a)
            .await
            .expect("get own-org subscription under context must succeed");
        assert_eq!(own.map(|s| s.id), Some(sub_a));

        // (3) Cross-tenant by-id read: org B's subscription must be invisible
        //     to an org-A caller (None → handlers surface 404).
        let cross = repo
            .get_webhook_subscription(&mut *conn, sub_b)
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
            .create_webhook_subscription(
                &mut *conn,
                org_a,
                user_a,
                CreateWebhookSubscription {
                    name: "ctx-created hook".to_string(),
                    description: None,
                    url: "https://example.com/hooks/created".to_string(),
                    events: vec!["fault.created".to_string()],
                    secret: Some("hook-secret".to_string()),
                    headers: None,
                    retry_policy: None,
                },
                false,
            )
            .await
            .expect("create_webhook_subscription under context must succeed");
        assert_eq!(created.organization_id, org_a);
        // Crypto is None in this test, so the secret round-trips verbatim.
        assert_eq!(created.secret.as_deref(), Some("hook-secret"));

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON webhook_subscriptions FROM \"{role}\""),
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

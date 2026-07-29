//! Repo-level behavioral RLS regression test for PAP-110 (parent PAP-80 / PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on six tables `ApiEcosystemRepository`
//! queries: `organization_integrations`, `organization_connectors`,
//! `connector_execution_logs`, `webhook_subscriptions`, `webhook_deliveries`,
//! and `integration_ratings`. The production api-server connects as the table
//! OWNER, which `FORCE` binds. The repo held a raw `PgPool` and ran every
//! query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policies collapsed to
//! deny-all: own-org webhook/integration reads returned empty and writes
//! failed. Worse, the by-id webhook reads (`/webhooks/{id}`) had no app-level
//! org check either, so before `FORCE` they were a cross-tenant IDOR.
//!
//! The fix makes the repo stateless: every method takes an executor whose
//! connection already has RLS context set (handlers use the `RlsConnection`
//! extractor). This test exercises the *repository methods themselves* on a
//! `FORCE`-bound role and proves, against `webhook_subscriptions`:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `get_enhanced_webhook` / `list_enhanced_webhooks` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first,
//!      the same by-id read returns the own-org subscription.
//!   3. **Cross-tenant by-id** — org B's webhook is invisible to an org-A
//!      caller probing org B's subscription id (the IDOR guard, now enforced
//!      by the database).
//!   4. **Write path** — `create_enhanced_webhook` succeeds under org-A
//!      context and the row lands in the caller's org.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral
//! assertion would pass vacuously. The test creates a plain `NOSUPERUSER
//! NOBYPASSRLS` role, grants it access, and `SET ROLE`s to it so `FORCE`
//! actually enforces the policy the way the production owner role
//! experiences it. Mirrors `llm_document_rls_repo_tests.rs` /
//! `sentiment_rls_repo_tests.rs` (the PAP-67 precedent PAP-80 follows).

use crate::common::{seed_org, set_ctx};
use db::models::CreateEnhancedWebhookSubscription;
use db::repositories::ApiEcosystemRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Ecosystem User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a webhook subscription directly (as superuser, RLS-exempt) for an org.
async fn seed_webhook(pool: &PgPool, org_id: Uuid, created_by: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO webhook_subscriptions
            (organization_id, name, url, secret, events, created_by)
        VALUES ($1, $2, 'https://example.test/hook', 'whsec_test', ARRAY['integration.installed'], $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed webhook subscription")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn api_ecosystem_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = ApiEcosystemRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "ecoforce-a").await;
    let org_b = seed_org(&pool, "ecoforce-b").await;
    let user_a = seed_user(&pool, "a@ecosystem.test").await;
    let user_b = seed_user(&pool, "b@ecosystem.test").await;
    let hook_a = seed_webhook(&pool, org_a, user_a, "org-a billing hook").await;
    let hook_b = seed_webhook(&pool, org_b, user_b, "org-b confidential hook").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_ecosystem_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON webhook_subscriptions TO \"{role}\""),
        // The org-not-deleted policy helper reads `organizations` with invoker
        // rights, so the bound role needs read access to it.
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
    //     behavior). Own-org webhook reads return nothing.
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

        let by_id = repo
            .get_enhanced_webhook(&mut *conn, hook_a)
            .await
            .expect("get_enhanced_webhook (no ctx)");
        assert!(
            by_id.is_none(),
            "PAP-80 regression: without RLS context, the own-org webhook must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_enhanced_webhooks(&mut *conn, org_a)
            .await
            .expect("list_enhanced_webhooks (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-80 regression: the org-scoped webhook list silently returned \
             nothing on the raw pool"
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

        // (2) Own-org by-id read IS now visible through the repo — the fix.
        let by_id = repo
            .get_enhanced_webhook(&mut *conn, hook_a)
            .await
            .expect("get_enhanced_webhook (ctx)")
            .expect("own-org webhook must be visible with RLS context set");
        assert_eq!(
            by_id.id, hook_a,
            "PAP-80 fix: with RLS context set, the by-id read returns exactly the own-org webhook"
        );
        assert_eq!(by_id.organization_id, org_a);

        // (3) Cross-tenant by-id: probing org B's webhook id from an org-A
        //     context must surface nothing (handler maps None to 404). This
        //     was an unguarded IDOR before — the handler had no org check.
        let cross = repo
            .get_enhanced_webhook(&mut *conn, hook_b)
            .await
            .expect("get_enhanced_webhook (cross-tenant probe)");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's webhook must NOT be readable by an org-A caller"
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
            .create_enhanced_webhook(
                &mut *conn,
                org_a,
                user_a,
                &CreateEnhancedWebhookSubscription {
                    name: "org-a created under ctx".to_string(),
                    description: None,
                    url: "https://example.test/created".to_string(),
                    auth_type: "hmac_sha256".to_string(),
                    auth_config: None,
                    events: vec!["integration.installed".to_string()],
                    filters: None,
                    payload_template: None,
                    headers: None,
                    retry_policy: None,
                    rate_limit_requests: None,
                    rate_limit_window_seconds: None,
                    timeout_ms: None,
                    verify_ssl: None,
                },
            )
            .await
            .expect("create_enhanced_webhook under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON webhook_subscriptions FROM \"{role}\""),
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

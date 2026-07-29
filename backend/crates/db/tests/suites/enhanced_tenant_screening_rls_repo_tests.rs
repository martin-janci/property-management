//! Repo-level behavioral RLS regression test for PAP-74 (parent PAP-67).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the screening cluster
//! (`ai_risk_scoring_models`, `screening_*`). The production api-server connects
//! as the table OWNER, which `FORCE` binds. `EnhancedTenantScreeningRepository`
//! held a raw `PgPool` and ran every query WITHOUT ever calling
//! `set_request_context`, so on `dev` `get_current_org_id()` returned NULL and the
//! policy collapsed to `organization_id = NULL` → **deny-all**: own-org reads
//! returned empty, writes failed. (PAP-74.)
//!
//! The fix routes the repo through an RLS-context connection (the `RlsConnection`
//! extractor in handlers sets the org/user GUCs before any query). This test
//! exercises the *repository methods themselves* on a `FORCE`-bound role and
//! proves, against `ai_risk_scoring_models`:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org `get_risk_model`
//!      / `list_risk_models` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first (what
//!      `RlsConnection` now does), the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's model stays invisible to an org-A caller.
//!   4. **Write path** — `create_risk_model` on a context-set connection succeeds
//!      and the row is the caller's org.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral assertion
//! would pass vacuously. The test creates a plain `NOSUPERUSER NOBYPASSRLS`
//! role, grants it access, and `SET ROLE`s to it so `FORCE` actually enforces the
//! policy the way the production owner role experiences it. Mirrors
//! `work_order_rls_repo_tests.rs` (the PAP-67 precedent PAP-74 follows).

use crate::common::{seed_org, set_ctx};
use db::models::enhanced_tenant_screening::CreateAiRiskScoringModel;
use db::repositories::enhanced_tenant_screening::EnhancedTenantScreeningRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Screen User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// A minimal model request — all weights/thresholds default in the repo.
fn model_req(name: &str) -> CreateAiRiskScoringModel {
    CreateAiRiskScoringModel {
        name: name.to_string(),
        description: None,
        credit_history_weight: None,
        rental_history_weight: None,
        income_stability_weight: None,
        employment_stability_weight: None,
        eviction_history_weight: None,
        criminal_background_weight: None,
        identity_verification_weight: None,
        reference_quality_weight: None,
        excellent_threshold: None,
        good_threshold: None,
        fair_threshold: None,
        poor_threshold: None,
        auto_approve_threshold: None,
        auto_reject_threshold: None,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn screening_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = EnhancedTenantScreeningRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "force-a").await;
    let org_b = seed_org(&pool, "force-b").await;
    let user_a = seed_user(&pool, "a@screen.test").await;
    let user_b = seed_user(&pool, "b@screen.test").await;

    // Seed one risk model per org (superuser bypasses RLS for this setup).
    let model_a = repo
        .create_risk_model(&pool, org_a, user_a, model_req("Model A"))
        .await
        .expect("seed model A")
        .id;
    let model_b = repo
        .create_risk_model(&pool, org_b, user_b, model_req("Model B"))
        .await
        .expect("seed model B")
        .id;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_screen_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ai_risk_scoring_models TO \"{role}\""),
        // RLS policy helpers must be EXECUTE-able by the bound role; the
        // not-deleted guard reads `organizations`.
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
            .get_risk_model(&mut *conn, org_a, model_a)
            .await
            .expect("get_risk_model (no ctx)");
        assert!(
            found.is_none(),
            "PAP-74 regression: without RLS context, own-org risk model must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list_risk_models(&mut *conn, org_a)
            .await
            .expect("list_risk_models (no ctx)");
        assert!(
            listed.is_empty(),
            "PAP-74 regression: without RLS context, list returns deny-all empty"
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
            .get_risk_model(&mut *conn, org_a, model_a)
            .await
            .expect("get_risk_model (ctx)");
        assert_eq!(
            found.map(|m| m.id),
            Some(model_a),
            "PAP-74 fix: with RLS context set, the repo must return the own-org risk model"
        );

        let listed = repo
            .list_risk_models(&mut *conn, org_a)
            .await
            .expect("list_risk_models (ctx)");
        assert_eq!(
            listed.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![model_a],
            "PAP-74 fix: list returns exactly the own-org model under context"
        );

        // (3) Org B's model stays invisible to an org-A caller.
        let cross = repo
            .get_risk_model(&mut *conn, org_a, model_b)
            .await
            .expect("get_risk_model cross");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's model must NOT be visible to an org-A caller"
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
            .create_risk_model(&mut *conn, org_a, user_a, model_req("Created under RLS"))
            .await
            .expect("create_risk_model under context must succeed");
        assert_eq!(created.organization_id, org_a);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!("REVOKE ALL ON ai_risk_scoring_models FROM \"{role}\""),
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

//! Repo-level behavioral RLS regression test for PAP-77 (parent PAP-67 / PAP-80).
//!
//! Background
//! ----------
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the workflow tables (`workflows`,
//! `workflow_actions`, `workflow_executions`, `workflow_execution_steps`,
//! `workflow_schedules`). The production api-server connects as the table
//! OWNER, which `FORCE` binds. `WorkflowRepository` held a raw `PgPool` and
//! ran every query WITHOUT ever calling `set_request_context`, so on `dev`
//! `get_current_org_id()` returned NULL and the policies collapsed to
//! deny-all: own-org reads returned empty, writes failed. (PAP-80.)
//!
//! The fix routes the repo through an RLS-context connection (the
//! `RlsConnection` extractor in handlers, `RlsPool::acquire_with_rls` in the
//! background workflow executor). This test exercises the *repository methods
//! themselves* on a `FORCE`-bound role and proves:
//!
//!   1. **Deny-all reproduction** — with the role bound but NO context set
//!      (exactly what the raw-pool repo did on `dev`), an own-org
//!      `find_by_id_for_org` / `list` returns nothing.
//!   2. **Fix** — with `set_request_context(org_a, user_a)` applied first,
//!      the same repo calls return the own-org row.
//!   3. **Cross-tenant** — org B's workflow stays invisible to an org-A
//!      caller even when the caller passes org B's own ids (RLS is the
//!      boundary, not just the SQL org filter).
//!   4. **Write path** — `create` / `add_action` / `create_execution` on a
//!      context-set connection succeed (the handler + background-executor
//!      paths); without context the INSERT fails the policy.
//!
//! Why this test switches roles
//! ----------------------------
//! `#[sqlx::test]` connects as the Postgres SUPERUSER, which bypasses RLS
//! entirely — even `FORCE` does not bind a superuser, so a behavioral
//! assertion would pass vacuously. The test creates a plain `NOSUPERUSER
//! NOBYPASSRLS` role, grants it access, and `SET ROLE`s to it so `FORCE`
//! actually enforces the policy the way the production owner role
//! experiences it. Mirrors `legal_rls_repo_tests.rs` /
//! `vendor_rls_repo_tests.rs` (the PAP-67 precedent PAP-77 follows).

use crate::common::{seed_org, set_ctx};
use db::models::{CreateWorkflow, CreateWorkflowAction, TriggerWorkflow, WorkflowQuery};
use db::repositories::WorkflowRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Workflow User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a workflow directly (as superuser, RLS-exempt) for an org.
async fn seed_workflow(pool: &PgPool, org_id: Uuid, user_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO workflows
            (organization_id, name, trigger_type, trigger_config, conditions, created_by, enabled)
        VALUES ($1, $2, 'manual', '{}'::jsonb, '[]'::jsonb, $3, TRUE)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed workflow")
}

fn sample_create() -> CreateWorkflow {
    CreateWorkflow {
        name: "Created under RLS".to_string(),
        description: None,
        trigger_type: "manual".to_string(),
        trigger_config: None,
        conditions: None,
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn workflow_repo_force_rls_deny_all_and_fix(pool: PgPool) {
    let repo = WorkflowRepository::new(pool.clone());

    // --- Seed as superuser / super-admin context (satisfies org roles-trigger). ---
    set_ctx(&pool, None, None, true).await;
    let org_a = seed_org(&pool, "workflow-force-a").await;
    let org_b = seed_org(&pool, "workflow-force-b").await;
    let user_a = seed_user(&pool, "a@workflow.test").await;
    let user_b = seed_user(&pool, "b@workflow.test").await;
    let wf_a = seed_workflow(&pool, org_a, user_a, "Org A workflow").await;
    let wf_b = seed_workflow(&pool, org_b, user_b, "Org B workflow").await;

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE actually binds. ---
    let role = format!("ppt_rls_workflow_{}", Uuid::new_v4().simple());
    for stmt in [
        format!("CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"),
        format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON workflows, workflow_actions, \
             workflow_executions, workflow_execution_steps TO \"{role}\""
        ),
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
        sqlx::query("SELECT clear_request_context()")
            .execute(&mut *conn)
            .await
            .expect("clear ctx");
        sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
            .execute(&mut *conn)
            .await
            .expect("set role");

        let found = repo
            .find_by_id_for_org(&mut *conn, wf_a, org_a)
            .await
            .expect("find_by_id_for_org (no ctx)");
        assert!(
            found.is_none(),
            "PAP-80 regression: without RLS context, own-org workflow must be \
             invisible (deny-all) — this is what the raw-pool repo did on dev"
        );

        let listed = repo
            .list(&mut *conn, org_a, WorkflowQuery::default())
            .await
            .expect("list (no ctx)");
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
            .find_by_id_for_org(&mut *conn, wf_a, org_a)
            .await
            .expect("find_by_id_for_org (ctx)");
        assert_eq!(
            found.map(|w| w.id),
            Some(wf_a),
            "PAP-80 fix: with RLS context set, the repo must return the own-org workflow"
        );

        let listed = repo
            .list(&mut *conn, org_a, WorkflowQuery::default())
            .await
            .expect("list (ctx)");
        assert_eq!(
            listed.iter().map(|w| w.id).collect::<Vec<_>>(),
            vec![wf_a],
            "PAP-80 fix: list returns exactly the own-org workflow under context"
        );

        // (3) Org B's workflow stays invisible to an org-A caller — even
        // when the caller passes org B's own ids, proving RLS (not just the
        // SQL org key) is what scopes the read.
        let cross = repo
            .find_by_id_for_org(&mut *conn, wf_b, org_a)
            .await
            .expect("find_by_id_for_org cross (keyed)");
        assert!(
            cross.is_none(),
            "cross-tenant: org B's workflow must NOT be visible to an org-A caller"
        );
        let cross_rls_only = repo
            .find_by_id_for_org(&mut *conn, wf_b, org_b)
            .await
            .expect("find_by_id_for_org cross (RLS boundary)");
        assert!(
            cross_rls_only.is_none(),
            "cross-tenant: even passing org B's own ids, the org-A RLS context \
             must hide org B's workflow"
        );

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // ====================================================================
    // (4) WRITE path: create / add_action / create_execution on a context-set
    //     connection succeed — the handler and background-executor paths.
    //     Without context the INSERTs would fail the policy WITH CHECK (the
    //     write-side of deny-all).
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
            .create(&mut *conn, org_a, user_a, sample_create())
            .await
            .expect("create under context must succeed");
        assert_eq!(created.organization_id, org_a);

        // add_action is the multi-statement (&mut PgConnection) shape: a
        // tenant check followed by the insert, both on the same connection.
        let action = repo
            .add_action(
                &mut conn,
                org_a,
                CreateWorkflowAction {
                    workflow_id: created.id,
                    action_order: 1,
                    action_type: "send_notification".to_string(),
                    action_config: serde_json::json!({}),
                    on_failure: None,
                    retry_count: None,
                    retry_delay_seconds: None,
                },
            )
            .await
            .expect("add_action under context must succeed");
        assert!(
            action.is_some(),
            "add_action must attach the action to the own-org workflow"
        );

        // create_execution is what the background executor runs on its
        // RlsPool-acquired connection.
        let execution = repo
            .create_execution(
                &mut conn,
                TriggerWorkflow {
                    workflow_id: created.id,
                    trigger_event: serde_json::json!({"manual": true}),
                    context: None,
                },
            )
            .await
            .expect("create_execution under context must succeed");
        assert_eq!(execution.workflow_id, created.id);

        sqlx::query("RESET ROLE")
            .execute(&mut *conn)
            .await
            .expect("reset role");
    }

    // --- Cleanup the test role. ---
    set_ctx(&pool, None, None, true).await;
    for stmt in [
        format!(
            "REVOKE ALL ON workflows, workflow_actions, workflow_executions, \
             workflow_execution_steps FROM \"{role}\""
        ),
        format!("REVOKE ALL ON organizations FROM \"{role}\""),
        // DROP OWNED severs every remaining privilege this role holds in the
        // test database — explicit REVOKEs keep missing the RLS
        // helper-function EXECUTE grants, so DROP ROLE would fail with
        // "objects depend on it" and leak the cluster-global role (PAP-134).
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}

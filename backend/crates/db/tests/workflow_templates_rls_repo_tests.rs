//! Compile-time RLS-routing guard for PAP-104 (parent PAP-80 / PAP-67).
//!
//! Why this is a *compile* guard and not a behavioral role-switch test
//! ------------------------------------------------------------------
//! Every other repo in the PAP-80 sweep (`ai_chat`, `work_order`, `vendor`,
//! `enhanced_tenant_screening`, …) ships a `#[sqlx::test]` that bounds a
//! `NOSUPERUSER NOBYPASSRLS` role and proves a by-id read returns deny-all
//! without RLS context and the own-org row with it. That pattern is **not
//! constructible for `WorkflowTemplateRepository`**:
//!
//!   * The `workflow_template*` tables its marketplace methods target
//!     (`workflow_templates`, `workflow_template_actions`,
//!     `workflow_template_variables`, `workflow_template_ratings`) are **not
//!     created by any migration** — the feature (Epic 94 marketplace) was never
//!     migrated, and `migrator = "db::MIGRATOR"` would have no such tables to
//!     seed or query. They also carry **no RLS policy**, so there is nothing for
//!     a role-switch test to assert.
//!   * The only `FORCE ROW LEVEL SECURITY` tables this repo actually writes
//!     (`workflows` / `workflow_actions`, migration 00179) are reached **only
//!     through `import_template`**, which first reads the non-existent
//!     `workflow_templates` table and so can never reach the protected writes.
//!   * The repository is **not constructed by any server** — the
//!     `/api/v1/ai/workflows/templates` handlers serve `get_builtin_templates()`
//!     instead, so there is no live handler path to exercise either.
//!
//! What this file *does* lock in
//! -----------------------------
//! The value of the PAP-104 conversion is that the repo no longer holds a raw
//! `PgPool`: every method now takes an executor whose connection already has RLS
//! context set (supplied by the handler's `RlsConnection`). This guard is a
//! never-run function that calls each method shape with a `&mut PgConnection`.
//! If anyone reverts a method to take `&self.pool` (or otherwise drops the
//! executor parameter), this file stops compiling — which fails the `cargo test`
//! / `cargo check --all-targets` gate. That is the regression we can enforce
//! without a schema that does not exist.
//!
//! See the PAP-104 issue thread for the dead-scaffold disposition question
//! (convert-for-future vs. delete) raised to the CTO.

use db::models::{
    CreateTemplateAction, CreateTemplateVariable, CreateWorkflowTemplate, ImportTemplateRequest,
    RateTemplateRequest, TemplateSearchQuery, UpdateWorkflowTemplate,
};
use db::repositories::WorkflowTemplateRepository;
use uuid::Uuid;

/// Never executed — its sole purpose is to fail compilation if any method stops
/// accepting an RLS-context executor (i.e. someone reintroduces a raw pool).
#[allow(dead_code, unused_must_use)]
async fn rls_executor_routing_compiles(
    conn: &mut sqlx::PgConnection,
    repo: &WorkflowTemplateRepository,
) {
    let id = Uuid::nil();
    let org = Some(Uuid::nil());

    // Single-statement, generic `E: Executor` — by-value reborrow of the conn.
    let _ = repo.find_by_id(&mut *conn, id).await;
    let _ = repo
        .search(&mut *conn, org, TemplateSearchQuery::default())
        .await;
    let _ = repo.list_by_category(&mut *conn, "ops", org).await;
    let _ = repo.list_featured(&mut *conn, org).await;
    let _ = repo.list_actions(&mut *conn, id).await;
    let _ = repo.list_variables(&mut *conn, id).await;
    let _ = repo.list_ratings(&mut *conn, id, 10, 0).await;
    let _ = repo.delete(&mut *conn, id).await;
    let _ = repo.delete_action(&mut *conn, id).await;
    let _ = repo.delete_variable(&mut *conn, id).await;
    let _ = repo
        .create(
            &mut *conn,
            CreateWorkflowTemplate {
                organization_id: None,
                name: String::new(),
                description: None,
                category: String::new(),
                trigger_type: String::new(),
                trigger_config: None,
                conditions: None,
                scope: None,
                tags: None,
                icon: None,
                created_by: None,
            },
        )
        .await;
    let _ = repo
        .update(
            &mut *conn,
            id,
            UpdateWorkflowTemplate {
                name: None,
                description: None,
                category: None,
                trigger_config: None,
                conditions: None,
                tags: None,
                icon: None,
                featured: None,
                active: None,
            },
        )
        .await;
    let _ = repo
        .add_action(
            &mut *conn,
            CreateTemplateAction {
                template_id: id,
                action_order: 0,
                action_type: String::new(),
                action_config: serde_json::Value::Null,
                description: None,
                on_failure: None,
                retry_count: None,
                retry_delay_seconds: None,
            },
        )
        .await;
    let _ = repo
        .add_variable(
            &mut *conn,
            CreateTemplateVariable {
                template_id: id,
                name: String::new(),
                label: String::new(),
                description: None,
                variable_type: String::new(),
                default_value: None,
                required: None,
                options: None,
                validation_pattern: None,
            },
        )
        .await;

    // Multi-statement methods — require a `&mut PgConnection` so every statement
    // runs on the SAME RLS-scoped connection.
    let _ = repo.find_with_details(&mut *conn, id).await;
    let _ = repo
        .import_template(
            &mut *conn,
            Uuid::nil(),
            Uuid::nil(),
            ImportTemplateRequest {
                template_id: id,
                name: None,
                variables: serde_json::Value::Null,
                enabled: None,
            },
        )
        .await;
    let _ = repo
        .rate_template(
            &mut *conn,
            id,
            Uuid::nil(),
            Uuid::nil(),
            RateTemplateRequest {
                rating: 0,
                review: None,
            },
        )
        .await;
    let _ = repo.seed_builtin_templates(&mut *conn).await;
}

/// A trivial passing test so the integration-test binary reports a green run
/// rather than "0 tests"; the real assertion is the compile of the guard above.
#[test]
fn workflow_templates_repo_holds_no_pool() {
    // `WorkflowTemplateRepository` is a zero-sized type (no `pool` field) — it
    // cannot issue an un-scoped, RLS-bypassing query. If a `PgPool` field is
    // ever reintroduced this assertion (and the guard above) break.
    assert_eq!(std::mem::size_of::<WorkflowTemplateRepository>(), 0);
}

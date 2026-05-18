//! Integration tests for the H1 cross-tenant IDOR fix on workflow endpoints.
//!
//! Audit history: round-21 workflow security review flagged nine handlers on
//! `/api/v1/ai/workflows/*` that took a workflow UUID via `Path<Uuid>` and
//! operated on the row WITHOUT verifying that the workflow's
//! `organization_id` matched the caller's tenant. The worst offender,
//! `POST /api/v1/ai/workflows/{id}/trigger`, actually executes the
//! workflow — so any authenticated user could trigger workflows belonging
//! to any other organization by guessing IDs.
//!
//! The fix pushes the tenant scope down into the repository layer
//! (`WorkflowRepository::find_by_id_for_org`, `update`, `delete`,
//! `list_actions`, `add_action`, `delete_action`,
//! `find_execution_by_id_for_org`, `list_execution_steps_for_org`) and uses
//! a uniform 404 response for both "doesn't exist" and "exists in another
//! org" so callers cannot enumerate cross-tenant IDs by status code.
//!
//! These tests exercise the HTTP surface end-to-end:
//!   1. Seed two orgs (A, B) and a workflow in Org A.
//!   2. Build an `X-Tenant-Context` that claims membership in Org B.
//!   3. Hit each vulnerable endpoint with Org A's workflow ID.
//!   4. Assert 404, not 200/204/500 — i.e. the operation never reached the
//!      row.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("IDOR Org {slug}"))
    .bind(format!("idor-org-{slug}"))
    .bind(format!("{slug}@idor-int.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'IDOR Int', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_workflow(pool: &PgPool, org_id: Uuid, creator: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO workflows
            (organization_id, name, description, trigger_type,
             trigger_config, conditions, created_by, enabled)
        VALUES ($1, 'Cross-tenant target', NULL, 'manual',
                '{}'::jsonb, '[]'::jsonb, $2, TRUE)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("seed workflow")
}

/// Forge an `X-Tenant-Context` header for `org_id` / `user_id`.
///
/// The handlers in `routes/ai.rs` parse this header directly via
/// `serde_json::from_str` into a `TenantContext`. Auth middleware would
/// normally populate it from a verified JWT; for the IDOR scenario we
/// don't need a real JWT — what we're testing is whether the handler
/// trusts the header's `tenant_id` enough to reach a different org's row.
/// If the fix is correct, the handler honors `tenant_id` and returns 404.
fn tenant_context_header(org_id: Uuid, user_id: Uuid) -> String {
    json!({
        "tenant_id": org_id,
        "user_id": user_id,
        "role": "OrgAdmin",
    })
    .to_string()
}

fn req(method: Method, uri: &str, ctx: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Tenant-Context", ctx);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let payload = body
        .map(|v| Body::from(v.to_string()))
        .unwrap_or_else(Body::empty);
    b.body(payload).unwrap()
}

// ---------------------------------------------------------------------------
// H1 — cross-tenant IDOR on every vulnerable handler
// ---------------------------------------------------------------------------

/// The flagship test: trigger_workflow MUST refuse to execute another
/// tenant's workflow. Returns 404 (not 403) so the caller cannot tell
/// the workflow exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn trigger_workflow_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "a").await;
    let org_b = seed_org(&pool, "b").await;
    let user_a = seed_user(&pool, "owner-a@idor-int.test").await;
    let user_b = seed_user(&pool, "owner-b@idor-int.test").await;
    let workflow_in_a = seed_workflow(&pool, org_a, user_a).await;

    // Caller identifies as Org B, tries to trigger Org A's workflow.
    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/{}/trigger", workflow_in_a);
    let body = json!({
        "workflow_id": workflow_in_a,
        "trigger_event": {"manual": true},
        "context": null,
    });

    let response = app
        .execute(req(Method::POST, &uri, &ctx_b, Some(body)))
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "cross-tenant trigger must return 404, got {} — body: {}",
        response.status,
        response.text()
    );
    let json = response.json_value();
    assert_eq!(json["code"].as_str().unwrap_or(""), "NOT_FOUND");
}

/// Sanity: the same call from the OWNING tenant must NOT 404. We can't
/// assert a successful execution without seeding actions and a full
/// executor environment, but we can assert that the response is
/// anything-other-than 404 (i.e. the tenant check passed and the handler
/// proceeded to the executor).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn trigger_workflow_from_owning_org_does_not_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "a-owner").await;
    let user_a = seed_user(&pool, "owner-a-owner@idor-int.test").await;
    let workflow_in_a = seed_workflow(&pool, org_a, user_a).await;

    let ctx_a = tenant_context_header(org_a, user_a);
    let uri = format!("/api/v1/ai/workflows/{}/trigger", workflow_in_a);
    let body = json!({
        "workflow_id": workflow_in_a,
        "trigger_event": {"manual": true},
        "context": null,
    });

    let response = app
        .execute(req(Method::POST, &uri, &ctx_a, Some(body)))
        .await;

    assert_ne!(
        response.status,
        StatusCode::NOT_FOUND,
        "owning-tenant trigger must NOT return 404 — body: {}",
        response.text()
    );
}

/// GET /workflows/{id} from another org → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_workflow_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "get-a").await;
    let org_b = seed_org(&pool, "get-b").await;
    let user_a = seed_user(&pool, "get-a@idor-int.test").await;
    let user_b = seed_user(&pool, "get-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/{}", wf);
    let response = app.execute(req(Method::GET, &uri, &ctx_b, None)).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

/// PUT /workflows/{id} from another org → 404 and no row mutation.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_workflow_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "upd-a").await;
    let org_b = seed_org(&pool, "upd-b").await;
    let user_a = seed_user(&pool, "upd-a@idor-int.test").await;
    let user_b = seed_user(&pool, "upd-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/{}", wf);
    let body = json!({"name": "hijacked"});
    let response = app
        .execute(req(Method::PUT, &uri, &ctx_b, Some(body)))
        .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);

    // Verify the row was NOT mutated.
    let name: String = sqlx::query_scalar("SELECT name FROM workflows WHERE id = $1")
        .bind(wf)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "Cross-tenant target");
}

/// DELETE /workflows/{id} from another org → 404 and the row survives.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_workflow_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "del-a").await;
    let org_b = seed_org(&pool, "del-b").await;
    let user_a = seed_user(&pool, "del-a@idor-int.test").await;
    let user_b = seed_user(&pool, "del-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/{}", wf);
    let response = app.execute(req(Method::DELETE, &uri, &ctx_b, None)).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workflows WHERE id = $1")
        .bind(wf)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 1,
        "workflow must still exist after cross-tenant DELETE"
    );
}

/// GET /workflows/{id}/actions from another org → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_actions_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "la-a").await;
    let org_b = seed_org(&pool, "la-b").await;
    let user_a = seed_user(&pool, "la-a@idor-int.test").await;
    let user_b = seed_user(&pool, "la-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/{}/actions", wf);
    let response = app.execute(req(Method::GET, &uri, &ctx_b, None)).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

/// POST /workflows/{id}/actions from another org → 404 and no row inserted.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_action_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "aa-a").await;
    let org_b = seed_org(&pool, "aa-b").await;
    let user_a = seed_user(&pool, "aa-a@idor-int.test").await;
    let user_b = seed_user(&pool, "aa-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/{}/actions", wf);
    let body = json!({
        "workflow_id": wf,
        "action_order": 1,
        "action_type": "send_notification",
        "action_config": {},
        "on_failure": "stop",
        "retry_count": 1,
        "retry_delay_seconds": 60,
    });
    let response = app
        .execute(req(Method::POST, &uri, &ctx_b, Some(body)))
        .await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_actions WHERE workflow_id = $1")
            .bind(wf)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "no actions must be inserted on cross-tenant POST");
}

/// DELETE /workflows/actions/{action_id} from another org → 404 and the
/// action survives.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_action_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "da-a").await;
    let org_b = seed_org(&pool, "da-b").await;
    let user_a = seed_user(&pool, "da-a@idor-int.test").await;
    let user_b = seed_user(&pool, "da-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;
    let action_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO workflow_actions
            (workflow_id, action_order, action_type, action_config, on_failure,
             retry_count, retry_delay_seconds)
        VALUES ($1, 1, 'send_notification', '{}'::jsonb, 'stop', 1, 60)
        RETURNING id
        "#,
    )
    .bind(wf)
    .fetch_one(&pool)
    .await
    .unwrap();

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/actions/{}", action_id);
    let response = app.execute(req(Method::DELETE, &uri, &ctx_b, None)).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workflow_actions WHERE id = $1")
        .bind(action_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(exists, 1, "action must survive cross-tenant DELETE");
}

/// GET /workflows/executions/{id} from another org → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_execution_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "ge-a").await;
    let org_b = seed_org(&pool, "ge-b").await;
    let user_a = seed_user(&pool, "ge-a@idor-int.test").await;
    let user_b = seed_user(&pool, "ge-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;

    let exec_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO workflow_executions (workflow_id, trigger_event, context, status, started_at)
        VALUES ($1, '{}'::jsonb, '{}'::jsonb, 'completed', NOW())
        RETURNING id
        "#,
    )
    .bind(wf)
    .fetch_one(&pool)
    .await
    .unwrap();

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/executions/{}", exec_id);
    let response = app.execute(req(Method::GET, &uri, &ctx_b, None)).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

/// GET /workflows/executions/{id}/steps from another org → 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_execution_steps_from_other_org_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "les-a").await;
    let org_b = seed_org(&pool, "les-b").await;
    let user_a = seed_user(&pool, "les-a@idor-int.test").await;
    let user_b = seed_user(&pool, "les-b@idor-int.test").await;
    let wf = seed_workflow(&pool, org_a, user_a).await;

    let exec_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO workflow_executions (workflow_id, trigger_event, context, status, started_at)
        VALUES ($1, '{}'::jsonb, '{}'::jsonb, 'completed', NOW())
        RETURNING id
        "#,
    )
    .bind(wf)
    .fetch_one(&pool)
    .await
    .unwrap();

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/workflows/executions/{}/steps", exec_id);
    let response = app.execute(req(Method::GET, &uri, &ctx_b, None)).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}

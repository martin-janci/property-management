//! BIT-268 Wave 4 — AI/Automation group — Batch 2: Automation + AI endpoints.
//!
//! Verifies success-path 200/201 responses for:
//!   - /api/v1/automation (rules, templates)
//!   - /api/v1/ai/chat (sessions, messages)
//!   - /api/v1/ai/sentiment
//!   - /api/v1/ai/equipment
//!   - /api/v1/ai/workflows

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, create_authenticated_user_with_org, seed_membership, seed_org,
    TestApp, TestUser,
};

/// Inject a `ResolvedTenant` extension so `RequestPrincipal` can resolve the
/// tenant without a running `host_tenant_middleware`.
fn inject_tenant(mut req: Request<Body>, org_id: Uuid) -> Request<Body> {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    req.extensions_mut().insert(ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    });
    req
}

/// Seed an active `user_memberships` row so `RequestPrincipal` membership
/// checks pass for endpoints that use it instead of `RlsConnection`.
async fn seed_user_membership(pool: &PgPool, user_id: Uuid, org_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, 'org_admin')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed user_membership");
}

const UUID: &str = "00000000-0000-0000-0000-000000000001";

fn authed(
    token: &str,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
    org_id: Uuid,
) -> Request<Body> {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string());
    match body {
        Some(j) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
         VALUES ($1, 'AI Test Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO equipment (organization_id, building_id, name, category, status) \
         VALUES ($1, $2, 'Test Equipment', 'hvac', 'operational') RETURNING id",
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

async fn seed_workflow(pool: &PgPool, org_id: Uuid, creator: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO workflows \
         (organization_id, name, trigger_type, trigger_config, conditions, created_by, enabled) \
         VALUES ($1, 'Test Workflow', 'manual', '{}'::jsonb, '[]'::jsonb, $2, TRUE) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("seed workflow")
}

// ---------------------------------------------------------------------------
// Automation — Rules
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_automation_rules_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-rule-1").await;

    let uri = format!("/api/v1/automation/organizations/{org_id}/rules");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list automation rules");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_automation_rule_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-rule-2").await;

    let uri = format!("/api/v1/automation/organizations/{org_id}/rules");
    let body = json!({
        "name": "Test Rule",
        "trigger_type": "schedule",
        "trigger_config": {"cron": "0 9 * * 1"},
        "actions": [{"type": "notify", "config": {}}]
    });
    let resp = app
        .execute(authed(&token, Method::POST, &uri, Some(body), org_id))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create automation rule");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_automation_rule_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-rule-3").await;

    let uri = format!("/api/v1/automation/rules/{UUID}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "get missing rule: {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_automation_templates_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-tmpl-1").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/automation/templates",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list automation templates");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_automation_template_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-tmpl-2").await;

    let uri = format!("/api/v1/automation/templates/{UUID}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "get missing template: {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// AI Chat — Sessions
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_chat_session_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-chat-1").await;

    let body = json!({"title": "Test Session"});
    let resp = app
        .execute(authed(
            &token,
            Method::POST,
            "/api/v1/ai/chat/sessions",
            Some(body),
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create chat session");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_chat_sessions_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-chat-2").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/chat/sessions",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list chat sessions");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_chat_session_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-chat-3").await;

    let uri = format!("/api/v1/ai/chat/sessions/{UUID}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert!(
        resp.status.is_client_error(),
        "get missing session: {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_chat_escalated_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-chat-4").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/chat/escalated",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list escalated sessions");
}

/// Deny-path regression for the #2317 role gate on `GET /api/v1/ai/chat/escalated`.
///
/// `list_escalated_messages` is deliberately org-scoped and NOT owner-scoped, so
/// the handler's `!is_super_admin() && !has_role(Manager)` check is the *only*
/// thing preventing a plain resident from reading every colleague's escalated
/// AI-chat messages. The positive `test_list_chat_escalated_returns_200` above
/// only passes because `create_authenticated_user_with_org` seeds the caller as
/// `org_admin` (which clears the Manager bar), so nothing there pins the refusal.
/// This test seeds a below-Manager `resident` and asserts 403 — if the gate is
/// ever removed or weakened, this fails where the admin-role positive test would
/// keep sailing through at 200. (Follow-up #2357 to PR #2356 / #2317.)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_chat_escalated_forbidden_for_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, _refresh) = create_authenticated_user(&app, &user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");
    let org_id = seed_org(&app.pool, "ai-chat-4-resident").await;
    seed_membership(&app.pool, org_id, user_id, "resident").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/chat/escalated",
            None,
            org_id,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a below-Manager resident must be refused escalated-message review"
    );
}

/// Positive companion to the deny-path test: a `manager`-role member (the exact
/// boundary the gate checks) must pass. Together these pin both sides of the
/// `has_role(TenantRole::Manager)` boundary. (Follow-up #2357.)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_chat_escalated_allowed_for_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, _refresh) = create_authenticated_user(&app, &user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");
    let org_id = seed_org(&app.pool, "ai-chat-4-manager").await;
    seed_membership(&app.pool, org_id, user_id, "manager").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/chat/escalated",
            None,
            org_id,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "a Manager-role member must be allowed escalated-message review"
    );
}

// ---------------------------------------------------------------------------
// AI Sentiment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_sentiment_trends_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-sent-1").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/sentiment/trends",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get sentiment trends");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_sentiment_alerts_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-sent-2").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/sentiment/alerts",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list sentiment alerts");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_sentiment_thresholds_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-sent-3").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/sentiment/thresholds",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get sentiment thresholds");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_sentiment_dashboard_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-sent-4").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/sentiment/dashboard",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get sentiment dashboard");
}

// ---------------------------------------------------------------------------
// AI Equipment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_equipment_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-1").await;
    let building = seed_building(&pool, org_id).await;

    let body = json!({
        "building_id": building,
        "name": "HVAC Unit A",
        "category": "hvac"
    });
    let resp = app
        .execute(authed(
            &token,
            Method::POST,
            "/api/v1/ai/equipment",
            Some(body),
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create ai equipment");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_equipment_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-2").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/equipment/",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list ai equipment");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_equipment_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-3").await;
    let building = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building).await;

    let uri = format!("/api/v1/ai/equipment/{eq_id}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get ai equipment");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_predictions_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-4").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/equipment/predictions",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list predictions");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_needing_maintenance_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-5").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/equipment/needing-maintenance",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list needing maintenance");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_maintenance_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-6").await;
    let building = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building).await;

    let uri = format!("/api/v1/ai/equipment/{eq_id}/maintenance");
    let body = json!({
        "equipment_id": eq_id,
        "maintenance_type": "preventive",
        "description": "Routine filter check"
    });
    let resp = app
        .execute(authed(&token, Method::POST, &uri, Some(body), org_id))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create maintenance record"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_maintenance_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-7").await;
    let building = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building).await;

    let uri = format!("/api/v1/ai/equipment/{eq_id}/maintenance");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list maintenance records");
}

// ---------------------------------------------------------------------------
// AI Workflows
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_create_workflow_returns_201(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-1").await;

    let body = json!({
        "name": "Test Workflow",
        "trigger_type": "manual"
    });
    let resp = app
        .execute(authed(
            &token,
            Method::POST,
            "/api/v1/ai/workflows/",
            Some(body),
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create workflow");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_workflows_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-2").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/workflows/",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list workflows");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_workflow_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-3").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");
    let wf_id = seed_workflow(&pool, org_id, user_id).await;

    let uri = format!("/api/v1/ai/workflows/{wf_id}");
    let resp = app
        .execute(authed(&token, Method::GET, &uri, None, org_id))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get workflow");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_workflow_executions_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-4").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/workflows/executions",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list workflow executions");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_workflow_templates_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-5").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/workflows/templates",
            None,
            org_id,
        ))
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list workflow templates");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_builtin_workflow_templates_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-6").await;

    let resp = app
        .execute(authed(
            &token,
            Method::GET,
            "/api/v1/ai/workflows/templates/builtin",
            None,
            org_id,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list builtin workflow templates"
    );
}

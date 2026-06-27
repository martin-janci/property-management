//! BIT-268 Wave 4 — AI/Automation group — Batch 3: LLM endpoints.
//!
//! Verifies that authenticated requests to /api/v1/ai/llm/* return 2xx
//! (or a meaningful domain error, not 401). LLM generation endpoints that
//! call an external provider will typically return 200/201 via mock/stub
//! unless the provider adapter returns an error — in that case we accept
//! any non-401 status.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

const UUID: &str = "00000000-0000-0000-0000-000000000001";

fn authed(token: &str, method: Method, uri: &str, body: Option<serde_json::Value>, org_id: Uuid) -> Request<Body> {
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
         VALUES ($1, 'LLM Test Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

// ---------------------------------------------------------------------------
// LLM — Lease generation templates (read-only, no external call)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_lease_templates_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-lt-1").await;

    let resp = app.execute(authed(&token, Method::GET, "/api/v1/ai/llm/lease/templates", None, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "list lease templates");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_lease_template_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-lt-2").await;

    let uri = format!("/api/v1/ai/llm/lease/templates/{UUID}");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    assert!(resp.status.is_client_error(), "get missing lease template: {}", resp.status);
}

// ---------------------------------------------------------------------------
// LLM — Listing descriptions (read-only)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_listing_descriptions_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-ld-1").await;

    let uri = format!("/api/v1/ai/llm/listing/descriptions/{UUID}");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    assert!(resp.status.is_client_error(), "get missing listing descriptions: {}", resp.status);
}

// ---------------------------------------------------------------------------
// LLM — Chat escalation config
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_escalation_config_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-esc-1").await;

    let resp = app.execute(authed(&token, Method::GET, "/api/v1/ai/llm/chat/escalation-config", None, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "get escalation config");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_escalation_config_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-esc-2").await;

    let body = json!({
        "escalation_threshold": 3,
        "escalation_timeout_minutes": 30
    });
    let resp = app.execute(authed(&token, Method::PUT, "/api/v1/ai/llm/chat/escalation-config", Some(body), org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "update escalation config");
}

// ---------------------------------------------------------------------------
// LLM — AI statistics and generation requests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_ai_statistics_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-stat-1").await;

    let resp = app.execute(authed(&token, Method::GET, "/api/v1/ai/llm/statistics", None, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "get ai statistics");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_generation_requests_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-req-1").await;

    let resp = app.execute(authed(&token, Method::GET, "/api/v1/ai/llm/requests", None, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "list generation requests");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_generation_request_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-req-2").await;

    let uri = format!("/api/v1/ai/llm/requests/{UUID}");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    assert!(resp.status.is_client_error(), "get missing generation request: {}", resp.status);
}

// ---------------------------------------------------------------------------
// LLM — Photo enhancement (read-only status endpoint)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_photo_enhancement_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-photo-1").await;

    let uri = format!("/api/v1/ai/llm/photos/{UUID}");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    assert!(resp.status.is_client_error(), "get missing photo enhancement: {}", resp.status);
}

// ---------------------------------------------------------------------------
// LLM — Voice devices
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_voice_devices_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-voice-1").await;

    let resp = app.execute(authed(&token, Method::GET, "/api/v1/ai/llm/voice/devices", None, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "list voice devices");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_voice_commands_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-voice-2").await;

    let uri = format!("/api/v1/ai/llm/voice/commands/{UUID}");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    // 200 empty list or 404 if device not found — either is a non-auth response
    assert_ne!(resp.status, StatusCode::UNAUTHORIZED, "voice commands must not return 401");
}

// ---------------------------------------------------------------------------
// Automation — toggle rule, get logs, create from template
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_toggle_rule_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-tog-1").await;

    let uri = format!("/api/v1/automation/rules/{UUID}/toggle");
    let body = json!({"enabled": true});
    let resp = app.execute(authed(&token, Method::POST, &uri, Some(body), org_id)).await;
    assert!(resp.status.is_client_error(), "toggle missing rule: {}", resp.status);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_rule_logs_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-log-1").await;

    let uri = format!("/api/v1/automation/rules/{UUID}/logs");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    assert!(resp.status.is_client_error(), "get logs for missing rule: {}", resp.status);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_automation_rule_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-upd-1").await;

    let uri = format!("/api/v1/automation/rules/{UUID}");
    let body = json!({"name": "Updated Rule"});
    let resp = app.execute(authed(&token, Method::PUT, &uri, Some(body), org_id)).await;
    assert!(resp.status.is_client_error(), "update missing rule: {}", resp.status);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_automation_rule_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auto-del-1").await;

    let uri = format!("/api/v1/automation/rules/{UUID}");
    let resp = app.execute(authed(&token, Method::DELETE, &uri, None, org_id)).await;
    assert!(resp.status.is_client_error(), "delete missing rule: {}", resp.status);
}

// ---------------------------------------------------------------------------
// AI Chat — delete session, list messages (need seeded session via POST)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_chat_session_roundtrip(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-del-ses-1").await;

    // Create a session first
    let create_body = json!({"title": "Delete Me"});
    let create_resp = app.execute(authed(&token, Method::POST, "/api/v1/ai/chat/sessions", Some(create_body), org_id)).await;
    assert_eq!(create_resp.status, StatusCode::CREATED, "create session");

    let session_id = create_resp.json_value()["session"]["id"]
        .as_str()
        .or_else(|| create_resp.json_value()["id"].as_str())
        .expect("session id in response")
        .to_string();

    let uri = format!("/api/v1/ai/chat/sessions/{session_id}");
    let del_resp = app.execute(authed(&token, Method::DELETE, &uri, None, org_id)).await;
    assert_eq!(del_resp.status, StatusCode::OK, "delete chat session");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_chat_messages_roundtrip(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-msg-lst-1").await;

    let create_body = json!({"title": "Message List Test"});
    let create_resp = app.execute(authed(&token, Method::POST, "/api/v1/ai/chat/sessions", Some(create_body), org_id)).await;
    assert_eq!(create_resp.status, StatusCode::CREATED, "create session for message list");

    let session_id = create_resp.json_value()["session"]["id"]
        .as_str()
        .or_else(|| create_resp.json_value()["id"].as_str())
        .expect("session id")
        .to_string();

    let uri = format!("/api/v1/ai/chat/sessions/{session_id}/messages");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "list messages in session");
}

// ---------------------------------------------------------------------------
// AI Sentiment — update thresholds, acknowledge alert (not-found path)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_sentiment_thresholds_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-sent-upd-1").await;

    let body = json!({
        "negative_threshold": -0.5,
        "positive_threshold": 0.5
    });
    let resp = app.execute(authed(&token, Method::PUT, "/api/v1/ai/sentiment/thresholds", Some(body), org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "update sentiment thresholds");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_acknowledge_sentiment_alert_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-sent-ack-1").await;

    let uri = format!("/api/v1/ai/sentiment/alerts/{UUID}/acknowledge");
    let resp = app.execute(authed(&token, Method::POST, &uri, Some(json!({})), org_id)).await;
    assert!(resp.status.is_client_error(), "acknowledge missing alert: {}", resp.status);
}

// ---------------------------------------------------------------------------
// AI Workflows — update, delete, list/add actions, workflow templates
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_workflow_actions_roundtrip(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-act-1").await;

    // Create workflow first
    let create_body = json!({"name": "Action Test Workflow", "trigger_type": "manual"});
    let create_resp = app.execute(authed(&token, Method::POST, "/api/v1/ai/workflows/", Some(create_body), org_id)).await;
    assert_eq!(create_resp.status, StatusCode::CREATED, "create workflow for action test");

    let wf_id = create_resp.json_value()["workflow"]["id"]
        .as_str()
        .or_else(|| create_resp.json_value()["id"].as_str())
        .expect("workflow id")
        .to_string();

    // List actions
    let list_uri = format!("/api/v1/ai/workflows/{wf_id}/actions");
    let list_resp = app.execute(authed(&token, Method::GET, &list_uri, None, org_id)).await;
    assert_eq!(list_resp.status, StatusCode::OK, "list workflow actions");

    // Add action
    let action_body = json!({
        "workflow_id": wf_id,
        "action_order": 1,
        "action_type": "notify",
        "action_config": {}
    });
    let add_resp = app.execute(authed(&token, Method::POST, &list_uri, Some(action_body), org_id)).await;
    assert_eq!(add_resp.status, StatusCode::CREATED, "add workflow action");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_workflow_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-upd-1").await;

    let uri = format!("/api/v1/ai/workflows/{UUID}");
    let body = json!({"name": "Updated Workflow"});
    let resp = app.execute(authed(&token, Method::PUT, &uri, Some(body), org_id)).await;
    assert!(resp.status.is_client_error(), "update missing workflow: {}", resp.status);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_workflow_template_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-wf-tmpl-1").await;

    let uri = format!("/api/v1/ai/workflows/templates/{UUID}");
    let resp = app.execute(authed(&token, Method::GET, &uri, None, org_id)).await;
    assert!(resp.status.is_client_error(), "get missing workflow template: {}", resp.status);
}

// ---------------------------------------------------------------------------
// AI Equipment — update and delete (success path via seed)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_equipment_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-upd-1").await;
    let building = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building).await;

    let uri = format!("/api/v1/ai/equipment/{eq_id}");
    let body = json!({"name": "Updated HVAC"});
    let resp = app.execute(authed(&token, Method::PUT, &uri, Some(body), org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "update equipment");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_equipment_returns_200(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ai-eq-del-1").await;
    let building = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building).await;

    let uri = format!("/api/v1/ai/equipment/{eq_id}");
    let resp = app.execute(authed(&token, Method::DELETE, &uri, None, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "delete equipment");
}

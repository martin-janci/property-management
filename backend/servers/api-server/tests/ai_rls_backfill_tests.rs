//! BIT-268 wave-4 (AI/automation) batch 2: equipment + sessions + sentiment
//! success-path tests.
//!
//! Behaviour-asserting (2xx success path) coverage for endpoints in
//! docs/endpoint-checklist/groups/ai-automation.md that use `RlsConnection`
//! (ValidatedTenantExtractor + X-Tenant-ID header) — the same harness as
//! batch 1's registry tests.
//!
//! ## equipment_router  (/api/v1/ai/equipment)
//! - POST   /api/v1/ai/equipment/
//! - GET    /api/v1/ai/equipment/
//! - GET    /api/v1/ai/equipment/{id}
//! - PUT    /api/v1/ai/equipment/{id}
//! - DELETE /api/v1/ai/equipment/{id}
//! - GET    /api/v1/ai/equipment/{id}/maintenance
//! - POST   /api/v1/ai/equipment/{id}/maintenance
//! - PUT    /api/v1/ai/equipment/maintenance/{id}
//! - GET    /api/v1/ai/equipment/predictions
//! - POST   /api/v1/ai/equipment/predictions/{id}/acknowledge
//! - GET    /api/v1/ai/equipment/needing-maintenance
//!
//! ## ai_chat_router  (/api/v1/ai/chat)
//! - POST   /api/v1/ai/chat/sessions
//! - GET    /api/v1/ai/chat/sessions
//! - GET    /api/v1/ai/chat/sessions/{session_id}
//! - DELETE /api/v1/ai/chat/sessions/{session_id}
//! - GET    /api/v1/ai/chat/sessions/{session_id}/messages
//! - POST   /api/v1/ai/chat/messages/{message_id}/feedback
//! - GET    /api/v1/ai/chat/escalated
//!
//! Note: POST /api/v1/ai/chat/sessions/{session_id}/messages (send_message)
//! is excluded — it makes a live LLM provider call which is not available in
//! the test environment. Remains partial; a future batch with LLM mocking can
//! promote it.
//!
//! ## sentiment_router  (/api/v1/ai/sentiment)
//! - GET    /api/v1/ai/sentiment/trends
//! - GET    /api/v1/ai/sentiment/alerts
//! - POST   /api/v1/ai/sentiment/alerts/{alert_id}/acknowledge
//! - GET    /api/v1/ai/sentiment/thresholds
//! - PUT    /api/v1/ai/sentiment/thresholds
//! - GET    /api/v1/ai/sentiment/dashboard

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, AuthenticatedSession, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Infrastructure seed helpers
// ---------------------------------------------------------------------------

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'AI Test Building', 'Test Street 1', 'Test City', '12345', 'SK', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

// ---------------------------------------------------------------------------
// Equipment seed helpers
// ---------------------------------------------------------------------------

/// Create equipment via the real endpoint and return its id.
async fn create_equipment(
    session: &AuthenticatedSession<'_>,
    app: &TestApp,
    building_id: Uuid,
) -> Uuid {
    let resp = app
        .execute(
            session
                .post("/api/v1/ai/equipment/")
                .json(json!({
                    "building_id": building_id,
                    "name": "HVAC Unit",
                    "category": "hvac"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    Uuid::parse_str(body["id"].as_str().expect("equipment id")).expect("uuid")
}

/// Seed an equipment record directly (for tests needing a prediction seed).
async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment (organization_id, building_id, name, category, status,
                               maintenance_interval_days)
        VALUES ($1, $2, 'Boiler', 'heating', 'operational', 90)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

/// Seed a high-risk maintenance prediction so it appears in list_predictions.
async fn seed_prediction(pool: &PgPool, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO maintenance_predictions
            (equipment_id, risk_score, confidence, recommendation)
        VALUES ($1, 75.0, 0.9, 'Replace filter within 2 weeks')
        RETURNING id
        "#,
    )
    .bind(equipment_id)
    .fetch_one(pool)
    .await
    .expect("seed prediction")
}

/// Seed a maintenance record for an equipment item.
async fn seed_maintenance(pool: &PgPool, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment_maintenance
            (equipment_id, maintenance_type, description, status)
        VALUES ($1, 'preventive', 'Regular filter check', 'scheduled')
        RETURNING id
        "#,
    )
    .bind(equipment_id)
    .fetch_one(pool)
    .await
    .expect("seed maintenance")
}

// ---------------------------------------------------------------------------
// AI chat session seed helpers
// ---------------------------------------------------------------------------

/// Create a chat session via the real endpoint and return its id.
async fn create_session(session: &AuthenticatedSession<'_>, app: &TestApp) -> Uuid {
    let resp = app
        .execute(
            session
                .post("/api/v1/ai/chat/sessions")
                .json(json!({ "title": "Test session" }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    let body = resp.json_value();
    Uuid::parse_str(body["id"].as_str().expect("session id")).expect("uuid")
}

/// Seed a chat message directly (needed for provide_feedback without LLM).
async fn seed_chat_message(pool: &PgPool, session_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_chat_messages (session_id, role, content)
        VALUES ($1, 'assistant', 'Hello, how can I help you today?')
        RETURNING id
        "#,
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .expect("seed chat message")
}

// ---------------------------------------------------------------------------
// Sentiment seed helpers
// ---------------------------------------------------------------------------

/// Seed a sentiment alert directly.
async fn seed_sentiment_alert(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO sentiment_alerts
            (organization_id, alert_type, threshold_breached, current_sentiment)
        VALUES ($1, 'spike_negative', -0.5, -0.65)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed sentiment alert")
}

// ---------------------------------------------------------------------------
// Equipment tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-c").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/equipment/")
                .json(json!({
                    "building_id": building_id,
                    "name": "Elevator Motor",
                    "category": "elevator",
                    "manufacturer": "KONE",
                    "maintenance_interval_days": 90
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-l").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let _ = create_equipment(&session, &app, building_id).await;

    let resp = app
        .execute(session.get("/api/v1/ai/equipment/").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["equipment"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-g").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = create_equipment(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/equipment/{equipment_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-u").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = create_equipment(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/ai/equipment/{equipment_id}"))
                .json(json!({ "name": "Updated HVAC", "status": "needs_maintenance" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-d").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = create_equipment(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/ai/equipment/{equipment_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-lm").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = create_equipment(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/equipment/{equipment_id}/maintenance"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["maintenance"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-cm").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = create_equipment(&session, &app, building_id).await;

    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/ai/equipment/{equipment_id}/maintenance"))
                .json(json!({
                    "equipment_id": equipment_id,
                    "maintenance_type": "preventive",
                    "description": "Annual filter replacement"
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-um").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = seed_equipment(&pool, org_id, building_id).await;
    let maintenance_id = seed_maintenance(&pool, equipment_id).await;

    let resp = app
        .execute(
            session
                .put(&format!(
                    "/api/v1/ai/equipment/maintenance/{maintenance_id}"
                ))
                .json(json!({ "status": "completed", "description": "Filter replaced" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_predictions_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-lp").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = seed_equipment(&pool, org_id, building_id).await;
    let _ = seed_prediction(&pool, equipment_id).await;

    let resp = app
        .execute(session.get("/api/v1/ai/equipment/predictions").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["predictions"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_prediction_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-ap").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let equipment_id = seed_equipment(&pool, org_id, building_id).await;
    let prediction_id = seed_prediction(&pool, equipment_id).await;

    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/ai/equipment/predictions/{prediction_id}/acknowledge"
                ))
                .json(json!({ "action_taken": "Scheduled for next week" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_needing_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eq-nm").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .get("/api/v1/ai/equipment/needing-maintenance")
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["equipment"].is_array());
}

// ---------------------------------------------------------------------------
// AI chat session tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_chat_session_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "chat-c").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/chat/sessions")
                .json(json!({ "title": "Help with billing" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_chat_sessions_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "chat-l").await;
    let session = app.session(token, org_id);
    let _ = create_session(&session, &app).await;

    let resp = app
        .execute(session.get("/api/v1/ai/chat/sessions").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["sessions"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_chat_session_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "chat-g").await;
    let session = app.session(token, org_id);
    let session_id = create_session(&session, &app).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/chat/sessions/{session_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_chat_session_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "chat-d").await;
    let session = app.session(token, org_id);
    let session_id = create_session(&session, &app).await;

    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/ai/chat/sessions/{session_id}"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_chat_messages_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "chat-lm").await;
    let session = app.session(token, org_id);
    let session_id = create_session(&session, &app).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/chat/sessions/{session_id}/messages"))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["messages"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn provide_chat_feedback_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "chat-fb").await;
    let session = app.session(token, org_id);
    let session_id = create_session(&session, &app).await;
    let message_id = seed_chat_message(&pool, session_id).await;

    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/ai/chat/messages/{message_id}/feedback"))
                .json(json!({ "rating": 5, "helpful": true, "feedback_text": "Very helpful!" }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_escalated_messages_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "chat-esc").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/chat/escalated").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["messages"].is_array());
}

// ---------------------------------------------------------------------------
// Sentiment tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_sentiment_trends_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sent-tr").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/sentiment/trends").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["trends"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_sentiment_alerts_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sent-al").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/sentiment/alerts").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["alerts"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_sentiment_alert_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sent-ack").await;
    let session = app.session(token, org_id);
    let alert_id = seed_sentiment_alert(&pool, org_id).await;

    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/ai/sentiment/alerts/{alert_id}/acknowledge"
                ))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_sentiment_thresholds_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sent-gt").await;
    let session = app.session(token, org_id);

    // get_thresholds auto-creates defaults when none exist
    let resp = app
        .execute(session.get("/api/v1/ai/sentiment/thresholds").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body = resp.json_value();
    assert!(body.get("id").is_some() || body.get("enabled").is_some());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_sentiment_thresholds_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sent-ut").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .put("/api/v1/ai/sentiment/thresholds")
                .json(json!({
                    "negative_threshold": -0.4,
                    "alert_on_spike": true,
                    "min_messages_for_alert": 10,
                    "enabled": true
                }))
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_sentiment_dashboard_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sent-db").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/sentiment/dashboard").build())
        .await;

    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

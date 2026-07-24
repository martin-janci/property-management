//! Batch 4 — success-path (200/201/204) integration tests for:
//!   • ai/sessions.rs   — chat sessions + sentiment (12 endpoints, RlsConnection)
//!   • ai/equipment.rs  — equipment + maintenance + predictions (11 endpoints, RlsConnection)
//!   • registry.rs      — pets, vehicles, parking-spots, rules (19 endpoints, TenantExtractor)
//!
//! Excluded: `send_message` (live LLM call at every path), voice (no migration for
//! voice_assistant_devices), llm.rs generate_* (live LLM), workflow executor endpoints.
//!
//! Registry endpoints use TenantExtractor (not RlsConnection / RequestPrincipal),
//! so the standard `app.session(token, org_id)` pattern works without inject_tenant.
//!
//! BIT-268 / BIT-304

#![allow(dead_code)]

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};
use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn resolve_user_id(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user_id")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country)
         VALUES ($1, 'Batch4 Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor, unit_type)
         VALUES ($1, '1A', 1, 'apartment') RETURNING id",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO equipment (organization_id, building_id, name, category, status)
         VALUES ($1, $2, 'HVAC Unit', 'hvac', 'operational') RETURNING id",
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

async fn seed_maintenance(pool: &PgPool, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO equipment_maintenance
             (equipment_id, maintenance_type, description, status)
         VALUES ($1, 'inspection', 'Routine check', 'scheduled') RETURNING id",
    )
    .bind(equipment_id)
    .fetch_one(pool)
    .await
    .expect("seed maintenance")
}

async fn seed_prediction(pool: &PgPool, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO maintenance_predictions
             (equipment_id, risk_score, confidence, recommendation, factors)
         VALUES ($1, 75.0, 0.9, 'Replace filter', '{}') RETURNING id",
    )
    .bind(equipment_id)
    .fetch_one(pool)
    .await
    .expect("seed prediction")
}

async fn seed_chat_session(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO ai_chat_sessions (organization_id, user_id, title)
         VALUES ($1, $2, 'Test Session') RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed chat session")
}

async fn seed_chat_message(pool: &PgPool, session_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO ai_chat_messages (session_id, role, content)
         VALUES ($1, 'assistant', 'Hello, how can I help?') RETURNING id",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .expect("seed chat message")
}

async fn seed_sentiment_alert(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO sentiment_alerts
             (organization_id, alert_type, threshold_breached, current_sentiment)
         VALUES ($1, 'spike_negative', 0.7, -0.8) RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed sentiment alert")
}

/// Seed a pet registration; requires a unit (which requires a building).
async fn seed_pet(pool: &PgPool, org_id: Uuid, unit_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO pet_registrations
             (tenant_id, unit_id, owner_id, pet_name, pet_type, pet_size)
         VALUES ($1, $2, $3, 'Buddy', 'dog', 'medium') RETURNING id",
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed pet registration")
}

/// Seed a vehicle registration.
async fn seed_vehicle(pool: &PgPool, org_id: Uuid, unit_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO vehicle_registrations
             (tenant_id, unit_id, owner_id, vehicle_type, make, model, license_plate)
         VALUES ($1, $2, $3, 'car', 'Toyota', 'Corolla', 'BA123AB') RETURNING id",
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed vehicle registration")
}

async fn seed_parking_spot(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO parking_spots (tenant_id, building_id, spot_number)
         VALUES ($1, $2, 'A-01') RETURNING id",
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed parking spot")
}

async fn seed_registry_rules(pool: &PgPool, org_id: Uuid, building_id: Uuid) {
    sqlx::query(
        "INSERT INTO building_registry_rules (tenant_id, building_id) VALUES ($1, $2) \
         ON CONFLICT (tenant_id, building_id) DO NOTHING",
    )
    .bind(org_id)
    .bind(building_id)
    .execute(pool)
    .await
    .expect("seed registry rules");
}

// ---------------------------------------------------------------------------
// AI Chat Sessions — ai/sessions.rs chat section (6 endpoints)
// send_message is excluded: makes live LLM calls.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_chat_session_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ccs").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post("/api/v1/ai/chat/sessions")
                .json(json!({"title": "New Session", "language": "en"}))
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
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lcs").await;
    let session = app.session(token, org_id);
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
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gcs").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let sess_id = seed_chat_session(&pool, org_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/chat/sessions/{sess_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert_eq!(
        resp.json_value()["id"].as_str().unwrap(),
        sess_id.to_string()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_chat_session_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "dcs").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let sess_id = seed_chat_session(&pool, org_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/ai/chat/sessions/{sess_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_chat_messages_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lcm").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let sess_id = seed_chat_session(&pool, org_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/chat/sessions/{sess_id}/messages"))
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
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "pcf").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let sess_id = seed_chat_session(&pool, org_id, user_id).await;
    let msg_id = seed_chat_message(&pool, sess_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/ai/chat/messages/{msg_id}/feedback"))
                // `ProvideFeedback` requires `message_id` in the body (the handler
                // re-derives it from the path, but the field is non-optional).
                .json(json!({"message_id": msg_id, "rating": 5, "helpful": true, "feedback_text": "Very helpful"}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_escalated_chat_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lec").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/ai/chat/escalated").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// AI Sentiment — ai/sessions.rs sentiment section (5 endpoints)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_sentiment_trends_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gst").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/ai/sentiment/trends").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_sentiment_alerts_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lsa").await;
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
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "asa").await;
    let alert_id = seed_sentiment_alert(&pool, org_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/ai/sentiment/alerts/{alert_id}/acknowledge"
                ))
                .json(json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_sentiment_thresholds_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gsth").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/ai/sentiment/thresholds").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_sentiment_thresholds_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ust").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .put("/api/v1/ai/sentiment/thresholds")
                .json(json!({"negative_threshold": 0.6, "positive_threshold": 0.4}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_sentiment_dashboard_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gsd").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/ai/sentiment/dashboard").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

// ---------------------------------------------------------------------------
// AI Equipment — ai/equipment.rs (11 endpoints, RlsConnection)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ceq").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post("/api/v1/ai/equipment")
                .json(json!({
                    "building_id": building_id,
                    "name": "Elevator",
                    "category": "elevator"
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
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "leq").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/ai/equipment").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["equipment"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "geq").await;
    let building_id = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/equipment/{eq_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert_eq!(resp.json_value()["id"].as_str().unwrap(), eq_id.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ueq").await;
    let building_id = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/ai/equipment/{eq_id}"))
                .json(json!({"name": "Updated HVAC"}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert_eq!(resp.json_value()["name"].as_str().unwrap(), "Updated HVAC");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_equipment_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "deq").await;
    let building_id = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/ai/equipment/{eq_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_equipment_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lem").await;
    let building_id = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/equipment/{eq_id}/maintenance"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["maintenance"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_equipment_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "cem").await;
    let building_id = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/ai/equipment/{eq_id}/maintenance"))
                .json(json!({
                    "equipment_id": eq_id,
                    "maintenance_type": "inspection",
                    "description": "Annual inspection"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_equipment_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "uem").await;
    let building_id = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building_id).await;
    let maint_id = seed_maintenance(&pool, eq_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/ai/equipment/maintenance/{maint_id}"))
                // `maintenance_type` is a TEXT column with a CHECK constraint
                // (preventive|corrective|emergency|inspection); "repair" is not a
                // permitted value and violated the constraint → 500.
                .json(json!({"maintenance_type": "corrective"}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_equipment_predictions_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lep").await;
    let session = app.session(token, org_id);
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
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ap").await;
    let building_id = seed_building(&pool, org_id).await;
    let eq_id = seed_equipment(&pool, org_id, building_id).await;
    let pred_id = seed_prediction(&pool, eq_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/ai/equipment/predictions/{pred_id}/acknowledge"
                ))
                .json(json!({"notes": "Scheduled for next week"}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_equipment_needing_maintenance_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lenm").await;
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
// Registry — registry.rs (TenantExtractor, no inject_tenant needed)
// Pets: 6 endpoints, Vehicles: 6 endpoints, Parking: 5 endpoints, Rules: 2 endpoints
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "cpr").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post("/api/v1/registry/pets")
                .json(json!({
                    "unit_id": unit_id,
                    "pet_name": "Buddy",
                    "pet_type": "dog",
                    "pet_size": "medium"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["registration"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_pet_registrations_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lpr").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/registry/pets").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["registrations"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gpr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let pet_id = seed_pet(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/pets/{pet_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert_eq!(
        resp.json_value()["registration"]["id"].as_str().unwrap(),
        pet_id.to_string()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "upr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let pet_id = seed_pet(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/pets/{pet_id}"))
                .json(json!({"pet_name": "Max"}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "dpr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let pet_id = seed_pet(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/registry/pets/{pet_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_pet_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rpr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let pet_id = seed_pet(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/registry/pets/{pet_id}/review"))
                .json(json!({"approve": true}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "cvr").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post("/api/v1/registry/vehicles")
                .json(json!({
                    "unit_id": unit_id,
                    "vehicle_type": "car",
                    "make": "Toyota",
                    "model": "Corolla",
                    "license_plate": "BA123AB"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["registration"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_vehicle_registrations_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lvr").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/registry/vehicles").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["registrations"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gvr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let veh_id = seed_vehicle(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/vehicles/{veh_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert_eq!(
        resp.json_value()["registration"]["id"].as_str().unwrap(),
        veh_id.to_string()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "uvr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let veh_id = seed_vehicle(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/vehicles/{veh_id}"))
                .json(json!({"make": "Honda"}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "dvr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let veh_id = seed_vehicle(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/registry/vehicles/{veh_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_vehicle_registration_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rvr").await;
    let user_id = resolve_user_id(&pool, &user.email).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let veh_id = seed_vehicle(&pool, org_id, unit_id, user_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/registry/vehicles/{veh_id}/review"))
                .json(json!({"approve": true}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "cps").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .post("/api/v1/registry/parking-spots")
                .json(json!({
                    "building_id": building_id,
                    "spot_number": "B-01"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "body: {}", resp.text());
    assert!(resp.json_value()["spot"]["id"].is_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_parking_spots_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lps").await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(session.get("/api/v1/registry/parking-spots").build())
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert!(resp.json_value()["spots"].is_array());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "gps").await;
    let building_id = seed_building(&pool, org_id).await;
    let spot_id = seed_parking_spot(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    assert_eq!(
        resp.json_value()["spot"]["id"].as_str().unwrap(),
        spot_id.to_string()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "ups").await;
    let building_id = seed_building(&pool, org_id).await;
    let spot_id = seed_parking_spot(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .json(json!({"spot_number": "B-02"}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_parking_spot_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "dps").await;
    let building_id = seed_building(&pool, org_id).await;
    let spot_id = seed_parking_spot(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/registry/parking-spots/{spot_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_registry_rules_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "grr").await;
    let building_id = seed_building(&pool, org_id).await;
    seed_registry_rules(&pool, org_id, building_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/registry/buildings/{building_id}/rules"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_registry_rules_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "urr").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/registry/buildings/{building_id}/rules"))
                .json(json!({"max_pets_per_unit": 2, "allowed_pet_types": ["dog", "cat"]}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_registry_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "grs").await;
    let building_id = seed_building(&pool, org_id).await;
    let session = app.session(token, org_id);
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/registry/buildings/{building_id}/statistics"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
}

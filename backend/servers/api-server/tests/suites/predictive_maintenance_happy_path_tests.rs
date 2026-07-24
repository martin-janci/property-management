//! Predictive-maintenance happy-path integration tests — Wave 5 (BIT-339,
//! parent BIT-258).
//!
//! Exercises the success (`2xx`) path of every `/api/v1/predictive-maintenance/*`
//! endpoint over real HTTP. Prior coverage (`predictive_maintenance_photos_idor_tests`)
//! only asserted the cross-tenant photo-read rejection; this file fills the
//! equipment / maintenance-log / prediction / alert / threshold / dashboard
//! gaps so each handler has at least one green happy-path assertion.
//!
//! # Harness
//!
//! These handlers acquire an [`RlsConnection`] and derive the authoritative org
//! from `rls.tenant_id()` (the `X-Tenant-ID` tenant the caller was validated
//! against). `create_authenticated_user_with_org` registers + logs in a user and
//! makes them an active `org_admin` of a fresh org, so `ValidatedTenantExtractor`
//! resolves the tenant. Every request carries `.bearer(token)` + `.tenant(org_id)`.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

const BASE: &str = "/api/v1/predictive-maintenance";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'PM Happy Building', 'PM St 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// app + authenticated org_admin + a building in the org.
async fn setup(pool: PgPool, slug: &str) -> (TestApp, String, Uuid, Uuid) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, slug).await;
    let building_id = seed_building(&app.pool, org_id).await;
    (app, token, org_id, building_id)
}

/// Create equipment over HTTP and return its id.
async fn create_equipment(app: &TestApp, token: &str, org_id: Uuid, building_id: Uuid) -> Uuid {
    let r = app
        .post(&format!("{BASE}/equipment"))
        .bearer(token)
        .tenant(org_id)
        .json(json!({
            "building_id": building_id,
            "name": "Boiler #1",
            "equipment_type": "hvac",
            "category": "heating",
        }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create equipment: {}",
        resp.text()
    );
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id")).expect("uuid")
}

/// Create a maintenance log over HTTP and return its id.
async fn create_maintenance_log(
    app: &TestApp,
    token: &str,
    org_id: Uuid,
    equipment_id: Uuid,
) -> Uuid {
    let r = app
        .post(&format!("{BASE}/maintenance-logs"))
        .bearer(token)
        .tenant(org_id)
        .json(json!({
            "equipment_id": equipment_id,
            "maintenance_type": "preventive",
            "description": "Routine service",
        }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create maintenance log: {}",
        resp.text()
    );
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id")).expect("uuid")
}

/// Seed a `maintenance_alerts` row directly (no public create endpoint) and
/// return its id. References `equipment_registry` + `organizations`.
async fn seed_alert(pool: &PgPool, org_id: Uuid, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO maintenance_alerts
               (equipment_id, organization_id, alert_type, severity, title, message, status)
           VALUES ($1, $2, 'health_threshold', 'high', 'Low health', 'Health below threshold', 'active')
           RETURNING id"#,
    )
    .bind(equipment_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed alert")
}

// ===========================================================================
// Equipment registry (Story 134.1)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_equipment_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "eq-create").await;
    let id = create_equipment(&app, &token, org_id, building_id).await;
    assert!(!id.is_nil());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_equipment_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "eq-list").await;
    create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/equipment"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list equipment: {}",
        resp.text()
    );
    assert!(resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_equipment_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "eq-get").await;
    let id = create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/equipment/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get equipment: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["id"].as_str(),
        Some(id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_equipment_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "eq-update").await;
    let id = create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .put(&format!("{BASE}/equipment/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "name": "Boiler #1 (renamed)" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update equipment: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["name"].as_str(),
        Some("Boiler #1 (renamed)")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_equipment_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "eq-delete").await;
    let id = create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .delete(&format!("{BASE}/equipment/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete equipment: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_and_list_equipment_documents_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "eq-docs").await;
    let id = create_equipment(&app, &token, org_id, building_id).await;

    let add = app
        .post(&format!("{BASE}/equipment/{id}/documents"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({
            "document_type": "manual",
            "title": "Operating manual",
            "file_path": "s3://docs/manual.pdf",
        }))
        .build();
    let resp = app.execute(add).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add document: {}",
        resp.text()
    );

    let list = app
        .get(&format!("{BASE}/equipment/{id}/documents"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(list).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list documents: {}",
        resp.text()
    );
    assert!(resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

// ===========================================================================
// Maintenance logs (Story 134.2)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_maintenance_log_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "log-create").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    let id = create_maintenance_log(&app, &token, org_id, equipment_id).await;
    assert!(!id.is_nil());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_maintenance_log_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "log-get").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    let id = create_maintenance_log(&app, &token, org_id, equipment_id).await;

    let r = app
        .get(&format!("{BASE}/maintenance-logs/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "get log: {}", resp.text());
    assert_eq!(
        resp.json_value()["id"].as_str(),
        Some(id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_maintenance_log_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "log-update").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    let id = create_maintenance_log(&app, &token, org_id, equipment_id).await;

    let r = app
        .put(&format!("{BASE}/maintenance-logs/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "description": "Updated work performed", "outcome": "completed" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "update log: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_equipment_maintenance_logs_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "log-list").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    create_maintenance_log(&app, &token, org_id, equipment_id).await;

    let r = app
        .get(&format!("{BASE}/equipment/{equipment_id}/maintenance-logs"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "list logs: {}", resp.text());
    assert!(resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_and_list_maintenance_photos_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "log-photos").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    let log_id = create_maintenance_log(&app, &token, org_id, equipment_id).await;

    let add = app
        .post(&format!("{BASE}/maintenance-logs/{log_id}/photos"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({
            "file_path": "s3://photos/before.jpg",
            "caption": "Before",
            "photo_type": "before",
        }))
        .build();
    let resp = app.execute(add).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add photo: {}",
        resp.text()
    );

    let list = app
        .get(&format!("{BASE}/maintenance-logs/{log_id}/photos"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(list).await;
    assert_eq!(resp.status, StatusCode::OK, "list photos: {}", resp.text());
    assert!(resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

// ===========================================================================
// Predictions (Story 134.3)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn run_prediction_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "pred-run").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("{BASE}/predictions/run"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "equipment_ids": [equipment_id] }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "run prediction: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn run_batch_predictions_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "pred-batch").await;
    create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("{BASE}/predictions/batch"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "building_id": building_id }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "batch predictions: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_equipment_predictions_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "pred-hist").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/equipment/{equipment_id}/predictions"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "prediction history: {}",
        resp.text()
    );
}

// ===========================================================================
// Alerts
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_alerts_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "alert-list").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    seed_alert(&app.pool, org_id, equipment_id).await;

    let r = app
        .get(&format!("{BASE}/alerts"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "list alerts: {}", resp.text());
    assert!(resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_alert_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "alert-ack").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    let alert_id = seed_alert(&app.pool, org_id, equipment_id).await;

    let r = app
        .post(&format!("{BASE}/alerts/{alert_id}/acknowledge"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "notes": "Investigating" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "acknowledge: {}", resp.text());
    assert_eq!(resp.json_value()["status"].as_str(), Some("acknowledged"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_alert_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "alert-resolve").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    let alert_id = seed_alert(&app.pool, org_id, equipment_id).await;

    let r = app
        .post(&format!("{BASE}/alerts/{alert_id}/resolve"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "notes": "Replaced part" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "resolve: {}", resp.text());
    assert_eq!(resp.json_value()["status"].as_str(), Some("resolved"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dismiss_alert_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "alert-dismiss").await;
    let equipment_id = create_equipment(&app, &token, org_id, building_id).await;
    let alert_id = seed_alert(&app.pool, org_id, equipment_id).await;

    let r = app
        .post(&format!("{BASE}/alerts/{alert_id}/dismiss"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "dismiss: {}", resp.text());
    assert_eq!(resp.json_value()["status"].as_str(), Some("dismissed"));
}

// ===========================================================================
// Health thresholds
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn set_and_list_health_thresholds_succeeds(pool: PgPool) {
    let (app, token, org_id, _building_id) = setup(pool, "thresholds").await;

    let set = app
        .post(&format!("{BASE}/thresholds"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({
            "equipment_type": "hvac",
            "critical_threshold": 30,
            "warning_threshold": 60,
        }))
        .build();
    let resp = app.execute(set).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "set threshold: {}",
        resp.text()
    );

    let list = app
        .get(&format!("{BASE}/thresholds"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(list).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list thresholds: {}",
        resp.text()
    );
    assert!(resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

// ===========================================================================
// Dashboard (Story 134.4)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_dashboard_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "dash").await;
    create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/dashboard"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "dashboard: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_equipment_by_health_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "by-health").await;
    create_equipment(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/equipment/by-health"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "by-health: {}", resp.text());
}

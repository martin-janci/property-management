//! Work-orders & maintenance-schedule happy-path integration tests — Wave 5
//! (BIT-339, parent BIT-258).
//!
//! These exercise the success (`2xx`) path of every `/api/v1/work-orders/*`
//! endpoint over real HTTP. Prior coverage (`work_order_cross_org_idor_tests`)
//! only asserted the cross-tenant rejection path plus two same-org reads; this
//! file fills the create / mutate / list / schedule gaps so each handler has at
//! least one green happy-path assertion.
//!
//! # Harness
//!
//! The work-order routes acquire an [`RlsConnection`], which resolves the
//! caller's org via `ValidatedTenantExtractor` from the `X-Tenant-ID` header,
//! and additionally run `verify_org_access` against `organization_members`.
//! `create_authenticated_user_with_org` registers + logs in a user and makes
//! them an active `org_admin` of a fresh org, satisfying both gates. Every
//! request therefore carries `.bearer(token)` + `.tenant(org_id)`. The
//! client-supplied `organization_id` (create/list/query) is set to the caller's
//! own org so `verify_org_access` passes.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

const BASE: &str = "/api/v1/work-orders";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'WO Happy Building', 'WO St 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO equipment (organization_id, building_id, name, category)
           VALUES ($1, $2, 'WO Happy HVAC', 'hvac')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

/// Boilerplate: app + authenticated org_admin + a building in the org.
async fn setup(pool: PgPool, slug: &str) -> (TestApp, String, Uuid, Uuid) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, slug).await;
    let building_id = seed_building(&app.pool, org_id).await;
    (app, token, org_id, building_id)
}

/// Create a work order over HTTP and return its id.
async fn create_work_order(app: &TestApp, token: &str, org_id: Uuid, building_id: Uuid) -> Uuid {
    let r = app
        .post(BASE)
        .bearer(token)
        .tenant(org_id)
        .json(json!({
            "organization_id": org_id,
            "building_id": building_id,
            "title": "Happy work order",
            "description": "Created by happy-path test",
            "priority": "medium",
            "work_type": "corrective",
        }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create work order: {}",
        resp.text()
    );
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id")).expect("uuid")
}

/// Create a maintenance schedule over HTTP and return its id.
async fn create_schedule(app: &TestApp, token: &str, org_id: Uuid, building_id: Uuid) -> Uuid {
    let r = app
        .post(&format!("{BASE}/schedules"))
        .bearer(token)
        .tenant(org_id)
        .json(json!({
            "organization_id": org_id,
            "building_id": building_id,
            "name": "Quarterly inspection",
            "description": "Happy-path schedule",
            "work_type": "preventive",
            "frequency": "monthly",
            "start_date": "2026-01-01",
            "auto_create_work_order": true,
        }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create schedule: {}",
        resp.text()
    );
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id")).expect("uuid")
}

// ===========================================================================
// Work Orders (Story 20.2)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_work_order_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-create").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;
    assert!(!id.is_nil());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_work_orders_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-list").await;
    create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}?organization_id={org_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "list: {}", resp.text());
    assert!(
        resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "list must contain the created work order"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_work_orders_with_details_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-details").await;
    create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/with-details?organization_id={org_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "with-details: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_statistics_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-stats").await;
    create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/statistics?organization_id={org_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "statistics: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_overdue_succeeds(pool: PgPool) {
    let (app, token, org_id, _building_id) = setup(pool, "wo-overdue").await;

    let r = app
        .get(&format!("{BASE}/overdue?organization_id={org_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "overdue: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_work_order_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-get").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "get: {}", resp.text());
    assert_eq!(
        resp.json_value()["id"].as_str(),
        Some(id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_work_order_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-update").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .patch(&format!("{BASE}/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "title": "Updated title", "priority": "high" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "update: {}", resp.text());
    assert_eq!(resp.json_value()["title"].as_str(), Some("Updated title"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn assign_work_order_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-assign").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;
    // Resolve the caller's own user id to use as the assignee.
    let assignee: Uuid =
        sqlx::query_scalar("SELECT user_id FROM organization_members WHERE organization_id = $1")
            .bind(org_id)
            .fetch_one(&app.pool)
            .await
            .expect("assignee id");

    let r = app
        .post(&format!("{BASE}/{id}/assign"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "assigned_to": assignee }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "assign: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_work_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-start").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/start"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "start: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_work_order_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-complete").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/complete"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "actual_cost": "120.50", "resolution_notes": "Fixed" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "complete: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn put_on_hold_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-hold").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("{BASE}/{id}/hold"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "reason": "Awaiting parts" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "hold: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_and_list_comments_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-comments").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;

    let add = app
        .post(&format!("{BASE}/{id}/comments"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "content": "Inspected on site" }))
        .build();
    let resp = app.execute(add).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add comment: {}",
        resp.text()
    );

    let list = app
        .get(&format!("{BASE}/{id}/comments"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(list).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list comments: {}",
        resp.text()
    );
    assert!(
        resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "comment list must contain the added comment"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_work_order_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "wo-delete").await;
    let id = create_work_order(&app, &token, org_id, building_id).await;

    let r = app
        .delete(&format!("{BASE}/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete: {}",
        resp.text()
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM work_orders WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "work order must be gone after delete");
}

// ===========================================================================
// Maintenance Schedules (Story 20.3)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_schedule_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-create").await;
    let id = create_schedule(&app, &token, org_id, building_id).await;
    assert!(!id.is_nil());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_schedules_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-list").await;
    create_schedule(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/schedules?organization_id={org_id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list schedules: {}",
        resp.text()
    );
    assert!(resp.json_value().as_array().map(|a| a.len()).unwrap_or(0) >= 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_upcoming_schedules_succeeds(pool: PgPool) {
    let (app, token, org_id, _building_id) = setup(pool, "sch-upcoming").await;

    let r = app
        .get(&format!(
            "{BASE}/schedules/upcoming?organization_id={org_id}"
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "upcoming: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn process_due_schedules_succeeds(pool: PgPool) {
    let (app, token, org_id, _building_id) = setup(pool, "sch-process").await;

    let r = app
        .post(&format!(
            "{BASE}/schedules/process-due?organization_id={org_id}"
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "process-due: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_schedule_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-get").await;
    let id = create_schedule(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/schedules/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "get schedule: {}", resp.text());
    assert_eq!(
        resp.json_value()["id"].as_str(),
        Some(id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_schedule_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-update").await;
    let id = create_schedule(&app, &token, org_id, building_id).await;

    let r = app
        .patch(&format!("{BASE}/schedules/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "name": "Renamed schedule" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update schedule: {}",
        resp.text()
    );
    assert_eq!(resp.json_value()["name"].as_str(), Some("Renamed schedule"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn activate_deactivate_schedule_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-toggle").await;
    let id = create_schedule(&app, &token, org_id, building_id).await;

    let deact = app
        .post(&format!("{BASE}/schedules/{id}/deactivate"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(deact).await;
    assert_eq!(resp.status, StatusCode::OK, "deactivate: {}", resp.text());
    assert_eq!(resp.json_value()["is_active"].as_bool(), Some(false));

    let act = app
        .post(&format!("{BASE}/schedules/{id}/activate"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(act).await;
    assert_eq!(resp.status, StatusCode::OK, "activate: {}", resp.text());
    assert_eq!(resp.json_value()["is_active"].as_bool(), Some(true));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn skip_schedule_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-skip").await;
    let id = create_schedule(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("{BASE}/schedules/{id}/skip"))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "reason": "Closed for holidays" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "skip: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_executions_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-exec").await;
    let id = create_schedule(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/schedules/{id}/executions"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "executions: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_schedule_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "sch-delete").await;
    let id = create_schedule(&app, &token, org_id, building_id).await;

    let r = app
        .delete(&format!("{BASE}/schedules/{id}"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete schedule: {}",
        resp.text()
    );
}

// ===========================================================================
// Service History (Story 20.4) + cost summary
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn equipment_service_history_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "svc-eq").await;
    let equipment_id = seed_equipment(&app.pool, org_id, building_id).await;

    let r = app
        .get(&format!("{BASE}/equipment/{equipment_id}/service-history"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "equipment service history: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_service_history_succeeds(pool: PgPool) {
    let (app, token, org_id, building_id) = setup(pool, "svc-bld").await;

    let r = app
        .get(&format!("{BASE}/buildings/{building_id}/service-history"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "building service history: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_cost_summary_succeeds(pool: PgPool) {
    let (app, token, org_id, _building_id) = setup(pool, "cost").await;

    let r = app
        .get(&format!(
            "{BASE}/cost-summary?organization_id={org_id}&start_date=2026-01-01&end_date=2026-12-31"
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "cost summary: {}", resp.text());
}

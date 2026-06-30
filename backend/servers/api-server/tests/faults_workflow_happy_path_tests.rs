//! Faults workflow + attachments happy-path (2xx) integration tests —
//! Wave 8 batch 2 (BIT-405, parent BIT-258).
//!
//! Wave 3 (`faults_behaviour_tests.rs`, BIT-267) covered the fault
//! lifecycle reads/transitions (get, update, status, resolve, confirm,
//! reopen, comments, work-notes, statistics, list_my). This file is the
//! additive batch-2 half: the remaining partial fault endpoints that had
//! no success-path HTTP assertion:
//!
//!   list_faults (GET /), triage_fault, assign_fault,
//!   list_attachments, add_attachment, delete_attachment, get_ai_suggestion.
//!
//! Harness mirrors `faults_behaviour_tests.rs`:
//!   * `create_authenticated_user_with_org` provisions an org-scoped
//!     principal (RLS tenant context via `organization_members`).
//!   * a `user_memberships` row with role `OrgAdmin` is seeded so the
//!     manager-gated workflow actions (`require_manager` →
//!     `is_manager_in_org`, which reads `user_memberships`) pass.
//!   * faults are created over HTTP; the table default status is `new`,
//!     which `triage_fault` requires.
//!
//! These tests change no production code — they assert the shipped
//! behaviour against an empty (migrated) DB.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Shared seed helpers
// ---------------------------------------------------------------------------

/// Resolve the user id for the authenticated test user.
async fn user_id_for(app: &TestApp, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&app.pool)
        .await
        .expect("user id")
}

/// Seed an `OrgAdmin` row in `user_memberships` — the table
/// `is_manager_in_org` (and therefore `require_manager`) reads. The common
/// harness seeds `organization_members` for RLS, which is a *different*
/// table, so manager-gated actions need this extra row.
async fn seed_manager_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, 'OrgAdmin') ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed manager membership");
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'Fault Workflow Building', 'Fault St 2', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Create a fault via API and return its id. Default status is `new`.
async fn create_fault(app: &TestApp, token: &str, org_id: Uuid, building_id: Uuid) -> Uuid {
    let payload = json!({
        "building_id": building_id,
        "unit_id": null,
        "title": "Leaking water pipe in basement",
        "description": "Water is leaking from a burst pipe near the drain.",
        "location_description": "Basement",
        "category": "plumbing",
        "priority": "medium",
        "idempotency_key": null
    });
    let r = app
        .post("/api/v1/faults")
        .bearer(token)
        .tenant(org_id)
        .json(&payload)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create fault: {}",
        resp.text()
    );
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id")).expect("uuid")
}

/// Add an attachment over HTTP and return its id.
async fn add_attachment(app: &TestApp, token: &str, org_id: Uuid, fault_id: Uuid) -> Uuid {
    let r = app
        .post(&format!("/api/v1/faults/{}/attachments", fault_id))
        .bearer(token)
        .tenant(org_id)
        .json(json!({
            "filename": "leak.jpg",
            "original_filename": "leak.jpg",
            "content_type": "image/jpeg",
            "size_bytes": 1024,
            "storage_url": "https://storage.example.com/leak.jpg",
            "thumbnail_url": null,
            "description": "Photo of the leak",
            "width": 800,
            "height": 600
        }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add attachment: {}",
        resp.text()
    );
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("attachment id")).expect("uuid")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_list_faults_returns_created_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lstflts").await;
    let building_id = seed_building(&app.pool, org_id).await;
    create_fault(&app, &token, org_id, building_id).await;

    let r = app
        .get("/api/v1/faults")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "list faults: {}", resp.text());
    let body = resp.json_value();
    assert!(
        body["count"].as_u64().map(|c| c >= 1).unwrap_or(false),
        "expected count >= 1, got: {}",
        body
    );
    assert!(
        body["faults"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "expected at least one fault in list"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_triage_new_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "triflt").await;
    let user_id = user_id_for(&app, &user.email).await;
    seed_manager_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = create_fault(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("/api/v1/faults/{}/triage", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "priority": "high", "category": "plumbing" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "triage fault: {}", resp.text());
    assert_eq!(
        resp.json_value()["fault"]["status"].as_str(),
        Some("triaged")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_assign_fault_to_self(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "asgflt").await;
    let user_id = user_id_for(&app, &user.email).await;
    seed_manager_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = create_fault(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("/api/v1/faults/{}/assign", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "assigned_to": user_id }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "assign fault: {}", resp.text());
    assert_eq!(
        resp.json_value()["fault"]["assigned_to"].as_str(),
        Some(user_id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_list_attachments_empty(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lstatt").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = create_fault(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("/api/v1/faults/{}/attachments", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list attachments: {}",
        resp.text()
    );
    assert!(
        resp.json_value()["attachments"].is_array(),
        "expected attachments array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_add_attachment(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "addatt").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = create_fault(&app, &token, org_id, building_id).await;

    let attachment_id = add_attachment(&app, &token, org_id, fault_id).await;

    // The new attachment shows up in the list.
    let r = app
        .get(&format!("/api/v1/faults/{}/attachments", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "list: {}", resp.text());
    let body = resp.json_value();
    let attachments = body["attachments"].as_array().expect("attachments array");
    let found = attachments
        .iter()
        .any(|att| att["id"].as_str() == Some(attachment_id.to_string().as_str()));
    assert!(found, "added attachment should appear in the list");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_delete_attachment(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "delatt").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = create_fault(&app, &token, org_id, building_id).await;
    let attachment_id = add_attachment(&app, &token, org_id, fault_id).await;

    let r = app
        .delete(&format!(
            "/api/v1/faults/{}/attachments/{}",
            fault_id, attachment_id
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete attachment: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn test_get_ai_suggestion(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "aisug").await;
    let user_id = user_id_for(&app, &user.email).await;
    seed_manager_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = create_fault(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("/api/v1/faults/{}/suggest", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "ai suggestion: {}",
        resp.text()
    );
    assert!(
        resp.json_value()["suggestion"].is_object(),
        "expected a suggestion object"
    );
}

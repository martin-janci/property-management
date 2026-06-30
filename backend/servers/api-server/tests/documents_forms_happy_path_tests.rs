//! BIT-358 wave-7: forms happy-path tests (documents-forms group).
//!
//! Wave 4 covered document core CRUD / versions / shares / intelligence /
//! templates (see `documents_core_crud_tests.rs`,
//! `documents_intelligence_templates_tests.rs`, `document_folder_tests.rs`,
//! `document_upload_tests.rs`, `document_download_preview_tests.rs`,
//! `document_share_access_tests.rs`). The remaining untested surface in the
//! documents-forms group is the **forms** subsystem (Epic 54): form CRUD,
//! fields, submissions, and download tracking.
//!
//! `form_cross_org_idor_tests.rs` already pins the cross-org isolation contract
//! for submit/download (and a same-org submit 201). This file adds the
//! happy-path `2xx` coverage for every other forms endpoint, with NO overlap:
//!
//! - POST   /api/v1/forms                                  (create_form)        201
//! - GET    /api/v1/forms                                  (list_forms)         200
//! - GET    /api/v1/forms/available                        (list_available)     200
//! - GET    /api/v1/forms/statistics                       (get_statistics)     200
//! - GET    /api/v1/forms/{id}                             (get_form)           200
//! - PUT    /api/v1/forms/{id}                             (update_form)        200
//! - DELETE /api/v1/forms/{id}                             (delete_form)        204
//! - POST   /api/v1/forms/{id}/publish                     (publish_form)       200
//! - POST   /api/v1/forms/{id}/archive                     (archive_form)       200
//! - GET    /api/v1/forms/{id}/fields                      (list_fields)        200
//! - POST   /api/v1/forms/{id}/fields                      (add_field)          201
//! - PUT    /api/v1/forms/{id}/fields/{field_id}           (update_field)       200
//! - DELETE /api/v1/forms/{id}/fields/{field_id}           (delete_field)       204
//! - POST   /api/v1/forms/{id}/fields/reorder              (reorder_fields)     200
//! - GET    /api/v1/forms/{id}/submissions                 (list_submissions)   200
//! - GET    /api/v1/forms/{id}/submissions/{submission_id} (get_submission)     200
//! - POST   /api/v1/forms/{id}/submissions/{submission_id}/review (review)      200
//! - POST   /api/v1/forms/{id}/download                    (record_download)    200
//!
//! Setup is API-first (drive the real handlers) rather than raw-SQL seeding, so
//! these tests stay resilient to schema drift in the seed layer.
//!
//! The org creator from `create_authenticated_user_with_org` is an `org_admin`,
//! which `TenantRole::is_manager()` treats as manager-level — so it clears the
//! manager gate on the management endpoints.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// API-first setup helpers
// ---------------------------------------------------------------------------

/// Create a draft form (with a single non-required text field) via the API and
/// return its id. The field lets the form be published later.
async fn create_draft_form(app: &TestApp, token: &str, org_id: Uuid, title: &str) -> Uuid {
    let res = app
        .execute(
            app.session(token.to_string(), org_id)
                .post("/api/v1/forms")
                .json(json!({
                    "title": title,
                    "description": "Wave-7 happy-path form",
                    "fields": [
                        {
                            "field_key": "q1",
                            "label": "Question 1",
                            "field_type": "text"
                        }
                    ]
                }))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::CREATED);
    res.json_value()["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("created form id")
}

/// Fetch the id of the form's first field via the list-fields endpoint.
async fn first_field_id(app: &TestApp, token: &str, org_id: Uuid, form_id: Uuid) -> Uuid {
    let res = app
        .execute(
            app.session(token.to_string(), org_id)
                .get(&format!("/api/v1/forms/{form_id}/fields"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    res.json_value()["fields"][0]["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("first field id")
}

/// Publish a draft form via the API (form must already have >=1 field).
async fn publish(app: &TestApp, token: &str, org_id: Uuid, form_id: Uuid) {
    let res = app
        .execute(
            app.session(token.to_string(), org_id)
                .post(&format!("/api/v1/forms/{form_id}/publish"))
                .build(),
        )
        .await;
    res.assert_status(StatusCode::OK);
}

/// Submit a published form via the API and return the submission id.
async fn submit(app: &TestApp, token: &str, org_id: Uuid, form_id: Uuid) -> Uuid {
    let res = app
        .execute(
            app.session(token.to_string(), org_id)
                .post(&format!("/api/v1/forms/{form_id}/submit"))
                .json(json!({ "data": { "q1": "answer" } }))
                .build(),
        )
        .await;
    res.assert_status(StatusCode::CREATED);
    res.json_value()["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("submission id")
}

// ---------------------------------------------------------------------------
// Form CRUD
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn create_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-create").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/forms")
                .json(json!({
                    "title": "Resident Intake",
                    "fields": [
                        { "field_key": "name", "label": "Name", "field_type": "text" }
                    ]
                }))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::CREATED);
    res.assert_json_field("id");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn list_forms_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-list").await;
    let _ = create_draft_form(&app, &token, org_id, "Listed form").await;

    let res = app
        .execute(app.session(token, org_id).get("/api/v1/forms").build())
        .await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert!(
        body["forms"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one form"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn list_available_forms_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-avail").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/forms/available")
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert!(body["forms"].is_array(), "expected forms array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-stats").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/forms/statistics")
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_json_field("statistics");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-get").await;
    let form_id = create_draft_form(&app, &token, org_id, "Fetched form").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    // `FormWithDetails` flattens the inner `Form`, so `id` sits directly under
    // the `form` key alongside the `fields` array.
    let body = res.json_value();
    assert_eq!(
        body["form"]["id"].as_str(),
        Some(form_id.to_string().as_str()),
        "body: {}",
        res.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn update_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-update").await;
    // update_form only accepts draft forms; a freshly created form is a draft.
    let form_id = create_draft_form(&app, &token, org_id, "Editable form").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .put(&format!("/api/v1/forms/{form_id}"))
                .json(json!({ "title": "Renamed form" }))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert_eq!(body["form"]["title"].as_str(), Some("Renamed form"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn delete_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-delete").await;
    let form_id = create_draft_form(&app, &token, org_id, "Disposable form").await;

    let res = app
        .execute(
            app.session(token.clone(), org_id)
                .delete(&format!("/api/v1/forms/{form_id}"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);

    // Confirm the form is gone (soft-deleted) from the manager's view.
    let res2 = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}"))
                .build(),
        )
        .await;
    res2.assert_status(StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn publish_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-publish").await;
    let form_id = create_draft_form(&app, &token, org_id, "Publishable form").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/publish"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_json_field("form");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn archive_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-archive").await;
    let form_id = create_draft_form(&app, &token, org_id, "Archivable form").await;
    publish(&app, &token, org_id, form_id).await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/archive"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_json_field("form");
}

// ---------------------------------------------------------------------------
// Form fields
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn list_fields_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "field-list").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form with fields").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}/fields"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert!(
        body["fields"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected the seeded field"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn add_field_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "field-add").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form gaining a field").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/fields"))
                .json(json!({
                    "field_key": "email",
                    "label": "Email",
                    "field_type": "text",
                    "field_order": 1
                }))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::CREATED);
    res.assert_json_field("field");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn update_field_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "field-update").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form with editable field").await;
    let field_id = first_field_id(&app, &token, org_id, form_id).await;

    let res = app
        .execute(
            app.session(token, org_id)
                .put(&format!("/api/v1/forms/{form_id}/fields/{field_id}"))
                .json(json!({ "label": "Renamed field" }))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert_eq!(body["field"]["label"].as_str(), Some("Renamed field"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn delete_field_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "field-delete").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form losing a field").await;
    let field_id = first_field_id(&app, &token, org_id, form_id).await;

    let res = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/forms/{form_id}/fields/{field_id}"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn reorder_fields_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "field-reorder").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form with reorderable fields").await;
    let field_id = first_field_id(&app, &token, org_id, form_id).await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/fields/reorder"))
                .json(json!({
                    "field_orders": [
                        { "field_id": field_id, "order": 0 }
                    ]
                }))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Submissions
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn list_submissions_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sub-list").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form with submissions").await;
    publish(&app, &token, org_id, form_id).await;
    let _ = submit(&app, &token, org_id, form_id).await;

    let res = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}/submissions"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert!(
        body["submissions"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one submission"
    );
}

#[ignore]
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_submission_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sub-get").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form for one submission").await;
    publish(&app, &token, org_id, form_id).await;
    let submission_id = submit(&app, &token, org_id, form_id).await;

    let res = app
        .execute(
            app.session(token, org_id)
                .get(&format!(
                    "/api/v1/forms/{form_id}/submissions/{submission_id}"
                ))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_json_field("submission");
}

#[ignore]
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn review_submission_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "sub-review").await;
    let form_id = create_draft_form(&app, &token, org_id, "Form for review").await;
    publish(&app, &token, org_id, form_id).await;
    let submission_id = submit(&app, &token, org_id, form_id).await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/forms/{form_id}/submissions/{submission_id}/review"
                ))
                .json(json!({ "status": "approved", "review_notes": "Looks good" }))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
    res.assert_json_field("submission");
}

// ---------------------------------------------------------------------------
// Download tracking
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn record_download_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-download").await;
    let form_id = create_draft_form(&app, &token, org_id, "Downloadable form").await;

    let res = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/download"))
                .build(),
        )
        .await;

    res.assert_status(StatusCode::OK);
}

//! BIT-268 wave-4 batch 5: lease abstraction and forms happy-path tests.
//!
//! Covers partial endpoints from docs/endpoint-checklist/groups/documents-forms.md:
//!
//! lease_abstraction.rs  (mount: /api/v1/lease-abstraction)
//! - GET   /api/v1/lease-abstraction/documents                          (list_documents)
//! - POST  /api/v1/lease-abstraction/documents                          (upload_document)
//! - GET   /api/v1/lease-abstraction/documents/{id}                     (get_document)
//! - DELETE /api/v1/lease-abstraction/documents/{id}                    (delete_document)
//! - POST  /api/v1/lease-abstraction/documents/{id}/process             (process_document)
//! - GET   /api/v1/lease-abstraction/documents/{id}/extractions         (list_extractions)
//! - GET   /api/v1/lease-abstraction/extractions/{id}                   (get_extraction)
//! - GET   /api/v1/lease-abstraction/extractions/{id}/fields            (get_extraction_fields)
//! - POST  /api/v1/lease-abstraction/extractions/{id}/approve           (approve_extraction)
//! - POST  /api/v1/lease-abstraction/extractions/{id}/reject            (reject_extraction)
//! - GET   /api/v1/lease-abstraction/extractions/{id}/corrections       (list_corrections)
//! - POST  /api/v1/lease-abstraction/extractions/{id}/corrections       (add_correction)
//! - GET   /api/v1/lease-abstraction/imports                            (list_imports)
//!
//! forms/crud.rs  (mount: /api/v1/forms)
//! - POST  /api/v1/forms                     (create_form)
//! - GET   /api/v1/forms                     (list_forms)
//! - GET   /api/v1/forms/available           (list_available_forms)
//! - GET   /api/v1/forms/statistics          (get_statistics)
//! - GET   /api/v1/forms/{id}                (get_form)
//! - PUT   /api/v1/forms/{id}                (update_form)
//! - DELETE /api/v1/forms/{id}               (delete_form)
//! - POST  /api/v1/forms/{id}/publish        (publish_form)
//! - POST  /api/v1/forms/{id}/archive        (archive_form)
//!
//! forms/fields.rs  (mount: /api/v1/forms)
//! - GET   /api/v1/forms/{id}/fields                     (list_fields)
//! - POST  /api/v1/forms/{id}/fields                     (add_field)
//! - POST  /api/v1/forms/{id}/fields/reorder             (reorder_fields)
//! - PUT   /api/v1/forms/{id}/fields/{field_id}          (update_field)
//! - DELETE /api/v1/forms/{id}/fields/{field_id}         (delete_field)
//!
//! forms/submissions.rs  (mount: /api/v1/forms)
//! - GET   /api/v1/forms/{id}/submissions                              (list_submissions)
//! - GET   /api/v1/forms/{id}/submissions/{submission_id}              (get_submission)
//! - POST  /api/v1/forms/{id}/submissions/{submission_id}/review       (review_submission)
//! - POST  /api/v1/forms/{id}/download                                 (record_download)

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

/// Seed a lease_document row directly (avoids S3 upload in tests).
async fn seed_lease_document(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lease_documents \
         (organization_id, uploaded_by, file_name, file_size_bytes, mime_type, storage_path, status) \
         VALUES ($1, $2, 'lease.pdf', 102400, 'application/pdf', 'abstractions/test/lease.pdf', 'completed') \
         RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed lease_document")
}

/// Seed a lease_extraction row in pending review status.
async fn seed_lease_extraction(pool: &PgPool, document_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lease_extractions \
         (document_id, fields_extracted, fields_flagged, review_status) \
         VALUES ($1, 3, 0, 'pending') \
         RETURNING id",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .expect("seed lease_extraction")
}

// ---------------------------------------------------------------------------
// Lease Abstraction — documents
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_lease_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-upload").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/lease-abstraction/documents")
                .json(json!({
                    "file_name": "lease_2026.pdf",
                    "file_size_bytes": 204800,
                    "mime_type": "application/pdf",
                    "storage_path": "abstractions/test/lease_2026.pdf"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    resp.assert_json_field("id");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_lease_documents_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-list-docs").await;
    let user_id = user_id_for(&pool, &user.email).await;
    seed_lease_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/lease-abstraction/documents")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["documents"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one lease document"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_lease_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-get-doc").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/lease-abstraction/documents/{doc_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["id"].as_str(), Some(doc_id.to_string().as_str()));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_lease_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-del-doc").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/lease-abstraction/documents/{doc_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn process_lease_document_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-process").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/lease-abstraction/documents/{doc_id}/process"
                ))
                .json(json!({}))
                .build(),
        )
        .await;

    // 200 = processing started; 409 = already processing; 503 = LLM unavailable in test
    let status = resp.status;
    assert!(
        status == StatusCode::OK
            || status == StatusCode::CONFLICT
            || status == StatusCode::SERVICE_UNAVAILABLE,
        "expected 200/409/503, got {status}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_lease_extractions_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-list-ext").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    seed_lease_extraction(&pool, doc_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!(
                    "/api/v1/lease-abstraction/documents/{doc_id}/extractions"
                ))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["extractions"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one extraction"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_lease_extraction_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-get-ext").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/lease-abstraction/extractions/{ext_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["id"].as_str(), Some(ext_id.to_string().as_str()));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_extraction_fields_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-fields").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!(
                    "/api/v1/lease-abstraction/extractions/{ext_id}/fields"
                ))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["fields"].is_array(), "expected fields array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn approve_lease_extraction_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-approve").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/lease-abstraction/extractions/{ext_id}/approve"
                ))
                .json(json!({"corrections": []}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reject_lease_extraction_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-reject").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/lease-abstraction/extractions/{ext_id}/reject"
                ))
                .json(json!({"reason": "Extracted dates look incorrect"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_extraction_corrections_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-list-corr").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!(
                    "/api/v1/lease-abstraction/extractions/{ext_id}/corrections"
                ))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["corrections"].is_array(), "expected corrections array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_extraction_correction_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-add-corr").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_lease_document(&pool, org_id, user_id).await;
    let ext_id = seed_lease_extraction(&pool, doc_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/lease-abstraction/extractions/{ext_id}/corrections"
                ))
                .json(json!({
                    "field_name": "tenant_name",
                    "original_value": "J. Doe",
                    "corrected_value": "John Doe",
                    "correction_reason": "Expanded abbreviation"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_lease_imports_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "la-list-imports").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/lease-abstraction/imports")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["imports"].is_array(), "expected imports array");
}

// ---------------------------------------------------------------------------
// Forms — CRUD
// ---------------------------------------------------------------------------

/// Create a form with a single text field; returns form id.
async fn create_form(app: &TestApp, token: &str, org_id: Uuid) -> Uuid {
    let resp = app
        .execute(
            app.session(token.to_string(), org_id)
                .post("/api/v1/forms")
                .json(json!({
                    "title": "Tenant Survey",
                    "description": "Annual tenant satisfaction survey",
                    "require_signatures": false,
                    "allow_multiple_submissions": false,
                    "fields": [
                        {
                            "field_key": "satisfaction",
                            "label": "Overall Satisfaction",
                            "field_type": "rating",
                            "required": true,
                            "field_order": 1,
                            "width": "full"
                        }
                    ]
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    body["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("form id")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-create").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/forms")
                .json(json!({
                    "title": "Maintenance Request",
                    "require_signatures": false,
                    "allow_multiple_submissions": true,
                    "fields": []
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    resp.assert_json_field("id");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_forms_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-list").await;
    create_form(&app, &token, org_id).await;

    let resp = app
        .execute(app.session(token, org_id).get("/api/v1/forms").build())
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["forms"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one form"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_available_forms_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-available").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/forms/available")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["forms"].is_array(), "expected forms array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_form_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-stats").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/forms/statistics")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-get").await;
    let form_id = create_form(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["form"]["id"].as_str(),
        Some(form_id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-update").await;
    let form_id = create_form(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .put(&format!("/api/v1/forms/{form_id}"))
                .json(json!({"title": "Updated Tenant Survey"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-delete").await;
    let form_id = create_form(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/forms/{form_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-publish").await;
    let form_id = create_form(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/publish"))
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn archive_form_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-archive").await;
    let form_id = create_form(&app, &token, org_id).await;

    // Publish first, then archive (archive requires published state)
    app.execute(
        app.session(token.clone(), org_id)
            .post(&format!("/api/v1/forms/{form_id}/publish"))
            .json(json!({}))
            .build(),
    )
    .await
    .assert_status(StatusCode::OK);

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/archive"))
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Forms — Fields
// ---------------------------------------------------------------------------

/// Add a field to a form; returns field id.
async fn add_field(app: &TestApp, token: &str, org_id: Uuid, form_id: Uuid) -> Uuid {
    let resp = app
        .execute(
            app.session(token.to_string(), org_id)
                .post(&format!("/api/v1/forms/{form_id}/fields"))
                .json(json!({
                    "field_key": "comments",
                    "label": "Additional Comments",
                    "field_type": "textarea",
                    "required": false,
                    "field_order": 2,
                    "width": "full"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    body["field"]["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("field id")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_form_fields_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-list-fields").await;
    let form_id = create_form(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}/fields"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(body["fields"].is_array(), "expected fields array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_form_field_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-add-field").await;
    let form_id = create_form(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/fields"))
                .json(json!({
                    "field_key": "phone",
                    "label": "Phone Number",
                    "field_type": "text",
                    "required": false,
                    "field_order": 2,
                    "width": "half"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    resp.assert_json_field("field");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reorder_form_fields_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-reorder").await;
    let form_id = create_form(&app, &token, org_id).await;

    // Get the field created by create_form
    let fields_resp = app
        .execute(
            app.session(token.clone(), org_id)
                .get(&format!("/api/v1/forms/{form_id}/fields"))
                .build(),
        )
        .await;
    let fields_body = fields_resp.json_value();
    let field_id = fields_body["fields"][0]["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("field id for reorder");

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/fields/reorder"))
                .json(json!({
                    "field_orders": [
                        {"field_id": field_id, "order": 1}
                    ]
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_form_field_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-upd-field").await;
    let form_id = create_form(&app, &token, org_id).await;
    let field_id = add_field(&app, &token, org_id, form_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .put(&format!("/api/v1/forms/{form_id}/fields/{field_id}"))
                .json(json!({"label": "Extra Comments", "required": true}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_form_field_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-del-field").await;
    let form_id = create_form(&app, &token, org_id).await;
    let field_id = add_field(&app, &token, org_id, form_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/forms/{form_id}/fields/{field_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

// ---------------------------------------------------------------------------
// Forms — Submissions
// ---------------------------------------------------------------------------

/// Submit a published form; returns submission id.
async fn submit_form(app: &TestApp, token: &str, org_id: Uuid, form_id: Uuid) -> Uuid {
    let resp = app
        .execute(
            app.session(token.to_string(), org_id)
                .post(&format!("/api/v1/forms/{form_id}/submit"))
                .json(json!({
                    "data": {"satisfaction": 5}
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    body["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("submission id")
}

/// Publish form and submit it; returns (form_id, submission_id).
async fn create_published_form_with_submission(
    app: &TestApp,
    token: &str,
    org_id: Uuid,
) -> (Uuid, Uuid) {
    let form_id = create_form(app, token, org_id).await;

    app.execute(
        app.session(token.to_string(), org_id)
            .post(&format!("/api/v1/forms/{form_id}/publish"))
            .json(json!({}))
            .build(),
    )
    .await
    .assert_status(StatusCode::OK);

    let sub_id = submit_form(app, token, org_id, form_id).await;
    (form_id, sub_id)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_form_submissions_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-list-subs").await;
    let (form_id, _) = create_published_form_with_submission(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}/submissions"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["submissions"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one submission"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_form_submission_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-get-sub").await;
    let (form_id, sub_id) = create_published_form_with_submission(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/forms/{form_id}/submissions/{sub_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["submission"]["id"].as_str(),
        Some(sub_id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_form_submission_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-review-sub").await;
    let (form_id, sub_id) = create_published_form_with_submission(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/forms/{form_id}/submissions/{sub_id}/review"
                ))
                .json(json!({
                    "status": "approved",
                    "review_notes": "Looks good"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn record_form_download_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "form-download").await;
    let form_id = create_form(&app, &token, org_id).await;

    // Publish so the form is findable for download tracking
    app.execute(
        app.session(token.clone(), org_id)
            .post(&format!("/api/v1/forms/{form_id}/publish"))
            .json(json!({}))
            .build(),
    )
    .await
    .assert_status(StatusCode::OK);

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/forms/{form_id}/download"))
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

//! BIT-268 wave-4 batch 2: documents intelligence and document templates happy-path tests.
//!
//! Covers partial endpoints from docs/endpoint-checklist/groups/documents-forms.md:
//! - POST   /api/v1/documents/{id}/ocr/reprocess          (reprocess_ocr)
//! - POST   /api/v1/documents/search                       (search_documents)
//! - GET    /api/v1/documents/{id}/classification          (get_classification)
//! - POST   /api/v1/documents/{id}/classification/feedback (submit_classification_feedback)
//! - GET    /api/v1/documents/{id}/classification/history  (get_classification_history)
//! - POST   /api/v1/documents/{id}/summarize               (request_summarization)
//! - GET    /api/v1/documents/intelligence/stats            (get_intelligence_stats)
//!
//! Note: ai_summarize_document (POST /api/v1/documents/{id}/ai-summarize) is intentionally
//! NOT covered here — it performs live LLM inference + S3 text extraction, so it cannot
//! return a deterministic success path under `cargo test` without external credentials.
//! - POST   /api/v1/templates                              (create_template)
//! - GET    /api/v1/templates                              (list_templates)
//! - GET    /api/v1/templates/{id}                         (get_template)
//! - PUT    /api/v1/templates/{id}                         (update_template)
//! - DELETE /api/v1/templates/{id}                         (delete_template)
//! - POST   /api/v1/templates/{id}/generate               (generate_document)

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_document(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO documents \
         (organization_id, title, category, file_key, file_name, mime_type, size_bytes, created_by) \
         VALUES ($1, 'Intel Test Doc', 'contracts', 'intel/key.pdf', 'intel.pdf', 'application/pdf', 1024, $2) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

// ---------------------------------------------------------------------------
// Document Intelligence tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reprocess_ocr_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "intel-ocr").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let req = app
        .session(token, org_id)
        .post(&format!("/api/v1/documents/{doc_id}/ocr/reprocess"))
        .json(json!({}))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_documents_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "intel-search").await;

    let req = app
        .session(token, org_id)
        .post("/api/v1/documents/search")
        .json(json!({
            "query": "contract",
            "limit": 10
        }))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert!(body["results"].is_array(), "expected results array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_classification_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "intel-class").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let req = app
        .session(token, org_id)
        .get(&format!("/api/v1/documents/{doc_id}/classification"))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_classification_feedback_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "intel-feedback").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let req = app
        .session(token, org_id)
        .post(&format!(
            "/api/v1/documents/{doc_id}/classification/feedback"
        ))
        .json(json!({
            "accepted": true,
            "correct_category": null
        }))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_classification_history_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "intel-class-hist").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let req = app
        .session(token, org_id)
        .get(&format!(
            "/api/v1/documents/{doc_id}/classification/history"
        ))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert!(body["history"].is_array(), "expected history array");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_summarization_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "intel-summ").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let doc_id = seed_document(&pool, org_id, user_id).await;

    let req = app
        .session(token, org_id)
        .post(&format!("/api/v1/documents/{doc_id}/summarize"))
        .json(json!({}))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_intelligence_stats_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "intel-stats").await;

    let req = app
        .session(token, org_id)
        .get("/api/v1/documents/intelligence/stats")
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Document Templates tests
// ---------------------------------------------------------------------------

async fn create_template(app: &TestApp, token: &str, org_id: Uuid) -> Uuid {
    let req = app
        .session(token.to_string(), org_id)
        .post("/api/v1/templates")
        .json(json!({
            "name": "Lease Agreement Template",
            "description": "Standard lease template",
            "template_type": "lease",
            "content": "Dear {{tenant_name}}, your lease starts on {{start_date}}.",
            "placeholders": [
                {
                    "name": "tenant_name",
                    "type": "text",
                    "required": true,
                    "default_value": null,
                    "description": "Full tenant name"
                },
                {
                    "name": "start_date",
                    "type": "date",
                    "required": true,
                    "default_value": null,
                    "description": "Lease start date"
                }
            ]
        }))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::CREATED);
    let body = res.json_value();
    body["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("template id")
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "tpl-create").await;

    let req = app
        .session(token, org_id)
        .post("/api/v1/templates")
        .json(json!({
            "name": "Notice Template",
            "template_type": "notice",
            "content": "Notice: {{content}}",
            "placeholders": [
                {
                    "name": "content",
                    "type": "text",
                    "required": true,
                    "default_value": null,
                    "description": null
                }
            ]
        }))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::CREATED);
    res.assert_json_field("id");
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_templates_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "tpl-list").await;
    create_template(&app, &token, org_id).await;

    let req = app.session(token, org_id).get("/api/v1/templates").build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert!(
        body["templates"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one template"
    );
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "tpl-get").await;
    let tpl_id = create_template(&app, &token, org_id).await;

    let req = app
        .session(token, org_id)
        .get(&format!("/api/v1/templates/{tpl_id}"))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert_eq!(
        body["template"]["id"].as_str(),
        Some(tpl_id.to_string().as_str())
    );
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "tpl-update").await;
    let tpl_id = create_template(&app, &token, org_id).await;

    let req = app
        .session(token, org_id)
        .put(&format!("/api/v1/templates/{tpl_id}"))
        .json(json!({
            "name": "Updated Lease Agreement Template"
        }))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::OK);
    let body = res.json_value();
    assert_eq!(
        body["template"]["name"].as_str(),
        Some("Updated Lease Agreement Template")
    );
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "tpl-delete").await;
    let tpl_id = create_template(&app, &token, org_id).await;

    let req = app
        .session(token.clone(), org_id)
        .delete(&format!("/api/v1/templates/{tpl_id}"))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::NO_CONTENT);

    // Confirm deleted
    let req = app
        .session(token, org_id)
        .get(&format!("/api/v1/templates/{tpl_id}"))
        .build();
    let res2 = app.execute(req).await;
    res2.assert_status(StatusCode::NOT_FOUND);
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-571)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn generate_document_from_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "tpl-generate").await;
    let tpl_id = create_template(&app, &token, org_id).await;

    let req = app
        .session(token, org_id)
        .post(&format!("/api/v1/templates/{tpl_id}/generate"))
        .json(json!({
            "values": {
                "tenant_name": "John Doe",
                "start_date": "2026-01-01"
            },
            "title": "Generated Lease for John Doe",
            "category": "contracts"
        }))
        .build();
    let res = app.execute(req).await;

    res.assert_status(StatusCode::CREATED);
    res.assert_json_field("document_id");
}

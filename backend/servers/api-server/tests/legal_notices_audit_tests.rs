//! BIT-268 wave-4 batch 4: legal notices, compliance templates, and audit trail happy-path tests.
//!
//! Covers partial endpoints from docs/endpoint-checklist/groups/documents-forms.md:
//!
//! legal.rs — notices  (mount: /api/v1/legal)
//! - POST  /api/v1/legal/notices                                (create_notice)
//! - GET   /api/v1/legal/notices                                (list_notices)
//! - GET   /api/v1/legal/notices/with-recipients                (list_notices_with_recipients)
//! - GET   /api/v1/legal/notices/statistics                     (get_notice_statistics)
//! - GET   /api/v1/legal/notices/{id}                           (get_notice)
//! - PATCH /api/v1/legal/notices/{id}                           (update_notice)
//! - DELETE /api/v1/legal/notices/{id}                          (delete_notice)
//! - POST  /api/v1/legal/notices/{id}/send                      (send_notice)
//! - GET   /api/v1/legal/notices/{id}/recipients                (list_recipients)
//! - POST  /api/v1/legal/notices/{notice_id}/acknowledge/{rid}  (acknowledge_notice)
//!
//! legal.rs — compliance templates
//! - POST  /api/v1/legal/templates                              (create_template)
//! - GET   /api/v1/legal/templates                              (list_templates)
//! - GET   /api/v1/legal/templates/{id}                         (get_template)
//! - PATCH /api/v1/legal/templates/{id}                         (update_template)
//! - DELETE /api/v1/legal/templates/{id}                        (delete_template)
//! - POST  /api/v1/legal/templates/apply                        (apply_template)
//!
//! legal.rs — audit trail
//! - GET  /api/v1/legal/audit-trail                             (list_audit_trail)
//! - POST /api/v1/legal/audit-trail                             (create_audit_entry)

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

/// Create a notice with a single user recipient; returns (notice_id, recipient_id).
async fn create_notice_with_recipient(
    app: &TestApp,
    token: &str,
    org_id: Uuid,
    user_id: Uuid,
) -> (Uuid, Uuid) {
    let resp = app
        .execute(
            app.session(token.to_string(), org_id)
                .post("/api/v1/legal/notices")
                .json(json!({
                    "notice_type": "maintenance",
                    "subject": "Maintenance window this Saturday",
                    "content": "We will have scheduled maintenance from 9am-11am.",
                    "recipient_ids": [
                        {
                            "recipient_type": "user",
                            "recipient_id": user_id
                        }
                    ]
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    let notice_id = body["notice"]["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("notice id");
    let recipient_id = body["notice"]["recipients"][0]["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("recipient id");
    (notice_id, recipient_id)
}

// ---------------------------------------------------------------------------
// Legal Notices tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_legal_notice_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "legal-notice-create").await;
    let user_id = user_id_for(&pool, &user.email).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/legal/notices")
                .json(json!({
                    "notice_type": "policy_update",
                    "subject": "Updated Terms of Service",
                    "content": "We have updated our terms of service.",
                    "recipient_ids": [
                        {"recipient_type": "user", "recipient_id": user_id}
                    ]
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    resp.assert_json_field("notice");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_legal_notices_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "legal-notice-list").await;
    let user_id = user_id_for(&pool, &user.email).await;
    create_notice_with_recipient(&app, &token, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/legal/notices")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["notices"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one notice"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_legal_notices_with_recipients_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "legal-notice-recip").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/legal/notices/with-recipients")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_notice_statistics_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "legal-notice-stats").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/legal/notices/statistics")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_legal_notice_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-notice-get").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let (notice_id, _) = create_notice_with_recipient(&app, &token, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/legal/notices/{notice_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(
        body["notice"]["id"].as_str(),
        Some(notice_id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_legal_notice_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-notice-upd").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let (notice_id, _) = create_notice_with_recipient(&app, &token, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .patch(&format!("/api/v1/legal/notices/{notice_id}"))
                .json(json!({"subject": "Updated: Maintenance window this Saturday"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_legal_notice_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-notice-del").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let (notice_id, _) = create_notice_with_recipient(&app, &token, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/legal/notices/{notice_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn send_legal_notice_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "legal-notice-send").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let (notice_id, _) = create_notice_with_recipient(&app, &token, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!("/api/v1/legal/notices/{notice_id}/send"))
                .json(json!({}))
                .build(),
        )
        .await;

    // 200 = sent; 400 may occur when email service not configured in test env
    let status = resp.status;
    assert!(
        status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
        "expected 200 or 400, got {status}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_notice_recipients_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "legal-notice-recips").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let (notice_id, _) = create_notice_with_recipient(&app, &token, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/legal/notices/{notice_id}/recipients"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["recipients"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one recipient"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_legal_notice_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-notice-ack").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let (notice_id, recipient_id) =
        create_notice_with_recipient(&app, &token, org_id, user_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post(&format!(
                    "/api/v1/legal/notices/{notice_id}/acknowledge/{recipient_id}"
                ))
                .json(json!({"acknowledgment_method": "digital"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

// ---------------------------------------------------------------------------
// Compliance Templates tests
// ---------------------------------------------------------------------------

async fn create_compliance_template(app: &TestApp, token: &str, org_id: Uuid) -> Uuid {
    let resp = app
        .execute(
            app.session(token.to_string(), org_id)
                .post("/api/v1/legal/templates")
                .json(json!({
                    "name": "Annual Safety Checklist",
                    "category": "safety",
                    "description": "Standard annual safety audit checklist"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    let body = resp.json_value();
    body["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("compliance template id")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_compliance_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-tpl-create").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/legal/templates")
                .json(json!({
                    "name": "Fire Safety Template",
                    "category": "safety"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
    resp.assert_json_field("id");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_compliance_templates_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-tpl-list").await;
    create_compliance_template(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/legal/templates")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["templates"].as_array().map(|a| a.len()).unwrap_or(0) >= 1,
        "expected at least one template"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_compliance_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-tpl-get").await;
    let tpl_id = create_compliance_template(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get(&format!("/api/v1/legal/templates/{tpl_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert_eq!(body["id"].as_str(), Some(tpl_id.to_string().as_str()));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_compliance_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-tpl-upd").await;
    let tpl_id = create_compliance_template(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .patch(&format!("/api/v1/legal/templates/{tpl_id}"))
                .json(json!({"name": "Updated Safety Checklist"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_compliance_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-tpl-del").await;
    let tpl_id = create_compliance_template(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .delete(&format!("/api/v1/legal/templates/{tpl_id}"))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn apply_compliance_template_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-tpl-apply").await;
    let tpl_id = create_compliance_template(&app, &token, org_id).await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/legal/templates/apply")
                .json(json!({
                    "template_id": tpl_id
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
}

// ---------------------------------------------------------------------------
// Legal Audit Trail tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_audit_trail_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "legal-audit-list").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .get("/api/v1/legal/audit-trail")
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    assert!(
        body["audit_entries"].is_array(),
        "expected audit_entries array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_audit_entry_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::default();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "legal-audit-create").await;

    let resp = app
        .execute(
            app.session(token, org_id)
                .post("/api/v1/legal/audit-trail")
                .json(json!({
                    "action": "document_reviewed",
                    "notes": "Annual compliance review completed"
                }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::CREATED);
}

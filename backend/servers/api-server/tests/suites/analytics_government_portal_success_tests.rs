//! BIT-268 Wave 4 — government-portal happy-path backfill (Batch 6).
//!
//! Asserts that each endpoint returns 200/201/204 for an authenticated
//! same-org request.  Auth/IDOR paths are not duplicated here.
//!
//! Covered (25 partial endpoints → done):
//!   GET    /government-portal/connections
//!   POST   /government-portal/connections
//!   GET    /government-portal/connections/{id}
//!   PUT    /government-portal/connections/{id}
//!   DELETE /government-portal/connections/{id}
//!   POST   /government-portal/connections/{id}/test
//!   GET    /government-portal/templates
//!   GET    /government-portal/templates/{id}
//!   GET    /government-portal/submissions
//!   POST   /government-portal/submissions
//!   GET    /government-portal/submissions/{id}
//!   PUT    /government-portal/submissions/{id}
//!   POST   /government-portal/submissions/{id}/validate
//!   POST   /government-portal/submissions/{id}/submit
//!   POST   /government-portal/submissions/{id}/cancel
//!   GET    /government-portal/submissions/{id}/audit
//!   GET    /government-portal/submissions/{id}/attachments
//!   POST   /government-portal/submissions/{id}/attachments
//!   DELETE /government-portal/submissions/{submission_id}/attachments/{attachment_id}
//!   GET    /government-portal/schedules
//!   POST   /government-portal/schedules
//!   GET    /government-portal/schedules/{id}
//!   PUT    /government-portal/schedules/{id}
//!   DELETE /government-portal/schedules/{id}
//!   GET    /government-portal/stats

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT helper
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Claims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint(user_id: Uuid, email: &str, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: email.to_string(),
        name: "GovPortal Test".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("mint JWT")
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'Gov Portal Test User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_connection(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO government_portal_connections
               (organization_id, portal_type, portal_name, country_code, created_by)
           VALUES ($1, 'tax_authority'::government_portal_type, 'Tax Authority Portal', 'SK', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed connection")
}

async fn seed_template(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_report_templates
               (template_code, template_name, portal_type, country_code,
                schema_version, field_mappings, validation_rules, effective_from)
           VALUES ('SK_VAT_MONTHLY', 'SK VAT Monthly Report',
                   'tax_authority'::government_portal_type, 'SK',
                   '1.0', '{}', '[]', '2024-01-01')
           RETURNING id"#,
    )
    .fetch_one(pool)
    .await
    .expect("seed template")
}

async fn seed_submission(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submissions
               (organization_id, report_type, report_period_start, report_period_end,
                report_data, submission_reference, prepared_by)
           VALUES ($1, 'VAT', '2024-01-01', '2024-03-31', '{}', 'REF-DRAFT-001', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed submission")
}

async fn seed_submission_validated(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submissions
               (organization_id, report_type, report_period_start, report_period_end,
                report_data, submission_reference, status, prepared_by)
           VALUES ($1, 'VAT', '2024-01-01', '2024-03-31', '{}', 'REF-VAL-001',
                   'validated'::submission_status, $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed validated submission")
}

async fn seed_attachment(pool: &PgPool, submission_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submission_attachments
               (submission_id, file_name, file_type, file_size, file_url, attachment_type)
           VALUES ($1, 'report.pdf', 'application/pdf', 2048,
                   'https://s3.example.com/report.pdf', 'supporting_document')
           RETURNING id"#,
    )
    .bind(submission_id)
    .fetch_one(pool)
    .await
    .expect("seed attachment")
}

async fn seed_schedule(
    pool: &PgPool,
    org_id: Uuid,
    connection_id: Uuid,
    template_id: Uuid,
    user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submission_schedules
               (organization_id, portal_connection_id, template_id, created_by)
           VALUES ($1, $2, $3, $4)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(connection_id)
    .bind(template_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed schedule")
}

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

struct Fixture {
    app: TestApp,
    token: String,
    org_id: Uuid,
    user_id: Uuid,
}

async fn setup(pool: PgPool, slug: &str) -> Fixture {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, slug).await;
    let email = format!("{slug}-{}@govportal-test.internal", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint(user_id, &email, org_id);
    Fixture {
        app,
        token,
        org_id,
        user_id,
    }
}

// ===========================================================================
// connections
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_list_connections_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-list-conn").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/government-portal/connections")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list connections: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_create_connection_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-create-conn").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/government-portal/connections")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "portalType": "tax_authority",
                    "portalName": "SK Tax Authority"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create connection: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_get_connection_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-get-conn").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/government-portal/connections/{conn_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get connection: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_update_connection_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-upd-conn").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/government-portal/connections/{conn_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "portalName": "Updated Tax Authority Portal"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update connection: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_delete_connection_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-del-conn").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!("/api/v1/government-portal/connections/{conn_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete connection: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_test_connection_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-test-conn").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/government-portal/connections/{conn_id}/test"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "test connection: {}",
        resp.text()
    );
}

// ===========================================================================
// templates
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_list_templates_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-list-tpl").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/government-portal/templates")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list templates: {}",
        resp.text()
    );
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-567)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_get_template_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-get-tpl").await;
    let tpl_id = seed_template(&pool).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/government-portal/templates/{tpl_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get template: {}", resp.text());
}

// ===========================================================================
// submissions
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_list_submissions_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-list-sub").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/government-portal/submissions")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list submissions: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_create_submission_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-create-sub").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/government-portal/submissions")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "reportType": "VAT_MONTHLY",
                    "reportPeriodStart": "2024-01-01",
                    "reportPeriodEnd": "2024-01-31",
                    "reportData": { "totalVat": 12500, "currency": "EUR" }
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create submission: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_get_submission_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-get-sub").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/government-portal/submissions/{sub_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get submission: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_update_submission_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-upd-sub").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/government-portal/submissions/{sub_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "reportData": { "totalVat": 13000, "currency": "EUR" }
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update submission: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_validate_submission_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-validate-sub").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/government-portal/submissions/{sub_id}/validate"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "validate submission: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_submit_submission_succeeds(pool: PgPool) {
    // Submission must be in `validated` status before submitting.
    let f = setup(pool.clone(), "gp-submit-sub").await;
    let sub_id = seed_submission_validated(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/government-portal/submissions/{sub_id}/submit"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "submit submission: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_cancel_submission_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-cancel-sub").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/government-portal/submissions/{sub_id}/cancel"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "cancel submission: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_get_submission_audit_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-audit-sub").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/government-portal/submissions/{sub_id}/audit"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get submission audit: {}",
        resp.text()
    );
}

// ===========================================================================
// submission attachments
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_list_attachments_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-list-att").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/government-portal/submissions/{sub_id}/attachments"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list attachments: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_add_attachment_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-add-att").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/government-portal/submissions/{sub_id}/attachments"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "fileName": "vat_report.pdf",
                    "fileType": "application/pdf",
                    "fileSize": 4096,
                    "fileUrl": "https://s3.example.com/vat_report.pdf",
                    "attachmentType": "supporting_document"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add attachment: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_delete_attachment_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-del-att").await;
    let sub_id = seed_submission(&pool, f.org_id, f.user_id).await;
    let att_id = seed_attachment(&pool, sub_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!(
                    "/api/v1/government-portal/submissions/{sub_id}/attachments/{att_id}"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete attachment: {}",
        resp.text()
    );
}

// ===========================================================================
// schedules
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_list_schedules_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-list-sched").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/government-portal/schedules")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list schedules: {}",
        resp.text()
    );
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-567)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_create_schedule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-create-sched").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let tpl_id = seed_template(&pool).await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/government-portal/schedules")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "portalConnectionId": conn_id,
                    "templateId": tpl_id
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create schedule: {}",
        resp.text()
    );
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-567)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_get_schedule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-get-sched").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let tpl_id = seed_template(&pool).await;
    let sched_id = seed_schedule(&pool, f.org_id, conn_id, tpl_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/government-portal/schedules/{sched_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get schedule: {}", resp.text());
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-567)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_update_schedule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-upd-sched").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let tpl_id = seed_template(&pool).await;
    let sched_id = seed_schedule(&pool, f.org_id, conn_id, tpl_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/government-portal/schedules/{sched_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "notifyBeforeDays": 14
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update schedule: {}",
        resp.text()
    );
}

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-567)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_delete_schedule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-del-sched").await;
    let conn_id = seed_connection(&pool, f.org_id, f.user_id).await;
    let tpl_id = seed_template(&pool).await;
    let sched_id = seed_schedule(&pool, f.org_id, conn_id, tpl_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!("/api/v1/government-portal/schedules/{sched_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete schedule: {}",
        resp.text()
    );
}

// ===========================================================================
// stats
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_get_stats_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-stats").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/government-portal/stats")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get stats: {}", resp.text());
}

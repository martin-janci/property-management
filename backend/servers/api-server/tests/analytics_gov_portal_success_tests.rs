//! BIT-268 Wave 4 — government-portal happy-path backfill (Batch 4a).
//!
//! Asserts 200/201/204 for authenticated same-org requests.
//!
//! Covered (25 partial endpoints → done):
//!   GET    /government-portal/connections
//!   POST   /government-portal/connections
//!   GET    /government-portal/connections/{id}
//!   PUT    /government-portal/connections/{id}
//!   DELETE /government-portal/connections/{id}
//!   POST   /government-portal/connections/{id}/test
//!   GET    /government-portal/templates
//!   GET    /government-portal/submissions
//!   POST   /government-portal/submissions
//!   GET    /government-portal/submissions/{id}
//!   PUT    /government-portal/submissions/{id}
//!   POST   /government-portal/submissions/{id}/validate
//!   GET    /government-portal/submissions/{id}/audit
//!   GET    /government-portal/submissions/{id}/attachments
//!   GET    /government-portal/schedules
//!   POST   /government-portal/schedules
//!   GET    /government-portal/schedules/{id}
//!   PUT    /government-portal/schedules/{id}
//!   DELETE /government-portal/schedules/{id}
//!   GET    /government-portal/stats

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp, TestConfig};

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
        name: "Gov Portal Test".to_string(),
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
               (organization_id, portal_type, portal_name, country_code,
                test_mode, auto_submit, created_by)
           VALUES ($1, 'cadastre', 'Test Portal', 'SK', true, false, $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed connection")
}

async fn seed_submission(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submissions
               (organization_id, submission_reference, report_type,
                report_period_start, report_period_end, status,
                report_data, created_by)
           VALUES ($1, $2, 'annual_tax', '2023-01-01', '2023-12-31',
                   'draft', '{}', $3)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(format!("REF-{}", Uuid::new_v4()))
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed submission")
}

async fn seed_schedule(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO regulatory_submission_schedules
               (organization_id, name, report_type, frequency,
                next_due_date, is_active, created_by)
           VALUES ($1, 'Annual Tax Schedule', 'annual_tax', 'annual',
                   '2025-01-31', true, $2)
           RETURNING id"#,
    )
    .bind(org_id)
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
    let email = format!("{slug}-{}@gov-test.internal", Uuid::new_v4());
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

// ---------------------------------------------------------------------------
// Tests — connections
// ---------------------------------------------------------------------------

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
                    "portal_type": "cadastre",
                    "portal_name": "My Portal"
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
                .json(serde_json::json!({ "portal_name": "Updated Portal" }))
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
                .json(serde_json::json!({}))
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

// ---------------------------------------------------------------------------
// Tests — templates
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_list_templates_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-list-tmpl").await;
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

// ---------------------------------------------------------------------------
// Tests — submissions
// ---------------------------------------------------------------------------

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
                    "report_type": "annual_tax",
                    "report_period_start": "2023-01-01",
                    "report_period_end": "2023-12-31",
                    "report_data": {}
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
                .json(serde_json::json!({ "report_data": {"updated": true} }))
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
                .json(serde_json::json!({}))
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
        "submission audit: {}",
        resp.text()
    );
}

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

// ---------------------------------------------------------------------------
// Tests — schedules
// ---------------------------------------------------------------------------

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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_create_schedule_succeeds(pool: PgPool) {
    let f = setup(pool, "gp-create-sched").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/government-portal/schedules")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "Annual Tax Schedule",
                    "report_type": "annual_tax",
                    "frequency": "annual",
                    "next_due_date": "2025-01-31"
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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_get_schedule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-get-sched").await;
    let sched_id = seed_schedule(&pool, f.org_id, f.user_id).await;
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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_update_schedule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-upd-sched").await;
    let sched_id = seed_schedule(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/government-portal/schedules/{sched_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "name": "Updated Schedule" }))
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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gp_delete_schedule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "gp-del-sched").await;
    let sched_id = seed_schedule(&pool, f.org_id, f.user_id).await;
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
    assert_eq!(resp.status, StatusCode::OK, "portal stats: {}", resp.text());
}

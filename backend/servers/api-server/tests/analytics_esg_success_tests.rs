//! BIT-268 Wave 4 — ESG reporting happy-path backfill (Batch 4b).
//!
//! Asserts 200/201/204 for authenticated same-org requests.
//! get_metric done in esg_reporting_cross_org_idor_tests.rs T9 — skipped here.
//!
//! Covered (37 partial endpoints → done):
//!   GET    /esg/configuration
//!   POST   /esg/configuration
//!   GET    /esg/metrics
//!   POST   /esg/metrics
//!   PUT    /esg/metrics/{id}
//!   POST   /esg/metrics/{id}/verify
//!   POST   /esg/metrics/{id}/delete
//!   GET    /esg/carbon
//!   POST   /esg/carbon
//!   GET    /esg/carbon/summary/{year}
//!   GET    /esg/carbon/{id}
//!   POST   /esg/carbon/{id}/delete
//!   GET    /esg/benchmarks
//!   POST   /esg/benchmarks
//!   POST   /esg/benchmarks/{id}/delete
//!   GET    /esg/targets
//!   POST   /esg/targets
//!   GET    /esg/targets/{id}
//!   PUT    /esg/targets/{id}
//!   POST   /esg/targets/{id}/delete
//!   GET    /esg/reports
//!   POST   /esg/reports
//!   GET    /esg/reports/{id}
//!   PUT    /esg/reports/{id}
//!   POST   /esg/reports/{id}/submit
//!   POST   /esg/reports/{id}/approve
//!   POST   /esg/reports/{id}/delete
//!   GET    /esg/eu-taxonomy
//!   POST   /esg/eu-taxonomy
//!   GET    /esg/eu-taxonomy/{id}
//!   PUT    /esg/eu-taxonomy/{id}
//!   GET    /esg/dashboard/{year}
//!   POST   /esg/dashboard/{year}/refresh
//!   GET    /esg/imports
//!   POST   /esg/imports
//!   GET    /esg/imports/{id}
//!   GET    /esg/statistics

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
        name: "ESG Test".to_string(),
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
           VALUES ($1, 'hash', 'ESG Test User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_metric(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_metrics
               (organization_id, period_start, period_end, category,
                metric_type, metric_name, value, unit, data_source, created_by)
           VALUES ($1, '2023-01-01', '2023-12-31', 'energy',
                   'electricity_consumption', 'Annual Electricity', 1000, 'kWh',
                   'manual', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed esg metric")
}

async fn seed_carbon(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO carbon_footprints
               (organization_id, year, source_type, consumption_value,
                consumption_unit, emission_factor, co2_equivalent_kg)
           VALUES ($1, 2023, 'scope_1', 1000, 'kWh', 0.233, 233)
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed carbon footprint")
}

async fn seed_benchmark(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_benchmarks
               (organization_id, name, category, metric_type, benchmark_value,
                unit, effective_date)
           VALUES ($1, 'EU Energy Benchmark', 'energy_efficiency', 'electricity',
                   150, 'kWh/sqm', '2023-01-01')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed esg benchmark")
}

async fn seed_target(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_targets
               (organization_id, name, category, metric_type, target_value,
                unit, target_date)
           VALUES ($1, 'Carbon Neutral 2030', 'carbon', 'co2_emissions',
                   0, 'tCO2e', '2030-12-31')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed esg target")
}

async fn seed_report(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_reports
               (organization_id, report_type, title, period_start, period_end,
                frameworks, status, created_by)
           VALUES ($1, 'annual', 'Annual ESG Report 2023', '2023-01-01',
                   '2023-12-31', '[]', 'draft', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed esg report")
}

async fn seed_eu_taxonomy(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO eu_taxonomy_assessments
               (organization_id, year)
           VALUES ($1, 2023)
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed eu taxonomy")
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
    let email = format!("{slug}-{}@esg-test.internal", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint(user_id, &email, org_id);
    Fixture { app, token, org_id, user_id }
}

// ---------------------------------------------------------------------------
// Tests — configuration
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_configuration_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-get-config").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/configuration")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "esg config: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_upsert_configuration_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-upsert-config").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/configuration")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "reporting_currency": "EUR",
                    "target_year": 2030
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "upsert esg config: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — metrics
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_metrics_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-metrics").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/metrics")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list esg metrics: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_metric_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-metric").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/metrics")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "period_start": "2023-01-01",
                    "period_end": "2023-12-31",
                    "category": "energy",
                    "metric_type": "electricity_consumption",
                    "metric_name": "Annual Electricity",
                    "value": "1000",
                    "unit": "kWh",
                    "data_source": "manual"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create esg metric: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_update_metric_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-upd-metric").await;
    let metric_id = seed_metric(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/esg/metrics/{metric_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "value": "1100" }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "update esg metric: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_verify_metric_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-verify-metric").await;
    let metric_id = seed_metric(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/metrics/{metric_id}/verify"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "verify esg metric: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_metric_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-del-metric").await;
    let metric_id = seed_metric(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/metrics/{metric_id}/delete"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "delete esg metric: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — carbon footprint
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_carbon_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-carbon").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/carbon")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list carbon: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_carbon_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-carbon").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/carbon")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "year": 2023,
                    "source_type": "scope_1",
                    "consumption_value": "1000",
                    "consumption_unit": "kWh",
                    "emission_factor": "0.233"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create carbon: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_carbon_summary_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-carbon-sum").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/carbon/summary/2023")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "carbon summary: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_carbon_footprint_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-carbon").await;
    let carbon_id = seed_carbon(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/carbon/{carbon_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get carbon: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_carbon_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-del-carbon").await;
    let carbon_id = seed_carbon(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/carbon/{carbon_id}/delete"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "delete carbon: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — benchmarks
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_benchmarks_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-bench").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/benchmarks")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list esg benchmarks: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-bench").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/benchmarks")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "EU Energy Benchmark",
                    "category": "energy_efficiency",
                    "metric_type": "electricity",
                    "benchmark_value": "150",
                    "unit": "kWh/sqm",
                    "effective_date": "2023-01-01"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create esg benchmark: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-del-bench").await;
    let bench_id = seed_benchmark(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/benchmarks/{bench_id}/delete"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "delete esg benchmark: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — targets
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_targets_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-targets").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/targets")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list esg targets: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_target_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-target").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/targets")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "Net Zero 2030",
                    "category": "carbon",
                    "metric_type": "co2_emissions",
                    "target_value": "0",
                    "unit": "tCO2e",
                    "target_date": "2030-12-31"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create esg target: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_target_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-target").await;
    let target_id = seed_target(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/targets/{target_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get esg target: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_update_target_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-upd-target").await;
    let target_id = seed_target(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/esg/targets/{target_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "target_value": "10" }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "update esg target: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_target_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-del-target").await;
    let target_id = seed_target(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/targets/{target_id}/delete"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "delete esg target: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — reports
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_reports_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-reports").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/reports")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list esg reports: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_report_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-report").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/reports")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "report_type": "annual",
                    "title": "ESG Report 2023",
                    "period_start": "2023-01-01",
                    "period_end": "2023-12-31",
                    "frameworks": []
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create esg report: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-report").await;
    let report_id = seed_report(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/reports/{report_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get esg report: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_update_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-upd-report").await;
    let report_id = seed_report(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/esg/reports/{report_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "title": "ESG Report 2023 (updated)" }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "update esg report: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_submit_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-submit-report").await;
    let report_id = seed_report(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/reports/{report_id}/submit"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "submit esg report: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_approve_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-approve-report").await;
    let report_id = seed_report(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/reports/{report_id}/approve"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "approve esg report: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-del-report").await;
    let report_id = seed_report(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/reports/{report_id}/delete"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "delete esg report: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — EU Taxonomy
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_eu_taxonomy_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-eutax").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/eu-taxonomy")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list eu taxonomy: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_eu_taxonomy_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-eutax").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/eu-taxonomy")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "year": 2023 }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create eu taxonomy: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_eu_taxonomy_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-eutax").await;
    let tax_id = seed_eu_taxonomy(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/eu-taxonomy/{tax_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get eu taxonomy: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_update_eu_taxonomy_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-upd-eutax").await;
    let tax_id = seed_eu_taxonomy(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/esg/eu-taxonomy/{tax_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "climate_mitigation_eligible": true }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "update eu taxonomy: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — dashboard
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_dashboard_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-dashboard").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/dashboard/2023")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "esg dashboard: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_refresh_dashboard_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-refresh-dash").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/dashboard/2023/refresh")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "refresh esg dashboard: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests — imports
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_imports_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-imports").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/imports")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list esg imports: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_import_job_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-import").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/imports")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "source_type": "csv",
                    "file_name": "metrics_2023.csv"
                }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create esg import: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_import_job_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-get-import").await;
    // Seed an import job directly
    let import_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_import_jobs
               (organization_id, source_type, file_name, status, created_by)
           VALUES ($1, 'csv', 'test.csv', 'pending', $2)
           RETURNING id"#,
    )
    .bind(f.org_id)
    .bind(f.user_id)
    .fetch_one(&f.app.pool)
    .await
    .expect("seed import job");

    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/imports/{import_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get esg import: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_statistics_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-statistics").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/statistics")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "esg statistics: {}", resp.text());
}

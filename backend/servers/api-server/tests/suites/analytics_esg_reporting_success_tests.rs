//! BIT-268 Wave 4 — ESG reporting happy-path backfill (Batch 7).
//!
//! Asserts that each endpoint returns 200/201/204 for an authenticated
//! same-org request.  Auth/IDOR paths are not duplicated here.
//!
//! ESG routes use `RlsConnection` (not TenantExtractor), so the tenant
//! context comes from the `X-Tenant-ID` header validated against org
//! membership — not the JWT `tenant_id` claim.  Tokens are minted via
//! `JwtService::generate_access_token` (no embedded tenant_id).
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

#![allow(dead_code)]

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};
use api_server::services::JwtService;

// ---------------------------------------------------------------------------
// JWT helper (uses JwtService — no embedded tenant_id, tenant comes from header)
// ---------------------------------------------------------------------------

fn mint_token(user_id: Uuid, email: &str) -> String {
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "ESG Test User", None, None)
        .expect("mint access token")
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
               (organization_id, period_start, period_end, category, metric_type,
                metric_name, value, unit, data_source, created_by)
           VALUES ($1, '2024-01-01', '2024-03-31', 'environmental', 'energy_consumption',
                   'Total electricity', 12345.0, 'kWh', 'manual', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed metric")
}

async fn seed_carbon(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO carbon_footprints
               (organization_id, year, source_type, consumption_value,
                consumption_unit, emission_factor, co2_equivalent_kg)
           VALUES ($1, 2024, 'scope_2_indirect', 1000.0, 'kWh', 0.233, 233.0)
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
               (organization_id, name, category, metric_type,
                benchmark_value, unit, effective_date)
           VALUES ($1, 'Industry Energy Avg', 'industry_average',
                   'energy_consumption', 100.0, 'kWh/sqm', '2024-01-01')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed benchmark")
}

async fn seed_target(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_targets
               (organization_id, name, category, metric_type,
                target_value, unit, target_date)
           VALUES ($1, 'Reduce energy 20%', 'environmental',
                   'energy_consumption', 9876.0, 'kWh', '2025-12-31')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed target")
}

async fn seed_report(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_reports
               (organization_id, report_type, title, period_start, period_end,
                status, created_by)
           VALUES ($1, 'annual', 'ESG Annual Report 2024', '2024-01-01', '2024-12-31',
                   'draft', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed draft report")
}

async fn seed_report_pending(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_reports
               (organization_id, report_type, title, period_start, period_end,
                status, created_by)
           VALUES ($1, 'annual', 'ESG Pending Report 2024', '2024-01-01', '2024-12-31',
                   'pending_review', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed pending_review report")
}

async fn seed_eu_taxonomy(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO eu_taxonomy_assessments (organization_id, year)
           VALUES ($1, 2024)
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed eu taxonomy assessment")
}

async fn seed_import_job(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO esg_import_jobs (organization_id, file_name, data_type, created_by)
           VALUES ($1, 'energy_data.csv', 'energy', $2)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed import job")
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
    let token = mint_token(user_id, &email);
    Fixture {
        app,
        token,
        org_id,
        user_id,
    }
}

// ===========================================================================
// configuration
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_configuration_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-get-cfg").await;
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
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get configuration: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_upsert_configuration_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-upsert-cfg").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/configuration")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "reporting_currency": "EUR",
                    "fiscal_year_start_month": 1,
                    "target_year": 2030
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "upsert configuration: {}",
        resp.text()
    );
}

// ===========================================================================
// metrics
// ===========================================================================

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
    assert_eq!(resp.status, StatusCode::OK, "list metrics: {}", resp.text());
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
                    "period_start": "2024-01-01",
                    "period_end": "2024-03-31",
                    "category": "Environmental",
                    "metric_type": "energy_consumption",
                    "metric_name": "Q1 Electricity",
                    "value": "5000.0",
                    "unit": "kWh",
                    "data_source": "manual"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create metric: {}",
        resp.text()
    );
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
                .json(serde_json::json!({ "value": "13000.0", "notes": "Revised figure" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update metric: {}",
        resp.text()
    );
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
                .json(serde_json::json!({ "status": "verified" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "verify metric: {}",
        resp.text()
    );
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
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete metric: {}",
        resp.text()
    );
}

// ===========================================================================
// carbon footprints
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_carbon_footprints_succeeds(pool: PgPool) {
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
async fn esg_create_carbon_footprint_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-carbon").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/carbon")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "year": 2024,
                    "month": 1,
                    "source_type": "Scope2Indirect",
                    "consumption_value": "850.0",
                    "consumption_unit": "kWh",
                    "emission_factor": "0.233"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create carbon: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_carbon_summary_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-carbon-summary").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/carbon/summary/2024")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "carbon summary: {}",
        resp.text()
    );
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
async fn esg_delete_carbon_footprint_succeeds(pool: PgPool) {
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
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete carbon: {}",
        resp.text()
    );
}

// ===========================================================================
// benchmarks
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_benchmarks_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-bm").await;
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
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list benchmarks: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-bm").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/benchmarks")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "EU Residential Energy Benchmark",
                    "category": "RegionalAverage",
                    "metric_type": "energy_consumption",
                    "benchmark_value": "120.0",
                    "unit": "kWh/sqm",
                    "effective_date": "2024-01-01"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create benchmark: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-del-bm").await;
    let bm_id = seed_benchmark(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/benchmarks/{bm_id}/delete"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete benchmark: {}",
        resp.text()
    );
}

// ===========================================================================
// targets
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_targets_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-tgt").await;
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
    assert_eq!(resp.status, StatusCode::OK, "list targets: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_target_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-tgt").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/targets")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "20% energy reduction by 2030",
                    "category": "Environmental",
                    "metric_type": "energy_consumption",
                    "target_value": "8000.0",
                    "unit": "kWh",
                    "target_date": "2030-12-31"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create target: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_target_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-tgt").await;
    let tgt_id = seed_target(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/targets/{tgt_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get target: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_update_target_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-upd-tgt").await;
    let tgt_id = seed_target(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/esg/targets/{tgt_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "target_value": "7000.0", "status": "on_track" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update target: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_target_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-del-tgt").await;
    let tgt_id = seed_target(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/esg/targets/{tgt_id}/delete"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete target: {}",
        resp.text()
    );
}

// ===========================================================================
// reports
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_reports_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-rep").await;
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
    assert_eq!(resp.status, StatusCode::OK, "list reports: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_report_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-rep").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/reports")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "report_type": "annual",
                    "title": "ESG Annual Report 2024",
                    "period_start": "2024-01-01",
                    "period_end": "2024-12-31",
                    "frameworks": ["EuTaxonomy", "Csrd"]
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create report: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-rep").await;
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
    assert_eq!(resp.status, StatusCode::OK, "get report: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_update_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-upd-rep").await;
    let report_id = seed_report(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/esg/reports/{report_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "title": "ESG Annual Report 2024 (Revised)" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update report: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_submit_report_succeeds(pool: PgPool) {
    // submit_report only accepts draft status reports.
    let f = setup(pool.clone(), "esg-submit-rep").await;
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
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "submit report: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_approve_report_succeeds(pool: PgPool) {
    // approve_report only accepts pending_review status reports.
    let f = setup(pool.clone(), "esg-approve-rep").await;
    let report_id = seed_report_pending(&pool, f.org_id, f.user_id).await;
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
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "approve report: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_delete_report_succeeds(pool: PgPool) {
    // delete_report only accepts draft status reports.
    let f = setup(pool.clone(), "esg-del-rep").await;
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
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete report: {}",
        resp.text()
    );
}

// ===========================================================================
// eu-taxonomy
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_eu_taxonomy_assessments_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-eu").await;
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
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list eu taxonomy: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_eu_taxonomy_assessment_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-eu").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/eu-taxonomy")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "year": 2024,
                    "climate_mitigation_eligible": true,
                    "energy_performance_class": "A"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create eu taxonomy: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_eu_taxonomy_assessment_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-eu").await;
    let eu_id = seed_eu_taxonomy(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/eu-taxonomy/{eu_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get eu taxonomy: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_update_eu_taxonomy_assessment_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-upd-eu").await;
    let eu_id = seed_eu_taxonomy(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/esg/eu-taxonomy/{eu_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "climate_mitigation_aligned": true,
                    "dnsh_water": true
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update eu taxonomy: {}",
        resp.text()
    );
}

// ===========================================================================
// dashboard
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_dashboard_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-get-dash").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/esg/dashboard/2024")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get dashboard: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_refresh_dashboard_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-refresh-dash").await;
    let _metric_id = seed_metric(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/dashboard/2024/refresh")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({}))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "refresh dashboard: {}",
        resp.text()
    );
}

// ===========================================================================
// imports
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_list_import_jobs_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-list-imp").await;
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
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list import jobs: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_create_import_job_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-create-imp").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/esg/imports")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "file_name": "2024_energy_consumption.csv",
                    "data_type": "energy"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create import job: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_import_job_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "esg-get-imp").await;
    let job_id = seed_import_job(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/esg/imports/{job_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get import job: {}",
        resp.text()
    );
}

// ===========================================================================
// statistics
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn esg_get_statistics_succeeds(pool: PgPool) {
    let f = setup(pool, "esg-stats").await;
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
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get statistics: {}",
        resp.text()
    );
}

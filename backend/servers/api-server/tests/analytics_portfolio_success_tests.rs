//! BIT-268 Wave 4 — portfolio-analytics and portfolio-performance happy-path
//! backfill (Batch 2).
//!
//! Asserts that each endpoint returns 200/201 for an authenticated same-org
//! request.  Auth/IDOR paths are not duplicated here.
//!
//! Covered (27 partial endpoints → done):
//!   portfolio-analytics (14):
//!     GET    /portfolio-analytics/summary
//!     GET    /portfolio-analytics/benchmarks
//!     POST   /portfolio-analytics/benchmarks
//!     GET    /portfolio-analytics/benchmarks/{id}
//!     PUT    /portfolio-analytics/benchmarks/{id}
//!     DELETE /portfolio-analytics/benchmarks/{id}
//!     GET    /portfolio-analytics/properties/metrics
//!     POST   /portfolio-analytics/properties/metrics
//!     GET    /portfolio-analytics/properties/{building_id}/metrics
//!     GET    /portfolio-analytics/metrics
//!     POST   /portfolio-analytics/metrics/calculate
//!     GET    /portfolio-analytics/comparisons
//!     POST   /portfolio-analytics/comparisons
//!     GET    /portfolio-analytics/trends
//!   portfolio-performance (13):
//!     POST   /portfolio-performance/portfolios
//!     GET    /portfolio-performance/portfolios
//!     GET    /portfolio-performance/portfolios/{id}
//!     PUT    /portfolio-performance/portfolios/{id}
//!     DELETE /portfolio-performance/portfolios/{id}
//!     POST   /portfolio-performance/portfolios/{id}/properties
//!     GET    /portfolio-performance/portfolios/{id}/properties
//!     POST   /portfolio-performance/portfolios/{id}/transactions
//!     GET    /portfolio-performance/portfolios/{id}/transactions
//!     POST   /portfolio-performance/portfolios/{id}/cash-flows
//!     GET    /portfolio-performance/portfolios/{id}/cash-flows
//!     POST   /portfolio-performance/portfolios/{id}/metrics/calculate
//!     GET    /portfolio-performance/portfolios/{id}/metrics/summary

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
        name: "Portfolio Test".to_string(),
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
           VALUES ($1, 'hash', 'Portfolio Test User', 'active', NOW())
           RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Portfolio Ave 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

// Seed a portfolio_benchmarks row (portfolio_analytics module).
async fn seed_pa_benchmark(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO portfolio_benchmarks
               (organization_id, name, category, target_value, scope)
           VALUES ($1, 'Test Benchmark', 'occupancy', 95, 'portfolio')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed pa benchmark")
}

// Seed a performance_portfolios row (portfolio_performance module).
async fn seed_perf_portfolio(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO performance_portfolios
               (organization_id, created_by, name, currency)
           VALUES ($1, $2, 'Perf Portfolio', 'EUR')
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed perf portfolio")
}

// Seed a portfolio_properties row in the performance module.
async fn seed_perf_property(pool: &PgPool, portfolio_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO portfolio_properties
               (portfolio_id, building_id, acquisition_date, acquisition_price,
                financing_type, ownership_percentage, currency)
           VALUES ($1, $2, '2022-01-01', 200000, 'mortgage', 100, 'EUR')
           RETURNING id"#,
    )
    .bind(portfolio_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed perf property")
}

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

struct Fixture {
    app: TestApp,
    token: String,
    org_id: Uuid,
    building_id: Uuid,
    user_id: Uuid,
}

async fn setup(pool: PgPool, slug: &str) -> Fixture {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, slug).await;
    let email = format!("{slug}-{}@portfolio-test.internal", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let building_id = seed_building(&pool, org_id).await;
    let token = mint(user_id, &email, org_id);
    Fixture {
        app,
        token,
        org_id,
        building_id,
        user_id,
    }
}

// ===========================================================================
// portfolio-analytics endpoints
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_get_portfolio_summary_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-summary").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/portfolio-analytics/summary")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "pa summary: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_list_benchmarks_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-list-bench").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/portfolio-analytics/benchmarks")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa list benchmarks: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_create_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-create-bench").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/portfolio-analytics/benchmarks")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "Occupancy Target",
                    "category": "occupancy",
                    "target_value": "95.0"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "pa create benchmark: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_get_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pa-get-bench").await;
    let bench_id = seed_pa_benchmark(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/portfolio-analytics/benchmarks/{bench_id}"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa get benchmark: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_update_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pa-upd-bench").await;
    let bench_id = seed_pa_benchmark(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!(
                    "/api/v1/portfolio-analytics/benchmarks/{bench_id}"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "target_value": "97.0" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa update benchmark: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_delete_benchmark_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pa-del-bench").await;
    let bench_id = seed_pa_benchmark(&pool, f.org_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!(
                    "/api/v1/portfolio-analytics/benchmarks/{bench_id}"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "pa delete benchmark: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_list_property_metrics_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-list-pm").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/portfolio-analytics/properties/metrics")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa list property metrics: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_upsert_property_metrics_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-upsert-pm").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/portfolio-analytics/properties/metrics")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "building_id": f.building_id,
                    "period_start": "2024-01-01",
                    "period_end": "2024-01-31",
                    "total_units": 10,
                    "occupied_units": 9,
                    "gross_rental_income": "9000.00"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa upsert property metrics: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_get_property_metrics_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-get-bldg-metrics").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/portfolio-analytics/properties/{}/metrics",
                    f.building_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa get property metrics: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_get_portfolio_metrics_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-get-metrics").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/portfolio-analytics/metrics")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa get portfolio metrics: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_calculate_portfolio_metrics_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-calc-metrics").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/portfolio-analytics/metrics/calculate")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "period_start": "2024-01-01",
                    "period_end": "2024-12-31"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa calc metrics: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_list_comparisons_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-list-comp").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/portfolio-analytics/comparisons")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa list comparisons: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_create_comparison_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-create-comp").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/portfolio-analytics/comparisons")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "Q1 Comparison",
                    "building_ids": [f.building_id],
                    "comparison_period_start": "2024-01-01",
                    "comparison_period_end": "2024-03-31"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "pa create comparison: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pa_get_trends_succeeds(pool: PgPool) {
    let f = setup(pool, "pa-get-trends").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/portfolio-analytics/trends")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pa get trends: {}",
        resp.text()
    );
}

// ===========================================================================
// portfolio-performance endpoints
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_create_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool, "pp-create-pf").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/portfolio-performance/portfolios")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "name": "My Investment Portfolio"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "pp create portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_list_portfolios_succeeds(pool: PgPool) {
    let f = setup(pool, "pp-list-pf").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/portfolio-performance/portfolios")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp list portfolios: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_get_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-get-pf").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/portfolio-performance/portfolios/{pf_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp get portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_update_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-upd-pf").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/portfolio-performance/portfolios/{pf_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "name": "Updated Portfolio" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp update portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_delete_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-del-pf").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!("/api/v1/portfolio-performance/portfolios/{pf_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "pp delete portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_add_property_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-add-prop").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/properties"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "building_id": f.building_id,
                    "acquisition_date": "2022-06-01",
                    "acquisition_price": "250000.00"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "pp add property: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_list_properties_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-list-props").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/properties"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp list properties: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_create_transaction_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-create-tx").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let prop_id = seed_perf_property(&pool, pf_id, f.building_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/transactions"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "property_id": prop_id,
                    "transaction_type": "income",
                    "amount": "1200.00",
                    "transaction_date": "2024-03-01"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "pp create transaction: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_list_transactions_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-list-tx").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/transactions"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp list transactions: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_upsert_cash_flow_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-upsert-cf").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let prop_id = seed_perf_property(&pool, pf_id, f.building_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/cash-flows"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "property_id": prop_id,
                    "period_year": 2024,
                    "period_month": 3,
                    "gross_rental_income": "1200.00",
                    "operating_expenses": "250.00"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp upsert cash flow: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_get_cash_flows_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-get-cf").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/cash-flows"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp get cash flows: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_calculate_metrics_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-calc-metrics").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/metrics/calculate"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "period_start": "2024-01-01",
                    "period_end": "2024-12-31"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp calc metrics: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pp_get_metrics_summary_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "pp-metrics-sum").await;
    let pf_id = seed_perf_portfolio(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/portfolio-performance/portfolios/{pf_id}/metrics/summary"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "pp metrics summary: {}",
        resp.text()
    );
}

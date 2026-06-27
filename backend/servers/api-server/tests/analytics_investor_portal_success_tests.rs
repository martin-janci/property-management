//! BIT-268 Wave 4 — investor-portal happy-path backfill (Batch 3).
//!
//! Asserts that each endpoint returns 200/201/204 for an authenticated
//! same-org request.  Auth/IDOR paths are not duplicated here.
//!
//! Covered (28 partial endpoints → done):
//!   GET    /investor-portal/investors
//!   POST   /investor-portal/investors
//!   GET    /investor-portal/investors/{investor_id}
//!   PUT    /investor-portal/investors/{investor_id}
//!   DELETE /investor-portal/investors/{investor_id}
//!   GET    /investor-portal/investors/{investor_id}/summary
//!   GET    /investor-portal/portfolios
//!   POST   /investor-portal/portfolios
//!   GET    /investor-portal/portfolios/{portfolio_id}
//!   PUT    /investor-portal/portfolios/{portfolio_id}
//!   DELETE /investor-portal/portfolios/{portfolio_id}
//!   GET    /investor-portal/investors/{investor_id}/portfolios
//!   POST   /investor-portal/portfolios/{portfolio_id}/properties
//!   PUT    /investor-portal/portfolios/{portfolio_id}/properties/{property_id}
//!   DELETE /investor-portal/portfolios/{portfolio_id}/properties/{property_id}
//!   GET    /investor-portal/roi
//!   POST   /investor-portal/roi
//!   GET    /investor-portal/portfolios/{portfolio_id}/roi/latest
//!   POST   /investor-portal/distributions
//!   GET    /investor-portal/investors/{investor_id}/distributions
//!   PUT    /investor-portal/distributions/{distribution_id}
//!   POST   /investor-portal/reports
//!   GET    /investor-portal/investors/{investor_id}/reports
//!   GET    /investor-portal/reports/{report_id}
//!   POST   /investor-portal/capital-calls
//!   GET    /investor-portal/investors/{investor_id}/capital-calls
//!   PUT    /investor-portal/capital-calls/{call_id}
//!   GET    /investor-portal/dashboard/{investor_id}
//!   POST   /investor-portal/dashboard/{investor_id}/metrics

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
        name: "Investor Test".to_string(),
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
           VALUES ($1, 'hash', 'Investor Test User', 'active', NOW())
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
           VALUES ($1, 'Investor Ave 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_investor(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO investor_profiles
               (organization_id, display_name, investor_type)
           VALUES ($1, 'Test Investor', 'individual')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed investor")
}

async fn seed_investor_portfolio(
    pool: &PgPool,
    org_id: Uuid,
    investor_id: Uuid,
    user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO investment_portfolios
               (organization_id, investor_id, name, initial_investment,
                investment_date, created_by)
           VALUES ($1, $2, 'Inv Portfolio', 100000, '2022-01-01', $3)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(investor_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed investment portfolio")
}

async fn seed_portfolio_property(pool: &PgPool, portfolio_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO investor_portfolio_properties
               (portfolio_id, building_id, allocation_percentage)
           VALUES ($1, $2, '100.00')
           RETURNING id"#,
    )
    .bind(portfolio_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed portfolio property")
}

async fn seed_distribution(
    pool: &PgPool,
    org_id: Uuid,
    portfolio_id: Uuid,
    investor_id: Uuid,
    user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO investor_distributions
               (organization_id, portfolio_id, investor_id, distribution_type,
                amount, scheduled_date, created_by)
           VALUES ($1, $2, $3, 'income', 500, '2024-03-31', $4)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(portfolio_id)
    .bind(investor_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed distribution")
}

async fn seed_report(pool: &PgPool, org_id: Uuid, investor_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO investor_reports
               (organization_id, investor_id, report_type, title, report_data, created_by)
           VALUES ($1, $2, 'quarterly', 'Q1 Report', '{}', $3)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(investor_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed investor report")
}

async fn seed_capital_call(
    pool: &PgPool,
    org_id: Uuid,
    portfolio_id: Uuid,
    investor_id: Uuid,
    user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO capital_calls
               (organization_id, portfolio_id, investor_id, call_number,
                amount, call_date, due_date, created_by)
           VALUES ($1, $2, $3, 1, 10000, '2024-03-01', '2024-04-01', $4)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(portfolio_id)
    .bind(investor_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed capital call")
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
    investor_id: Uuid,
    portfolio_id: Uuid,
}

async fn setup(pool: PgPool, slug: &str) -> Fixture {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, slug).await;
    let email = format!("{slug}-{}@investor-test.internal", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let building_id = seed_building(&pool, org_id).await;
    let investor_id = seed_investor(&pool, org_id).await;
    let portfolio_id = seed_investor_portfolio(&pool, org_id, investor_id, user_id).await;
    let token = mint(user_id, &email, org_id);
    Fixture {
        app,
        token,
        org_id,
        building_id,
        user_id,
        investor_id,
        portfolio_id,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_list_investors_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-list-inv").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/investor-portal/investors")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list investors: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_create_investor_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-create-inv").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/investor-portal/investors")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "display_name": "New Investor" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create investor: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_get_investor_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-get-inv").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/investors/{}",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get investor: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_update_investor_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-upd-inv").await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!(
                    "/api/v1/investor-portal/investors/{}",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "display_name": "Updated Investor" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update investor: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_delete_investor_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-del-inv").await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!(
                    "/api/v1/investor-portal/investors/{}",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete investor: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_get_investor_summary_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-inv-sum").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/investors/{}/summary",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "investor summary: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_list_portfolios_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-list-pf").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/investor-portal/portfolios")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list portfolios: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_create_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-create-pf").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/investor-portal/portfolios")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "investor_id": f.investor_id,
                    "name": "New Investment",
                    "initial_investment": "50000.00",
                    "investment_date": "2024-01-01"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_get_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-get-pf").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/portfolios/{}",
                    f.portfolio_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_update_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-upd-pf").await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!(
                    "/api/v1/investor-portal/portfolios/{}",
                    f.portfolio_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "name": "Updated Investment" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_delete_portfolio_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-del-pf").await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!(
                    "/api/v1/investor-portal/portfolios/{}",
                    f.portfolio_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete portfolio: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_list_investor_portfolios_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-inv-pfs").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/investors/{}/portfolios",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list investor portfolios: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_add_portfolio_property_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-add-prop").await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/investor-portal/portfolios/{}/properties",
                    f.portfolio_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "building_id": f.building_id,
                    "allocation_percentage": "100.00"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add portfolio property: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_update_portfolio_property_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "ip-upd-prop").await;
    let prop_id = seed_portfolio_property(&pool, f.portfolio_id, f.building_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!(
                    "/api/v1/investor-portal/portfolios/{}/properties/{prop_id}",
                    f.portfolio_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "allocation_percentage": "50.00" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update portfolio property: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_remove_portfolio_property_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "ip-rm-prop").await;
    let prop_id = seed_portfolio_property(&pool, f.portfolio_id, f.building_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!(
                    "/api/v1/investor-portal/portfolios/{}/properties/{prop_id}",
                    f.portfolio_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "remove portfolio property: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_list_roi_calculations_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-list-roi").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/investor-portal/roi")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list roi: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_create_roi_calculation_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-create-roi").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/investor-portal/roi")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "portfolio_id": f.portfolio_id,
                    "period_type": "annual",
                    "period_start": "2023-01-01",
                    "period_end": "2023-12-31",
                    "beginning_value": "100000.00",
                    "ending_value": "108000.00"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create roi: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_get_latest_roi_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-latest-roi").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/portfolios/{}/roi/latest",
                    f.portfolio_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get latest roi: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_create_distribution_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-create-dist").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/investor-portal/distributions")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "portfolio_id": f.portfolio_id,
                    "investor_id": f.investor_id,
                    "distribution_type": "income",
                    "amount": "500.00",
                    "scheduled_date": "2024-03-31"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create distribution: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_list_investor_distributions_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-list-dist").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/investors/{}/distributions",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list distributions: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_update_distribution_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "ip-upd-dist").await;
    let dist_id =
        seed_distribution(&pool, f.org_id, f.portfolio_id, f.investor_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/investor-portal/distributions/{dist_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "amount": "600.00" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update distribution: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_create_report_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-create-rep").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/investor-portal/reports")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "investor_id": f.investor_id,
                    "report_type": "quarterly",
                    "title": "Q1 2024",
                    "report_data": {}
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
async fn ip_list_investor_reports_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-list-rep").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/investors/{}/reports",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "list reports: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_get_report_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "ip-get-rep").await;
    let report_id = seed_report(&pool, f.org_id, f.investor_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/investor-portal/reports/{report_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "get report: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_create_capital_call_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-create-cc").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/investor-portal/capital-calls")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "portfolio_id": f.portfolio_id,
                    "investor_id": f.investor_id,
                    "call_number": 1,
                    "amount": "10000.00",
                    "call_date": "2024-03-01",
                    "due_date": "2024-04-01"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create capital call: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_list_investor_capital_calls_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-list-cc").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/investors/{}/capital-calls",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list capital calls: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_update_capital_call_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "ip-upd-cc").await;
    let cc_id = seed_capital_call(&pool, f.org_id, f.portfolio_id, f.investor_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/investor-portal/capital-calls/{cc_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "amount": "12000.00" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update capital call: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_get_investor_dashboard_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-dashboard").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/investor-portal/dashboard/{}",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "investor dashboard: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_upsert_dashboard_metrics_succeeds(pool: PgPool) {
    let f = setup(pool, "ip-dash-metrics").await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/investor-portal/dashboard/{}/metrics",
                    f.investor_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "investor_id": f.investor_id,
                    "metric_date": "2024-03-31",
                    "total_invested": "100000.00",
                    "total_value": "108000.00"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "upsert dashboard metrics: {}",
        resp.text()
    );
}

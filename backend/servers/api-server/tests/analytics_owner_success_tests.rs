//! BIT-268 Wave 4 — owner-analytics happy-path backfill (Batch 1).
//!
//! Asserts that each endpoint in `/api/v1/owner-analytics/*` returns 200/201
//! for an authenticated, same-org request.  These tests do NOT re-test IDOR or
//! 401 paths (those live in owner_analytics_cross_org_idor_tests.rs).
//!
//! Covered (17 partial endpoints → done):
//!   POST   /units/{unit_id}/valuation
//!   GET    /valuations/{valuation_id}
//!   POST   /valuations/{valuation_id}/comparables
//!   GET    /units/{unit_id}/value-history
//!   GET    /units/{unit_id}/value-trend
//!   POST   /units/{unit_id}/roi
//!   GET    /units/{unit_id}/cash-flow
//!   GET    /units/{unit_id}/roi-dashboard
//!   GET    /portfolio
//!   POST   /portfolio/compare
//!   GET    /expense-rules
//!   POST   /expense-rules
//!   PUT    /expense-rules/{id}
//!   DELETE /expense-rules/{id}
//!   POST   /expenses/submit
//!   GET    /expenses
//!   POST   /expenses/{id}/review

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT minting
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
        name: "Test User".to_string(),
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
           VALUES ($1, 'hash', 'Analytics Test User', 'active', NOW())
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
           VALUES ($1, 'Test St 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, unit_type)
           VALUES ($1, '1A', 1, 'apartment')
           RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_valuation(pool: &PgPool, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO property_valuations
               (unit_id, value_low, value_high, estimated_value, valuation_method, confidence_score)
           VALUES ($1, 90000, 110000, 100000, 'avm', 80)
           RETURNING id"#,
    )
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed valuation")
}

async fn seed_expense_rule(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO expense_auto_approval_rules
               (organization_id, created_by, max_amount_per_expense, is_active)
           VALUES ($1, $2, 500, true)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed expense rule")
}

async fn seed_expense(pool: &PgPool, unit_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO expense_approval_requests
               (unit_id, amount, category, description, status, submitted_by)
           VALUES ($1, 100, 'maintenance', 'Test expense', 'pending', $2)
           RETURNING id"#,
    )
    .bind(unit_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed expense")
}

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

struct Fixture {
    app: TestApp,
    token: String,
    org_id: Uuid,
    unit_id: Uuid,
    user_id: Uuid,
}

async fn setup(pool: PgPool, slug: &str) -> Fixture {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, slug).await;
    let email = format!("{slug}-{}@analytics-test.internal", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, &email, org_id);
    Fixture {
        app,
        token,
        org_id,
        unit_id,
        user_id,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_valuation_succeeds(pool: PgPool) {
    let f = setup(pool, "create-val").await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/owner-analytics/units/{}/valuation",
                    f.unit_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "unit_id": f.unit_id,
                    "estimated_value": "100000.00",
                    "valuation_method": "avm"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_valuation: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_valuation_with_comparables_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "get-val-comp").await;
    let val_id = seed_valuation(&pool, f.unit_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!("/api/v1/owner-analytics/valuations/{val_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_valuation_with_comparables: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_comparable_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "add-comp").await;
    let val_id = seed_valuation(&pool, f.unit_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/owner-analytics/valuations/{val_id}/comparables"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "address": "Main St 5, Bratislava",
                    "sale_price": "98000.00",
                    "size_sqm": 55,
                    "rooms": 2,
                    "distance_km": "0.5",
                    "similarity_score": "0.85",
                    "source": "manual"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add_comparable: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_value_history_succeeds(pool: PgPool) {
    let f = setup(pool, "val-history").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/owner-analytics/units/{}/value-history",
                    f.unit_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_value_history: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_value_trend_succeeds(pool: PgPool) {
    let f = setup(pool, "val-trend").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/owner-analytics/units/{}/value-trend",
                    f.unit_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_value_trend: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn calculate_roi_succeeds(pool: PgPool) {
    let f = setup(pool, "calc-roi").await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!("/api/v1/owner-analytics/units/{}/roi", f.unit_id))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "unit_id": f.unit_id,
                    "total_investment": "200000.00",
                    "from_date": "2023-01-01",
                    "to_date": "2024-01-01"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "calculate_roi: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_cash_flow_breakdown_succeeds(pool: PgPool) {
    let f = setup(pool, "cash-flow").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/owner-analytics/units/{}/cash-flow",
                    f.unit_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_cash_flow_breakdown: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_roi_dashboard_succeeds(pool: PgPool) {
    let f = setup(pool, "roi-dash").await;
    let resp = f
        .app
        .execute(
            f.app
                .get(&format!(
                    "/api/v1/owner-analytics/units/{}/roi-dashboard",
                    f.unit_id
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_roi_dashboard: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_portfolio_summary_succeeds(pool: PgPool) {
    let f = setup(pool, "portfolio-sum").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/owner-analytics/portfolio")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_portfolio_summary: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn compare_properties_succeeds(pool: PgPool) {
    let f = setup(pool, "compare-props").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/owner-analytics/portfolio/compare")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({ "unit_ids": [f.unit_id] }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "compare_properties: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_auto_approval_rules_succeeds(pool: PgPool) {
    let f = setup(pool, "list-rules").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/owner-analytics/expense-rules")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_auto_approval_rules: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_auto_approval_rule_succeeds(pool: PgPool) {
    let f = setup(pool, "create-rule").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/owner-analytics/expense-rules")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "max_amount_per_expense": "500.00"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_auto_approval_rule: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_auto_approval_rule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "update-rule").await;
    let rule_id = seed_expense_rule(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .put(&format!("/api/v1/owner-analytics/expense-rules/{rule_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "max_amount_per_expense": "750.00",
                    "is_active": true
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_auto_approval_rule: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_auto_approval_rule_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "delete-rule").await;
    let rule_id = seed_expense_rule(&pool, f.org_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .delete(&format!("/api/v1/owner-analytics/expense-rules/{rule_id}"))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_auto_approval_rule: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_expense_succeeds(pool: PgPool) {
    let f = setup(pool, "submit-expense").await;
    let resp = f
        .app
        .execute(
            f.app
                .post("/api/v1/owner-analytics/expenses/submit")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "unit_id": f.unit_id,
                    "amount": "150.00",
                    "category": "maintenance",
                    "description": "Replace light fixture"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "submit_expense: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_expense_requests_succeeds(pool: PgPool) {
    let f = setup(pool, "list-expenses").await;
    let resp = f
        .app
        .execute(
            f.app
                .get("/api/v1/owner-analytics/expenses")
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_expense_requests: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn review_expense_succeeds(pool: PgPool) {
    let f = setup(pool.clone(), "review-expense").await;
    let expense_id = seed_expense(&pool, f.unit_id, f.user_id).await;
    let resp = f
        .app
        .execute(
            f.app
                .post(&format!(
                    "/api/v1/owner-analytics/expenses/{expense_id}/review"
                ))
                .bearer(&f.token)
                .header("X-Tenant-ID", &f.org_id.to_string())
                .json(serde_json::json!({
                    "decision": "approved",
                    "notes": "Looks good"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "review_expense: {}",
        resp.text()
    );
}

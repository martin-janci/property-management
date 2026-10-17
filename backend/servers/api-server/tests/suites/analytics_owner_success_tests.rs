//! Happy-path tests for owner-analytics endpoints (Epic 74).
//! Covers all 17 partial endpoints → promoted to done.
#![allow(dead_code)]

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT mint
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct TestClaims {
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
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: email.to_string(),
        name: "Owner Analytics User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("OA Org {slug}"))
    .bind(format!("oa-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}@oa.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'OA User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'OA Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, unit_type)
           VALUES ($1, '1A', 1, 'apartment') RETURNING id"#,
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
           VALUES ($1, 95000, 105000, 100000, 'avm', 85)
           RETURNING id"#,
    )
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed valuation")
}

async fn seed_value_history(pool: &PgPool, unit_id: Uuid) {
    sqlx::query(r#"INSERT INTO property_value_history (unit_id, value) VALUES ($1, 100000)"#)
        .bind(unit_id)
        .execute(pool)
        .await
        .expect("seed value history");
}

async fn seed_rule(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO expense_auto_approval_rules
               (organization_id, owner_id, max_amount_per_expense, max_monthly_total, allowed_categories)
           VALUES ($1, $2, 300.00, 1000.00, ARRAY['maintenance'])
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed rule")
}

async fn seed_expense(pool: &PgPool, unit_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO expense_approval_requests
               (unit_id, submitted_by, amount, category, description, status)
           VALUES ($1, $2, 150.00, 'maintenance', 'Test expense', 'pending')
           RETURNING id"#,
    )
    .bind(unit_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed expense")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_create_valuation_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "create-val").await;
    let user_id = seed_user(&pool, &format!("oa-cv-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, "oa-cv@oa.test", org_id);

    let body = serde_json::json!({
        "organization_id": org_id,
        "unit_id": unit_id,
        "estimated_value": "100000.0",
        "valuation_method": "avm"
    });

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/owner-analytics/units/{unit_id}/valuation"
            ))
            .bearer(&token)
            .json(&body)
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_get_valuation_with_comparables_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "get-valc").await;
    let user_id = seed_user(&pool, &format!("oa-gvc-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let valuation_id = seed_valuation(&pool, unit_id).await;
    let token = mint(user_id, "oa-gvc@oa.test", org_id);

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/owner-analytics/valuations/{valuation_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_add_comparable_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "add-comp").await;
    let user_id = seed_user(&pool, &format!("oa-ac-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let valuation_id = seed_valuation(&pool, unit_id).await;
    let token = mint(user_id, "oa-ac@oa.test", org_id);

    let body = serde_json::json!({
        "address": "Comparable St 5, Bratislava",
        "sale_price": "95000.0",
        "size_sqm": 60,
        "rooms": 2,
        "distance_km": "0.5",
        "similarity_score": "85.0",
        "source": "manual"
    });

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/owner-analytics/valuations/{valuation_id}/comparables"
            ))
            .bearer(&token)
            .json(&body)
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_get_value_history_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "val-hist").await;
    let user_id = seed_user(&pool, &format!("oa-vh-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, "oa-vh@oa.test", org_id);

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/owner-analytics/units/{unit_id}/value-history"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_get_value_trend_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "val-trend").await;
    let user_id = seed_user(&pool, &format!("oa-vt-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    seed_value_history(&pool, unit_id).await;
    let token = mint(user_id, "oa-vt@oa.test", org_id);

    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/owner-analytics/units/{unit_id}/value-trend"
            ))
            .bearer(&token)
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_calculate_roi_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "calc-roi").await;
    let user_id = seed_user(&pool, &format!("oa-cr-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, "oa-cr@oa.test", org_id);

    let body = serde_json::json!({
        "organization_id": org_id,
        "unit_id": unit_id,
        "total_investment": "150000.0",
        "from_date": "2022-01-01",
        "to_date": "2024-12-31"
    });

    let resp = app
        .execute(
            app.post(&format!("/api/v1/owner-analytics/units/{unit_id}/roi"))
                .bearer(&token)
                .json(&body)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_get_cash_flow_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cash-flow").await;
    let user_id = seed_user(&pool, &format!("oa-cf-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, "oa-cf@oa.test", org_id);

    let resp = app
        .execute(app
        .get(&format!(
            "/api/v1/owner-analytics/units/{unit_id}/cash-flow?from_date=2024-01-01&to_date=2024-12-31"
        ))
        .bearer(&token)
        .build())
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_get_roi_dashboard_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "roi-dash").await;
    let user_id = seed_user(&pool, &format!("oa-rd-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, "oa-rd@oa.test", org_id);

    let resp = app
        .execute(app
        .get(&format!(
            "/api/v1/owner-analytics/units/{unit_id}/roi-dashboard?from_date=2024-01-01&to_date=2024-12-31"
        ))
        .bearer(&token)
        .build())
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_get_portfolio_summary_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "portfolio").await;
    let user_id = seed_user(&pool, &format!("oa-ps-{}@oa.test", Uuid::new_v4())).await;
    let token = mint(user_id, "oa-ps@oa.test", org_id);

    let resp = app
        .execute(
            app.get("/api/v1/owner-analytics/portfolio")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_compare_properties_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "compare").await;
    let user_id = seed_user(&pool, &format!("oa-cp-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, "oa-cp@oa.test", org_id);

    let body = serde_json::json!({ "unit_ids": [unit_id] });

    let resp = app
        .execute(
            app.post("/api/v1/owner-analytics/portfolio/compare")
                .bearer(&token)
                .json(&body)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_list_auto_approval_rules_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "list-rules").await;
    let user_id = seed_user(&pool, &format!("oa-lr-{}@oa.test", Uuid::new_v4())).await;
    let token = mint(user_id, "oa-lr@oa.test", org_id);

    let resp = app
        .execute(
            app.get("/api/v1/owner-analytics/expense-rules")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_create_auto_approval_rule_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "create-rule").await;
    let user_id = seed_user(&pool, &format!("oa-car-{}@oa.test", Uuid::new_v4())).await;
    let token = mint(user_id, "oa-car@oa.test", org_id);

    let body = serde_json::json!({
        "organization_id": org_id,
        "max_amount_per_expense": "300.0",
        "max_monthly_total": "1000.0",
        "allowed_categories": ["maintenance", "utilities"]
    });

    let resp = app
        .execute(
            app.post("/api/v1/owner-analytics/expense-rules")
                .bearer(&token)
                .json(&body)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_update_auto_approval_rule_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "update-rule").await;
    let user_id = seed_user(&pool, &format!("oa-ur-{}@oa.test", Uuid::new_v4())).await;
    let rule_id = seed_rule(&pool, org_id, user_id).await;
    let token = mint(user_id, "oa-ur@oa.test", org_id);

    let body = serde_json::json!({
        "max_amount_per_expense": "500.0",
        "is_active": true
    });

    let resp = app
        .execute(
            app.put(&format!("/api/v1/owner-analytics/expense-rules/{rule_id}"))
                .bearer(&token)
                .json(&body)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_delete_auto_approval_rule_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "delete-rule").await;
    let user_id = seed_user(&pool, &format!("oa-dr-{}@oa.test", Uuid::new_v4())).await;
    let rule_id = seed_rule(&pool, org_id, user_id).await;
    let token = mint(user_id, "oa-dr@oa.test", org_id);

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/owner-analytics/expense-rules/{rule_id}"))
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_submit_expense_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "submit-exp").await;
    let user_id = seed_user(&pool, &format!("oa-se-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint(user_id, "oa-se@oa.test", org_id);

    let body = serde_json::json!({
        "organization_id": org_id,
        "unit_id": unit_id,
        "amount": "150.0",
        "category": "maintenance",
        "description": "Replace broken window"
    });

    let resp = app
        .execute(
            app.post("/api/v1/owner-analytics/expenses/submit")
                .bearer(&token)
                .json(&body)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_list_expense_requests_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "list-exp").await;
    let user_id = seed_user(&pool, &format!("oa-le-{}@oa.test", Uuid::new_v4())).await;
    let token = mint(user_id, "oa-le@oa.test", org_id);

    let resp = app
        .execute(
            app.get("/api/v1/owner-analytics/expenses")
                .bearer(&token)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oa_review_expense_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "review-exp").await;
    let user_id = seed_user(&pool, &format!("oa-re-{}@oa.test", Uuid::new_v4())).await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let expense_id = seed_expense(&pool, unit_id, user_id).await;
    let token = mint(user_id, "oa-re@oa.test", org_id);

    let body = serde_json::json!({
        "decision": "approved",
        "notes": "Looks good"
    });

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/owner-analytics/expenses/{expense_id}/review"
            ))
            .bearer(&token)
            .json(&body)
            .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK);
}

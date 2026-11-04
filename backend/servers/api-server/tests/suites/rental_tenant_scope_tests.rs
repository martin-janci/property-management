//! Tenant-isolation tests for rentals endpoints that take a `unit_id` or
//! `building_id` from the request body.
//!
//! `create_calendar_block` (and `create_booking`, `create_connection`,
//! `create_ical_feed`) wrote rows against any `unit_id` without checking that
//! the unit belongs to the caller's org, and the authority-report preview read
//! the building name of any `building_id`. A foreign unit/building is now 404,
//! matching the existing `ensure_unit_in_org` convention (#804).

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig, TestResponse};

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

fn mint_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "rental-scope@test.local".to_string(),
        name: "Rental Scope Tester".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TestConfig::default().jwt_secret.as_bytes()),
    )
    .expect("mint token")
}

struct OrgFixture {
    org: Uuid,
    token: String,
    building: Uuid,
    unit: Uuid,
}

async fn seed_fixture(pool: &PgPool, tag: &str) -> OrgFixture {
    let org = seed_org(pool, &format!("rscope-{tag}")).await;
    let user = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Rental Scope User', 'active', NOW()) RETURNING id"#,
    )
    .bind(format!("{tag}-{}@rental-scope.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user");
    seed_membership(pool, org, user, "manager").await;
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
           VALUES ($1, $2, 'Main 1', 'Bratislava', '81101', 'SK', 'active') RETURNING id"#,
    )
    .bind(org)
    .bind(format!("Building {tag}"))
    .fetch_one(pool)
    .await
    .expect("seed building");
    let unit = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, $2, 1) RETURNING id",
    )
    .bind(building)
    .bind(format!("U-{tag}"))
    .fetch_one(pool)
    .await
    .expect("seed unit");
    OrgFixture {
        org,
        token: mint_token(user, org),
        building,
        unit,
    }
}

async fn post(app: &TestApp, f: &OrgFixture, uri: &str, body: Value) -> TestResponse {
    app.execute(
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {}", f.token))
            .header("X-Tenant-ID", f.org.to_string())
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
}

fn block_body(unit: Uuid) -> Value {
    let start = (Utc::now() + Duration::days(30)).date_naive();
    let end = start + Duration::days(3);
    json!({ "unit_id": unit, "block_start": start, "block_end": end, "reason": "maintenance" })
}

fn preview_body(building: Uuid) -> Value {
    let end = Utc::now().date_naive();
    let start = end - Duration::days(30);
    json!({ "building_id": building, "period_start": start, "period_end": end })
}

async fn count_blocks(pool: &PgPool, unit: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM rental_calendar_blocks WHERE unit_id = $1")
        .bind(unit)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn calendar_block_on_foreign_unit_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "ba").await;
    let b = seed_fixture(&pool, "bb").await;

    let resp = post(
        &app,
        &b,
        "/api/v1/rentals/calendar/blocks",
        block_body(a.unit),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org block must be 404: {}",
        resp.text()
    );
    assert_eq!(
        count_blocks(&pool, a.unit).await,
        0,
        "no block may be written"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn calendar_block_on_own_unit_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "bo").await;

    let resp = post(
        &app,
        &a,
        "/api/v1/rentals/calendar/blocks",
        block_body(a.unit),
    )
    .await;
    assert!(
        resp.status.is_success(),
        "same-org block should 2xx, got {}: {}",
        resp.status,
        resp.text()
    );
    assert_eq!(count_blocks(&pool, a.unit).await, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn other_unit_writes_on_foreign_unit_are_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "wa").await;
    let b = seed_fixture(&pool, "wb").await;
    let check_in = (Utc::now() + Duration::days(40)).date_naive();

    let cases = [
        (
            "/api/v1/rentals/bookings",
            json!({
                "unit_id": a.unit, "platform": "direct", "guest_name": "Eve",
                "check_in": check_in, "check_out": check_in + Duration::days(2)
            }),
        ),
        (
            "/api/v1/rentals/connections",
            json!({ "unit_id": a.unit, "platform": "airbnb" }),
        ),
        (
            "/api/v1/rentals/ical",
            json!({ "unit_id": a.unit, "feed_name": "Leak" }),
        ),
    ];
    for (uri, body) in cases {
        let resp = post(&app, &b, uri, body).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "POST {uri} on a foreign unit must be 404, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    let rows: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM rental_bookings WHERE unit_id = $1) \
              + (SELECT COUNT(*) FROM rental_platform_connections WHERE unit_id = $1) \
              + (SELECT COUNT(*) FROM rental_ical_feeds WHERE unit_id = $1)",
    )
    .bind(a.unit)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rows, 0, "nothing may be written against a foreign unit");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn report_preview_on_foreign_building_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "pa").await;
    let b = seed_fixture(&pool, "pb").await;

    let resp = post(
        &app,
        &b,
        "/api/v1/rentals/reports/preview",
        preview_body(a.building),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org preview must be 404: {}",
        resp.text()
    );
    assert!(
        !resp.text().contains("Building pa"),
        "foreign building name must not leak"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn report_preview_on_own_building_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "po").await;

    let resp = post(
        &app,
        &a,
        "/api/v1/rentals/reports/preview",
        preview_body(a.building),
    )
    .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "same-org preview should 200: {}",
        resp.text()
    );
    assert_eq!(
        resp.json_value()["building_name"].as_str(),
        Some("Building po")
    );
}

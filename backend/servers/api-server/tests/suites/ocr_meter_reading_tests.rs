//! Integration tests for OCR meter-reading endpoints (Epic 128).
//!
//! Covers:
//! - `POST /api/v1/ai/ocr/meter-reading` — creates a photo-backed meter
//!   reading in the DB (`source=photo`, `status=pending`).
//! - `POST /api/v1/ai/ocr/correction` — persists an OCR correction record in
//!   `ocr_meter_corrections`.
//! - Auth gates: unauthenticated requests are rejected; cross-tenant requests
//!   (caller not in the meter's org) are rejected.

#![allow(dead_code)]

use axum::http::{header, Method, Request, StatusCode};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use tower::ServiceExt;
use uuid::Uuid;

use crate::common::{TestApp, TestConfig};

// ============================================================================
// Seed helpers
// ============================================================================

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("OCR Test Org {slug}"))
    .bind(format!("ocr-test-org-{slug}"))
    .bind(format!("{slug}@ocr.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'OCR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role_type, status)
        VALUES ($1, $2, 'org_admin', 'active')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, 'OCR Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_meter(pool: &PgPool, org_id: Uuid, building_id: Uuid, number: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO meters (
            organization_id, building_id, meter_number, meter_type,
            initial_reading, current_reading, unit_of_measure, is_active
        )
        VALUES ($1, $2, $3, 'electricity', 0, 0, 'kWh', true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(format!("OCR-{number}"))
    .fetch_one(pool)
    .await
    .expect("seed meter")
}

fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "OCR User", None, None)
        .expect("mint token")
}

fn multipart_body(meter_id: Uuid, reading_value: &str) -> (Vec<u8>, String) {
    let boundary = "----TestBoundary1234";
    let body = format!(
        "--{b}\r\nContent-Disposition: form-data; name=\"meter_id\"\r\n\r\n{mid}\r\n\
         --{b}\r\nContent-Disposition: form-data; name=\"reading_value\"\r\n\r\n{rv}\r\n\
         --{b}--\r\n",
        b = boundary,
        mid = meter_id,
        rv = reading_value
    );
    let content_type = format!("multipart/form-data; boundary={boundary}");
    (body.into_bytes(), content_type)
}

// ============================================================================
// process_meter_reading tests
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn process_meter_reading_creates_pending_reading(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "read-a").await;
    let user = seed_user(&pool, "ocr-read-a@test.local").await;
    seed_membership(&pool, org, user).await;
    let building = seed_building(&pool, org).await;
    let meter = seed_meter(&pool, org, building, "READ-A-1").await;

    let token = mint_token(user, "ocr-read-a@test.local");
    let (body, content_type) = multipart_body(meter, "123.45");

    let resp = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/ai/ocr/meter-reading")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, content_type)
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "process_meter_reading should return 201"
    );

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(
        body["reading_id"].is_string(),
        "response must include reading_id"
    );
    let reading_id =
        Uuid::parse_str(body["reading_id"].as_str().unwrap()).expect("reading_id is a UUID");

    // Verify the row actually landed in the DB.
    let row = sqlx::query("SELECT source, status, photo_url FROM meter_readings WHERE id = $1")
        .bind(reading_id)
        .fetch_one(&pool)
        .await
        .expect("reading must exist in DB");

    let source: String = row.get("source");
    let status: String = row.get("status");
    let photo_url: Option<String> = row.get("photo_url");
    assert_eq!(source, "photo", "source must be 'photo'");
    assert_eq!(status, "pending", "status must be 'pending'");
    assert!(photo_url.is_some(), "photo_url must be set");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn process_meter_reading_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "noauth-b").await;
    let building = seed_building(&pool, org).await;
    let meter = seed_meter(&pool, org, building, "NOAUTH-B-1").await;

    let (body, content_type) = multipart_body(meter, "10.0");

    let resp = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/ai/ocr/meter-reading")
                .header(header::CONTENT_TYPE, content_type)
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status().as_u16() >= 400,
        "unauthenticated must be rejected"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn process_meter_reading_cross_tenant_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "xta-a").await;
    let org_b = seed_org(&pool, "xta-b").await;

    let user_b = seed_user(&pool, "ocr-xtb@test.local").await;
    seed_membership(&pool, org_b, user_b).await; // user_b is in org_b only

    let building_a = seed_building(&pool, org_a).await;
    let meter_a = seed_meter(&pool, org_a, building_a, "XTA-A-1").await;

    let token_b = mint_token(user_b, "ocr-xtb@test.local");
    let (body, content_type) = multipart_body(meter_a, "99.0");

    let resp = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/ai/ocr/meter-reading")
                .header(header::AUTHORIZATION, format!("Bearer {}", token_b))
                .header(header::CONTENT_TYPE, content_type)
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status().as_u16() >= 400,
        "cross-tenant access must be rejected (got {})",
        resp.status()
    );
}

// ============================================================================
// submit_correction tests
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_correction_persists_record(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "corr-a").await;
    let user = seed_user(&pool, "ocr-corr-a@test.local").await;
    seed_membership(&pool, org, user).await;

    let token = mint_token(user, "ocr-corr-a@test.local");

    let payload = json!({
        "organization_id": org,
        "original_value": 100.0_f64,
        "corrected_value": 105.5_f64,
        "image_url": "https://example.com/meter.jpg",
        "bounding_box": {"x": 10, "y": 20, "width": 50, "height": 30},
        "meter_reading_id": null
    });

    let resp = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/ai/ocr/correction")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_vec(&payload).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "submit_correction should return 201"
    );

    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();

    assert!(body["id"].is_string(), "response must include id");
    let correction_id = Uuid::parse_str(body["id"].as_str().unwrap()).expect("id is a UUID");

    // Round-trip: verify the row is in the DB with correct values.
    let row = sqlx::query(
        "SELECT original_value, corrected_value, image_url FROM ocr_meter_corrections WHERE id = $1",
    )
    .bind(correction_id)
    .fetch_one(&pool)
    .await
    .expect("correction must exist in DB");

    let image_url: String = row.get("image_url");
    let corrected_value: rust_decimal::Decimal = row.get("corrected_value");
    assert_eq!(image_url, "https://example.com/meter.jpg");
    assert_eq!(
        corrected_value,
        rust_decimal::Decimal::try_from(105.5_f64).unwrap()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_correction_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "corr-noauth").await;

    let payload = json!({
        "organization_id": org,
        "original_value": 100.0_f64,
        "corrected_value": 105.5_f64,
        "image_url": "https://example.com/meter.jpg"
    });

    let resp = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/ai/ocr/correction")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_vec(&payload).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status().as_u16() >= 400,
        "unauthenticated must be rejected"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_correction_cross_tenant_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "corr-xta-a").await;
    let org_b = seed_org(&pool, "corr-xta-b").await;

    let user_b = seed_user(&pool, "ocr-corr-xtb@test.local").await;
    seed_membership(&pool, org_b, user_b).await; // user_b only in org_b

    let token_b = mint_token(user_b, "ocr-corr-xtb@test.local");

    // Try to submit a correction for org_a while authenticated as org_b member.
    let payload = json!({
        "organization_id": org_a,
        "original_value": 100.0_f64,
        "corrected_value": 110.0_f64,
        "image_url": "https://evil.example.com/meter.jpg"
    });

    let resp = app
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/ai/ocr/correction")
                .header(header::AUTHORIZATION, format!("Bearer {}", token_b))
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(
                    serde_json::to_vec(&payload).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.status().as_u16() >= 400,
        "cross-tenant correction must be rejected (got {})",
        resp.status()
    );
}

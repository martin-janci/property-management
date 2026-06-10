//! Regression tests for the cross-org IDOR fix on Epic 134 predictive
//! maintenance photo endpoints (#848).
//!
//! Audit history: `list_maintenance_photos` (GET
//! `/api/v1/predictive-maintenance/maintenance-logs/{id}/photos`) loaded photos
//! by `maintenance_log_id` only and discarded `_auth`, so any authenticated
//! user who knew (or guessed) another org's `log_id` could read that org's
//! maintenance photos. `add_maintenance_photo` (POST) similarly wrote without
//! checking the parent log's org.
//!
//! Since the PAP-80 RLS conversion (PAP-111), every predictive-maintenance
//! handler acquires an `RlsConnection`: the tenant comes from the validated
//! `X-Tenant-ID` header (membership checked against `organization_members`),
//! the handler confirms the parent `maintenance_logs` row belongs to that org
//! (404 if not), and the repository join `maintenance_log_photos →
//! maintenance_logs` stays org-scoped. A foreign-org `log_id` therefore
//! yields 404, never a cross-tenant read.
//!
//! These tests mint a real access token per org, seed the membership rows the
//! tenant validation requires, send `X-Tenant-ID`, and drive the handlers
//! end-to-end:
//!   - Org A's member listing photos on Org A's log -> 200 (same-org succeeds).
//!   - Org B's member listing photos on Org A's log -> 404 (cross-org blocked).

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT minting (matches api_core::extractors::auth::Claims)
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

fn access_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "idor-test@pm-photos.test".to_string(),
        name: "PM Photos IDOR User".to_string(),
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
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("PM Photos IDOR Org {slug}"))
    .bind(format!("pm-photos-idor-{slug}"))
    .bind(format!("{slug}@pm-photos-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'PM Photos User', 'active', NOW())
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
        INSERT INTO organization_members (organization_id, user_id, role_type, status, joined_at)
        VALUES ($1, $2, 'manager', 'active', NOW())
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment_registry
            (organization_id, building_id, name, equipment_type, status)
        VALUES ($1, $2, 'Boiler', 'hvac', 'operational')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

async fn seed_maintenance_log(pool: &PgPool, org_id: Uuid, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO maintenance_logs
            (equipment_id, organization_id, maintenance_type, description)
        VALUES ($1, $2, 'preventive', 'Annual service')
        RETURNING id
        "#,
    )
    .bind(equipment_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed maintenance log")
}

async fn seed_photo(pool: &PgPool, log_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO maintenance_log_photos
            (maintenance_log_id, file_path, photo_type)
        VALUES ($1, 's3://bucket/photo.jpg', 'after')
        RETURNING id
        "#,
    )
    .bind(log_id)
    .fetch_one(pool)
    .await
    .expect("seed photo")
}

fn photos_uri(log_id: Uuid) -> String {
    format!("/api/v1/predictive-maintenance/maintenance-logs/{log_id}/photos")
}

// ---------------------------------------------------------------------------
// Test 1: same-org listing succeeds
// ---------------------------------------------------------------------------

/// A user whose JWT `tenant_id` matches the log's org can read its photos.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_photos_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "same-org@pm-photos-idor.test").await;
    let org_a = seed_org(&pool, "same-a").await;
    let building = seed_building(&pool, org_a, "same-a").await;
    let equipment = seed_equipment(&pool, org_a, building).await;
    let log_id = seed_maintenance_log(&pool, org_a, equipment).await;
    let photo_id = seed_photo(&pool, log_id).await;
    seed_membership(&pool, org_a, user).await;

    let token = access_token(user, org_a);
    let req = app
        .get(&photos_uri(log_id))
        .bearer(&token)
        .header("X-Tenant-ID", &org_a.to_string())
        .build();
    let resp = app.execute(req).await;

    resp.assert_status(StatusCode::OK);
    let body = resp.json_value();
    let arr = body.as_array().expect("photos array");
    assert_eq!(arr.len(), 1, "same-org list must return the seeded photo");
    assert_eq!(
        arr[0]["id"].as_str(),
        Some(photo_id.to_string().as_str()),
        "same-org list must return the right photo"
    );
}

// ---------------------------------------------------------------------------
// Test 2: cross-org listing is blocked (404)
// ---------------------------------------------------------------------------

/// A user from Org B cannot read Org A's maintenance photos by log id — the
/// org-scoped log lookup finds no row and the handler returns 404 (#848 IDOR).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_photos_cross_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "owner@pm-photos-idor.test").await;
    let user_b = seed_user(&pool, "attacker@pm-photos-idor.test").await;
    let org_a = seed_org(&pool, "x-a").await;
    let org_b = seed_org(&pool, "x-b").await;
    let building = seed_building(&pool, org_a, "x-a").await;
    let equipment = seed_equipment(&pool, org_a, building).await;
    let log_id = seed_maintenance_log(&pool, org_a, equipment).await;
    let _ = seed_photo(&pool, log_id).await;
    seed_membership(&pool, org_a, user_a).await;
    seed_membership(&pool, org_b, user_b).await;

    // Org B member (valid org-B tenant context) targeting Org A's log.
    let token = access_token(user_b, org_b);
    let req = app
        .get(&photos_uri(log_id))
        .bearer(&token)
        .header("X-Tenant-ID", &org_b.to_string())
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org photo list must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// Test 3: cross-org photo add is blocked and does not insert a row
// ---------------------------------------------------------------------------

/// A cross-org POST must not attach a photo to another org's log.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_photo_cross_org_does_not_insert(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "add-owner@pm-photos-idor.test").await;
    let user_b = seed_user(&pool, "add-attacker@pm-photos-idor.test").await;
    let org_a = seed_org(&pool, "a-a").await;
    let org_b = seed_org(&pool, "a-b").await;
    let building = seed_building(&pool, org_a, "a-a").await;
    let equipment = seed_equipment(&pool, org_a, building).await;
    let log_id = seed_maintenance_log(&pool, org_a, equipment).await;
    seed_membership(&pool, org_a, user_a).await;
    seed_membership(&pool, org_b, user_b).await;

    let token = access_token(user_b, org_b);
    let req = app
        .post(&photos_uri(log_id))
        .bearer(&token)
        .header("X-Tenant-ID", &org_b.to_string())
        .json(serde_json::json!({ "file_path": "s3://bucket/hijack.jpg" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org photo add must be 404, got {} (body: {})",
        resp.status,
        resp.text()
    );

    // No photo row must have been inserted for this log.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM maintenance_log_photos WHERE maintenance_log_id = $1",
    )
    .bind(log_id)
    .fetch_one(&pool)
    .await
    .expect("count photos");
    assert_eq!(count, 0, "cross-org POST must not insert a photo");
}

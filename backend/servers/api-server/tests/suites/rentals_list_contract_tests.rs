//! Contract tests for the rentals list endpoints consumed by ppt-web
//! (`/rentals`, `/rentals/bookings`, `/rentals/guests`).
//!
//! * `GET /api/v1/rentals/bookings` must apply its filters to the returned
//!   rows (not only to `total`), accept enum-valued `status` / `platform`
//!   filters, and decode the primary guest's registration status.
//! * `GET /api/v1/rentals/guests` lists the org's guests with booking context,
//!   honours `status` / `booking_id`, never leaks another org's guests,
//!   rejects an unknown status with 400 and is manager-gated (403).

#![allow(dead_code)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp};

// Must match `TestConfig::default().jwt_secret` in tests/common/mod.rs.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

#[derive(Serialize)]
struct AccessClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_access_token(user_id: Uuid, tenant_id: Uuid, role: &str) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some(role.to_string()),
        email: "rentals-list@test.local".to_string(),
        name: "Rentals List Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', 'Rentals List User', 'active', NOW()) RETURNING id",
    )
    .bind(format!("rentals-list-{}@test.local", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_unit(pool: &PgPool, org_id: Uuid, designation: &str) -> Uuid {
    let building = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
         VALUES ($1, 'Rental St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building");
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, $2, 1) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_booking(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    guest_name: &str,
    platform: &str,
    status: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_bookings \
             (organization_id, unit_id, platform, external_booking_id, guest_name, \
              check_in, check_out, status) \
         VALUES ($1, $2, $3::rental_platform, $6, $4, CURRENT_DATE + 3, CURRENT_DATE + 6, \
                 $5::rental_booking_status) RETURNING id",
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(platform)
    .bind(guest_name)
    .bind(status)
    // `unique_external_booking` is NULLS NOT DISTINCT on (platform,
    // external_booking_id), so each seeded booking needs its own external id.
    .bind(format!("ext-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

async fn seed_guest(
    pool: &PgPool,
    org_id: Uuid,
    booking_id: Uuid,
    first_name: &str,
    status: &str,
    is_primary: bool,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO rental_guests \
             (organization_id, booking_id, first_name, last_name, status, is_primary) \
         VALUES ($1, $2, $3, 'Doe', $4::guest_registration_status, $5) RETURNING id",
    )
    .bind(org_id)
    .bind(booking_id)
    .bind(first_name)
    .bind(status)
    .bind(is_primary)
    .fetch_one(pool)
    .await
    .expect("seed guest")
}

fn get(uri: &str, token: &str, org: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .body(Body::empty())
        .unwrap()
}

async fn seed_manager(pool: &PgPool, org: Uuid) -> String {
    let user = seed_user(pool).await;
    seed_membership(pool, org, user, "manager").await;
    mint_access_token(user, org, "manager")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_bookings_applies_filters_to_rows(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rlbk").await;
    let token = seed_manager(&pool, org).await;
    let unit = seed_unit(&pool, org, "RL-1").await;

    let confirmed = seed_booking(&pool, org, unit, "Alice", "airbnb", "confirmed").await;
    let cancelled = seed_booking(&pool, org, unit, "Bob", "booking", "cancelled").await;
    // A primary guest makes the summary decode `guest_registration_status`.
    seed_guest(&pool, org, confirmed, "Alice", "registered", true).await;

    // Unfiltered: both rows, with the primary guest's status decoded.
    let res = app
        .execute(get("/api/v1/rentals/bookings", &token, org))
        .await;
    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    let body = res.json_value();
    assert_eq!(body["total"], 2);
    let rows = body["bookings"].as_array().expect("bookings array");
    assert_eq!(rows.len(), 2);
    let alice = rows
        .iter()
        .find(|r| r["id"] == confirmed.to_string())
        .expect("confirmed booking listed");
    assert_eq!(alice["guest_registration_status"], "registered");
    assert_eq!(alice["unit_name"], "RL-1");

    // status filter narrows the rows, not just the total.
    let res = app
        .execute(get(
            "/api/v1/rentals/bookings?status=cancelled",
            &token,
            org,
        ))
        .await;
    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    let body = res.json_value();
    assert_eq!(body["total"], 1);
    let rows = body["bookings"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], cancelled.to_string());

    // platform filter (enum column) works too.
    let res = app
        .execute(get("/api/v1/rentals/bookings?platform=airbnb", &token, org))
        .await;
    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    let rows = res.json_value()["bookings"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], confirmed.to_string());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_guests_returns_org_guests_with_booking_context(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rlgs").await;
    let token = seed_manager(&pool, org).await;
    let unit = seed_unit(&pool, org, "RG-1").await;
    let booking = seed_booking(&pool, org, unit, "Carol", "direct", "confirmed").await;
    let pending = seed_guest(&pool, org, booking, "Carol", "pending", true).await;
    let registered = seed_guest(&pool, org, booking, "Dan", "registered", false).await;

    // Another org's guest must never be listed.
    let other_org = seed_org(&pool, "rlgs-other").await;
    let other_unit = seed_unit(&pool, other_org, "RG-X").await;
    let other_booking = seed_booking(
        &pool,
        other_org,
        other_unit,
        "Mallory",
        "direct",
        "confirmed",
    )
    .await;
    let foreign = seed_guest(&pool, other_org, other_booking, "Mallory", "pending", true).await;

    let res = app
        .execute(get("/api/v1/rentals/guests", &token, org))
        .await;
    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    let rows = res.json_value().as_array().expect("array").clone();
    let ids: Vec<String> = rows
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids.len(), 2, "rows: {rows:?}");
    assert!(ids.contains(&pending.to_string()));
    assert!(ids.contains(&registered.to_string()));
    assert!(
        !ids.contains(&foreign.to_string()),
        "cross-org guest leaked"
    );
    // Primary guest first, with booking context flattened alongside.
    assert_eq!(rows[0]["id"], pending.to_string());
    assert_eq!(rows[0]["booking_id"], booking.to_string());
    assert_eq!(rows[0]["booking_guest_name"], "Carol");
    assert_eq!(rows[0]["unit_name"], "RG-1");
    assert_eq!(rows[0]["booking_status"], "confirmed");
    assert_eq!(rows[0]["status"], "pending");
    assert!(rows[0]["check_in"].is_string());

    // status filter.
    let res = app
        .execute(get("/api/v1/rentals/guests?status=pending", &token, org))
        .await;
    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    let rows = res.json_value().as_array().unwrap().clone();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], pending.to_string());

    // booking_id filter scoped to org: the foreign booking yields nothing.
    let res = app
        .execute(get(
            &format!("/api/v1/rentals/guests?booking_id={other_booking}"),
            &token,
            org,
        ))
        .await;
    assert_eq!(res.status, StatusCode::OK, "body: {}", res.text());
    assert_eq!(res.json_value().as_array().unwrap().len(), 0);

    // Unknown status → 400.
    let res = app
        .execute(get("/api/v1/rentals/guests?status=bogus", &token, org))
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "body: {}", res.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_guests_is_manager_gated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "rlgg").await;
    let resident = seed_user(&pool).await;
    seed_membership(&pool, org, resident, "resident").await;
    let token = mint_access_token(resident, org, "resident");

    let res = app
        .execute(get("/api/v1/rentals/guests", &token, org))
        .await;
    assert_eq!(res.status, StatusCode::FORBIDDEN, "body: {}", res.text());
}

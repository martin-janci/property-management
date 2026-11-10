//! Regression tests for the cross-user IDOR fix on `GET /api/v1/bookings/{id}` (#852).
//!
//! Audit history: `get_booking` extracted `AuthUser` but discarded it
//! (`_auth`) and returned any facility booking by UUID. Any authenticated
//! user could read another user's booking (purpose, attendees, notes, times).
//!
//! The fix adds an owner check (`booking.user_id == auth.user_id`), mirroring
//! `update_booking` / `cancel_booking`. Managers read facility bookings via
//! the org-scoped `list_facility_bookings` instead.
//!
//! `AuthUser` decodes the bearer JWT without a DB lookup, so we mint access
//! tokens directly (matching `api_server`'s Claims shape). Only the booking
//! *owner* needs to be a real user row (FK); the caller does not.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::TestApp;
use std::sync::Once;

/// Must be the SAME secret the shared `TestConfig::default()` harness stamps
/// into `JWT_SECRET` (see `tests/common/mod.rs`). The `AuthUser` extractor's
/// `JWT_VERIFIER` is a process-global `OnceLock`: the first request in the test
/// binary caches a `DecodingKey` from whatever `JWT_SECRET` is set then, and
/// every later request in that process reuses it. These cases run inside the
/// bundled `suite_2` binary alongside ~400 tests that use the canonical secret,
/// so that secret wins the cache. Minting with a *different* key here made every
/// token fail signature validation → 401 instead of the expected 403/200 (#2906).
/// Keep this identical to the harness secret so tokens validate regardless of
/// which test initializes the verifier first.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

static SECRET_ONCE: Once = Once::new();

/// Stamp `JWT_SECRET` so the `AuthUser` extractor validates against the same
/// key our tokens are minted with. Must run before `TestApp::new`, which only
/// sets a default when the var is unset (CI does not export it). Sets the
/// canonical harness secret, so it is consistent with every other `suite_2`
/// test sharing the process-global `JWT_VERIFIER`.
fn ensure_jwt_secret() {
    SECRET_ONCE.call_once(|| std::env::set_var("JWT_SECRET", JWT_SECRET));
}

/// Mirrors `api_server`'s access-token Claims as consumed by the `AuthUser`
/// extractor (`token_type` must be "access"; email/name are required).
#[derive(Serialize)]
struct AccessClaims {
    sub: Uuid,
    iat: i64,
    exp: i64,
    token_type: String,
    email: String,
    name: String,
}

fn mint_access_token(user_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        email: format!("{user_id}@booking-idor.test"),
        name: "Booking IDOR User".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("BookingIDOR Org {slug}"))
    .bind(format!("booking-idor-org-{slug}"))
    .bind(format!("{slug}@booking-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Booking Owner', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_facility(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO facilities (building_id, name, facility_type, is_bookable)
           VALUES ($1, 'Test Gym', 'gym', TRUE) RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed facility")
}

async fn seed_booking(pool: &PgPool, facility_id: Uuid, user_id: Uuid) -> Uuid {
    let start = Utc::now() + Duration::days(1);
    let end = start + Duration::hours(2);
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO facility_bookings
             (facility_id, user_id, start_time, end_time, status, purpose, total_fee, deposit_paid)
           VALUES ($1, $2, $3, $4, 'approved', 'private meeting', 0, FALSE) RETURNING id"#,
    )
    .bind(facility_id)
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

fn get_with_token(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(Method::GET).uri(uri);
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    b.body(Body::empty()).unwrap()
}

/// Headline regression: a different authenticated user must NOT read another
/// user's booking. On dev `get_booking` ignored `_auth` and returned 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_booking_from_other_user_is_rejected(pool: PgPool) {
    ensure_jwt_secret();
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "x").await;
    let owner = seed_user(&pool, "owner@booking-idor.test").await;
    let building = seed_building(&pool, org, "idor").await;
    let facility = seed_facility(&pool, building).await;
    let booking = seed_booking(&pool, facility, owner).await;

    // A completely different user (need not exist in the DB).
    let attacker_token = mint_access_token(Uuid::new_v4());
    let resp = app
        .execute(get_with_token(
            &format!("/api/v1/bookings/{booking}"),
            Some(&attacker_token),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "another user reading a booking must get 403, got {}",
        resp.status
    );
}

/// The owner can still read their own booking (the fix must not over-restrict).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_booking_as_owner_is_allowed(pool: PgPool) {
    ensure_jwt_secret();
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "y").await;
    let owner = seed_user(&pool, "owner2@booking-idor.test").await;
    let building = seed_building(&pool, org, "owner").await;
    let facility = seed_facility(&pool, building).await;
    let booking = seed_booking(&pool, facility, owner).await;

    let owner_token = mint_access_token(owner);
    let resp = app
        .execute(get_with_token(
            &format!("/api/v1/bookings/{booking}"),
            Some(&owner_token),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "owner must be able to read their own booking, got {}",
        resp.status
    );
}

/// No token at all is rejected (4xx), never 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_booking_without_auth_is_rejected(pool: PgPool) {
    ensure_jwt_secret();
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "z").await;
    let owner = seed_user(&pool, "owner3@booking-idor.test").await;
    let building = seed_building(&pool, org, "noauth").await;
    let facility = seed_facility(&pool, building).await;
    let booking = seed_booking(&pool, facility, owner).await;

    let resp = app
        .execute(get_with_token(&format!("/api/v1/bookings/{booking}"), None))
        .await;

    let code = resp.status.as_u16();
    assert!(
        (400..500).contains(&code),
        "unauthenticated read must be 4xx, got {}",
        resp.status
    );
}

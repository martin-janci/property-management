//! Regression tests for the rental booking/guest PII manager-gate (#1766).
//!
//! Airbnb/Booking.com sync persists full guest PII (name, email, phone, stay
//! dates, amounts) into the rentals tables. The four read endpoints that return
//! that data were gated only by `TenantExtractor` (tenant-scoped / IDOR-safe per
//! #804) with NO manager-role check, so any plain `resident` org member could
//! enumerate the org's guest PII — the same threat #1741 closed on the live
//! Airbnb proxy, via the persisted path. The fix applies the canonical
//! `TenantRole::is_manager` gate (DB-role, not JWT claim) to all four:
//!
//!   * GET /api/v1/rentals/bookings
//!   * GET /api/v1/rentals/bookings/{id}
//!   * GET /api/v1/rentals/bookings/{id}/guests
//!   * GET /api/v1/rentals/guests/{id}
//!
//! For EACH route this pins:
//!   (a) a `resident` DB member — even with a forged `manager` JWT role claim —
//!       gets 403 (proves the gate reads the DB role, not the token; this FAILS
//!       on pre-fix code where the handlers extracted only `TenantExtractor` and
//!       returned 200/404);
//!   (b) a real `manager` DB member is NOT rejected by the gate (200 on the
//!       resources we seeded), proving the gate is role-scoped, not blanket-deny.
//!
//! Auth mirrors `rental_guest_id_document_tests.rs`: minted access JWT +
//! `X-Tenant-ID` header, with `organization_members` seeded so the manager gate
//! resolves a real DB role.

#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp};

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

/// Mint an access token. The JWT `role` claim is ALWAYS `manager` here so the
/// resident cases prove the gate ignores the (forgeable) JWT role and reads the
/// DB membership instead.
fn mint_access_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some("manager".to_string()),
        email: "rental-pii@test.local".to_string(),
        name: "Rental PII Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Rental PII User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
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

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation, floor)
        VALUES ($1, $2, 1) RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_booking(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings (
            organization_id, unit_id, platform, guest_name, check_in, check_out
        )
        VALUES ($1, $2, 'airbnb', 'Booking Guest', CURRENT_DATE, CURRENT_DATE + 2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

async fn seed_guest(pool: &PgPool, org_id: Uuid, booking_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_guests (
            organization_id, booking_id, first_name, last_name
        )
        VALUES ($1, $2, 'Jane', 'Doe')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(booking_id)
    .fetch_one(pool)
    .await
    .expect("seed guest")
}

/// A seeded org with one booking + guest, plus a member of the given `role`.
struct Scenario {
    org: Uuid,
    booking: Uuid,
    guest: Uuid,
    token: String,
}

/// Seed an org + a `role` member + one booking + one guest, returning the ids
/// and a token for that member. The token always carries a `manager` JWT claim
/// so a `resident` scenario exercises the DB-role gate, not the JWT.
async fn seed_scenario(pool: &PgPool, slug: &str, role: &str) -> Scenario {
    let org = seed_org(pool, slug).await;
    let user = seed_user(pool, &format!("{slug}-{}@pii.test", Uuid::new_v4())).await;
    seed_membership(pool, org, user, role).await;
    let building = seed_building(pool, org, slug).await;
    let unit = seed_unit(pool, building, &format!("{slug}-U1")).await;
    let booking = seed_booking(pool, org, unit).await;
    let guest = seed_guest(pool, org, booking).await;
    Scenario {
        org,
        booking,
        guest,
        token: mint_access_token(user, org),
    }
}

/// The four manager-gated read URIs for a given scenario.
fn read_uris(s: &Scenario) -> [String; 4] {
    [
        "/api/v1/rentals/bookings".to_string(),
        format!("/api/v1/rentals/bookings/{}", s.booking),
        format!("/api/v1/rentals/bookings/{}/guests", s.booking),
        format!("/api/v1/rentals/guests/{}", s.guest),
    ]
}

fn get_request(uri: &str, token: &str, org: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .body(Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// (a) resident DB member (with a forged manager JWT claim) -> 403 on ALL four
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_is_forbidden_on_all_pii_reads(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let s = seed_scenario(&pool, "pii-res", "resident").await;

    for uri in read_uris(&s) {
        let resp = app.execute(get_request(&uri, &s.token, s.org)).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "resident must be 403 on {uri}, got {}: {}",
            resp.status,
            resp.text()
        );
    }
}

// ---------------------------------------------------------------------------
// (b) manager DB member -> NOT 403 (200 on the seeded resources)
//
// Proves the gate is role-scoped, not a blanket deny that would also lock out
// managers.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_passes_gate_on_all_pii_reads(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let s = seed_scenario(&pool, "pii-mgr", "manager").await;

    for uri in read_uris(&s) {
        let resp = app.execute(get_request(&uri, &s.token, s.org)).await;
        assert_ne!(
            resp.status,
            StatusCode::FORBIDDEN,
            "manager must NOT be 403 on {uri} (gate is role-scoped), got 403: {}",
            resp.text()
        );
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "manager read of a seeded resource on {uri} should be 200, got {}: {}",
            resp.status,
            resp.text()
        );
    }
}

//! Integration tests for the Airbnb linked-listing list route (Coverage 83-1):
//! `GET /api/v1/integrations/organizations/{org_id}/airbnb/connections`.
//!
//! # What is tested
//!
//! - Rejects unauthenticated requests (401/403).
//! - IDOR guard: a caller who is NOT a member of the target org is rejected
//!   with 403 before any DB read.
//! - Returns 200 with an empty list when the org has no Airbnb connection
//!   (unlike the live-proxy `/airbnb/listings`, which returns 404).
//! - Returns the org's stored Airbnb connections, token-free, and is scoped to
//!   the path org (rows for other orgs / other platforms are not leaked).
//!
//! These tests run fully offline — the route never calls the Airbnb API.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp};

// Must match `TestConfig::default().jwt_secret`.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Mirror of `api_core::extractors::auth::Claims`.
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

fn mint_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some("manager".to_string()),
        email: "airbnb-conn-test@test.local".to_string(),
        name: "Airbnb Conn Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

// ---------------------------------------------------------------------------
// Database fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Airbnb Conn Test Org {tag}"))
    .bind(format!(
        "airbnb-conn-test-{tag}-{}",
        Uuid::new_v4().simple()
    ))
    .bind(format!("{tag}@airbnb-conn.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Airbnb Conn User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{tag} Street"))
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

/// Seed an Airbnb connection row for the given org/unit, returning its id.
async fn seed_airbnb_connection(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    listing_id: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (
            organization_id, unit_id, platform,
            access_token, external_property_id, is_active
        )
        VALUES ($1, $2, 'airbnb', 'test-plaintext-token', $3, true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(listing_id)
    .fetch_one(pool)
    .await
    .expect("seed airbnb connection")
}

/// Seed a non-Airbnb (booking) connection so we can assert platform scoping.
async fn seed_booking_connection(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (
            organization_id, unit_id, platform,
            access_token, external_property_id, is_active
        )
        VALUES ($1, $2, 'booking', 'booking-token', 'HOTEL-1', true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed booking connection")
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

fn authed_get(uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn anon_get(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn is_protected(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
    )
}

// ===========================================================================
// Tests
// ===========================================================================

/// Unauthenticated GET must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connections_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cx-unauth").await;
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/connections");
    let resp = app.execute(anon_get(&uri)).await;
    assert!(
        is_protected(resp.status),
        "unauthenticated connections request must be rejected; got {}",
        resp.status
    );
}

/// Non-member caller must be rejected with 403 (IDOR guard fires before DB read).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connections_idor_guard_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "cx-idor-a").await;
    let org_b = seed_org(&pool, "cx-idor-b").await;
    let user_b = seed_user(&pool, "cx-idor-b@test.local").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let token_b = mint_token(user_b, org_b);
    let uri = format!("/api/v1/integrations/organizations/{org_a}/airbnb/connections");
    let resp = app.execute(authed_get(&uri, &token_b)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member connections request must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Manager gate (#1626, regression guard for #1639): a member whose org role is
/// `resident` must be rejected with 403 even though they pass the membership IDOR
/// check. Mirrors the manager `200` path (`connections_returns_empty_list_when_none`).
/// The role is read from `organization_members.role_type`, not the JWT — so the
/// resident is rejected despite `mint_token` minting a `manager` claim.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connections_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cx-resident").await;
    let user_id = seed_user(&pool, "cx-resident@test.local").await;
    seed_membership(&pool, org_id, user_id, "resident").await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/connections");
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident member must be 403 (manager-level read); got {}: {}",
        resp.status,
        resp.text()
    );
}

/// With no Airbnb connection, the route returns 200 + empty list (not 404).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connections_returns_empty_list_when_none(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cx-empty").await;
    let user_id = seed_user(&pool, "cx-empty@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/connections");
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "empty connection list must return 200; got {}: {}",
        resp.status,
        resp.text()
    );
    let body: serde_json::Value = serde_json::from_str(&resp.text()).expect("parse body");
    assert_eq!(body["count"], 0);
    assert!(body["connections"].as_array().unwrap().is_empty());
}

/// Returns the org's stored Airbnb connections, token-free, scoped to the org
/// and the airbnb platform (no leakage of other orgs / other platforms).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connections_returns_org_scoped_airbnb_rows_without_tokens(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cx-list").await;
    let other_org = seed_org(&pool, "cx-list-other").await;
    let user_id = seed_user(&pool, "cx-list@test.local").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let building = seed_building(&pool, org_id, "Cx").await;
    let unit1 = seed_unit(&pool, building, "A1").await;
    let unit2 = seed_unit(&pool, building, "A2").await;
    seed_airbnb_connection(&pool, org_id, unit1, "LISTING-1").await;
    seed_airbnb_connection(&pool, org_id, unit2, "LISTING-2").await;
    // Same org, different platform — must be excluded.
    seed_booking_connection(&pool, org_id, unit1).await;
    // Different org — must not leak even though caller could in theory enumerate.
    let other_building = seed_building(&pool, other_org, "Other").await;
    let other_unit = seed_unit(&pool, other_building, "B1").await;
    seed_airbnb_connection(&pool, other_org, other_unit, "OTHER-LISTING").await;

    let token = mint_token(user_id, org_id);
    let uri = format!("/api/v1/integrations/organizations/{org_id}/airbnb/connections");
    let resp = app.execute(authed_get(&uri, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "connections list must return 200; got {}: {}",
        resp.status,
        resp.text()
    );

    let raw = resp.text();
    let body: serde_json::Value = serde_json::from_str(&raw).expect("parse body");
    assert_eq!(body["count"], 2, "only the org's 2 airbnb rows: {raw}");
    let conns = body["connections"].as_array().unwrap();
    assert_eq!(conns.len(), 2);

    let listing_ids: Vec<&str> = conns
        .iter()
        .map(|c| c["external_property_id"].as_str().unwrap())
        .collect();
    assert!(listing_ids.contains(&"LISTING-1"));
    assert!(listing_ids.contains(&"LISTING-2"));
    assert!(!listing_ids.contains(&"OTHER-LISTING"));

    for c in conns {
        assert_eq!(c["platform"], "airbnb");
        assert_eq!(c["organization_id"], org_id.to_string());
        // Secrets must never be serialised.
        assert!(
            c.get("access_token").is_none(),
            "leaked access_token: {raw}"
        );
        assert!(
            c.get("refresh_token").is_none(),
            "leaked refresh_token: {raw}"
        );
        assert_eq!(c["is_connected"], true);
    }
}

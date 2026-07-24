//! Regression tests for #887 / #804 — rental platform connection OAuth token
//! leak + cross-tenant IDOR.
//!
//! Audit history (origin/dev): `GET /api/v1/rentals/connections/{id}` had **no**
//! auth extractor and returned the raw `RentalPlatformConnection` row —
//! including `access_token` / `refresh_token`. Any caller with a connection
//! UUID could harvest OAuth tokens for any org. Sibling by-id reads
//! (`bookings/{id}`, `guests/{id}`) were also unauthenticated, and the
//! mutations extracted `TenantExtractor` but discarded `_tenant`, issuing
//! id-only repo calls (cross-tenant write/IDOR).
//!
//! The fix:
//!   1. Requires `TenantExtractor` (JWT + `X-Tenant-ID`) on every connection /
//!      booking / guest / report read+mutate handler.
//!   2. Routes by-id access through `*_for_org(org_id, id)` repo methods whose
//!      WHERE clause carries `AND organization_id = $org` (cross-tenant → 404).
//!   3. Returns a token-free `PlatformConnectionDetail` DTO from
//!      `GET/PUT /connections/{id}` so tokens are never serialized.
//!
//! Tenant mechanism: `TenantExtractor` validates a bearer JWT (`AuthUser`) and
//! then reads the tenant from the `X-Tenant-ID` header (TestApp does not wire
//! `host_tenant_middleware`). We mint a JWT carrying the `Claims` shape the
//! `AuthUser` extractor expects (`sub`, `exp`, `iat`, `token_type:"access"`,
//! `email`, `name`), and supply the org via `X-Tenant-ID`.

#![allow(dead_code)]

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use db::models::UpdateBookingStatus;
use db::repositories::RentalRepository;

use crate::common::TestApp;

// Must match `TestConfig::default().jwt_secret` in tests/common/mod.rs — the
// `AuthUser` extractor reads `JWT_SECRET` (set once from that value).
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Mirror of `api_core::extractors::auth::Claims` (the on-the-wire access-token
/// shape produced by `JwtService`).
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

fn mint_access_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some("manager".to_string()),
        email: "rental-idor@test.local".to_string(),
        name: "Rental IDOR Test".to_string(),
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

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Rental IDOR Org {slug}"))
    .bind(format!("rental-idor-org-{slug}"))
    .bind(format!("{slug}@rental-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Rental IDOR User', 'active', NOW())
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
    // `units` has no `organization_id`/`name` column — org is via the building,
    // and the unit identifier is `designation`.
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

/// Seed a connection with real OAuth tokens so we can assert they never leak.
async fn seed_connection(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (
            organization_id, unit_id, platform,
            access_token, refresh_token, is_active
        )
        VALUES ($1, $2, 'airbnb', $3, $4, true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(SECRET_ACCESS_TOKEN)
    .bind(SECRET_REFRESH_TOKEN)
    .fetch_one(pool)
    .await
    .expect("seed connection")
}

const SECRET_ACCESS_TOKEN: &str = "super-secret-access-token-DO-NOT-LEAK";
const SECRET_REFRESH_TOKEN: &str = "super-secret-refresh-token-DO-NOT-LEAK";

fn authed_get(uri: &str, token: &str, tenant: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("X-Tenant-ID", tenant.to_string())
        .body(Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// (a) cross-org caller cannot read another org's connection → 404
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_connection_from_other_org_is_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "get-conn-a").await;
    let org_b = seed_org(&pool, "get-conn-b").await;
    let user_b = seed_user(&pool, "get-conn-b@rental-idor.test").await;
    let building_a = seed_building(&pool, org_a, "get-conn-a").await;
    let unit_a = seed_unit(&pool, building_a, "A-1").await;
    let conn_in_a = seed_connection(&pool, org_a, unit_a).await;

    // Org B caller (valid JWT, X-Tenant-ID=org_b) tries to read org A's connection.
    let token_b = mint_access_token(user_b, org_b);
    let uri = format!("/api/v1/rentals/connections/{conn_in_a}");
    let resp = app.execute(authed_get(&uri, &token_b, org_b)).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org connection read must be 404, got {}: {}",
        resp.status,
        resp.text()
    );

    // And no secret leaks even in the error body.
    let body = resp.text();
    assert!(
        !body.contains(SECRET_ACCESS_TOKEN) && !body.contains(SECRET_REFRESH_TOKEN),
        "cross-org 404 body must not contain OAuth tokens: {body}"
    );
}

// ---------------------------------------------------------------------------
// (b) same-org SUCCESS case — proves real org context (must PASS)
// (c) response body does NOT contain the token fields
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_connection_same_org_succeeds_without_leaking_tokens(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "ok-conn-a").await;
    let user_a = seed_user(&pool, "ok-conn-a@rental-idor.test").await;
    let building_a = seed_building(&pool, org_a, "ok-conn-a").await;
    let unit_a = seed_unit(&pool, building_a, "OK-1").await;
    let conn_in_a = seed_connection(&pool, org_a, unit_a).await;

    let token_a = mint_access_token(user_a, org_a);
    let uri = format!("/api/v1/rentals/connections/{conn_in_a}");
    let resp = app.execute(authed_get(&uri, &token_a, org_a)).await;

    // (b) Same-org read must succeed — this proves the org context is real and
    //     the scoped query returns the row (not a false 404).
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "same-org connection read must be 200, got {}: {}",
        resp.status,
        resp.text()
    );

    let body = resp.text();

    // (c) The response must NOT contain the OAuth secret values, and the
    //     token-free DTO must not even carry the `access_token`/`refresh_token`
    //     keys.
    assert!(
        !body.contains(SECRET_ACCESS_TOKEN) && !body.contains(SECRET_REFRESH_TOKEN),
        "connection response leaked an OAuth token value: {body}"
    );

    let json: serde_json::Value = serde_json::from_str(&body).expect("response is JSON");
    assert!(
        json.get("access_token").is_none(),
        "response must not contain an access_token field: {body}"
    );
    assert!(
        json.get("refresh_token").is_none(),
        "response must not contain a refresh_token field: {body}"
    );
    // The token-free DTO still confirms a token is present via a boolean.
    assert_eq!(
        json.get("is_connected").and_then(|v| v.as_bool()),
        Some(true),
        "is_connected should reflect that a token is stored: {body}"
    );
    // And it correctly identifies the row.
    assert_eq!(
        json.get("id").and_then(|v| v.as_str()),
        Some(conn_in_a.to_string().as_str()),
        "response id must match the requested connection: {body}"
    );
}

// ---------------------------------------------------------------------------
// (a, extra) unauthenticated read is rejected (no JWT at all)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_connection_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "noauth-a").await;
    let building_a = seed_building(&pool, org_a, "noauth-a").await;
    let unit_a = seed_unit(&pool, building_a, "NA-1").await;
    let conn_in_a = seed_connection(&pool, org_a, unit_a).await;

    let uri = format!("/api/v1/rentals/connections/{conn_in_a}");
    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

    let code = resp.status.as_u16();
    assert!(
        (400..500).contains(&code),
        "unauthenticated connection read must be 4xx, got {}",
        resp.status
    );
    let body = resp.text();
    assert!(
        !body.contains(SECRET_ACCESS_TOKEN) && !body.contains(SECRET_REFRESH_TOKEN),
        "unauthenticated response must not contain OAuth tokens: {body}"
    );
}

// ---------------------------------------------------------------------------
// PAP-141: the legacy unauthenticated OAuth callbacks
// (`/api/v1/rentals/oauth/{airbnb,booking}/callback`, Story 98.2) were removed.
// They trusted a client-supplied connection id from the OAuth `state` parameter
// with no auth, no org check, and no CSRF-state validation — letting any caller
// bind tokens to another org's connection. The routes must no longer exist
// (Airbnb is served securely by Story 83.1 under `/api/v1/integrations/...`).
// ---------------------------------------------------------------------------

async fn assert_route_removed(app: &TestApp, uri: &str) {
    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "legacy OAuth callback route {uri} must be removed (expected 404), got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn legacy_rental_oauth_callback_routes_are_removed(pool: PgPool) {
    // The rentals router IS mounted here (the connection tests above hit
    // `/api/v1/rentals/connections/{id}`), so a 404 on the callback paths proves
    // the route itself is gone, not that the whole router is absent.
    let app = TestApp::new(pool.clone()).await;

    // A would-be attacker hits the callback with an arbitrary connection id in
    // `state` and a forged `code`. With the routes gone there is no handler to
    // bind tokens to a foreign org's connection.
    let forged_state = format!("{}:attacker-nonce", Uuid::new_v4());
    assert_route_removed(
        &app,
        &format!("/api/v1/rentals/oauth/airbnb/callback?code=forged&state={forged_state}"),
    )
    .await;
    assert_route_removed(
        &app,
        &format!("/api/v1/rentals/oauth/booking/callback?code=forged&state={forged_state}"),
    )
    .await;
}

// ---------------------------------------------------------------------------
// PAP-141: the Airbnb OTA webhook now cancels a booking via
// `update_booking_status_for_org(booking.organization_id, ...)`. Prove the
// org-keyed mutation refuses to touch a booking owned by a different org even
// on the superuser test pool (which bypasses FORCE RLS), so a forged/replayed
// webhook can never flip another org's booking.
// ---------------------------------------------------------------------------

async fn seed_booking(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    // `total_amount`/`platform_fee`/`cleaning_fee` are seeded non-null: the
    // `RentalBooking` model decodes them via `#[sqlx(try_from = "Decimal")]`,
    // which rejects SQL NULL — an unrelated pre-existing model quirk we sidestep
    // so this test exercises only the org-scope guard.
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings (
            organization_id, unit_id, platform, guest_name, check_in, check_out,
            total_amount, platform_fee, cleaning_fee
        )
        VALUES ($1, $2, 'airbnb', 'Guest', '2030-01-10', '2030-01-12', 200.00, 20.00, 30.00)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

async fn read_status(pool: &PgPool, booking: Uuid) -> String {
    sqlx::query_scalar::<_, String>("SELECT status::text FROM rental_bookings WHERE id = $1")
        .bind(booking)
        .fetch_one(pool)
        .await
        .expect("read status")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_booking_status_for_org_ignores_other_org_booking(pool: PgPool) {
    let repo = RentalRepository::new(pool.clone());

    let org_a = seed_org(&pool, "bk-a").await;
    let org_b = seed_org(&pool, "bk-b").await;
    let building_a = seed_building(&pool, org_a, "bk-a").await;
    let unit_a = seed_unit(&pool, building_a, "BK-1").await;
    let booking = seed_booking(&pool, org_a, unit_a).await;

    let cancel = UpdateBookingStatus {
        status: "cancelled".to_string(),
        cancellation_reason: Some("forged webhook".to_string()),
    };

    // Org B (attacker) cannot cancel org A's booking — no row matches the org
    // guard, so the mutation is a no-op and returns None.
    let cross = repo
        .update_booking_status_for_org(org_b, booking, cancel.clone())
        .await
        .expect("query ok");
    assert!(
        cross.is_none(),
        "cross-org booking status update must be a no-op (None)"
    );
    assert_eq!(
        read_status(&pool, booking).await,
        "pending",
        "cross-org update must not change the booking status"
    );

    // The owning org CAN cancel it (proves the guard is not a false negative).
    let owned = repo
        .update_booking_status_for_org(org_a, booking, cancel)
        .await
        .expect("query ok");
    assert!(owned.is_some(), "same-org status update must succeed");
    assert_eq!(
        read_status(&pool, booking).await,
        "cancelled",
        "same-org update must change the booking status"
    );
}

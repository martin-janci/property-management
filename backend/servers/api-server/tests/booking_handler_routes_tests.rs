//! Route-level coverage for the Booking.com channel-manager handlers
//! (Gap 83-2 follow-up: "No test coverage for any Booking.com handler").
//!
//! Endpoints under test (all in `routes/integrations/install.rs`):
//!
//! | Method | Path                                                        | Handler                     |
//! |--------|-------------------------------------------------------------|-----------------------------|
//! | GET    | `/organizations/{org_id}/booking/status`                     | `get_booking_status`        |
//! | POST   | `/organizations/{org_id}/booking/connect`                    | `connect_booking`           |
//! | POST   | `/organizations/{org_id}/booking/sync`                       | `sync_booking`              |
//! | DELETE | `/organizations/{org_id}/booking`                            | `disconnect_booking`        |
//! | POST   | `/organizations/{org_id}/booking/push-availability`          | `push_booking_availability` |
//! | POST   | `/organizations/{org_id}/booking/push-rates`                 | `push_booking_rates`        |
//!
//! Coverage: authentication (anon rejected), cross-org IDOR (membership gate
//! for status/connect/sync/disconnect; JWT-tenant gate for the push routes),
//! request validation (`NO_UPDATES`, `BATCH_TOO_LARGE`,
//! `INVALID_AVAILABLE_COUNT`), connection-state branches (no connection → 404,
//! `external_property_id` NULL → 400 `NOT_CONFIGURED`), disconnect credential
//! revocation, and no-credential-leak on the status surface.
//!
//! # Scope / known limitation (external Booking API)
//!
//! `BookingClient` posts to a compile-time-constant Supply-XML host that is
//! not injectable (the same limitation documented in
//! `booking_oauth_routes_tests.rs` and `booking_connect_encryption_tests.rs`).
//! Every assertion in this file therefore stops *before* an outbound call is
//! made:
//!
//! - `connect` happy path (unconditional `fetch_property`) is NOT exercised —
//!   its encrypt-at-rest contract is covered in
//!   `booking_connect_encryption_tests.rs`;
//! - `sync` / `push-*` are exercised up to the 404 (no connection) and 400
//!   (`NOT_CONFIGURED`, hotel id missing) branches, which return before the
//!   client is used.
//!
//! Making the base URL injectable (wiremock happy paths) is tracked as the
//! `oauth_common` refactor in #1374 and remains out of scope here.

#![allow(dead_code)]

mod common;

use axum::http::{Method, StatusCode};
use chrono::{Duration, Utc};
use integrations::{encrypt_required, IntegrationCrypto};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp};

// Must match `TestConfig::default().jwt_secret`.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Deterministic AES-256 test key (never used in production).
const TEST_KEY_HEX: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

const HOTEL_ID: &str = "9876543";
const PLAINTEXT_USERNAME: &str = "booking-supply-user";
const PLAINTEXT_PASSWORD: &str = "sup3r-s3cret-supply-xml-passw0rd";

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
        email: "booking-handler-test@test.local".to_string(),
        name: "Booking Handler Test".to_string(),
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
    .bind(format!("Booking Handler Test Org {tag}"))
    .bind(format!(
        "booking-handler-test-{tag}-{}",
        Uuid::new_v4().simple()
    ))
    .bind(format!("{tag}@booking-handler.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Booking Handler User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a building + unit for `org_id` (the `rental_platform_connections`
/// table has a NOT NULL FK on `unit_id`). Returns the unit id.
async fn seed_unit(pool: &PgPool, org_id: Uuid) -> Uuid {
    let building_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
           VALUES ($1, 'Booking Test Building', 'Street 1', 'City', '00000', 'Country', 'active')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building");

    sqlx::query_scalar(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, 'A1', 1) RETURNING id",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Insert a Booking.com connection row for the org, storing encrypted
/// credentials exactly as `connect_booking` does (BIT-99 envelope).
///
/// `hotel_id = None` produces the `NOT_CONFIGURED` state the sync/push
/// handlers guard against.
async fn insert_booking_connection(pool: &PgPool, org_id: Uuid, hotel_id: Option<&str>) -> Uuid {
    let unit_id = seed_unit(pool, org_id).await;
    let crypto = IntegrationCrypto::new(TEST_KEY_HEX).expect("valid test key");
    let username = encrypt_required(Some(&crypto), PLAINTEXT_USERNAME).expect("encrypt username");
    let password = encrypt_required(Some(&crypto), PLAINTEXT_PASSWORD).expect("encrypt password");

    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO rental_platform_connections (
               organization_id, unit_id, platform, external_property_id,
               access_token, refresh_token, is_active
           )
           VALUES ($1, $2, 'booking', $3, $4, $5, true)
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(hotel_id)
    .bind(&username)
    .bind(&password)
    .fetch_one(pool)
    .await
    .expect("insert booking connection")
}

/// Seed an org with a manager member and return `(org_id, token)`.
async fn seed_org_with_manager(pool: &PgPool, tag: &str) -> (Uuid, String) {
    let org_id = seed_org(pool, tag).await;
    let user_id = seed_user(pool, &format!("{tag}@booking-handler.test")).await;
    seed_membership(pool, org_id, user_id, "manager").await;
    (org_id, mint_token(user_id, org_id))
}

// ---------------------------------------------------------------------------
// URI helpers
// ---------------------------------------------------------------------------

fn status_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking/status")
}
fn connect_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking/connect")
}
fn sync_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking/sync")
}
fn disconnect_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking")
}
fn push_availability_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking/push-availability")
}
fn push_rates_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/booking/push-rates")
}

fn one_availability_update() -> serde_json::Value {
    json!({
        "room_mappings": [],
        "updates": [{
            "room_type_id": "DBL",
            "date": "2026-08-01",
            "available_count": 2
        }]
    })
}

fn one_rate_update() -> serde_json::Value {
    json!({
        "updates": [{
            "room_type_id": "DBL",
            "rate_plan_code": "STD",
            "date": "2026-08-01",
            "base_rate": "129.00",
            "currency": "EUR"
        }]
    })
}

/// `protected` = rejected in the auth range, not 404/405/2xx.
fn is_protected(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
    )
}

// ===========================================================================
// Route-mounting smoke — all six Booking.com routes must be wired
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn all_booking_routes_are_mounted(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = Uuid::new_v4(); // arbitrary — we want an auth 4xx, not 404

    let surfaces: Vec<(Method, String)> = vec![
        (Method::GET, status_uri(org_id)),
        (Method::POST, connect_uri(org_id)),
        (Method::POST, sync_uri(org_id)),
        (Method::DELETE, disconnect_uri(org_id)),
        (Method::POST, push_availability_uri(org_id)),
        (Method::POST, push_rates_uri(org_id)),
    ];

    for (method, uri) in surfaces {
        let builder = match method {
            Method::GET => app.get(&uri),
            Method::POST => app.post(&uri).json(json!({})),
            Method::DELETE => app.delete(&uri),
            _ => unreachable!(),
        };
        let resp = builder.send().await;
        assert_ne!(
            resp.status,
            StatusCode::NOT_FOUND,
            "{method} {uri} must be mounted (got 404 — router wiring broken)"
        );
        assert_ne!(
            resp.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "{method} {uri} must accept this method (got 405)"
        );
    }
}

// ===========================================================================
// GET /booking/status
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn status_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "st-anon").await;
    let resp = app.get(&status_uri(org_id)).send().await;
    assert!(
        is_protected(resp.status),
        "anonymous booking status must be rejected; got {}",
        resp.status
    );
}

/// IDOR: a member of org B must not read org A's Booking.com connection state.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn status_idor_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "st-idor-a").await;
    let (_org_b, token_b) = seed_org_with_manager(&pool, "st-idor-b").await;
    // Give org A a real connection so a leak would carry actual data.
    insert_booking_connection(&pool, org_a, Some(HOTEL_ID)).await;

    let resp = app.get(&status_uri(org_a)).bearer(&token_b).send().await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member must not read another org's booking status; got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn status_without_connection_reports_disconnected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "st-none").await;

    let resp = app.get(&status_uri(org_id)).bearer(&token).send().await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: serde_json::Value = resp.json();
    assert_eq!(body["connected"], false);
    assert!(body["hotel_id"].is_null());
    assert_eq!(body["properties_count"], 0);
}

/// With an active connection the status endpoint reports it — and must never
/// leak the stored credentials (neither plaintext nor the `enc:` ciphertext).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn status_with_connection_reports_connected_without_leaking_credentials(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "st-conn").await;
    insert_booking_connection(&pool, org_id, Some(HOTEL_ID)).await;

    let resp = app.get(&status_uri(org_id)).bearer(&token).send().await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: serde_json::Value = resp.json();
    assert_eq!(body["connected"], true);
    assert_eq!(body["hotel_id"], HOTEL_ID);
    assert_eq!(body["properties_count"], 1);

    let text = resp.text();
    assert!(
        !text.contains(PLAINTEXT_USERNAME) && !text.contains(PLAINTEXT_PASSWORD),
        "status response must not leak Supply-XML credentials: {text}"
    );
    assert!(
        !text.contains("enc:"),
        "status response must not leak the credential ciphertext envelope: {text}"
    );
}

// ===========================================================================
// POST /booking/connect
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "cn-anon").await;
    let resp = app
        .post(&connect_uri(org_id))
        .json(json!({"hotel_id": HOTEL_ID, "username": "u", "password": "p"}))
        .send()
        .await;
    assert!(
        is_protected(resp.status),
        "anonymous booking connect must be rejected; got {}",
        resp.status
    );
}

/// IDOR: the membership gate must fire before the external credential
/// validation call AND before anything is persisted — a non-member's forged
/// `org_id` must leave zero connection rows behind.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_idor_rejects_non_member_and_persists_nothing(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "cn-idor-a").await;
    let (_org_b, token_b) = seed_org_with_manager(&pool, "cn-idor-b").await;

    let resp = app
        .post(&connect_uri(org_a))
        .bearer(&token_b)
        .json(json!({"hotel_id": HOTEL_ID, "username": "u", "password": "p"}))
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member must not connect another org's Booking.com account; got {}: {}",
        resp.status,
        resp.text()
    );

    let rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rental_platform_connections
         WHERE organization_id = $1 AND platform = 'booking'",
    )
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .expect("count connections");
    assert_eq!(
        rows, 0,
        "rejected connect must not persist a connection row"
    );
}

// NOTE: the connect happy path is intentionally NOT covered here — the handler
// makes an unconditional outbound `fetch_property` call to the compile-time
// Booking Supply-XML host before persisting. The encrypt-at-rest contract of
// the store step is pinned in `booking_connect_encryption_tests.rs`.

// ===========================================================================
// POST /booking/sync
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "sy-anon").await;
    let resp = app.post(&sync_uri(org_id)).send().await;
    assert!(
        is_protected(resp.status),
        "anonymous booking sync must be rejected; got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_idor_rejects_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "sy-idor-a").await;
    let (_org_b, token_b) = seed_org_with_manager(&pool, "sy-idor-b").await;
    insert_booking_connection(&pool, org_a, Some(HOTEL_ID)).await;

    let resp = app.post(&sync_uri(org_a)).bearer(&token_b).send().await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member must not trigger another org's sync; got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_without_connection_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "sy-none").await;

    let resp = app.post(&sync_uri(org_id)).bearer(&token).send().await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "sync without a connection must 404; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(resp.text().contains("NOT_FOUND"), "body: {}", resp.text());
}

/// A connection row without `external_property_id` must fail fast with 400
/// `NOT_CONFIGURED` — before any credential decryption or upstream call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_with_unconfigured_hotel_id_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "sy-nocfg").await;
    insert_booking_connection(&pool, org_id, None).await;

    let resp = app.post(&sync_uri(org_id)).bearer(&token).send().await;
    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "sync with missing hotel id must 400; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("NOT_CONFIGURED"),
        "body: {}",
        resp.text()
    );
}

// ===========================================================================
// DELETE /booking (disconnect)
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dc-anon").await;
    let resp = app.delete(&disconnect_uri(org_id)).send().await;
    assert!(
        is_protected(resp.status),
        "anonymous booking disconnect must be rejected; got {}",
        resp.status
    );
}

/// IDOR: a non-member must not be able to revoke another org's connection —
/// the stored credentials must survive the rejected call untouched.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_idor_rejects_non_member_and_keeps_credentials(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "dc-idor-a").await;
    let (_org_b, token_b) = seed_org_with_manager(&pool, "dc-idor-b").await;
    let conn_id = insert_booking_connection(&pool, org_a, Some(HOTEL_ID)).await;

    let resp = app
        .delete(&disconnect_uri(org_a))
        .bearer(&token_b)
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-member must not disconnect another org's Booking.com; got {}: {}",
        resp.status,
        resp.text()
    );

    let (is_active, has_token): (bool, bool) = sqlx::query_as(
        "SELECT is_active, access_token IS NOT NULL
         FROM rental_platform_connections WHERE id = $1",
    )
    .bind(conn_id)
    .fetch_one(&pool)
    .await
    .expect("re-read connection");
    assert!(is_active, "rejected disconnect must not deactivate the row");
    assert!(has_token, "rejected disconnect must not clear credentials");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_without_connection_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "dc-none").await;

    let resp = app
        .delete(&disconnect_uri(org_id))
        .bearer(&token)
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "disconnect without a connection must 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Happy path: disconnect returns 204, clears the stored credentials, marks
/// the row inactive, and the status surface flips to disconnected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_revokes_credentials_and_flips_status(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "dc-ok").await;
    let conn_id = insert_booking_connection(&pool, org_id, Some(HOTEL_ID)).await;

    let resp = app
        .delete(&disconnect_uri(org_id))
        .bearer(&token)
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "disconnect must return 204; got {}: {}",
        resp.status,
        resp.text()
    );

    // Credentials cleared + row deactivated (revoke_booking_connection contract).
    let (is_active, access_token, refresh_token): (bool, Option<String>, Option<String>) =
        sqlx::query_as(
            "SELECT is_active, access_token, refresh_token
             FROM rental_platform_connections WHERE id = $1",
        )
        .bind(conn_id)
        .fetch_one(&pool)
        .await
        .expect("re-read connection");
    assert!(!is_active, "disconnect must deactivate the connection");
    assert!(
        access_token.is_none() && refresh_token.is_none(),
        "disconnect must clear the stored Supply-XML credentials"
    );

    // Status surface now reports disconnected.
    let resp = app.get(&status_uri(org_id)).bearer(&token).send().await;
    assert_eq!(resp.status, StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert_eq!(body["connected"], false);
}

// ===========================================================================
// POST /booking/push-availability
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_availability_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "pa-anon").await;
    let resp = app
        .post(&push_availability_uri(org_id))
        .json(one_availability_update())
        .send()
        .await;
    assert!(
        is_protected(resp.status),
        "anonymous availability push must be rejected; got {}",
        resp.status
    );
}

/// Cross-tenant gate: the push routes authorize on the JWT `tenant_id` claim.
/// A caller whose token is scoped to org B must get 403 for org A's path —
/// before validation, connection lookup, or any upstream call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_availability_rejects_cross_tenant_caller(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pa-xt-a").await;
    let (org_b, token_b) = seed_org_with_manager(&pool, "pa-xt-b").await;
    insert_booking_connection(&pool, org_a, Some(HOTEL_ID)).await;

    let resp = app
        .post(&push_availability_uri(org_a))
        .bearer(&token_b)
        .tenant(org_b)
        .json(one_availability_update())
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "org-B-scoped token must not push availability to org A; got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_availability_rejects_empty_updates(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pa-empty").await;

    let resp = app
        .post(&push_availability_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({"room_mappings": [], "updates": []}))
        .send()
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert!(resp.text().contains("NO_UPDATES"), "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_availability_rejects_negative_available_count(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pa-neg").await;

    let resp = app
        .post(&push_availability_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({
            "room_mappings": [],
            "updates": [{
                "room_type_id": "DBL",
                "date": "2026-08-01",
                "available_count": -1
            }]
        }))
        .send()
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert!(
        resp.text().contains("INVALID_AVAILABLE_COUNT"),
        "body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_availability_rejects_oversized_batch(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pa-big").await;

    // 501 entries — one over the shared MAX_BATCH_SIZE cap of 500.
    let updates: Vec<serde_json::Value> = (0..501)
        .map(|i| {
            json!({
                "room_type_id": "DBL",
                "date": "2026-08-01",
                "available_count": i % 3
            })
        })
        .collect();
    let resp = app
        .post(&push_availability_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({"room_mappings": [], "updates": updates}))
        .send()
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert!(
        resp.text().contains("BATCH_TOO_LARGE"),
        "body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_availability_without_connection_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pa-none").await;

    let resp = app
        .post(&push_availability_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(one_availability_update())
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "push without a connection must 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_availability_with_unconfigured_hotel_id_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pa-nocfg").await;
    insert_booking_connection(&pool, org_id, None).await;

    let resp = app
        .post(&push_availability_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(one_availability_update())
        .send()
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert!(
        resp.text().contains("NOT_CONFIGURED"),
        "body: {}",
        resp.text()
    );
}

// ===========================================================================
// POST /booking/push-rates
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_rates_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "pr-anon").await;
    let resp = app
        .post(&push_rates_uri(org_id))
        .json(one_rate_update())
        .send()
        .await;
    assert!(
        is_protected(resp.status),
        "anonymous rates push must be rejected; got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_rates_rejects_cross_tenant_caller(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pr-xt-a").await;
    let (org_b, token_b) = seed_org_with_manager(&pool, "pr-xt-b").await;
    insert_booking_connection(&pool, org_a, Some(HOTEL_ID)).await;

    let resp = app
        .post(&push_rates_uri(org_a))
        .bearer(&token_b)
        .tenant(org_b)
        .json(one_rate_update())
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "org-B-scoped token must not push rates to org A; got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_rates_rejects_empty_updates(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pr-empty").await;

    let resp = app
        .post(&push_rates_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({"updates": []}))
        .send()
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert!(resp.text().contains("NO_UPDATES"), "body: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_rates_rejects_oversized_batch(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pr-big").await;

    let updates: Vec<serde_json::Value> = (0..501)
        .map(|_| {
            json!({
                "room_type_id": "DBL",
                "rate_plan_code": "STD",
                "date": "2026-08-01",
                "base_rate": "129.00",
                "currency": "EUR"
            })
        })
        .collect();
    let resp = app
        .post(&push_rates_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({"updates": updates}))
        .send()
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert!(
        resp.text().contains("BATCH_TOO_LARGE"),
        "body: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_rates_without_connection_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pr-none").await;

    let resp = app
        .post(&push_rates_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(one_rate_update())
        .send()
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "push without a connection must 404; got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_rates_with_unconfigured_hotel_id_returns_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (org_id, token) = seed_org_with_manager(&pool, "pr-nocfg").await;
    insert_booking_connection(&pool, org_id, None).await;

    let resp = app
        .post(&push_rates_uri(org_id))
        .bearer(&token)
        .tenant(org_id)
        .json(one_rate_update())
        .send()
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert!(
        resp.text().contains("NOT_CONFIGURED"),
        "body: {}",
        resp.text()
    );
}

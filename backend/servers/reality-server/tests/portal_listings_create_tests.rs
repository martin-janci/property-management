//! Happy-path tests for portal listing creation (POST /api/v1/my/listings).
//!
//! Closes the coverage gap noted in `docs/endpoint-checklist/groups/reality-server.md`:
//! the existing `portal_listings_idor_tests.rs` covers only the 400-validation and
//! IDOR paths. This file adds the successful create path (201 Created).
//!
//! Uses `PortalPrincipal` — seeded users must have `principal_kind = 'public'`.

mod common;

use axum::{http::Method, Router};
use common::{make_app_state, mint_token, send, send_json};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn portal_listings_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/my/listings", routes::portal_listings::router())
        .with_state(state)
}

async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@portal-listings-create.test"))
    .bind(format!("PortalCreate {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({tag}): {e}"))
}

// ── create_listing auth boundary ─────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_listing_unauthenticated_returns_401(pool: PgPool) {
    let app = portal_listings_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/my/listings",
        None,
        json!({
            "title": "Test Listing",
            "propertyType": "apartment",
            "transactionType": "sale",
            "price": "150000",
            "street": "Main St 1",
            "city": "Bratislava",
            "postalCode": "81101"
        }),
    )
    .await;
    assert_eq!(
        status, 401,
        "create_listing must return 401 without auth"
    );
}

// ── create_listing happy path ─────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_listing_authenticated_returns_201(pool: PgPool) {
    let user = seed_portal_user(&pool, "create").await;
    let token = mint_token(user);
    let app = portal_listings_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/my/listings",
        Some(&token),
        json!({
            "title": "My Test Apartment",
            "description": "A nice place",
            "propertyType": "apartment",
            "transactionType": "sale",
            "price": "150000.00",
            "currency": "EUR",
            "street": "Main Street 1",
            "city": "Bratislava",
            "postalCode": "81101",
            "country": "SK",
            "sizeSqm": "65.5",
            "rooms": 3,
            "floor": 2,
            "totalFloors": 8
        }),
    )
    .await;
    assert_eq!(
        status, 201,
        "authenticated create_listing with valid body must return 201"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_listing_authenticated_minimal_body_returns_201(pool: PgPool) {
    let user = seed_portal_user(&pool, "create-min").await;
    let token = mint_token(user);
    let app = portal_listings_router(pool);
    // Only required fields
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/my/listings",
        Some(&token),
        json!({
            "title": "Minimal Listing",
            "propertyType": "house",
            "transactionType": "rent",
            "price": "1200.00",
            "street": "Side Lane 5",
            "city": "Košice",
            "postalCode": "04001"
        }),
    )
    .await;
    assert_eq!(
        status, 201,
        "authenticated create_listing with minimal required body must return 201"
    );
}

// ── create_listing validation guards ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_listing_invalid_property_type_returns_400(pool: PgPool) {
    let user = seed_portal_user(&pool, "create-bad-type").await;
    let token = mint_token(user);
    let app = portal_listings_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/my/listings",
        Some(&token),
        json!({
            "title": "Bad Type",
            "propertyType": "yacht",
            "transactionType": "sale",
            "price": "99999",
            "street": "Harbor 1",
            "city": "Bratislava",
            "postalCode": "81101"
        }),
    )
    .await;
    assert_eq!(
        status, 400,
        "create_listing with invalid propertyType must return 400"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_listing_invalid_transaction_type_returns_400(pool: PgPool) {
    let user = seed_portal_user(&pool, "create-bad-txn").await;
    let token = mint_token(user);
    let app = portal_listings_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/my/listings",
        Some(&token),
        json!({
            "title": "Bad Txn",
            "propertyType": "apartment",
            "transactionType": "lease",
            "price": "99999",
            "street": "Test St 1",
            "city": "Bratislava",
            "postalCode": "81101"
        }),
    )
    .await;
    assert_eq!(
        status, 400,
        "create_listing with invalid transactionType must return 400"
    );
}

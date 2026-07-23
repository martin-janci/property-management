//! Happy-path 2xx tests for the inquiries surface
//! (`reality-server/src/routes/inquiries.rs`) — BIT-344 Wave 6.
//!
//! Complements `inquiries_authz_tests.rs` (401/403 only).
//!
//! Two auth shapes here:
//! - `send_contact_message` / `request_viewing` are public; they look up the
//!   listing's `created_by` as the inquiry's `realtor_id`, so a seeded `active`
//!   listing is enough (FK → `users` after migration 00148).
//! - the listing endpoints (`list_my_inquiries`, `get_inquiry`, …) use
//!   `RequestPrincipal`; the caller is seeded `principal_kind = 'platform'` to
//!   clear the tenant gate on the tenant-less test router, and owned inquiry
//!   rows are seeded with `realtor_id` = that user.

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn inquiries_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/inquiries", routes::inquiries::router())
        .with_state(state)
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@inq-hp.test"))
    .bind(format!("InqHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

/// Seed an `active` listing owned by `owner` and return its id.
async fn seed_listing(pool: &PgPool, tag: &str, owner: Uuid) -> Uuid {
    let org = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Inq Org {tag}"))
    .bind(format!("inq-org-{tag}"))
    .bind(format!("{tag}@inq-org.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_org({tag}): {e}"));

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Inquiry Listing', 'apartment', 200000.00, 'EUR',
                'Test Street 3', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_listing({tag}): {e}"))
}

/// Seed an inquiry addressed to `realtor` (the authenticated caller).
async fn seed_inquiry(pool: &PgPool, listing_id: Uuid, realtor: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, name, email, message)
        VALUES ($1, $2, 'Buyer', 'buyer@inq-hp.test', 'Is this available?')
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(realtor)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_inquiry: {e}"))
}

// ── send_contact_message (POST /contact/{listing_id}) ────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn send_contact_message_returns_2xx(pool: PgPool) {
    let owner = seed_user(&pool, "contact-owner").await;
    let listing_id = seed_listing(&pool, "contact", owner).await;
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/contact/{listing_id}"),
        None,
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com",
            "message": "Is this property still available?"
        }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── request_viewing (POST /viewing/{listing_id}) ─────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_viewing_returns_2xx(pool: PgPool) {
    let owner = seed_user(&pool, "viewing-owner").await;
    let listing_id = seed_listing(&pool, "viewing", owner).await;
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/viewing/{listing_id}"),
        None,
        json!({
            "name": "Jane Buyer",
            "email": "jane@example.com"
        }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_my_inquiries (GET /) ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_inquiries_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list-realtor").await;
    let token = mint_token(user);
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_buyer_inquiries (GET /mine) ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_buyer_inquiries_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list-buyer").await;
    let token = mint_token(user);
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries/mine", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_inquiry (GET /{id}) ──────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_inquiry_returns_2xx(pool: PgPool) {
    let realtor = seed_user(&pool, "get-realtor").await;
    let listing_id = seed_listing(&pool, "get", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/{inquiry_id}"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── mark_as_read (PUT /{id}/read) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_as_read_returns_2xx(pool: PgPool) {
    let realtor = seed_user(&pool, "read-realtor").await;
    let listing_id = seed_listing(&pool, "read", realtor).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor).await;
    let token = mint_token(realtor);
    let app = inquiries_router(pool);
    let status = send(
        &app,
        Method::PUT,
        &format!("/api/v1/inquiries/{inquiry_id}/read"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

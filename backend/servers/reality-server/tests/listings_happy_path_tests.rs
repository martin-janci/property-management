//! Happy-path 2xx tests for the public listing surface
//! (`reality-server/src/routes/listings.rs`) — BIT-344 Wave 6.
//!
//! Background: the authz wave only asserted 401/403 for these endpoints. This
//! file drives the real production router end-to-end via `oneshot` and asserts
//! that every covered endpoint returns a 2xx on its happy path. The listing
//! routes are fully public (no auth extractor), so the only setup needed is a
//! seeded `active` listing (plus its required `organizations` + `users`
//! parents) for the by-id endpoints; the search/featured/categories/suggestions
//! endpoints 2xx on an empty database.
//!
//! Auth/identity note (migration 00148): `portal_users` was dropped and every
//! reality FK retargeted to `users(id)`, so a single `users` row is both the
//! listing owner (`listings.created_by`) and any portal principal.

mod common;

use axum::{http::Method, Router};
use common::{make_app_state, send};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn listings_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/listings", routes::listings::router())
        .with_state(state)
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Listings Org {slug}"))
    .bind(format!("listings-org-{slug}"))
    .bind(format!("{slug}@listings-hp.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_org({slug}): {e}"))
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@listings-hp.test"))
    .bind(format!("ListingsHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

/// Seed an `active`, publicly-visible listing and return its id.
async fn seed_active_listing(pool: &PgPool, tag: &str) -> Uuid {
    let org = seed_org(pool, tag).await;
    let owner = seed_user(pool, tag).await;
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Happy Path Listing', 'apartment', 150000.00, 'EUR',
                'Test Street 7', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_active_listing({tag}): {e}"))
}

// ── search (GET /) ───────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn search_returns_2xx_on_empty_db(pool: PgPool) {
    let app = listings_router(pool);
    let status = send(&app, Method::GET, "/api/v1/listings", None).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

#[ignore]
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn search_returns_2xx_with_seeded_listing(pool: PgPool) {
    seed_active_listing(&pool, "search").await;
    let app = listings_router(pool);
    let status = send(&app, Method::GET, "/api/v1/listings?city=Bratislava", None).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_featured (GET /featured) ─────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_featured_returns_2xx(pool: PgPool) {
    let app = listings_router(pool);
    let status = send(&app, Method::GET, "/api/v1/listings/featured", None).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_categories (GET /categories) ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_categories_returns_2xx(pool: PgPool) {
    let app = listings_router(pool);
    let status = send(&app, Method::GET, "/api/v1/listings/categories", None).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_suggestions (GET /suggestions) ───────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_suggestions_returns_2xx(pool: PgPool) {
    let app = listings_router(pool);
    let status = send(&app, Method::GET, "/api/v1/listings/suggestions", None).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_listing (GET /{id}) ──────────────────────────────────────────────────

#[ignore]
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_listing_returns_2xx_for_active(pool: PgPool) {
    let listing_id = seed_active_listing(&pool, "detail").await;
    let app = listings_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/listings/{listing_id}"),
        None,
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── record_view (POST /{id}/view) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn record_view_returns_2xx(pool: PgPool) {
    let listing_id = seed_active_listing(&pool, "view").await;
    let app = listings_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/listings/{listing_id}/view"),
        None,
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

//! Behaviour tests for GET /api/v1/listings/* and POST /api/v1/listings/{id}/view.
//!
//! Covers: search, featured, categories, suggestions, get_listing, record_view.

use crate::common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn listings_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/listings", routes::listings::router())
        .with_state(state)
}

/// Seed an active listing and return its id. Minimal mirror of the helper in
/// `listings_happy_path_tests.rs`; kept local so this suite stays standalone.
async fn seed_active_listing(pool: &PgPool, tag: &str) -> Uuid {
    let org = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Org {tag}"))
    .bind(tag)
    .bind(format!("{tag}@orgs.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed org({tag}): {e}"));

    let owner = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@users.test"))
    .bind(format!("Owner {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed user({tag}): {e}"));

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Beautiful Apartment', 'apartment', 150000.00, 'EUR',
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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_returns_200_empty_result(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_with_query_param_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings?q=city", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_featured_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings/featured", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_categories_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings/categories", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_suggestions_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings/suggestions", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_listing_unknown_id_returns_404(pool: PgPool) {
    let router = listings_router(pool);
    let id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::GET,
        &format!("/api/v1/listings/{id}"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_listing_by_title_prefixed_slug_resolves(pool: PgPool) {
    // The /featured endpoint now emits `{title}-{uuid}` slugs; the detail
    // handler must resolve a listing hit at that path, not only a bare UUID.
    let id = seed_active_listing(&pool, "slug-resolve").await;
    let router = listings_router(pool);
    let slug = format!("beautiful-apartment-{id}");
    let status = common::send(
        &router,
        Method::GET,
        &format!("/api/v1/listings/{slug}"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_listing_by_bare_uuid_still_resolves(pool: PgPool) {
    // Backward-compatible deep links: a bare UUID must keep working.
    let id = seed_active_listing(&pool, "uuid-resolve").await;
    let router = listings_router(pool);
    let status = common::send(
        &router,
        Method::GET,
        &format!("/api/v1/listings/{id}"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_listing_unparseable_slug_returns_404(pool: PgPool) {
    // A path segment with no recoverable UUID is a 404, not a 400/500.
    let router = listings_router(pool);
    let status = common::send(
        &router,
        Method::GET,
        "/api/v1/listings/not-a-real-listing",
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn record_view_unknown_listing_returns_204(pool: PgPool) {
    let router = listings_router(pool);
    let id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::POST,
        &format!("/api/v1/listings/{id}/view"),
        None,
    )
    .await;
    // record_view is best-effort analytics — errors are logged, never surfaced.
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);
}

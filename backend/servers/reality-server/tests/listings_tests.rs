//! Behaviour tests for GET /api/v1/listings/* and POST /api/v1/listings/{id}/view.
//!
//! Covers: search, featured, categories, suggestions, get_listing, record_view.

mod common;

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

#[sqlx::test(migrations = "../../db/migrations")]
async fn search_returns_200_empty_result(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrations = "../../db/migrations")]
async fn search_with_query_param_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings?q=city", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrations = "../../db/migrations")]
async fn get_featured_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings/featured", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrations = "../../db/migrations")]
async fn get_categories_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings/categories", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrations = "../../db/migrations")]
async fn get_suggestions_returns_200(pool: PgPool) {
    let router = listings_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/listings/suggestions", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrations = "../../db/migrations")]
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

#[sqlx::test(migrations = "../../db/migrations")]
async fn record_view_unknown_listing_returns_404(pool: PgPool) {
    let router = listings_router(pool);
    let id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::POST,
        &format!("/api/v1/listings/{id}/view"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

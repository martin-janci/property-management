//! Behaviour tests for GET /api/v1/price-map.

mod common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;

fn price_map_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/price-map", routes::price_map::router())
        .with_state(state)
}

#[sqlx::test(migrations = "../../db/migrations")]
async fn get_price_map_returns_200(pool: PgPool) {
    let router = price_map_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/price-map", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrations = "../../db/migrations")]
async fn get_price_map_with_filters_returns_200(pool: PgPool) {
    let router = price_map_router(pool);
    let status = common::send(
        &router,
        Method::GET,
        "/api/v1/price-map?transaction_type=sale&property_type=apartment",
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

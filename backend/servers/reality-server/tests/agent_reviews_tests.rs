//! Behaviour tests for GET /api/v1/realtors/{id}/reviews
//! and POST /api/v1/realtors/{id}/reviews.

mod common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn agent_reviews_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest(
            "/api/v1/realtors/{id}/reviews",
            routes::agent_reviews::router(),
        )
        .with_state(state)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_reviews_unknown_realtor_returns_200_empty(pool: PgPool) {
    let router = agent_reviews_router(pool);
    let realtor_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::GET,
        &format!("/api/v1/realtors/{realtor_id}/reviews"),
        None,
    )
    .await;
    // Returns 200 with empty array when realtor has no reviews
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_review_unauthenticated_returns_401(pool: PgPool) {
    let router = agent_reviews_router(pool);
    let realtor_id = Uuid::new_v4();
    let status = common::send_json(
        &router,
        Method::POST,
        &format!("/api/v1/realtors/{realtor_id}/reviews"),
        None,
        serde_json::json!({ "rating": 5, "body": "Great realtor" }),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

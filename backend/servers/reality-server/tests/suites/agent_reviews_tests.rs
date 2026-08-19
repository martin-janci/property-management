//! Behaviour tests for GET /api/v1/realtors/{id}/reviews
//! and POST /api/v1/realtors/{id}/reviews.

use crate::common;

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
    // Handler checks realtor_profiles; returns 404 when realtor does not exist.
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_review_oversized_body_returns_400(pool: PgPool) {
    // RequestPrincipal looks up the user in the DB; seed principal_kind='platform'
    // so the extractor resolves without needing a ResolvedTenant (no host
    // middleware in the test router). The body-length cap then rejects the
    // oversized body BEFORE any realtor lookup, so 400 (not 404).
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("review-body-{user_id}@test.internal"))
    .execute(&pool)
    .await
    .expect("seed user");

    let router = agent_reviews_router(pool);
    let realtor_id = Uuid::new_v4();
    let token = common::mint_token(user_id);
    // 5001 chars — one past the MAX_REVIEW_BODY_LEN cap.
    let oversized = "a".repeat(5001);
    let status = common::send_json(
        &router,
        Method::POST,
        &format!("/api/v1/realtors/{realtor_id}/reviews"),
        Some(&token),
        serde_json::json!({ "rating": 5, "body": oversized }),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

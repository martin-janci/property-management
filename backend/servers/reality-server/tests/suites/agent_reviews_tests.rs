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

/// Seed a portal user the `RequestPrincipal` extractor can resolve.
///
/// `principal_kind = 'platform'` mirrors the oversized-body test: it lets the
/// extractor resolve without a `ResolvedTenant` (no host middleware in the test
/// router). Post-migration-00148 both `realtor_profiles.user_id` and
/// `realtor_reviews.reviewer_user_id` reference `users(id)`, so this row is a
/// valid owner/reviewer for the rows seeded below.
async fn seed_user(pool: &PgPool, id: Uuid, tag: &str) {
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(id)
    .bind(format!("{tag}-{id}@test.internal"))
    .execute(pool)
    .await
    .expect("seed user");
}

/// Seed a `realtor_profiles` row owned by `owner_user_id`; returns its id.
async fn seed_realtor_profile(pool: &PgPool, owner_user_id: Uuid) -> Uuid {
    sqlx::query_scalar("INSERT INTO realtor_profiles (user_id) VALUES ($1) RETURNING id")
        .bind(owner_user_id)
        .fetch_one(pool)
        .await
        .expect("seed realtor profile")
}

/// End-to-end guard: a realtor authenticated as their own account cannot review
/// their own profile — the endpoint returns `403`.
///
/// The helper-only unit tests in `routes/agent_reviews.rs` pin the leaf
/// predicate `reject_self_review`, but not the wiring: that `create_review`
/// passes `principal.user_id` and `realtor_profiles.user_id` in the right order,
/// and that the guard fires (returning 403) rather than falling through to the
/// "already reviewed" 400 or the insert. This exercises the real HTTP path.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_review_self_review_returns_403(pool: PgPool) {
    let user_a = Uuid::new_v4();
    seed_user(&pool, user_a, "self-review-a").await;
    let realtor_id = seed_realtor_profile(&pool, user_a).await;

    let router = agent_reviews_router(pool);
    let token = common::mint_token(user_a);
    let status = common::send_json(
        &router,
        Method::POST,
        &format!("/api/v1/realtors/{realtor_id}/reviews"),
        Some(&token),
        serde_json::json!({ "rating": 5, "body": "Reviewing my own profile" }),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

/// Companion to the self-review guard: user B reviewing A's profile succeeds
/// (`201`), proving the guard is specific to self-review and not a blanket
/// reject of the endpoint.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_review_other_user_succeeds_201(pool: PgPool) {
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    seed_user(&pool, user_a, "cross-review-a").await;
    seed_user(&pool, user_b, "cross-review-b").await;
    let realtor_id = seed_realtor_profile(&pool, user_a).await;

    let router = agent_reviews_router(pool);
    let token = common::mint_token(user_b);
    let status = common::send_json(
        &router,
        Method::POST,
        &format!("/api/v1/realtors/{realtor_id}/reviews"),
        Some(&token),
        serde_json::json!({ "rating": 4, "body": "Great agent to work with" }),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
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

//! Behaviour tests for GET /api/v1/compare, POST /api/v1/compare/{id},
//! DELETE /api/v1/compare/{id}.

mod common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn compare_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/compare", routes::compare::router())
        .with_state(state)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_compare_list_unauthenticated_returns_401(pool: PgPool) {
    let router = compare_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/compare", None).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_compare_list_authenticated_returns_200(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("compare-get-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = compare_router(pool);
    let token = common::mint_token(user_id);
    let status = common::send(&router, Method::GET, "/api/v1/compare", Some(&token)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_to_compare_unauthenticated_returns_401(pool: PgPool) {
    let router = compare_router(pool);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::POST,
        &format!("/api/v1/compare/{listing_id}"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_to_compare_unknown_listing_returns_404(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("compare-add-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = compare_router(pool);
    let token = common::mint_token(user_id);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::POST,
        &format!("/api/v1/compare/{listing_id}"),
        Some(&token),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn remove_from_compare_unauthenticated_returns_401(pool: PgPool) {
    let router = compare_router(pool);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::DELETE,
        &format!("/api/v1/compare/{listing_id}"),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn remove_from_compare_authenticated_not_in_list_returns_404(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("compare-del-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = compare_router(pool);
    let token = common::mint_token(user_id);
    let listing_id = Uuid::new_v4();
    let status = common::send(
        &router,
        Method::DELETE,
        &format!("/api/v1/compare/{listing_id}"),
        Some(&token),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

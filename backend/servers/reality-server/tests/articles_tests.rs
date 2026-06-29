//! Behaviour tests for GET /api/v1/articles, /{slug}, /{slug}/comments
//! and POST /api/v1/articles/{slug}/comments.

mod common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn articles_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/articles", routes::articles::router())
        .with_state(state)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_articles_returns_200(pool: PgPool) {
    let router = articles_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/articles", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_article_unknown_slug_returns_404(pool: PgPool) {
    let router = articles_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/articles/no-such-slug", None).await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_comments_unknown_slug_returns_404(pool: PgPool) {
    let router = articles_router(pool);
    let status = common::send(
        &router,
        Method::GET,
        "/api/v1/articles/no-such-slug/comments",
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_comment_unauthenticated_returns_401(pool: PgPool) {
    let router = articles_router(pool);
    let status = common::send_json(
        &router,
        Method::POST,
        "/api/v1/articles/some-slug/comments",
        None,
        serde_json::json!({ "body": "test comment" }),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_comment_authenticated_unknown_article_returns_404(pool: PgPool) {
    let user_id = Uuid::new_v4();
    // RequestPrincipal looks up the user in the DB; seed it so auth passes.
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'public', 'active')",
    )
    .bind(user_id)
    .bind(format!("art-comment-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = articles_router(pool);
    let token = common::mint_token(user_id);
    let status = common::send_json(
        &router,
        Method::POST,
        "/api/v1/articles/no-such-article/comments",
        Some(&token),
        serde_json::json!({ "body": "test comment" }),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

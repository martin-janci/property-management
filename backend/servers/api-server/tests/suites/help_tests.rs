//! Contextual Help endpoint integration tests (Epic 10B, Story 10B.7).
//!
//! Validates the HTTP surface of help articles, FAQ, and tooltips routes.
//!
//! # What these tests verify
//!
//! The help routes are **public** — they do not require authentication for
//! read operations (articles, FAQ, tooltips are accessible to all users).
//! Only feedback submission requires a valid bearer token.
//!
//! This file validates:
//! 1. GET /api/v1/help/articles — returns 200 (empty list is fine)
//! 2. GET /api/v1/help/articles/search?q=... — returns 200
//! 3. GET /api/v1/help/articles/category/:cat — returns 200
//! 4. GET /api/v1/help/articles/context/:key — returns 200
//! 5. GET /api/v1/help/articles/:slug — returns 404 when no article
//! 6. GET /api/v1/help/categories — returns 200
//! 7. GET /api/v1/help/faq — returns 200
//! 8. GET /api/v1/help/faq/search?q=... — returns 200
//! 9. GET /api/v1/help/tooltips — returns 200
//! 10. GET /api/v1/help/tooltips/:key — returns 404 when no tooltip
//! 11. POST /api/v1/help/articles/:slug/feedback — 401 without auth

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::TestApp;

// ==================== Helper ====================

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

// ==================== Articles ====================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_articles_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app.execute(get_request("/api/v1/help/articles")).await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/articles should return 200 (empty list acceptable)"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_search_articles_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request("/api/v1/help/articles/search?q=fault"))
        .await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/articles/search should return 200"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_articles_by_category_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request(
            "/api/v1/help/articles/category/getting-started",
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/articles/category/:cat should return 200"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_articles_by_context_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request(
            "/api/v1/help/articles/context/page%3Adashboard",
        ))
        .await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/articles/context/:key should return 200"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_article_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request("/api/v1/help/articles/nonexistent-article"))
        .await;
    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "GET /api/v1/help/articles/:slug should return 404 when article does not exist"
    );
}

// ==================== Categories ====================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_categories_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app.execute(get_request("/api/v1/help/categories")).await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/categories should return 200"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_category_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request("/api/v1/help/categories/nonexistent-category"))
        .await;
    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "GET /api/v1/help/categories/:slug should return 404 when category does not exist"
    );
}

// ==================== FAQ ====================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_faq_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app.execute(get_request("/api/v1/help/faq")).await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/faq should return 200"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_search_faq_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request("/api/v1/help/faq/search?q=password"))
        .await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/faq/search should return 200"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_faq_by_category_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request("/api/v1/help/faq/category/account"))
        .await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/faq/category/:cat should return 200"
    );
}

// ==================== Tooltips ====================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_tooltips_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app.execute(get_request("/api/v1/help/tooltips")).await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/tooltips should return 200"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_tooltip_not_found_returns_404(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request("/api/v1/help/tooltips/nonexistent-key"))
        .await;
    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "GET /api/v1/help/tooltips/:key should return 404 when tooltip does not exist"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_tooltips_by_prefix_returns_200(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let response = app
        .execute(get_request("/api/v1/help/tooltips/prefix/field"))
        .await;
    assert_eq!(
        response.status,
        StatusCode::OK,
        "GET /api/v1/help/tooltips/prefix/:prefix should return 200"
    );
}

// ==================== Feedback Auth Gate ====================

/// Article feedback requires a valid bearer token.
/// Without auth it must be rejected with 401.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_submit_feedback_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let body = serde_json::json!({
        "is_helpful": true,
        "feedback": "Very helpful article"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/help/articles/some-article/feedback")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.execute(request).await;
    assert!(
        response.status.is_client_error(),
        "POST /api/v1/help/articles/:slug/feedback must reject unauthenticated requests with 4xx, got {}",
        response.status
    );
}

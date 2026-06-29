//! Behaviour tests for POST /api/v1/reports and GET /api/v1/reports/me.

mod common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn reports_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/reports", routes::reports::router())
        .with_state(state)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_report_unauthenticated_invalid_listing_returns_404_or_unprocessable(pool: PgPool) {
    let router = reports_router(pool);
    let listing_id = Uuid::new_v4();
    let status = common::send_json(
        &router,
        Method::POST,
        "/api/v1/reports",
        None,
        serde_json::json!({
            "listing_id": listing_id,
            "reason": "spam",
            "description": "Test report"
        }),
    )
    .await;
    // Either 404 (listing not found) or 422 (validation error) are valid
    assert!(
        status == axum::http::StatusCode::NOT_FOUND
            || status == axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        "expected 404 or 422, got {status}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_reports_unauthenticated_returns_401(pool: PgPool) {
    let router = reports_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/reports/me", None).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_reports_authenticated_returns_200(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("reports-me-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = reports_router(pool);
    let token = common::mint_token(user_id);
    let status = common::send(&router, Method::GET, "/api/v1/reports/me", Some(&token)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

//! Behaviour tests for GET /health and GET /readiness.

use crate::common;

use axum::{http::Method, routing::get, Router};
use reality_server::routes;
use sqlx::PgPool;

fn health_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .route("/health", get(routes::health::liveness))
        .route("/readiness", get(routes::health::readiness))
        .with_state(state)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn liveness_returns_200(pool: PgPool) {
    let router = health_router(pool);
    let status = common::send(&router, Method::GET, "/health", None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn readiness_returns_200_or_degraded(pool: PgPool) {
    // Readiness may return 200 (healthy) or 503 (degraded/unhealthy) depending
    // on whether the PM API is reachable in the test environment; either is
    // a valid response (we assert the endpoint exists and responds, not the
    // exact health state of external dependencies).
    let router = health_router(pool);
    let status = common::send(&router, Method::GET, "/readiness", None).await;
    assert!(
        status == axum::http::StatusCode::OK
            || status == axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "expected 200 or 503 from /readiness, got {status}"
    );
}

//! Shared test utilities for reality-server integration tests.
//!
//! Provides: env setup, JWT minting, AppState construction, and
//! a convenience oneshot-request helper.

use api_core::{
    middleware::TenantRateLimiterSet, middleware::TenantResolutionCache, PrincipalClaims,
};
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use reality_server::state::AppState;
use sqlx::PgPool;
use std::sync::{Arc, Once};
use tower::ServiceExt;
use uuid::Uuid;

pub const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

static TEST_ENV: Once = Once::new();

pub fn ensure_test_env() {
    TEST_ENV.call_once(|| {
        if std::env::var("RUST_ENV").is_err() {
            std::env::set_var("RUST_ENV", "development");
        }
        if std::env::var("JWT_SECRET").is_err() {
            std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
        }
    });
}

pub fn make_app_state(pool: PgPool) -> AppState {
    ensure_test_env();
    let tenant_cache = Arc::new(TenantResolutionCache::new(60, 60, 1000));
    let rate_limiters = Arc::new(TenantRateLimiterSet::new());
    AppState::new(pool, tenant_cache, rate_limiters)
}

#[allow(dead_code)]
pub fn mint_token(user_id: Uuid) -> String {
    let claims = PrincipalClaims {
        sub: user_id,
        iat: 0,
        exp: i64::MAX,
        kind: None,
        token_type: Some("access".to_string()),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("token mint failed")
}

pub async fn send(router: &Router, method: Method, path: &str, token: Option<&str>) -> StatusCode {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let response = router
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    response.status()
}

pub async fn send_json(
    router: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: serde_json::Value,
) -> StatusCode {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let response = router
        .clone()
        .oneshot(
            req.body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    response.status()
}

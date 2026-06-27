//! Handler-level auth tests for SSO endpoints.
//!
//! Drives the real Axum router (routes::sso) end-to-end via `oneshot`.
//!
//! Auth patterns in this module:
//! - Public (no auth): sso_login (302 redirect), sso_callback (400/non-401),
//!   get_mapped_roles (200 static).
//! - Cookie-protected (portal_session cookie): sso_logout — 401 without cookie.
//!   Non-401 test omitted: requires a live session record in the DB that the
//!   test-DB session service cannot produce without a real PM OAuth round-trip.
//! - Bearer-protected (Authorization header): get_session, refresh_session —
//!   401 without token. With a token the session lookup also returns 401
//!   (no session in test DB), so only the unauthenticated direction is asserted.
//! - Body-validated (PM/SSO token in JSON body): exchange_pm_token, sync_session,
//!   create_mobile_sso_token, validate_mobile_sso_token — missing body yields 422
//!   (non-401, proves handler is reached); invalid-token body yields 401 (proves
//!   PM-token validation rejects garbage).

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use common::make_app_state;
use reality_server::routes;
use sqlx::PgPool;
use tower::ServiceExt;

fn sso_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/sso", routes::sso::router())
        .with_state(state)
}

/// Issue a raw request (no body, optional Bearer token) and return the status.
async fn raw_send(app: &Router, method: Method, path: &str, token: Option<&str>) -> StatusCode {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    app.clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

/// Issue a JSON body request and return the status.
async fn raw_send_json(
    app: &Router,
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
    app.clone()
        .oneshot(
            req.body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

// ── sso_login (public — redirects to PM OAuth) ───────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sso_login_unauthenticated_returns_non_401(pool: PgPool) {
    let app = sso_router(pool);
    // Returns 302 (redirect to PM OAuth) or 400 (bad redirect_uri), never 401
    let status = raw_send(&app, Method::GET, "/api/v1/sso/login", None).await;
    assert_ne!(status, 401, "sso_login must not require auth");
}

// ── sso_callback (public — OAuth callback, returns 400 without params) ───────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sso_callback_unauthenticated_returns_non_401(pool: PgPool) {
    let app = sso_router(pool);
    // No code/state params → 400 (missing_code), not 401
    let status = raw_send(&app, Method::GET, "/api/v1/sso/callback", None).await;
    assert_ne!(status, 401, "sso_callback must not require auth");
}

// ── sso_logout (cookie-protected) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sso_logout_without_session_cookie_returns_401(pool: PgPool) {
    let app = sso_router(pool);
    // No portal_session cookie → handler returns 401 immediately
    let status = raw_send(&app, Method::POST, "/api/v1/sso/logout", None).await;
    assert_eq!(status, 401, "sso_logout must return 401 without session cookie");
}

// ── get_session (Bearer-protected) ───────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_session_unauthenticated_returns_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send(&app, Method::GET, "/api/v1/sso/session", None).await;
    assert_eq!(status, 401, "GET /sso/session must return 401 without token");
}

// ── refresh_session (Bearer-protected) ──────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn refresh_session_unauthenticated_returns_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send(&app, Method::POST, "/api/v1/sso/refresh", None).await;
    assert_eq!(status, 401, "POST /sso/refresh must return 401 without token");
}

// ── create_mobile_sso_token (body-validated: PM access token required) ───────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_mobile_sso_token_without_body_returns_non_401(pool: PgPool) {
    let app = sso_router(pool);
    // Missing body → 422 (deserialization error), not 401
    let status = raw_send(&app, Method::POST, "/api/v1/sso/mobile/token", None).await;
    assert_ne!(status, 401, "create_mobile_sso_token without body must not return 401");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_mobile_sso_token_with_invalid_pm_token_returns_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send_json(
        &app,
        Method::POST,
        "/api/v1/sso/mobile/token",
        None,
        serde_json::json!({"pm_access_token": "invalid-garbage-token"}),
    )
    .await;
    assert_eq!(
        status, 401,
        "create_mobile_sso_token with invalid PM token must return 401"
    );
}

// ── validate_mobile_sso_token (body-validated: SSO token required) ───────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validate_mobile_sso_token_without_body_returns_non_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send(&app, Method::POST, "/api/v1/sso/mobile/validate", None).await;
    assert_ne!(status, 401, "validate_mobile_sso_token without body must not return 401");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn validate_mobile_sso_token_with_invalid_token_returns_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send_json(
        &app,
        Method::POST,
        "/api/v1/sso/mobile/validate",
        None,
        serde_json::json!({"sso_token": "invalid-garbage-sso-token"}),
    )
    .await;
    assert_eq!(
        status, 401,
        "validate_mobile_sso_token with invalid SSO token must return 401"
    );
}

// ── exchange_pm_token (body-validated: PM access token required) ──────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn exchange_pm_token_without_body_returns_non_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send(&app, Method::POST, "/api/v1/sso/exchange", None).await;
    assert_ne!(status, 401, "exchange_pm_token without body must not return 401");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn exchange_pm_token_with_invalid_pm_token_returns_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send_json(
        &app,
        Method::POST,
        "/api/v1/sso/exchange",
        None,
        serde_json::json!({"pm_access_token": "invalid-pm-token"}),
    )
    .await;
    assert_eq!(
        status, 401,
        "exchange_pm_token with invalid PM token must return 401"
    );
}

// ── sync_session (body-validated: PM token required) ─────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_session_without_body_returns_non_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send(&app, Method::POST, "/api/v1/sso/sync", None).await;
    assert_ne!(status, 401, "sync_session without body must not return 401");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sync_session_with_invalid_pm_token_returns_401(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send_json(
        &app,
        Method::POST,
        "/api/v1/sso/sync",
        None,
        serde_json::json!({"pm_token": "invalid-pm-token", "portal_session": null}),
    )
    .await;
    assert_eq!(
        status, 401,
        "sync_session with invalid PM token must return 401"
    );
}

// ── get_mapped_roles (public — static role-mapping config) ───────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_mapped_roles_unauthenticated_returns_200(pool: PgPool) {
    let app = sso_router(pool);
    let status = raw_send(&app, Method::GET, "/api/v1/sso/roles", None).await;
    assert_eq!(status, 200, "get_mapped_roles must return 200 without auth (static public endpoint)");
}

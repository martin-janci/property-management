//! Handler-level auth tests for users endpoints (reality portal).
//!
//! Drives the real Axum router (routes::users) end-to-end via `oneshot`.
//!
//! Public endpoints (no auth required): register, login, password-reset,
//! password-reset/confirm — verified to return non-401 without a token.
//!
//! Protected endpoints (RequestPrincipal): logout, GET /me, PUT /me —
//! verified to return 401 without a token and non-401 with a seeded user.

mod common;

use axum::{http::Method, Router};
use common::{make_app_state, mint_token, send, send_json};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn users_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/users", routes::users::router())
        .with_state(state)
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@users-authz.test"))
    .bind(format!("UsersAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

// ── register (public) ────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_unauthenticated_returns_non_401(pool: PgPool) {
    let app = users_router(pool);
    // POST with an empty body → 422/400 (validation), not 401 — endpoint is public
    let status = send(&app, Method::POST, "/api/v1/users/register", None).await;
    assert_ne!(status, 401, "register must not require auth");
}

// ── login (public) ───────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_unauthenticated_returns_non_401(pool: PgPool) {
    let app = users_router(pool);
    let status = send(&app, Method::POST, "/api/v1/users/login", None).await;
    assert_ne!(status, 401, "login must not require auth");
}

// ── request_password_reset (public) ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_password_reset_unauthenticated_returns_non_401(pool: PgPool) {
    let app = users_router(pool);
    let status = send(&app, Method::POST, "/api/v1/users/password-reset", None).await;
    assert_ne!(
        status, 401,
        "request_password_reset must not require auth"
    );
}

// ── confirm_password_reset (public) ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn confirm_password_reset_unauthenticated_returns_non_401(pool: PgPool) {
    let app = users_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/users/password-reset/confirm",
        None,
    )
    .await;
    assert_ne!(status, 401, "confirm_password_reset must not require auth");
}

// ── logout (protected) ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn logout_unauthenticated_returns_401(pool: PgPool) {
    let app = users_router(pool);
    let status = send(&app, Method::POST, "/api/v1/users/logout", None).await;
    assert_eq!(status, 401, "logout must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn logout_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "logout").await;
    let token = mint_token(user);
    let app = users_router(pool);
    let status = send(&app, Method::POST, "/api/v1/users/logout", Some(&token)).await;
    assert_ne!(
        status, 401,
        "authenticated logout must not return 401"
    );
}

// ── get_me (protected) ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_me_unauthenticated_returns_401(pool: PgPool) {
    let app = users_router(pool);
    let status = send(&app, Method::GET, "/api/v1/users/me", None).await;
    assert_eq!(status, 401, "GET /me must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_me_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "get-me").await;
    let token = mint_token(user);
    let app = users_router(pool);
    let status = send(&app, Method::GET, "/api/v1/users/me", Some(&token)).await;
    assert_ne!(
        status, 401,
        "authenticated GET /me must not return 401"
    );
}

// ── update_me (protected) ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_me_unauthenticated_returns_401(pool: PgPool) {
    let app = users_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        "/api/v1/users/me",
        None,
        json!({"name": "Test"}),
    )
    .await;
    assert_eq!(status, 401, "PUT /me must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_me_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "update-me").await;
    let token = mint_token(user);
    let app = users_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        "/api/v1/users/me",
        Some(&token),
        json!({"name": "Updated Name"}),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated PUT /me must not return 401"
    );
}

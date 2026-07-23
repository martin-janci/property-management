//! Happy-path 2xx tests for the portal-user surface
//! (`reality-server/src/routes/users.rs`) — BIT-344 Wave 6.
//!
//! - `register` and `request_password_reset` are public and 2xx on an empty DB
//!   given a valid body (password policy: ≥8 chars, 1 uppercase, 1 number).
//! - `get_me` / `update_me` / `logout` use `RequestPrincipal`; the caller is
//!   seeded `principal_kind = 'platform'` so the extractor admits it on the
//!   tenant-less test router (after migration 00148 a single `users` row is the
//!   unified portal identity).
//!
//! `login` (needs a verified credential) and `confirm_password_reset` (needs a
//! hashed reset-token row) are intentionally deferred — see the BIT-344 report.

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
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
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@users-hp.test"))
    .bind(format!("UsersHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

// ── register (POST /register) ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn register_returns_2xx(pool: PgPool) {
    let app = users_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/users/register",
        None,
        json!({
            "email": "newbie@users-hp.test",
            "password": "Password123",
            "name": "New Portal User"
        }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── request_password_reset (POST /password-reset) ────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn request_password_reset_returns_2xx(pool: PgPool) {
    let app = users_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/users/password-reset",
        None,
        json!({ "email": "anyone@users-hp.test" }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_me (GET /me) ─────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn get_me_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "get-me").await;
    let token = mint_token(user);
    let app = users_router(pool);
    let status = send(&app, Method::GET, "/api/v1/users/me", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── update_me (PUT /me) ──────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn update_me_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "update-me").await;
    let token = mint_token(user);
    let app = users_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        "/api/v1/users/me",
        Some(&token),
        json!({ "name": "Updated Name" }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── logout (POST /logout) ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn logout_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "logout").await;
    let token = mint_token(user);
    let app = users_router(pool);
    let status = send(&app, Method::POST, "/api/v1/users/logout", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

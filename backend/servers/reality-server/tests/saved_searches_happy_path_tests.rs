//! Happy-path 2xx tests for the saved-searches surface
//! (`reality-server/src/routes/saved_searches.rs`) — BIT-344 Wave 6.
//!
//! Complements `saved_searches_authz_tests.rs` (401/403 only). Same auth model
//! as the favorites happy-path tests: `RequestPrincipal` endpoints, so the
//! authenticated user is seeded with `principal_kind = 'platform'` to clear the
//! tenant gate on a tenant-less test router. Owned rows are seeded directly
//! into `portal_saved_searches` (FK → `users` after migration 00148).

mod common;

use axum::{http::Method, Router};
use common::{make_app_state, mint_token, send, send_json};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn saved_searches_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/saved-searches", routes::saved_searches::router())
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
    .bind(format!("{tag}@saved-hp.test"))
    .bind(format!("SavedHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

async fn seed_saved_search(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO portal_saved_searches (user_id, name, criteria)
        VALUES ($1, 'Seeded Search', '{}'::jsonb)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_saved_search: {e}"))
}

// ── list_saved_searches (GET /) ──────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_saved_searches_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(&app, Method::GET, "/api/v1/saved-searches", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── create_saved_search (POST /) ─────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_saved_search_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "create").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/saved-searches",
        Some(&token),
        json!({ "name": "My Search", "criteria": { "city": "Bratislava" } }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_search_alerts (GET /alerts) ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_search_alerts_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "alerts").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::GET,
        "/api/v1/saved-searches/alerts",
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── mark_all_alerts_read (POST /alerts/read-all) ─────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_all_alerts_read_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "mark-all").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/saved-searches/alerts/read-all",
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_saved_search (GET /{id}) ─────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_saved_search_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "get").await;
    let search_id = seed_saved_search(&pool, user).await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/saved-searches/{search_id}"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── update_saved_search (PUT /{id}) ──────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_saved_search_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "update").await;
    let search_id = seed_saved_search(&pool, user).await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/saved-searches/{search_id}"),
        Some(&token),
        json!({ "name": "Renamed Search" }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── run_saved_search (POST /{id}/run) ────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn run_saved_search_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "run").await;
    let search_id = seed_saved_search(&pool, user).await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/saved-searches/{search_id}/run"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── delete_saved_search (DELETE /{id}) ───────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_saved_search_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "delete").await;
    let search_id = seed_saved_search(&pool, user).await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/saved-searches/{search_id}"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

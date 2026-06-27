//! Handler-level auth tests for favorites endpoints.
//!
//! Drives the real Axum router (routes::favorites) end-to-end via `oneshot`,
//! verifying that every endpoint enforces authentication:
//! - No bearer token → 401
//! - Valid token for an authenticated user → non-401 (may be 200, 404, 400 etc.)
//!
//! list_favorite_ids uses OptionalRequestPrincipal (SSR-anonymous), so it
//! returns 200 with an empty list for unauthenticated callers.

mod common;

use axum::{http::Method, Router};
use common::{make_app_state, mint_token, send};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

fn favorites_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/favorites", routes::favorites::router())
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
    .bind(format!("{tag}@favorites-authz.test"))
    .bind(format!("FavAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

// ── list_favorites ──────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorites_unauthenticated_returns_401(pool: PgPool) {
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites", None).await;
    assert_eq!(status, 401, "expected 401 for unauthenticated list_favorites");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorites_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "list-fav").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites", Some(&token)).await;
    assert_ne!(status, 401, "authenticated list_favorites must not return 401");
}

// ── list_favorite_alerts ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorite_alerts_unauthenticated_returns_401(pool: PgPool) {
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites/alerts", None).await;
    assert_eq!(status, 401, "expected 401 for unauthenticated list_favorite_alerts");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorite_alerts_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "list-fav-alerts").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites/alerts", Some(&token)).await;
    assert_ne!(status, 401, "authenticated list_favorite_alerts must not return 401");
}

// ── mark_all_favorite_alerts_read ──────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_all_favorite_alerts_read_unauthenticated_returns_401(pool: PgPool) {
    let app = favorites_router(pool);
    let status = send(&app, Method::POST, "/api/v1/favorites/alerts/read-all", None).await;
    assert_eq!(status, 401, "expected 401 for unauthenticated mark_all_favorite_alerts_read");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_all_favorite_alerts_read_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "mark-all-fav-alerts").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(&app, Method::POST, "/api/v1/favorites/alerts/read-all", Some(&token)).await;
    assert_ne!(status, 401, "authenticated mark_all_favorite_alerts_read must not return 401");
}

// ── mark_favorite_alert_read ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_favorite_alert_read_unauthenticated_returns_401(pool: PgPool) {
    let alert_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/favorites/alerts/{alert_id}/read"),
        None,
    )
    .await;
    assert_eq!(status, 401, "expected 401 for unauthenticated mark_favorite_alert_read");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_favorite_alert_read_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "mark-fav-alert").await;
    let token = mint_token(user);
    let alert_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/favorites/alerts/{alert_id}/read"),
        Some(&token),
    )
    .await;
    assert_ne!(status, 401, "authenticated mark_favorite_alert_read must not return 401");
}

// ── list_favorite_ids (SSR-anonymous) ──────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorite_ids_unauthenticated_returns_200(pool: PgPool) {
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites/ids", None).await;
    assert_eq!(status, 200, "list_favorite_ids must return 200 for unauthenticated (SSR-anonymous)");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorite_ids_authenticated_returns_200(pool: PgPool) {
    let user = seed_user(&pool, "list-fav-ids").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites/ids", Some(&token)).await;
    assert_eq!(status, 200, "authenticated list_favorite_ids must return 200");
}

// ── add_favorite ────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_favorite_unauthenticated_returns_401(pool: PgPool) {
    let listing_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/favorites/{listing_id}"),
        None,
    )
    .await;
    assert_eq!(status, 401, "expected 401 for unauthenticated add_favorite");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_favorite_authenticated_unknown_listing_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "add-fav").await;
    let token = mint_token(user);
    let listing_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/favorites/{listing_id}"),
        Some(&token),
    )
    .await;
    assert_ne!(status, 401, "authenticated add_favorite must not return 401");
}

// ── remove_favorite ─────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn remove_favorite_unauthenticated_returns_401(pool: PgPool) {
    let listing_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/favorites/{listing_id}"),
        None,
    )
    .await;
    assert_eq!(status, 401, "expected 401 for unauthenticated remove_favorite");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn remove_favorite_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "remove-fav").await;
    let token = mint_token(user);
    let listing_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/favorites/{listing_id}"),
        Some(&token),
    )
    .await;
    assert_ne!(status, 401, "authenticated remove_favorite must not return 401");
}

// ── check_favorite ──────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn check_favorite_unauthenticated_returns_401(pool: PgPool) {
    let listing_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/favorites/{listing_id}/check"),
        None,
    )
    .await;
    assert_eq!(status, 401, "expected 401 for unauthenticated check_favorite");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn check_favorite_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "check-fav").await;
    let token = mint_token(user);
    let listing_id = Uuid::new_v4();
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/favorites/{listing_id}/check"),
        Some(&token),
    )
    .await;
    assert_ne!(status, 401, "authenticated check_favorite must not return 401");
}

//! Happy-path 2xx tests for the favorites surface
//! (`reality-server/src/routes/favorites.rs`) — BIT-344 Wave 6.
//!
//! Complements `favorites_authz_tests.rs` (which only asserts 401/403) by
//! driving every endpoint to a successful 2xx outcome.
//!
//! Auth model: the favorites endpoints use `RequestPrincipal`. In a bare test
//! router there is no `ResolvedTenant`, so a `public`/`staff` principal would
//! be rejected with 403 ("tenant not resolved"). We therefore seed the
//! authenticated user with `principal_kind = 'platform'`, which the extractor
//! admits with `effective_org = None` on a tenant-less host. After migration
//! 00148 (`portal_users` dropped, FKs retargeted to `users`), that same
//! `users` row also owns the `portal_favorites` data.

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
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
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@favorites-hp.test"))
    .bind(format!("FavoritesHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

async fn seed_listing(pool: &PgPool, tag: &str, owner: Uuid) -> Uuid {
    let org = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Fav Org {tag}"))
    .bind(format!("fav-org-{tag}"))
    .bind(format!("{tag}@fav-org.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_org({tag}): {e}"));

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Fav Listing', 'apartment', 120000.00, 'EUR',
                'Test Street 9', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_listing({tag}): {e}"))
}

async fn seed_favorite(pool: &PgPool, user_id: Uuid, listing_id: Uuid) {
    sqlx::query("INSERT INTO portal_favorites (user_id, listing_id) VALUES ($1, $2)")
        .bind(user_id)
        .bind(listing_id)
        .execute(pool)
        .await
        .expect("seed_favorite failed");
}

// ── list_favorites (GET /) ───────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorites_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_favorite_alerts (GET /alerts) ───────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorite_alerts_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "alerts").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites/alerts", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── mark_all_favorite_alerts_read (POST /alerts/read-all) ────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_all_favorite_alerts_read_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "mark-all").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/favorites/alerts/read-all",
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_favorite_ids (GET /ids) — OptionalRequestPrincipal ──────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorite_ids_anonymous_returns_2xx(pool: PgPool) {
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites/ids", None).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_favorite_ids_authenticated_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "ids").await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(&app, Method::GET, "/api/v1/favorites/ids", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── check_favorite (GET /{listing_id}/check) ─────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn check_favorite_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "check").await;
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
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── add_favorite (POST /{listing_id}) ────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_favorite_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "add").await;
    let listing_id = seed_listing(&pool, "add", user).await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/favorites/{listing_id}"),
        Some(&token),
        json!({}),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── remove_favorite (DELETE /{listing_id}) ───────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn remove_favorite_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "remove").await;
    let listing_id = seed_listing(&pool, "remove", user).await;
    seed_favorite(&pool, user, listing_id).await;
    let token = mint_token(user);
    let app = favorites_router(pool);
    let status = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/favorites/{listing_id}"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

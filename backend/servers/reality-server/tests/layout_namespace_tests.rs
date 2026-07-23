//! Namespace restriction tests for the public layout resolved endpoint.
//!
//! The reality-server endpoint is unauthenticated: it must only serve
//! `reality/*` screens. Internal namespaces (`ppt/*`) 404 before any DB
//! access, even when a published config exists for them.

mod common;

use axum::http::Method;
use axum::Router;
use db::repositories::LayoutRepository;
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;

fn layout_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/layout", routes::layout::router())
        .with_state(state)
}

/// Seed a screen with a published config plus a web manifest so a permitted
/// request would fully resolve.
async fn seed_published(pool: &PgPool, screen: &str) {
    let repo = LayoutRepository::new();
    let mut conn = pool.acquire().await.unwrap();
    let draft = json!({"screen": screen, "version": 0,
                       "sections": [{"type": "gallery.v1"}]});
    repo.upsert_draft(&mut *conn, screen, &draft, None)
        .await
        .unwrap();
    repo.publish(&mut conn, screen, &draft, None).await.unwrap();
    let manifest = json!({"platform": "web",
                          "components": {"gallery.v1": {"required": true}}});
    repo.upsert_manifest(&mut *conn, "web", &manifest, None)
        .await
        .unwrap();
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn internal_ppt_screen_is_404_even_when_published(pool: PgPool) {
    seed_published(&pool, "ppt/dashboard").await;
    let router = layout_router(pool);
    let status = common::send(
        &router,
        Method::GET,
        "/api/v1/layout/resolved/ppt/dashboard",
        None,
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::NOT_FOUND,
        "internal ppt/* screens must not be servable on the public portal"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reality_screen_still_resolves(pool: PgPool) {
    seed_published(&pool, "reality/listing-detail").await;
    let router = layout_router(pool);
    let status = common::send(
        &router,
        Method::GET,
        "/api/v1/layout/resolved/reality/listing-detail",
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unpublished_reality_screen_is_404(pool: PgPool) {
    let router = layout_router(pool);
    let status = common::send(
        &router,
        Method::GET,
        "/api/v1/layout/resolved/reality/nonexistent",
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

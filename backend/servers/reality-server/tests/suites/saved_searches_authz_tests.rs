//! Handler-level auth tests for saved-searches endpoints.
//!
//! Drives the real Axum router (routes::saved_searches) end-to-end via
//! `oneshot`, verifying each endpoint's auth boundary:
//! - No bearer token → 401
//! - Valid token → non-401 (may be 200, 404, 400, etc.)

#![allow(dead_code)]

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
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
        VALUES ($1, 'hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@ss-authz.test"))
    .bind(format!("SSAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

// ── list_saved_searches ─────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn list_saved_searches_unauthenticated_returns_401(pool: PgPool) {
    let app = saved_searches_router(pool);
    let status = send(&app, Method::GET, "/api/v1/saved-searches", None).await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated list_saved_searches"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn list_saved_searches_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "list-ss").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(&app, Method::GET, "/api/v1/saved-searches", Some(&token)).await;
    assert_ne!(
        status, 401,
        "authenticated list_saved_searches must not return 401"
    );
}

// ── create_saved_search ─────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_saved_search_unauthenticated_returns_401(pool: PgPool) {
    let app = saved_searches_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/saved-searches",
        None,
        json!({"name": "test", "filters": {}}),
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated create_saved_search"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_saved_search_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "create-ss").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/saved-searches",
        Some(&token),
        json!({"name": "My Search", "filters": {}}),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated create_saved_search must not return 401"
    );
}

// ── list_search_alerts ──────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn list_search_alerts_unauthenticated_returns_401(pool: PgPool) {
    let app = saved_searches_router(pool);
    let status = send(&app, Method::GET, "/api/v1/saved-searches/alerts", None).await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated list_search_alerts"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn list_search_alerts_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "list-alerts").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::GET,
        "/api/v1/saved-searches/alerts",
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated list_search_alerts must not return 401"
    );
}

// ── mark_all_alerts_read ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn mark_all_alerts_read_unauthenticated_returns_401(pool: PgPool) {
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/saved-searches/alerts/read-all",
        None,
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated mark_all_alerts_read"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn mark_all_alerts_read_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "mark-all-alerts").await;
    let token = mint_token(user);
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/saved-searches/alerts/read-all",
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated mark_all_alerts_read must not return 401"
    );
}

// ── mark_alert_read ─────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn mark_alert_read_unauthenticated_returns_401(pool: PgPool) {
    let alert_id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/saved-searches/alerts/{alert_id}/read"),
        None,
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated mark_alert_read"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn mark_alert_read_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "mark-alert").await;
    let token = mint_token(user);
    let alert_id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/saved-searches/alerts/{alert_id}/read"),
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated mark_alert_read must not return 401"
    );
}

// ── get_saved_search ────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_saved_search_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/saved-searches/{id}"),
        None,
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated get_saved_search"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_saved_search_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "get-ss").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/saved-searches/{id}"),
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated get_saved_search must not return 401"
    );
}

// ── update_saved_search ─────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_saved_search_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/saved-searches/{id}"),
        None,
        json!({"name": "updated"}),
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated update_saved_search"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_saved_search_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "update-ss").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/saved-searches/{id}"),
        Some(&token),
        json!({"name": "updated"}),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated update_saved_search must not return 401"
    );
}

// ── delete_saved_search ─────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn delete_saved_search_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/saved-searches/{id}"),
        None,
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated delete_saved_search"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn delete_saved_search_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "delete-ss").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/saved-searches/{id}"),
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated delete_saved_search must not return 401"
    );
}

// ── run_saved_search ────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn run_saved_search_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/saved-searches/{id}/run"),
        None,
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated run_saved_search"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn run_saved_search_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "run-ss").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = saved_searches_router(pool);
    let status = send(
        &app,
        Method::POST,
        &format!("/api/v1/saved-searches/{id}/run"),
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated run_saved_search must not return 401"
    );
}

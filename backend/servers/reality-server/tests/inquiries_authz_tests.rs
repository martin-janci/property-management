//! Handler-level auth tests for inquiries endpoints.
//!
//! Drives the real Axum router (routes::inquiries) end-to-end via `oneshot`.
//!
//! Auth behavior per endpoint:
//! - send_contact_message, request_viewing: PUBLIC — no auth required (201/400/404).
//! - list_my_inquiries, list_buyer_inquiries, get_inquiry, mark_as_read,
//!   respond_to_inquiry: AUTHENTICATED — no token → 401.

#![allow(dead_code)]

mod common;

use axum::{http::Method, Router};
use common::{make_app_state, mint_token, send, send_json};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn inquiries_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/inquiries", routes::inquiries::router())
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
    .bind(format!("{tag}@inq-authz.test"))
    .bind(format!("InqAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

// ── send_contact_message (public) ───────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn send_contact_message_unauthenticated_does_not_return_401(pool: PgPool) {
    let listing_id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/contact/{listing_id}"),
        None,
        json!({
            "name": "Test Buyer",
            "email": "buyer@test.com",
            "message": "I am interested"
        }),
    )
    .await;
    assert_ne!(
        status, 401,
        "public send_contact_message must not require auth"
    );
}

// ── request_viewing (public) ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_viewing_unauthenticated_does_not_return_401(pool: PgPool) {
    let listing_id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/viewing/{listing_id}"),
        None,
        json!({
            "name": "Test Viewer",
            "email": "viewer@test.com",
            "message": "I want to view"
        }),
    )
    .await;
    assert_ne!(status, 401, "public request_viewing must not require auth");
}

// ── list_my_inquiries ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_inquiries_unauthenticated_returns_401(pool: PgPool) {
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries", None).await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated list_my_inquiries"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_inquiries_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "list-inq").await;
    let token = mint_token(user);
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries", Some(&token)).await;
    assert_ne!(
        status, 401,
        "authenticated list_my_inquiries must not return 401"
    );
}

// ── list_buyer_inquiries ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_buyer_inquiries_unauthenticated_returns_401(pool: PgPool) {
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries/mine", None).await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated list_buyer_inquiries"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_buyer_inquiries_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "buyer-inq").await;
    let token = mint_token(user);
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, "/api/v1/inquiries/mine", Some(&token)).await;
    assert_ne!(
        status, 401,
        "authenticated list_buyer_inquiries must not return 401"
    );
}

// ── get_inquiry ─────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_inquiry_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send(&app, Method::GET, &format!("/api/v1/inquiries/{id}"), None).await;
    assert_eq!(status, 401, "expected 401 for unauthenticated get_inquiry");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_inquiry_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "get-inq").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/inquiries/{id}"),
        Some(&token),
    )
    .await;
    assert_ne!(status, 401, "authenticated get_inquiry must not return 401");
}

// ── mark_as_read ────────────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_as_read_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send(
        &app,
        Method::PUT,
        &format!("/api/v1/inquiries/{id}/read"),
        None,
    )
    .await;
    assert_eq!(status, 401, "expected 401 for unauthenticated mark_as_read");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mark_as_read_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "mark-read").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send(
        &app,
        Method::PUT,
        &format!("/api/v1/inquiries/{id}/read"),
        Some(&token),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated mark_as_read must not return 401"
    );
}

// ── respond_to_inquiry ──────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn respond_to_inquiry_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/{id}/respond"),
        None,
        json!({"message": "Thank you for your inquiry"}),
    )
    .await;
    assert_eq!(
        status, 401,
        "expected 401 for unauthenticated respond_to_inquiry"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn respond_to_inquiry_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "respond-inq").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = inquiries_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/inquiries/{id}/respond"),
        Some(&token),
        json!({"message": "Thank you for your inquiry"}),
    )
    .await;
    assert_ne!(
        status, 401,
        "authenticated respond_to_inquiry must not return 401"
    );
}

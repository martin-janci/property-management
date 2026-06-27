//! Handler-level auth tests for agencies endpoints.
//!
//! Drives the real Axum router (routes::agencies) end-to-end via `oneshot`.
//!
//! Public endpoints (no RequestPrincipal): list_agencies, get_agency,
//! get_agency_by_slug, list_members — return non-401 without a token.
//!
//! Protected endpoints (RequestPrincipal): create_agency, update_agency,
//! create_invitation, accept_invitation — return 401 without a token and
//! non-401 with a seeded user token.

mod common;

use axum::{http::Method, Router};
use common::{make_app_state, mint_token, send, send_json};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn agencies_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/agencies", routes::agencies::router())
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
    .bind(format!("{tag}@agencies-authz.test"))
    .bind(format!("AgenciesAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

// ── list_agencies (public) ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_agencies_unauthenticated_returns_non_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies", None).await;
    assert_ne!(status, 401, "list_agencies must not require auth (public directory)");
}

// ── get_agency (public) ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_unauthenticated_returns_non_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, &format!("/api/v1/agencies/{id}"), None).await;
    assert_ne!(status, 401, "get_agency must not require auth (may return 404 for unknown id)");
}

// ── get_agency_by_slug (public) ───────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_by_slug_unauthenticated_returns_non_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies/by-slug/unknown-slug", None).await;
    assert_ne!(status, 401, "get_agency_by_slug must not require auth (may return 404)");
}

// ── list_members (public) ─────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_members_unauthenticated_returns_non_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, &format!("/api/v1/agencies/{id}/members"), None).await;
    assert_ne!(status, 401, "list_members must not require auth (may return empty list or 404)");
}

// ── create_agency (protected) ─────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_agency_unauthenticated_returns_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/agencies",
        None,
        json!({"name": "Test Agency", "slug": "test-agency"}),
    )
    .await;
    assert_eq!(status, 401, "create_agency must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_agency_authenticated_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "create-agency").await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/agencies",
        Some(&token),
        json!({"name": "Test Agency", "slug": "test-agency"}),
    )
    .await;
    assert_ne!(status, 401, "authenticated create_agency must not return 401");
}

// ── update_agency (protected) ─────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_agency_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{id}"),
        None,
        json!({"name": "Updated"}),
    )
    .await;
    assert_eq!(status, 401, "update_agency must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_agency_authenticated_unknown_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "update-agency").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{id}"),
        Some(&token),
        json!({"name": "Updated"}),
    )
    .await;
    assert_ne!(status, 401, "authenticated update_agency must not return 401");
}

// ── create_invitation (protected) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_unauthenticated_returns_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{id}/invitations"),
        None,
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_eq!(status, 401, "create_invitation must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_authenticated_unknown_agency_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "create-inv").await;
    let token = mint_token(user);
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{id}/invitations"),
        Some(&token),
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_ne!(status, 401, "authenticated create_invitation must not return 401");
}

// ── accept_invitation (protected) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_invitation_unauthenticated_returns_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/agencies/invitations/fake-token/accept",
        None,
    )
    .await;
    assert_eq!(status, 401, "accept_invitation must return 401 without auth");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_invitation_authenticated_unknown_token_returns_non_401(pool: PgPool) {
    let user = seed_user(&pool, "accept-inv").await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::POST,
        "/api/v1/agencies/invitations/fake-token/accept",
        Some(&token),
    )
    .await;
    assert_ne!(status, 401, "authenticated accept_invitation must not return 401");
}

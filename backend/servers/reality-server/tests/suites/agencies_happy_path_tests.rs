//! Happy-path 2xx tests for the agencies surface
//! (`reality-server/src/routes/agencies.rs`) — BIT-344 Wave 6.
//!
//! Public reads (`list_agencies`, `get_agency`, `get_agency_by_slug`) need only
//! a seeded `reality_agencies` row. The authenticated calls (`create_agency`,
//! `update_agency`, `list_members`) use `RequestPrincipal`, so the caller is
//! seeded `principal_kind = 'platform'` to clear the tenant gate; `update_agency`
//! and `list_members` additionally need an active `reality_agency_members` row
//! (FK → `users` after migration 00148) — the member roster is agency-internal
//! PII, gated by `check_agency_membership` (members-idor audit).

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
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
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@agencies-hp.test"))
    .bind(format!("AgenciesHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

async fn seed_agency(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reality_agencies (name, slug, email)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(format!("Agency {slug}"))
    .bind(format!("agency-{slug}"))
    .bind(format!("{slug}@agencies-hp.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_agency({slug}): {e}"))
}

async fn seed_membership(pool: &PgPool, agency_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO reality_agency_members (agency_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(agency_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed_membership failed");
}

// ── list_agencies (GET /) ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_agencies_returns_2xx(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies", None).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_agency (GET /{id}) ───────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_returns_2xx(pool: PgPool) {
    let agency_id = seed_agency(&pool, "get").await;
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/agencies/{agency_id}"),
        None,
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_agency_by_slug (GET /by-slug/{slug}) ─────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_by_slug_returns_2xx(pool: PgPool) {
    seed_agency(&pool, "byslug").await;
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        "/api/v1/agencies/by-slug/agency-byslug",
        None,
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_members (GET /{id}/members) ─────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_members_returns_2xx(pool: PgPool) {
    // members-idor audit: listing members now requires an authenticated,
    // active member of the target agency.
    let member = seed_user(&pool, "members-caller").await;
    let agency_id = seed_agency(&pool, "members").await;
    seed_membership(&pool, agency_id, member).await;
    let token = mint_token(member);
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/agencies/{agency_id}/members"),
        Some(&token),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── create_agency (POST /) ───────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_agency_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "create").await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/agencies",
        Some(&token),
        json!({ "name": "Brand New Agency", "email": "new@agencies-hp.test" }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── update_agency (PUT /{id}) ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_agency_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "update").await;
    let agency_id = seed_agency(&pool, "update").await;
    seed_membership(&pool, agency_id, user).await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/agencies/{agency_id}"),
        Some(&token),
        json!({ "name": "Renamed Agency" }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_my_agency (GET /me) — Issue #2359 ─────────────────────────────────────
//
// Pins the exact contract `useMyAgency()` depends on: 200 for an
// authenticated caller with an (active) agency, 404 for an authenticated
// caller with no agency, 401 for an unauthenticated caller. The 404 case is
// what lets the frontend show the "Create Agency" onboarding CTA instead of a
// generic retry/error screen.

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_my_agency_with_agency_returns_200(pool: PgPool) {
    let user = seed_user(&pool, "me-200").await;
    let agency_id = seed_agency(&pool, "me-200").await;
    seed_membership(&pool, agency_id, user).await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies/me", Some(&token)).await;
    assert!(
        status.is_success(),
        "expected 2xx for a member, got {status}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_my_agency_without_agency_returns_404(pool: PgPool) {
    let user = seed_user(&pool, "me-404").await;
    let token = mint_token(user);
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies/me", Some(&token)).await;
    assert_eq!(
        status, 404,
        "a caller with no agency must get 404 (drives the onboarding CTA), got {status}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_my_agency_unauthenticated_returns_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies/me", None).await;
    assert_eq!(status, 401, "GET /me must require auth, got {status}");
}

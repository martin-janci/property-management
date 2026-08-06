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

/// Seed a `platform`-kind user so the `RequestPrincipal` extractor clears the
/// tenant gate on the host-less test router and the request reaches the
/// handler's own membership check (mirrors the happy-path suite).
async fn seed_platform_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@agencies-authz.test"))
    .bind(format!("AgenciesAuthz {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_platform_user({tag}): {e}"))
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
    .bind(format!("{slug}@agencies-authz.test"))
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

// ── list_agencies (public) ────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_agencies_unauthenticated_returns_non_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, "/api/v1/agencies", None).await;
    assert_ne!(
        status, 401,
        "list_agencies must not require auth (public directory)"
    );
}

// ── get_agency (public) ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_unauthenticated_returns_non_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send(&app, Method::GET, &format!("/api/v1/agencies/{id}"), None).await;
    assert_ne!(
        status, 401,
        "get_agency must not require auth (may return 404 for unknown id)"
    );
}

// ── get_agency_by_slug (public) ───────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_by_slug_unauthenticated_returns_non_401(pool: PgPool) {
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        "/api/v1/agencies/by-slug/unknown-slug",
        None,
    )
    .await;
    assert_ne!(
        status, 401,
        "get_agency_by_slug must not require auth (may return 404)"
    );
}

// ── list_members (public) ─────────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_members_unauthenticated_returns_non_401(pool: PgPool) {
    let id = Uuid::new_v4();
    let app = agencies_router(pool);
    let status = send(
        &app,
        Method::GET,
        &format!("/api/v1/agencies/{id}/members"),
        None,
    )
    .await;
    assert_ne!(
        status, 401,
        "list_members must not require auth (may return empty list or 404)"
    );
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
    assert_ne!(
        status, 401,
        "authenticated create_agency must not return 401"
    );
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
    assert_ne!(
        status, 401,
        "authenticated update_agency must not return 401"
    );
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
    assert_eq!(
        status, 401,
        "create_invitation must return 401 without auth"
    );
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
    assert_ne!(
        status, 401,
        "authenticated create_invitation must not return 401"
    );
}

// REGRESSION (invite-authz audit): `create_invitation` had NO membership
// gate, so any authenticated user could invite themselves/others into ANY
// agency. A non-member acting on an *existing* agency must be rejected with
// 403 — not allowed to create the invitation. Uses a `platform` caller so the
// request clears the `RequestPrincipal` extractor and the 403 can only come
// from the handler's own `check_agency_membership` gate.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_non_member_returns_403(pool: PgPool) {
    let outsider = seed_platform_user(&pool, "inv-outsider").await;
    let agency_id = seed_agency(&pool, "inv-403").await;
    // NB: no membership seeded for `outsider` in this agency.
    let token = mint_token(outsider);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{agency_id}/invitations"),
        Some(&token),
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_eq!(
        status, 403,
        "a non-member must NOT be able to create invitations for an agency, got {status}"
    );
}

// Companion: the gate must not over-reject — an active member of the agency
// is allowed past the membership check (i.e. does NOT get 403).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_invitation_member_not_forbidden(pool: PgPool) {
    let member = seed_platform_user(&pool, "inv-member").await;
    let agency_id = seed_agency(&pool, "inv-member").await;
    seed_membership(&pool, agency_id, member).await;
    let token = mint_token(member);
    let app = agencies_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/agencies/{agency_id}/invitations"),
        Some(&token),
        json!({"email": "invite@example.com", "role": "agent"}),
    )
    .await;
    assert_ne!(
        status, 403,
        "an active member must pass the membership gate, got {status}"
    );
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
    assert_eq!(
        status, 401,
        "accept_invitation must return 401 without auth"
    );
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
    assert_ne!(
        status, 401,
        "authenticated accept_invitation must not return 401"
    );
}

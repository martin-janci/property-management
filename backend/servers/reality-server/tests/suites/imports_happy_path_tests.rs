//! Happy-path 2xx tests for the agency-import surface
//! (`reality-server/src/routes/imports.rs`) — BIT-344 Wave 6.
//!
//! These endpoints use `PortalPrincipal`, which authenticates the JWT and
//! re-derives the kind from `users` but requires no tenant/membership, so any
//! seeded `users` row authenticates. Owned `portal_import_jobs` rows are seeded
//! directly (FK → `users` after migration 00148). `list_feeds` additionally
//! resolves the caller's agency, so it needs an active `reality_agency_members`
//! row.
//!
//! `start_import_job` / `cancel_import_job` (state-machine transitions) and the
//! feed CRUD endpoints are deferred — see the BIT-344 report.

use crate::common::{make_app_state, mint_token, send, send_json};
use axum::{http::Method, Router};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn imports_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest("/api/v1/imports", routes::imports::router())
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
    .bind(format!("{tag}@imports-hp.test"))
    .bind(format!("ImportsHP {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

async fn seed_import_job(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO portal_import_jobs (user_id, source_type)
        VALUES ($1, 'csv')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_import_job: {e}"))
}

async fn seed_agency_membership(pool: &PgPool, user_id: Uuid) {
    let agency_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reality_agencies (name, slug, email)
        VALUES ('Imports Agency', 'imports-agency', 'agency@imports-hp.test')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed_agency failed");
    sqlx::query(
        "INSERT INTO reality_agency_members (agency_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(agency_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed_agency_membership failed");
}

// ── list_import_jobs (GET /jobs) ─────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_import_jobs_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list-jobs").await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let status = send(&app, Method::GET, "/api/v1/imports/jobs", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── create_import_job (POST /jobs) ───────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_import_job_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "create-job").await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/imports/jobs",
        Some(&token),
        json!({ "source_type": "csv" }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── get_import_job (GET /jobs/{id}) ──────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_import_job_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "get-job").await;
    let job_id = seed_import_job(&pool, user).await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let path = format!("/api/v1/imports/jobs/{job_id}");
    let status = send(&app, Method::GET, &path, Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── update_import_job (PUT /jobs/{id}) ───────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_import_job_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "update-job").await;
    let job_id = seed_import_job(&pool, user).await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let path = format!("/api/v1/imports/jobs/{job_id}");
    let status = send_json(
        &app,
        Method::PUT,
        &path,
        Some(&token),
        json!({ "source_url": "https://example.com/feed.csv" }),
    )
    .await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

// ── list_feeds (GET /feeds) ──────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_feeds_returns_2xx(pool: PgPool) {
    let user = seed_user(&pool, "list-feeds").await;
    seed_agency_membership(&pool, user).await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let status = send(&app, Method::GET, "/api/v1/imports/feeds", Some(&token)).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
}

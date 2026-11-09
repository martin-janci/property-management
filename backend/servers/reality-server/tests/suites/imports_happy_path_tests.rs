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
use axum::{
    body::Body,
    http::{header, Method, Request},
    Router,
};
use reality_server::routes;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// GET helper that returns the decoded JSON body (the shared `send` helper
/// only surfaces the status code).
async fn get_json(router: &Router, path: &str, token: &str) -> serde_json::Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(path)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "expected 2xx, got {}",
        response.status()
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

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

// Regression: `total` must be the full count of matching rows, not the length
// of the returned page. Before the fix the handler set `total = jobs.len()`,
// so a `limit`ed page reported `total == page_len`, breaking client paging.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_import_jobs_total_is_full_count_not_page_len(pool: PgPool) {
    let user = seed_user(&pool, "list-jobs-total").await;
    // Three owned jobs, but we only ask for a page of two.
    seed_import_job(&pool, user).await;
    seed_import_job(&pool, user).await;
    seed_import_job(&pool, user).await;
    let token = mint_token(user);
    let app = imports_router(pool);

    let body = get_json(&app, "/api/v1/imports/jobs?limit=2", &token).await;

    let page_len = body["jobs"].as_array().expect("jobs array").len();
    let total = body["total"].as_i64().expect("total i64");
    assert_eq!(page_len, 2, "page should honour limit=2");
    assert_eq!(
        total, 3,
        "total must be the full match count (3), not the page length ({page_len})"
    );
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

// Regression: the utoipa contract documents `201 Created` for this
// resource-creation POST; the handler must agree (it previously returned 200).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_import_job_returns_201(pool: PgPool) {
    let user = seed_user(&pool, "create-job-201").await;
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
    assert_eq!(status, 201, "create_import_job must return 201 Created");
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

// ── create_feed (POST /feeds) ────────────────────────────────────────────────

// Regression: the utoipa contract documents `201 Created` for this
// resource-creation POST; the handler must agree (it previously returned 200).
// `create_feed` resolves the caller's agency, so the user needs an active
// `reality_agency_members` row.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_feed_returns_201(pool: PgPool) {
    let user = seed_user(&pool, "create-feed-201").await;
    seed_agency_membership(&pool, user).await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/imports/feeds",
        Some(&token),
        json!({ "name": "Nightly feed", "feed_url": "https://example.com/feed.xml" }),
    )
    .await;
    assert_eq!(status, 201, "create_feed must return 201 Created");
}

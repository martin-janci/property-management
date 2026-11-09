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
    http::{Method, StatusCode},
    Router,
};
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

// ── list_import_jobs `limit` clamping (regression) ───────────────────────────
//
// The handler must clamp the caller-supplied `limit` to `1..=100` before it
// reaches SQL. Before the fix the raw value bound straight into `LIMIT $3`:
// a negative `limit` reached Postgres as `LIMIT -1`, which it rejects (bubbling
// up as a 500), and an arbitrarily large `limit` was handed through unbounded.

/// Oneshot a GET and return `(status, parsed JSON body)`. Local to this suite
/// because the shared `send` helper only surfaces the status code.
async fn get_json(router: &Router, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
    use axum::body::Body;
    use axum::http::{header, Request};
    use tower::ServiceExt;

    let req = Request::builder()
        .method(Method::GET)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let response = router.clone().oneshot(req).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// Bulk-seed `n` owned import jobs for `user_id` in a single statement.
async fn seed_import_jobs(pool: &PgPool, user_id: Uuid, n: i32) {
    sqlx::query(
        "INSERT INTO portal_import_jobs (user_id, source_type) \
         SELECT $1, 'csv' FROM generate_series(1, $2)",
    )
    .bind(user_id)
    .bind(n)
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_import_jobs({n}): {e}"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_import_jobs_negative_limit_is_clamped_not_500(pool: PgPool) {
    let user = seed_user(&pool, "neg-limit").await;
    let token = mint_token(user);
    let app = imports_router(pool);
    // `limit=-1` reaches SQL as `LIMIT -1` without clamping → Postgres 500.
    // After clamping it floors to 1 and the request succeeds.
    let status = send(
        &app,
        Method::GET,
        "/api/v1/imports/jobs?limit=-1",
        Some(&token),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "negative limit must be clamped, not passed to SQL (got 500)"
    );
    assert!(
        status.is_success(),
        "expected 2xx after clamp, got {status}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_import_jobs_over_max_limit_is_capped_at_100(pool: PgPool) {
    let user = seed_user(&pool, "max-limit").await;
    seed_import_jobs(&pool, user, 101).await;
    let token = mint_token(user);
    let app = imports_router(pool);
    // An unbounded `limit` would return all 101 rows; clamping caps it at 100.
    let (status, body) = get_json(&app, "/api/v1/imports/jobs?limit=100000", &token).await;
    assert!(status.is_success(), "expected 2xx, got {status}");
    let jobs = body["jobs"]
        .as_array()
        .expect("jobs array in response body");
    assert_eq!(
        jobs.len(),
        100,
        "over-max limit must be capped at 100, got {}",
        jobs.len()
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

// Security: create_import_job must reject SSRF-prone `source_url` values
// (parity with agency_imports). `http://169.254.169.254/` is the AWS
// metadata endpoint — rejected by the literal-IP range check regardless of
// scheme/dev-mode. The handler returns 400 before any row is inserted.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_import_job_rejects_ssrf_source_url(pool: PgPool) {
    let user = seed_user(&pool, "create-job-ssrf").await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/imports/jobs",
        Some(&token),
        json!({ "source_type": "url", "source_url": "http://169.254.169.254/latest/meta-data/" }),
    )
    .await;
    assert_eq!(
        status, 400,
        "create_import_job must reject SSRF source_url with 400, got {status}"
    );
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

// Security: update_import_job must reject SSRF-prone `source_url` values
// (parity with create_import_job / agency_imports). Returns 400 before the
// repo update runs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_import_job_rejects_ssrf_source_url(pool: PgPool) {
    let user = seed_user(&pool, "update-job-ssrf").await;
    let job_id = seed_import_job(&pool, user).await;
    let token = mint_token(user);
    let app = imports_router(pool);
    let path = format!("/api/v1/imports/jobs/{job_id}");
    let status = send_json(
        &app,
        Method::PUT,
        &path,
        Some(&token),
        json!({ "source_url": "http://169.254.169.254/latest/meta-data/" }),
    )
    .await;
    assert_eq!(
        status, 400,
        "update_import_job must reject SSRF source_url with 400, got {status}"
    );
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

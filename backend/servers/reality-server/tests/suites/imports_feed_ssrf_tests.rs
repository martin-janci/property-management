//! Regression: the feed-subscription endpoints
//! (`reality-server/src/routes/imports.rs`) must not be usable for SSRF.
//!
//! `create_feed` / `update_feed` took the user-supplied `feed_url` straight
//! into the DB without validation, while the sibling import-job handlers
//! (`create_import_job` / `update_import_job`) already ran
//! `common::url_validation::validate_external_url`. A feed's `feed_url` is
//! fetched server-side by the sync path (`sync_feed` / scheduled sync), so an
//! unvalidated URL let an attacker point the fetcher at cloud-metadata
//! (`http://169.254.169.254/`), loopback/private ranges, or a `file://` URL.
//! The fix rejects such URLs with `400 Bad Request` *before* persisting.
//!
//! Validation runs before the agency lookup / DB write, so a seeded user (for
//! the `PortalPrincipal` extractor) is all these cases need — the SSRF URLs
//! never reach the repo.

use crate::common::{make_app_state, mint_token, send_json};
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
    .bind(format!("{tag}@imports-ssrf.test"))
    .bind(format!("ImportsSSRF {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

/// URLs that must be rejected as SSRF-prone by both feed handlers. Mirrors the
/// vectors in `common::url_validation` tests and the workflow-webhook SSRF suite.
const SSRF_URLS: &[&str] = &[
    "http://169.254.169.254/latest/meta-data/", // AWS/GCP link-local metadata
    "https://metadata.google.internal/computeMetadata/v1/", // GCP metadata host
    "http://127.0.0.1:8080/internal",           // loopback
    "http://localhost/admin",                   // loopback hostname
    "http://10.0.0.5/feed.xml",                 // RFC1918 private
    "http://192.168.1.1/feed.xml",              // RFC1918 private
    "file:///etc/passwd",                       // non-http(s) scheme
];

// ── create_feed (POST /feeds) ────────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_feed_rejects_ssrf_urls(pool: PgPool) {
    let user = seed_user(&pool, "create").await;
    let token = mint_token(user);
    let app = imports_router(pool);

    for url in SSRF_URLS {
        let status = send_json(
            &app,
            Method::POST,
            "/api/v1/imports/feeds",
            Some(&token),
            json!({ "name": "evil feed", "feed_url": url }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "feed_url {url} must be rejected with 400 before it is persisted / fetched, got {status}"
        );
    }
}

// ── update_feed (PUT /feeds/{id}) ────────────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_feed_rejects_ssrf_urls(pool: PgPool) {
    let user = seed_user(&pool, "update").await;
    let token = mint_token(user);
    let app = imports_router(pool);
    // A random id is fine: validation runs before the row is looked up, so an
    // SSRF url must 400 regardless of whether the feed exists.
    let feed_id = Uuid::new_v4();

    for url in SSRF_URLS {
        let status = send_json(
            &app,
            Method::PUT,
            &format!("/api/v1/imports/feeds/{feed_id}"),
            Some(&token),
            json!({ "feed_url": url }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "update feed_url {url} must be rejected with 400 before fetch, got {status}"
        );
    }
}

/// A well-formed public https feed_url must NOT be rejected by the SSRF guard.
/// Without an agency the caller is not authorised to own feeds, so the request
/// gets past validation and resolves to `403` (not a member of any agency) —
/// crucially *not* `400`, proving the guard does not over-reject legitimate URLs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_feed_allows_public_https_url_past_validation(pool: PgPool) {
    let user = seed_user(&pool, "public").await;
    let token = mint_token(user);
    let app = imports_router(pool);

    let status = send_json(
        &app,
        Method::POST,
        "/api/v1/imports/feeds",
        Some(&token),
        json!({ "name": "good feed", "feed_url": "https://feed.example.com/listings.xml" }),
    )
    .await;
    assert_ne!(
        status,
        StatusCode::BAD_REQUEST,
        "a public https feed_url must pass the SSRF guard (expected non-400), got {status}"
    );
}

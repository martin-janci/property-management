//! Route-level pagination-clamp regression tests for
//! `GET /api/v1/listings` (`reality-server/src/routes/listings.rs`).
//!
//! # Background
//!
//! PR #959 (issue #953) added handler-side clamping of the `page`/`limit`
//! query params so a negative `limit` can never reach SQL (Postgres rejects
//! `LIMIT -1` with a raw "LIMIT must not be negative" error, which surfaced as
//! a 500) and an unbounded `limit`/`page` can't become a resource-exhaustion
//! vector. That PR shipped with *unit* tests on the private `clamp_pagination`
//! helper only — there was no route/integration test proving the wired
//! endpoint actually clamps. This file closes that gap.
//!
//! The `search` handler echoes the *effective* (post-clamp) `page` and `limit`
//! back in `ListingSearchResponse`, so these tests drive the real production
//! router end-to-end via `oneshot` and assert on the echoed values:
//!
//! | Case | Query                 | Expected echoed |
//! |------|-----------------------|-----------------|
//! | C1   | (none)                | page 1, limit 20 (defaults) |
//! | C2   | `?limit=-1`           | limit 1 (negative floored)  |
//! | C3   | `?limit=0`            | limit 1 (zero floored)      |
//! | C4   | `?limit=100000`       | limit 100 (capped at max)   |
//! | C5   | `?page=-5`            | page 1 (negative floored)   |
//! | C6   | `?page=2&limit=50`    | page 2, limit 50 (untouched)|
//!
//! The search endpoint returns 200 on an empty database (see the sibling
//! `listings_tests.rs`), so no seeding is required — the clamp is observable
//! purely from the response metadata.

use crate::common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use reality_server::routes;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

fn listings_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/listings", routes::listings::router())
        .with_state(state)
}

/// Drive `GET /api/v1/listings{query}` through the real router and return the
/// status plus the decoded JSON body.
async fn search(router: &Router, query: &str) -> (StatusCode, Value) {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/listings{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    let json: Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!(
            "body was not JSON ({e}): {:?}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, json)
}

// C1 — no params: the endpoint echoes the documented defaults.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn defaults_when_no_pagination_params(pool: PgPool) {
    let router = listings_router(pool);
    let (status, body) = search(&router, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["page"], 1, "default page must be 1");
    assert_eq!(body["limit"], 20, "default limit must be 20");
}

// C2 — a negative limit must never reach SQL; it is floored to 1 and the
// request still succeeds (no raw "LIMIT must not be negative" 500).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn negative_limit_is_clamped_to_one(pool: PgPool) {
    let router = listings_router(pool);
    let (status, body) = search(&router, "?limit=-1").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "negative limit must be clamped, not 500"
    );
    assert_eq!(body["limit"], 1, "limit=-1 must be floored to 1");
}

// C3 — a zero limit is raised to the minimum of 1.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn zero_limit_is_clamped_to_one(pool: PgPool) {
    let router = listings_router(pool);
    let (status, body) = search(&router, "?limit=0").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["limit"], 1, "limit=0 must be raised to 1");
}

// C4 — an oversized limit is capped at MAX_LISTINGS_LIMIT (100).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn oversized_limit_is_capped_at_max(pool: PgPool) {
    let router = listings_router(pool);
    let (status, body) = search(&router, "?limit=100000").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["limit"], 100, "oversized limit must be capped at 100");
}

// C5 — a negative page is floored to 1.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn negative_page_is_clamped_to_one(pool: PgPool) {
    let router = listings_router(pool);
    let (status, body) = search(&router, "?page=-5").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["page"], 1, "page=-5 must be floored to 1");
}

// C6 — in-range values pass through unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn valid_pagination_passes_through(pool: PgPool) {
    let router = listings_router(pool);
    let (status, body) = search(&router, "?page=2&limit=50").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["page"], 2, "valid page must pass through");
    assert_eq!(body["limit"], 50, "valid limit must pass through");
}

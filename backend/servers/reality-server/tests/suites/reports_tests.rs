//! Behaviour tests for POST /api/v1/reports and GET /api/v1/reports/me.

use crate::common;

use api_core::middleware::TenantRateLimiterSet;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use reality_server::routes;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

fn reports_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/reports", routes::reports::router())
        .with_state(state)
}

/// Router whose anonymous per-IP report throttle is capped at `rpm` requests
/// per minute, so a regression test can trip it in a handful of calls instead
/// of the production default. Reuses the same `inquiry_rate_limiters` slot the
/// route enforces against.
fn reports_router_rate_limited(pool: PgPool, rpm: u32) -> Router {
    common::ensure_test_env();
    let mut state = common::make_app_state(pool);
    state.inquiry_rate_limiters = Arc::new(TenantRateLimiterSet::with_default_bounded(
        rpm,
        Duration::from_secs(3600),
        1000,
    ));
    Router::new()
        .nest("/api/v1/reports", routes::reports::router())
        .with_state(state)
}

/// Seed an `active` listing and return its id (org + owner created inline).
async fn seed_active_listing(pool: &PgPool, tag: &str) -> Uuid {
    let owner = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, principal_kind)
        VALUES ($1, 'hash', $2, 'active', 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@reports-rl.test"))
    .bind(format!("Reports RL {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"));

    let org = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Reports Org {tag}"))
    .bind(format!("reports-org-{tag}"))
    .bind(format!("{tag}@reports-org.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_org({tag}): {e}"));

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Report Listing', 'apartment', 100000.00, 'EUR',
                'Test Street 7', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(owner)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_listing({tag}): {e}"))
}

/// POST /api/v1/reports for `listing_id` with a fixed `X-Forwarded-For`.
async fn post_report_from_ip(router: &Router, listing_id: Uuid, xff: &str) -> StatusCode {
    let body = serde_json::json!({
        "listing_id": listing_id,
        "problem_type": "fraudulent_listing",
        "description": "spam report body",
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/reports")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-forwarded-for", xff)
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    router.clone().oneshot(req).await.unwrap().status()
}

async fn reports_count(pool: &PgPool, listing_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM listing_reports WHERE listing_id = $1")
        .bind(listing_id)
        .fetch_one(pool)
        .await
        .expect("count listing_reports")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_report_unauthenticated_invalid_listing_returns_404_or_unprocessable(pool: PgPool) {
    let router = reports_router(pool);
    let listing_id = Uuid::new_v4();
    let status = common::send_json(
        &router,
        Method::POST,
        "/api/v1/reports",
        None,
        serde_json::json!({
            "listing_id": listing_id,
            "reason": "spam",
            "description": "Test report"
        }),
    )
    .await;
    // Either 404 (listing not found) or 422 (validation error) are valid
    assert!(
        status == axum::http::StatusCode::NOT_FOUND
            || status == axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        "expected 404 or 422, got {status}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_reports_unauthenticated_returns_401(pool: PgPool) {
    let router = reports_router(pool);
    let status = common::send(&router, Method::GET, "/api/v1/reports/me", None).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_reports_authenticated_returns_200(pool: PgPool) {
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Test User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("reports-me-{}@test.internal", user_id))
    .execute(&pool)
    .await
    .expect("seed user");
    let router = reports_router(pool);
    let token = common::mint_token(user_id);
    let status = common::send(&router, Method::GET, "/api/v1/reports/me", Some(&token)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

/// Regression (security: anonymous report flood): the anonymous POST is
/// per-IP throttled. After the quota is exhausted from one IP, further requests
/// get 429 and no new `listing_reports` rows are written.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_report_is_rate_limited_per_ip(pool: PgPool) {
    const RPM: u32 = 3;
    let listing_id = seed_active_listing(&pool, "rl-single").await;
    let router = reports_router_rate_limited(pool.clone(), RPM);
    let ip = "203.0.113.5";

    // First RPM requests are accepted and each inserts a row.
    for i in 0..RPM {
        let status = post_report_from_ip(&router, listing_id, ip).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "request {i} within quota should be accepted"
        );
    }
    // The next request from the same IP trips the throttle.
    let denied = post_report_from_ip(&router, listing_id, ip).await;
    assert_eq!(
        denied,
        StatusCode::TOO_MANY_REQUESTS,
        "request over quota should be 429"
    );

    // The 429 short-circuits before the INSERT: only the accepted requests wrote rows.
    assert_eq!(
        reports_count(&pool, listing_id).await,
        RPM as i64,
        "rate-limited request must not touch the DB"
    );
}

/// Regression: the throttle buckets by client IP — two distinct
/// `X-Forwarded-For` values each get their own independent quota.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_report_rate_limit_buckets_per_ip(pool: PgPool) {
    const RPM: u32 = 2;
    let listing_id = seed_active_listing(&pool, "rl-multi").await;
    let router = reports_router_rate_limited(pool.clone(), RPM);
    let ip_a = "198.51.100.10";
    let ip_b = "198.51.100.20";

    // Interleave two IPs: each should get RPM accepts then a 429, independently.
    for _ in 0..RPM {
        assert_eq!(
            post_report_from_ip(&router, listing_id, ip_a).await,
            StatusCode::CREATED
        );
        assert_eq!(
            post_report_from_ip(&router, listing_id, ip_b).await,
            StatusCode::CREATED
        );
    }
    assert_eq!(
        post_report_from_ip(&router, listing_id, ip_a).await,
        StatusCode::TOO_MANY_REQUESTS,
        "IP A over its own quota should be 429"
    );
    assert_eq!(
        post_report_from_ip(&router, listing_id, ip_b).await,
        StatusCode::TOO_MANY_REQUESTS,
        "IP B over its own quota should be 429"
    );

    // Each bucket wrote exactly RPM rows ⇒ 2 * RPM total.
    assert_eq!(reports_count(&pool, listing_id).await, (2 * RPM) as i64);
}

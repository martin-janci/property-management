//! Behaviour tests for POST /api/v1/reports and GET /api/v1/reports/me.

use crate::common;

use axum::{http::Method, Router};
use reality_server::routes;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn reports_router(pool: PgPool) -> Router {
    common::ensure_test_env();
    let state = common::make_app_state(pool);
    Router::new()
        .nest("/api/v1/reports", routes::reports::router())
        .with_state(state)
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

/// Regression: `total` in the paginated response must be the full matching
/// row count, not the length of the current page. Before the fix
/// `list_my_reports` returned `total = reports.len()`, so any non-final page
/// reported `total == limit` and `ceil(total / limit)` pagination broke.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_reports_total_reflects_full_count_not_page_len(pool: PgPool) {
    // Seed an org + listing + reporter user, then 3 reports by that user.
    let org_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ('Rep Org', 'rep-org-total', 'rep-total@org.test', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed org");

    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, name, principal_kind, status) \
         VALUES ($1, $2, 'x', 'Rep User', 'platform', 'active')",
    )
    .bind(user_id)
    .bind(format!("reports-total-{user_id}@test.internal"))
    .execute(&pool)
    .await
    .expect("seed user");

    let listing_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO listings \
            (organization_id, created_by, status, transaction_type, title, \
             property_type, price, currency, street, city, postal_code, country) \
         VALUES ($1, $2, 'active', 'sale', 'Rep Listing', 'apartment', 100000.00, \
                 'EUR', 'Test Street 1', 'Bratislava', '81101', 'SK') RETURNING id",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("seed listing");

    for i in 0..3 {
        sqlx::query(
            "INSERT INTO listing_reports \
                (listing_id, reporter_user_id, problem_type, description, status) \
             VALUES ($1, $2, 'spam', $3, 'received')",
        )
        .bind(listing_id)
        .bind(user_id)
        .bind(format!("report {i}"))
        .execute(&pool)
        .await
        .expect("seed report");
    }

    let router = reports_router(pool);
    let token = common::mint_token(user_id);

    // Ask for a page of 2 out of 3.
    let req = axum::http::Request::builder()
        .method(Method::GET)
        .uri("/api/v1/reports/me?limit=2")
        .header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("parse body");

    assert_eq!(
        json["reports"].as_array().expect("reports array").len(),
        2,
        "page should hold exactly `limit` (2) rows"
    );
    assert_eq!(
        json["total"].as_u64().expect("total is a number"),
        3,
        "total must report the full matching count (3), not the page length (2)"
    );
}

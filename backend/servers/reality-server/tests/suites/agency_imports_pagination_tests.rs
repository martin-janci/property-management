//! Pagination regression tests for the agency import-history surface
//! (`reality-server/src/routes/agency_imports.rs`, `GET
//! /api/v1/agencies/{id}/imports`).
//!
//! The list query caps the page at `LIMIT 100`. The response `total` must be
//! the full count of matching rows (same `agency_id` filter, ignoring the
//! `LIMIT`) so clients can page — not the length of the returned page. Before
//! the fix the handler set `total = rows.len() as i64`, so once an agency had
//! more than 100 jobs the reported `total` saturated at 100 (the capped page
//! length), hiding the remaining rows from any paging client.

use crate::common::{make_app_state, mint_token};
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use reality_server::routes;
use sqlx::PgPool;
use uuid::Uuid;

/// Mount only the agency-imports sub-router at its production path so the
/// `{id}` path parameter resolves exactly as in `main.rs`.
fn agency_imports_router(pool: PgPool) -> Router {
    let state = make_app_state(pool);
    Router::new()
        .nest(
            "/api/v1/agencies/{id}/imports",
            routes::agency_imports::router(),
        )
        .with_state(state)
}

/// Seed a platform user and return its id.
async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', $2, 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(format!("{tag}@agency-imports-pg.test"))
    .bind(format!("AgencyImportsPG {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
}

/// Seed an agency with `user_id` as an active `owner` member and return the
/// agency id.
async fn seed_agency_with_member(pool: &PgPool, user_id: Uuid, tag: &str) -> Uuid {
    let agency_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO reality_agencies (name, slug, email)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(format!("Agency {tag}"))
    .bind(format!("agency-{tag}"))
    .bind(format!("{tag}@agency-imports-pg.test"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_agency({tag}): {e}"));
    sqlx::query(
        "INSERT INTO reality_agency_members (agency_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(agency_id)
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_agency_membership({tag}): {e}"));
    agency_id
}

/// Bulk-seed `n` import jobs owned by `user_id` and scoped to `agency_id`.
async fn seed_agency_import_jobs(pool: &PgPool, agency_id: Uuid, user_id: Uuid, n: i32) {
    sqlx::query(
        "INSERT INTO portal_import_jobs (agency_id, user_id, source_type) \
         SELECT $1, $2, 'csv' FROM generate_series(1, $3)",
    )
    .bind(agency_id)
    .bind(user_id)
    .bind(n)
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_agency_import_jobs({n}): {e}"));
}

/// Oneshot a GET and return `(status, parsed JSON body)`.
async fn get_json(router: &Router, path: &str, token: &str) -> (StatusCode, serde_json::Value) {
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

// Regression: `total` must be the full match count, not the capped page length.
// With 101 agency jobs and a hard `LIMIT 100` on the page query, the page holds
// 100 rows but `total` must report 101.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_import_history_total_is_full_count_not_page_len(pool: PgPool) {
    let user = seed_user(&pool, "total-count").await;
    let agency_id = seed_agency_with_member(&pool, user, "total-count").await;
    seed_agency_import_jobs(&pool, agency_id, user, 101).await;
    let token = mint_token(user);
    let app = agency_imports_router(pool);

    let path = format!("/api/v1/agencies/{agency_id}/imports");
    let (status, body) = get_json(&app, &path, &token).await;
    assert!(status.is_success(), "expected 2xx, got {status}");

    let page_len = body["jobs"].as_array().expect("jobs array").len();
    let total = body["total"].as_i64().expect("total i64");
    assert_eq!(page_len, 100, "page must be capped at LIMIT 100");
    assert_eq!(
        total, 101,
        "total must be the full match count (101), not the capped page length ({page_len})"
    );
}

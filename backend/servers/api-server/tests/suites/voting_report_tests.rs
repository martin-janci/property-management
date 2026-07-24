//! Voting **report** integration tests (UC-04, Story 5.6/5.8).
//!
//! These exercise the full PDF-report flow (register → seed building/vote →
//! `GET /report.pdf`), so every test needs the complete schema.
//!
//! As originally written (in #1625) inside `voting_tests.rs`, these failed
//! deterministically on `dev`: register 500'd with `relation "users" does not
//! exist`. Root cause (pinned in #1665): the report tests were authored as a
//! **bare `#[sqlx::test]`** (no `migrator = ...`), so sqlx 0.9 created an empty
//! per-test database and attached no migrator — the schema was never applied.
//! (`#[sqlx::test]`'s inferred-path discovery looks for a `migrations/` dir in
//! *this* crate, which doesn't exist.) The earlier "mysterious binary-specific
//! macro no-op" theory was wrong: the macro expands each test fn independently,
//! so a `migrator = "db::MIGRATOR"` test is never disabled by a schema-less
//! sibling. See the test-author guide on `db::MIGRATOR` for the full mechanism.
//!
//! We keep the bare `#[sqlx::test]` and run the migrations **explicitly** on
//! the per-test pool via `db::run_migrations` (the exact production path;
//! idempotent + advisory-locked). This is one of the two canonical schema-full
//! idioms (the other being `#[sqlx::test(migrator = "db::MIGRATOR")]`); both
//! apply the same migration set. The explicit form turns any migration failure
//! into a loud, attributable panic instead of a downstream `users does not
//! exist`. Kept in a dedicated binary to isolate the heavy schema-full E2E flow
//! from `voting_tests.rs`'s schema-less auth tests. (BIT-158, #1665)

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_vote(pool: &PgPool, org_id: Uuid, building_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO votes (organization_id, building_id, title, end_at, created_by)
        VALUES ($1, $2, 'Annual budget vote', NOW() + interval '7 days', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed vote")
}

#[sqlx::test]
async fn test_get_report_pdf_returns_application_pdf_and_archives_document(pool: PgPool) {
    db::run_migrations(&pool)
        .await
        .expect("apply full schema migrations");
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "test-org").await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let building_id = seed_building(&app.pool, org_id, "vote-building").await;
    let vote_id = seed_vote(&app.pool, org_id, building_id, user_id).await;

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/voting/{}/report.pdf", vote_id))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();

    let response = app.execute(request).await;
    assert_eq!(response.status, StatusCode::OK);

    // Assert content type is application/pdf
    let content_type = response
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(content_type, "application/pdf");

    // Assert body is not empty
    assert!(
        !response.body.is_empty(),
        "PDF response body should not be empty"
    );

    // Assert that a document was archived in the database
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE organization_id = $1 AND category = 'reports'::document_category")
        .bind(org_id)
        .fetch_one(&app.pool)
        .await
        .expect("query documents count");
    assert_eq!(
        count, 1,
        "There should be exactly one archived document for the vote report"
    );
}

#[sqlx::test]
async fn test_get_report_json_with_format_pdf_returns_application_pdf(pool: PgPool) {
    db::run_migrations(&pool)
        .await
        .expect("apply full schema migrations");
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "test-org-2").await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let building_id = seed_building(&app.pool, org_id, "vote-building-2").await;
    let vote_id = seed_vote(&app.pool, org_id, building_id, user_id).await;

    // Query parameter: ?format=pdf
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/voting/{}/report?format=pdf", vote_id))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();

    let response = app.execute(request).await;
    assert_eq!(response.status, StatusCode::OK);

    let content_type = response
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(content_type, "application/pdf");

    // Accept header: Accept: application/pdf
    let request_accept = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/voting/{}/report", vote_id))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::ACCEPT, "application/pdf")
        .body(Body::empty())
        .unwrap();

    let response_accept = app.execute(request_accept).await;
    assert_eq!(response_accept.status, StatusCode::OK);

    let content_type_accept = response_accept
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(content_type_accept, "application/pdf");
}

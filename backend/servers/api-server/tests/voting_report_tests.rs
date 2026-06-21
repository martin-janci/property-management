//! Voting **report** integration tests (UC-04, Story 5.6/5.8).
//!
//! These exercise the full PDF-report flow (register → seed building/vote →
//! `GET /report.pdf`), so every test needs the complete schema and therefore
//! uses `#[sqlx::test(migrator = "db::MIGRATOR")]`.
//!
//! They live in their own test binary (separate from `voting_tests.rs`) on
//! purpose: `voting_tests.rs` mixes schema-less `#[sqlx::test]` auth-rejection
//! tests with these schema-full ones, and under sqlx 0.9 mixing
//! migrator/no-migrator tests in a single test binary makes the `db::MIGRATOR`
//! tests come up on an un-migrated database (register 500s with
//! `relation "users" does not exist`). Keeping every test in this binary on the
//! same `db::MIGRATOR` setup mirrors the proven-good `auth_enumeration_tests`
//! and keeps the suite green. (BIT-158)

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_report_pdf_returns_application_pdf_and_archives_document(pool: PgPool) {
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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_report_json_with_format_pdf_returns_application_pdf(pool: PgPool) {
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

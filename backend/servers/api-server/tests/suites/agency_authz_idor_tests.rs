//! Regression tests for the agency endpoint authorization gaps (#818).
//!
//! Audit history: several `/api/v1/agencies/*` handlers exposed agency-scoped
//! data without verifying the caller belongs to the agency:
//!
//! - `GET /api/v1/agencies/{id}`            (`get_agency`)      — NO auth at all.
//! - `GET /api/v1/agencies/{id}/import/{job_id}` (`get_import_job`) — NO auth.
//! - `GET /api/v1/agencies/{id}/members`    (`list_members`)    — extractor but
//!   no membership check.
//! - `GET /api/v1/agencies/{id}/import`     (`list_import_jobs`) — ditto.
//! - `POST /api/v1/agencies/{id}/import`    (`create_import_job`)— authenticated
//!   but no per-agency authorization.
//!
//! The fix gates reads behind `verify_agency_member` and the import-creation
//! write behind `verify_agency_admin`, and binds a fetched import job to the
//! agency in the path.
//!
//! TestApp wiring caveat (see `equipment_cross_tenant_idor_tests.rs`): the
//! harness mounts the router without `host_tenant_middleware`, so a request
//! lacking a fully-provisioned bearer JWT is rejected at the auth/extractor
//! gate. That is exactly the security contract under test here — an
//! unauthenticated caller must never receive agency data. The headline
//! regression is `get_agency`, which previously returned 200 with the full
//! agency record and now returns 4xx.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::TestApp;

async fn seed_agency(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO agencies (name, slug, email, status)
        VALUES ($1, $2, $3, 'verified') RETURNING id
        "#,
    )
    .bind(format!("AuthzIDOR Agency {slug}"))
    .bind(format!("authz-idor-agency-{slug}"))
    .bind(format!("{slug}@authz-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed agency")
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: expected a 4xx rejection, got {status}"
    );
    assert_ne!(
        status,
        StatusCode::OK,
        "{ctx}: agency data must NOT be returned to an unauthenticated caller"
    );
}

/// Headline regression: `GET /api/v1/agencies/{id}` had NO auth extractor and
/// returned the full agency record to anyone. With a seeded agency, dev
/// returns 200; the fix returns 4xx.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agency_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let agency_id = seed_agency(&pool, "get").await;

    let resp = app
        .execute(get(&format!("/api/v1/agencies/{agency_id}")))
        .await;

    assert_rejected(resp.status, "GET /api/v1/agencies/{id} without auth");
}

/// `GET /api/v1/agencies/{id}/members` must not list the roster anonymously.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_members_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let agency_id = seed_agency(&pool, "members").await;

    let resp = app
        .execute(get(&format!("/api/v1/agencies/{agency_id}/members")))
        .await;

    assert_rejected(
        resp.status,
        "GET /api/v1/agencies/{id}/members without auth",
    );
}

/// `GET /api/v1/agencies/{id}/import/{job_id}` had NO auth extractor.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_import_job_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let agency_id = seed_agency(&pool, "getjob").await;
    let job_id = Uuid::new_v4();

    let resp = app
        .execute(get(&format!(
            "/api/v1/agencies/{agency_id}/import/{job_id}"
        )))
        .await;

    assert_rejected(
        resp.status,
        "GET /api/v1/agencies/{id}/import/{job_id} without auth",
    );
}

/// `GET /api/v1/agencies/{id}/import` must not list import jobs anonymously.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_import_jobs_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let agency_id = seed_agency(&pool, "listjobs").await;

    let resp = app
        .execute(get(&format!("/api/v1/agencies/{agency_id}/import")))
        .await;

    assert_rejected(resp.status, "GET /api/v1/agencies/{id}/import without auth");
}

/// `POST /api/v1/agencies/{id}/import` must not start an import anonymously.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_import_job_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let agency_id = seed_agency(&pool, "createjob").await;

    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/agencies/{agency_id}/import"))
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"source":"csv"}"#))
        .unwrap();
    let resp = app.execute(req).await;

    assert_rejected(
        resp.status,
        "POST /api/v1/agencies/{id}/import without auth",
    );
}

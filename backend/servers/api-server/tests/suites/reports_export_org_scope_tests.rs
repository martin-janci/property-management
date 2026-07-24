//! Real-JWT cross-tenant regression tests for report export endpoints (#832).
//!
//! `export_report` (`POST /api/v1/reports/export`) trusted the client-supplied
//! `organization_id`, and `get_export_job_status`
//! (`GET /api/v1/reports/export/{job_id}/status`) returned any job's status and
//! download URL by UUID. Neither verified the resource's org against the
//! caller's tenant — a cross-tenant IDOR.
//!
//! The fix threads `RlsConnection` into both and rejects when
//! `!is_super_admin() && org != tenant_id()`, matching the other report
//! handlers. These tests exercise the path end-to-end with a real JWT for a
//! Manager in Org B (so the auth gate passes — not a 401) targeting Org A.
//!
//! TestApp note (see report_schedule_org_scope_jwt_tests.rs): no
//! `host_tenant_middleware`, so the tenant comes from the `X-Tenant-ID` header;
//! `RlsConnection` validates membership against `organization_members`, seeded
//! below.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, TestApp, TestUser};

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("ExportScope {slug}"))
    .bind(format!("export-scope-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@export-scope.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Seed a background job (an export job) owned by `org_id`.
async fn seed_export_job(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO background_jobs (job_type, queue, org_id)
           VALUES ('report_export', 'reports', $1) RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed export job")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user id")
}

/// Authenticate a manager in Org B; returns (token, org_a, org_b).
async fn setup(pool: &PgPool, app: &TestApp) -> (String, Uuid, Uuid) {
    let org_a = seed_org(pool, "a").await;
    let org_b = seed_org(pool, "b").await;
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(app, &user).await;
    let uid = user_id_for(pool, &user.email).await;
    seed_membership(pool, org_b, uid, "manager").await;
    (token, org_a, org_b)
}

/// `export_report` must reject a request whose `organization_id` (Org A) does
/// not match the caller's tenant (Org B). On dev it trusts the body and
/// proceeds; the fix returns 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn export_report_for_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_a, org_b) = setup(&pool, &app).await;

    let body = json!({
        "organization_id": org_a, // foreign org
        "report_type": "faults",
        "format": "csv"
    });
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/reports/export")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_b.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "exporting another org's report must be 403, got {}",
        resp.status
    );
}

/// `get_export_job_status` must not reveal another org's export job. The job is
/// owned by Org A; the caller is in Org B. On dev it returns the status; the
/// fix returns 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_export_job_status_for_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_a, org_b) = setup(&pool, &app).await;
    let job_id = seed_export_job(&pool, org_a).await;

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/reports/export/{job_id}/status"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_b.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "reading another org's export job must be 404, got {}",
        resp.status
    );
}

/// Same-tenant export job status is readable (the guard must not over-reject).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_export_job_status_for_own_org_is_allowed(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _org_a, org_b) = setup(&pool, &app).await;
    let job_id = seed_export_job(&pool, org_b).await; // owned by caller's org

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/reports/export/{job_id}/status"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_b.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "owner org must read its own export job, got {}",
        resp.status
    );
}

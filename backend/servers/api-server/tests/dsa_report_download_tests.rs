//! Regression guard for GH #1201 / BIT-92: the DSA report download endpoint
//! must dereference `report_file_path` to a presigned storage URL, not return
//! a self-referential `download_url` that points back to itself.
//!
//! # What we assert
//!
//! `GET /api/v1/aml-dsa/dsa/reports/{id}/download` must:
//!
//! 1. Return `503` when storage is not configured (CI/TestApp path) — proving
//!    the request passed auth + role gate + DB lookup and reached the presigner,
//!    not short-circuited by the self-ref bug (which would return `200` with a
//!    `download_url` field).
//! 2. Return `404` for a report without a `report_file_path`, not a self-ref.
//! 3. Return `401` for unauthenticated callers.
//! 4. Return `403` for a tenant-role caller (platform_compliance role required).
//!
//! # Why 503 is the success signal
//!
//! `TestApp::new` leaves `storage_service = None`; the handler reaches the
//! presigner only AFTER auth + role + DB lookup have all passed. A 503 here
//! means "would have presigned but no storage configured" — exactly the
//! dereference path. The self-ref bug returned 200 with a JSON body containing
//! `download_url` pointing back to the same path, so asserting 503 (not 200)
//! is a clean regression guard.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role.to_string()),
        email: format!("{user_id}@dsa-dl.test"),
        name: "DSA DL Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

async fn seed_org(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status)
         VALUES ($1, $2, $3, 'active') RETURNING id",
    )
    .bind(format!("DSA DL Org {}", Uuid::new_v4()))
    .bind(format!("dsa-dl-{}", Uuid::new_v4()))
    .bind(format!("{}@dsa-dl.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at)
         VALUES ($1, 'x', 'DSA DL User', 'active', NOW()) RETURNING id",
    )
    .bind(format!("{}@dsa-dl.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a platform-kind user. `RequestPrincipal` reads `principal_kind` from
/// the DB (not the JWT), so platform-gated routes require this.
async fn seed_platform_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
         VALUES ($1, 'x', 'DSA DL Platform', 'active', NOW(), 'platform') RETURNING id",
    )
    .bind(format!("{}@dsa-dl-platform.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed platform user")
}

/// Seed a DSA report with `report_file_path` set (simulates a published report).
async fn seed_report_with_file(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO dsa_transparency_reports (
             period_start, period_end, status,
             total_moderation_actions, content_removed_count, content_restricted_count,
             warnings_issued_count, user_reports_received, user_reports_resolved,
             avg_resolution_time_hours, automated_decisions_count, automated_decisions_overturned,
             appeals_received, appeals_upheld, appeals_rejected,
             content_type_breakdown, violation_type_breakdown,
             generated_at, generated_by,
             published_at, report_file_path
         )
         VALUES (
             NOW() - INTERVAL '30 days', NOW(), 'published',
             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
             '{}', '{}',
             NOW(), $1,
             NOW(), 'reports/dsa/test-report.pdf'
         )
         RETURNING id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed report with file")
}

/// Seed a DSA report WITHOUT `report_file_path` (not yet generated).
async fn seed_report_no_file(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO dsa_transparency_reports (
             period_start, period_end, status,
             total_moderation_actions, content_removed_count, content_restricted_count,
             warnings_issued_count, user_reports_received, user_reports_resolved,
             avg_resolution_time_hours, automated_decisions_count, automated_decisions_overturned,
             appeals_received, appeals_upheld, appeals_rejected,
             content_type_breakdown, violation_type_breakdown,
             generated_at, generated_by
         )
         VALUES (
             NOW() - INTERVAL '60 days', NOW() - INTERVAL '30 days', 'generated',
             0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
             '{}', '{}',
             NOW(), $1
         )
         RETURNING id",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed report no file")
}

fn get_download(report_id: Uuid, org: Uuid, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v1/aml-dsa/dsa/reports/{report_id}/download"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .body(Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// T1 — Unauthenticated → 401
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dsa_download_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let resp = app
        .execute(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/aml-dsa/dsa/reports/{}/download",
                    Uuid::new_v4()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED, "must require auth");
}

// ---------------------------------------------------------------------------
// T2 — Tenant-role caller → 403 (platform_compliance required)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dsa_download_requires_platform_compliance_role(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool).await;
    let user = seed_user(&pool).await;
    let report = seed_report_with_file(&pool, user).await;

    let token = mint_token(user, org, "manager");
    let resp = app.execute(get_download(report, org, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "tenant-role caller must be refused (403); got {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// T3 — Report with file, platform_admin → 503 (not 200 self-ref)
//
// This is the core regression guard for GH #1201: the old code returned 200
// with `{"download_url": "/api/v1/aml-dsa/dsa/reports/{id}/download"}`.
// The fix reaches the presigner and fails with 503 (no storage in CI).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dsa_download_dereferences_to_presigner_not_self_ref(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool).await;
    let user = seed_platform_user(&pool).await;
    let report = seed_report_with_file(&pool, user).await;

    let token = mint_token(user, org, "platform_admin");
    let resp = app.execute(get_download(report, org, &token)).await;

    // 503 = reached presigner (storage not configured in CI) — proves the
    // handler dereferenced report_file_path instead of returning a self-ref URL.
    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "expected 503 (presigner reached, no storage in CI), got {} — \
         if 200, the self-referential bug has regressed (body: {})",
        resp.status,
        String::from_utf8_lossy(&resp.body)
    );

    // Belt-and-suspenders: confirm the body does NOT contain a download_url
    // pointing back at the same path (the exact shape of the old bug).
    let body_str = String::from_utf8_lossy(&resp.body);
    assert!(
        !body_str.contains("download_url"),
        "response must not contain 'download_url' (self-ref shape); got: {body_str}"
    );
}

// ---------------------------------------------------------------------------
// T4 — Report without file_path → 404
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dsa_download_no_file_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool).await;
    let user = seed_platform_user(&pool).await;
    let report = seed_report_no_file(&pool, user).await;

    let token = mint_token(user, org, "platform_admin");
    let resp = app.execute(get_download(report, org, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "report without file must return 404, got {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// T5 — Unknown report_id → 404
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dsa_download_unknown_report_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool).await;
    let user = seed_platform_user(&pool).await;

    let token = mint_token(user, org, "platform_admin");
    let resp = app.execute(get_download(Uuid::new_v4(), org, &token)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "unknown report must return 404, got {}",
        resp.status
    );
}

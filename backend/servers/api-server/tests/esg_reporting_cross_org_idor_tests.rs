//! Regression tests for the cross-tenant IDOR fix on the ESG reporting
//! endpoints (`/api/v1/esg/*`, Epic 136 — PAP-139, unblocks PAP-72).
//!
//! Audit history (PAP-136): every by-id ESG handler carried `_auth: AuthUser`
//! and performed no org scoping, and the repository ran on the raw pool with
//! no `organization_id` predicate on any by-id query (get/update/verify/
//! delete/submit/approve across esg_metrics, carbon_footprints, esg_targets,
//! esg_reports, eu_taxonomy_assessments, esg_import_jobs). A foreign caller
//! could read/mutate/delete any other org's ESG data by UUID.
//!
//! Since the PAP-139 RLS conversion, every handler acquires an `RlsConnection`
//! (tenant validated against `organization_members`) and the repository is
//! stateless: queries run on the request's RLS-scoped connection AND stay
//! org-keyed — every ESG table carries `organization_id`, so each by-id query
//! is `WHERE id = $1 AND organization_id = $2`. A cross-tenant probe made with
//! the attacker's own valid tenant context resolves to no row → `404`/no-op;
//! "missing" and "forbidden" are indistinguishable. The CI test pool runs as
//! superuser (bypasses FORCE RLS), so these tests specifically prove the
//! org-keyed SQL layer.
//!
//! These tests exercise the HTTP surface end-to-end with real HS256 JWTs:
//!   1. Seed two orgs (A, B), a member user in each, and an ESG metric +
//!      report in Org A.
//!   2. Org B's member probes Org A's resources → rejected (4xx); no leak,
//!      no write.
//!   3. Org A's member reads its own metric → allowed (2xx).

#![allow(dead_code)]

mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, RequestBuilder, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("EsgIDOR Org {slug}"))
    .bind(format!("esg-idor-org-{slug}"))
    .bind(format!("{slug}@esg-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'EsgIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed an ESG metric in `org_id` created by `created_by`, return its id.
async fn seed_metric(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO esg_metrics (
            organization_id, period_start, period_end, category, metric_type,
            metric_name, value, unit, data_source, created_by
        ) VALUES (
            $1, '2026-01-01', '2026-03-31', 'environmental', 'energy_consumption',
            'Total electricity', 12345.0, 'kWh', 'manual', $2
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed metric")
}

/// Seed a draft ESG report in `org_id` created by `created_by`, return its id.
async fn seed_report(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO esg_reports (
            organization_id, report_type, title, period_start, period_end,
            status, created_by
        ) VALUES (
            $1, 'annual', 'Confidential ESG Report 2026', '2026-01-01', '2026-12-31',
            'draft', $2
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed report")
}

/// Mint a real HS256 access token for `user_id`, signed with the same secret
/// the TestApp configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "EsgIDOR User", None, None)
        .expect("mint access token")
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: cross-tenant/unauthenticated request must be rejected with 4xx, got {status}"
    );
}

/// Seed two orgs with one member each plus a metric + report in Org A.
/// Returns (org_a, org_b, user_a, user_b, metric_a, report_a).
async fn seed_two_org_fixture(pool: &PgPool, tag: &str) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
    let org_a = seed_org(pool, &format!("{tag}-a")).await;
    let org_b = seed_org(pool, &format!("{tag}-b")).await;
    let user_a = seed_user(pool, &format!("{tag}-a@esg-idor.test")).await;
    let user_b = seed_user(pool, &format!("{tag}-b@esg-idor.test")).await;
    seed_membership(pool, org_a, user_a, "org_admin").await;
    seed_membership(pool, org_b, user_b, "org_admin").await;
    let metric_a = seed_metric(pool, org_a, user_a).await;
    let report_a = seed_report(pool, org_a, user_a).await;
    (org_a, org_b, user_a, user_b, metric_a, report_a)
}

// ---------------------------------------------------------------------------
// T1 — unauthenticated get_metric is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_metric_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "noauth-a").await;
    let user_a = seed_user(&pool, "noauth-a@esg-idor.test").await;
    let metric_a = seed_metric(&pool, org_a, user_a).await;

    let uri = format!("/api/v1/esg/metrics/{metric_a}");
    let resp = app.execute(app.get(&uri).build()).await;

    assert_rejected(resp.status, "get_metric without bearer token");
}

// ---------------------------------------------------------------------------
// T2 — cross-org get_metric by UUID is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_metric_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, metric_a, _report_a) =
        seed_two_org_fixture(&pool, "get").await;

    let token_b = mint_token(user_b, "get-b@esg-idor.test");
    let uri = format!("/api/v1/esg/metrics/{metric_a}");
    // Valid context for the attacker's OWN org — the by-id probe must fail on
    // row scoping (404), not on a missing tenant header.
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_rejected(resp.status, "get_metric cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A metric must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// T3 — cross-org update_metric is rejected (mutate IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_metric_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, metric_a, _report_a) =
        seed_two_org_fixture(&pool, "upd").await;

    let token_b = mint_token(user_b, "upd-b@esg-idor.test");
    let uri = format!("/api/v1/esg/metrics/{metric_a}");
    let body = json!({ "value": "99999.0" });
    let resp = app
        .execute(
            RequestBuilder::new(Method::PUT, &uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_rejected(resp.status, "update_metric cross-tenant");

    // The metric value must be unchanged.
    let value: rust_decimal::Decimal =
        sqlx::query_scalar("SELECT value FROM esg_metrics WHERE id = $1")
            .bind(metric_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        value.to_string(),
        "12345.0000",
        "Org A metric must not be mutated cross-tenant"
    );
}

// ---------------------------------------------------------------------------
// T4 — cross-org verify_metric is rejected (mutate IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn verify_metric_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, metric_a, _report_a) =
        seed_two_org_fixture(&pool, "ver").await;

    let token_b = mint_token(user_b, "ver-b@esg-idor.test");
    let uri = format!("/api/v1/esg/metrics/{metric_a}/verify");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(json!({ "status": "verified" }))
                .build(),
        )
        .await;

    assert_rejected(resp.status, "verify_metric cross-tenant");

    let status: String =
        sqlx::query_scalar("SELECT verification_status FROM esg_metrics WHERE id = $1")
            .bind(metric_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        status, "unverified",
        "Org A metric must not be verified cross-tenant"
    );
}

// ---------------------------------------------------------------------------
// T5 — cross-org delete_metric is rejected (destructive IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_metric_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, metric_a, _report_a) =
        seed_two_org_fixture(&pool, "del").await;

    let token_b = mint_token(user_b, "del-b@esg-idor.test");
    let uri = format!("/api/v1/esg/metrics/{metric_a}/delete");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(json!({}))
                .build(),
        )
        .await;

    assert_rejected(resp.status, "delete_metric cross-tenant");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM esg_metrics WHERE id = $1")
        .bind(metric_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "Org A metric must not be deleted cross-tenant");
}

// ---------------------------------------------------------------------------
// T6 — cross-org get_report by UUID is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_report_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, _metric_a, report_a) =
        seed_two_org_fixture(&pool, "rep").await;

    let token_b = mint_token(user_b, "rep-b@esg-idor.test");
    let uri = format!("/api/v1/esg/reports/{report_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_rejected(resp.status, "get_report cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A report must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// T7 — cross-org submit_report is rejected (status-transition IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submit_report_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, _metric_a, report_a) =
        seed_two_org_fixture(&pool, "sub").await;

    let token_b = mint_token(user_b, "sub-b@esg-idor.test");
    let uri = format!("/api/v1/esg/reports/{report_a}/submit");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(json!({}))
                .build(),
        )
        .await;

    assert_rejected(resp.status, "submit_report cross-tenant");

    let status: String = sqlx::query_scalar("SELECT status::text FROM esg_reports WHERE id = $1")
        .bind(report_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        status, "draft",
        "Org A report must not transition to pending_review cross-tenant"
    );
}

// ---------------------------------------------------------------------------
// T8 — cross-org delete_report is rejected (destructive IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_report_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, _metric_a, report_a) =
        seed_two_org_fixture(&pool, "rdl").await;

    let token_b = mint_token(user_b, "rdl-b@esg-idor.test");
    let uri = format!("/api/v1/esg/reports/{report_a}/delete");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(json!({}))
                .build(),
        )
        .await;

    assert_rejected(resp.status, "delete_report cross-tenant");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM esg_reports WHERE id = $1")
        .bind(report_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "Org A report must not be deleted cross-tenant");
}

// ---------------------------------------------------------------------------
// T9 — legitimate same-org access succeeds
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_metric_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "own-a").await;
    let user_a = seed_user(&pool, "own-a@esg-idor.test").await;
    seed_membership(&pool, org_a, user_a, "org_admin").await;
    let metric_a = seed_metric(&pool, org_a, user_a).await;

    let token_a = mint_token(user_a, "own-a@esg-idor.test");
    let uri = format!("/api/v1/esg/metrics/{metric_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_a)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to read its own metric: {}",
        resp.text()
    );
    let detail = resp.json_value();
    assert_eq!(
        detail.get("id").and_then(|v| v.as_str()),
        Some(metric_a.to_string().as_str()),
        "own-org read must return the seeded metric"
    );
}

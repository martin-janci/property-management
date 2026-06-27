//! Authz regression tests for platform-admin surface — Batch 2.
//!
//! Continues from `platform_admin_authz_tests.rs` (PR #1862, batch 1).
//! Batch 1 covered: capabilities, impersonation, agencies, memberships,
//! audit, metrics, principals, tenant-branding, tenant-feature-flags.
//!
//! This batch covers:
//!   * `/api/v1/admin/users/*`            — user lifecycle (list/get/suspend/reactivate/delete)
//!   * `/api/v1/admin/mfa/enroll/*`       — TOTP enrollment start + verify
//!   * `/api/v1/admin/notifications/analytics` — notification analytics read
//!   * `/api/v1/admin/tenants/{id}/export|purge` + `/admin/tenants/restore` — lifecycle ops
//!   * `/api/v1/infrastructure/**`        — full infra surface (traces, flags, jobs, health)
//!   * `/api/v1/operations/**`            — deployment/migration/backup/DR/cost surface
//!
//! Pattern (same as batch 1):
//!   1. Unauthenticated → 401
//!   2. Authenticated ordinary user (no capabilities) → 403

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use common::{create_authenticated_user, TestApp, TestUser};

const UUID: &str = "00000000-0000-0000-0000-000000000001";

fn anon(method: Method, uri: &str, body: Option<&str>) -> Request<Body> {
    let b = Request::builder().method(method).uri(uri);
    match body {
        Some(j) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

fn authed(token: &str, method: Method, uri: &str, body: Option<&str>) -> Request<Body> {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    match body {
        Some(j) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Case tables
// ---------------------------------------------------------------------------

/// Admin user-lifecycle: /api/v1/admin/users/*
fn admin_users_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/users";
    vec![
        (Method::GET, format!("{base}"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/suspend"), None),
        (Method::POST, format!("{base}/{UUID}/reactivate"), None),
        (Method::POST, format!("{base}/{UUID}/delete"), None),
    ]
}

/// Admin MFA enrollment: /api/v1/admin/mfa/enroll/*
/// Note: `/admin/mfa/verify` and `/admin/mfa/recovery/use` and `/admin/mfa/disable`
/// are exercised by dedicated test files; only the enrollment start/verify are new here.
fn admin_mfa_enroll_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/mfa";
    vec![
        (Method::POST, format!("{base}/enroll/start"), None),
        (
            Method::POST,
            format!("{base}/enroll/verify"),
            Some(r#"{"code":"123456"}"#),
        ),
    ]
}

/// Admin notification analytics: /api/v1/admin/notifications/analytics
fn admin_notifications_cases() -> Vec<(Method, String, Option<&'static str>)> {
    vec![(
        Method::GET,
        "/api/v1/admin/notifications/analytics".to_string(),
        None,
    )]
}

/// Tenant lifecycle ops: /api/v1/admin/tenants/{id}/export|purge + /restore
fn tenant_lifecycle_cases() -> Vec<(Method, String, Option<&'static str>)> {
    vec![
        (
            Method::POST,
            format!("/api/v1/admin/tenants/{UUID}/export"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/admin/tenants/{UUID}/purge"),
            None,
        ),
        // restore is multipart; JSON body is enough to reach the 401/403 gate
        (
            Method::POST,
            "/api/v1/admin/tenants/restore".to_string(),
            None,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Infrastructure cases (/api/v1/infrastructure/*)
// ---------------------------------------------------------------------------

fn infra_traces_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/infrastructure/traces";
    vec![
        (Method::GET, format!("{base}"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::GET, format!("{base}/{UUID}/spans"), None),
    ]
}

fn infra_feature_flags_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/infrastructure/feature-flags";
    vec![
        (Method::GET, format!("{base}"), None),
        (
            Method::POST,
            format!("{base}"),
            Some(r#"{"key":"test-flag","enabled":false}"#),
        ),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}"),
            Some(r#"{"enabled":true}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/toggle"), None),
        (Method::GET, format!("{base}/{UUID}/overrides"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/overrides"),
            Some(r#"{"entity_id":"00000000-0000-0000-0000-000000000002","entity_type":"user","enabled":true}"#),
        ),
        (
            Method::DELETE,
            format!("{base}/{UUID}/overrides/{UUID}"),
            None,
        ),
        (Method::GET, format!("{base}/{UUID}/audit-log"), None),
        (
            Method::POST,
            format!("{base}/evaluate"),
            Some(r#"{"key":"test-flag"}"#),
        ),
    ]
}

fn infra_dashboard_cases() -> Vec<(Method, String, Option<&'static str>)> {
    vec![(
        Method::GET,
        "/api/v1/infrastructure/dashboard".to_string(),
        None,
    )]
}

fn infra_jobs_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/infrastructure/jobs";
    vec![
        (Method::GET, format!("{base}"), None),
        (
            Method::POST,
            format!("{base}"),
            Some(r#"{"job_type":"example","payload":{}}"#),
        ),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/retry"), None),
        (Method::POST, format!("{base}/{UUID}/cancel"), None),
        (Method::GET, format!("{base}/{UUID}/executions"), None),
        (Method::GET, format!("{base}/queues/stats"), None),
        (Method::GET, format!("{base}/types/stats"), None),
    ]
}

fn infra_health_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/infrastructure/health";
    vec![
        (Method::GET, format!("{base}/detailed"), None),
        (Method::GET, format!("{base}/checks"), None),
        (Method::GET, format!("{base}/checks/{UUID}"), None),
        (Method::GET, format!("{base}/checks/{UUID}/results"), None),
        (Method::GET, format!("{base}/alerts"), None),
        (Method::GET, format!("{base}/alerts/{UUID}"), None),
        (Method::POST, format!("{base}/alerts/{UUID}/acknowledge"), None),
        (Method::POST, format!("{base}/alerts/{UUID}/resolve"), None),
        (Method::GET, format!("{base}/alert-rules"), None),
        (
            Method::POST,
            format!("{base}/alert-rules"),
            Some(r#"{"name":"test","condition":"cpu>90","severity":"warning"}"#),
        ),
        (Method::GET, format!("{base}/alert-rules/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/alert-rules/{UUID}"),
            Some(r#"{"name":"updated"}"#),
        ),
        (Method::DELETE, format!("{base}/alert-rules/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/alert-rules/{UUID}/toggle"),
            None,
        ),
        (Method::GET, format!("{base}/metrics"), None),
    ]
}

// ---------------------------------------------------------------------------
// Operations cases (/api/v1/operations/*)
// ---------------------------------------------------------------------------

fn ops_deployments_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations/deployments";
    vec![
        (Method::GET, format!("{base}"), None),
        (
            Method::POST,
            format!("{base}"),
            Some(r#"{"version":"1.0.0","environment":"staging"}"#),
        ),
        (Method::GET, format!("{base}/dashboard"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/status"),
            Some(r#"{"status":"in_progress"}"#),
        ),
        (Method::POST, format!("{base}/{UUID}/switch"), None),
        (Method::POST, format!("{base}/{UUID}/rollback"), None),
        (Method::GET, format!("{base}/{UUID}/health-checks"), None),
        (Method::POST, format!("{base}/{UUID}/health-checks"), None),
    ]
}

fn ops_migrations_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations/migrations";
    vec![
        (Method::GET, format!("{base}"), None),
        (
            Method::POST,
            format!("{base}"),
            Some(r#"{"name":"test_migration","description":"test"}"#),
        ),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/progress"),
            Some(r#"{"progress":50}"#),
        ),
        (Method::GET, format!("{base}/{UUID}/logs"), None),
        (Method::POST, format!("{base}/{UUID}/rollback"), None),
        (Method::GET, format!("{base}/{UUID}/safety-check"), None),
    ]
}

fn ops_schema_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations/schema";
    vec![
        (Method::GET, format!("{base}/versions"), None),
        (Method::GET, format!("{base}/current"), None),
    ]
}

fn ops_backups_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations/backups";
    vec![
        (Method::GET, format!("{base}"), None),
        (
            Method::POST,
            format!("{base}"),
            Some(r#"{"description":"manual"}"#),
        ),
        (Method::GET, format!("{base}/dashboard"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/verify"), None),
    ]
}

fn ops_recovery_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations";
    vec![
        (
            Method::POST,
            format!("{base}/recovery"),
            Some(r#"{"backup_id":"00000000-0000-0000-0000-000000000001"}"#),
        ),
        (Method::GET, format!("{base}/recovery/{UUID}"), None),
    ]
}

fn ops_dr_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations/dr";
    vec![
        (Method::GET, format!("{base}/drills"), None),
        (
            Method::POST,
            format!("{base}/drills"),
            Some(r#"{"description":"quarterly drill","outcome":"passed"}"#),
        ),
    ]
}

fn ops_costs_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations/costs";
    vec![
        (Method::GET, format!("{base}"), None),
        (
            Method::POST,
            format!("{base}"),
            Some(r#"{"amount":100,"currency":"EUR","category":"compute"}"#),
        ),
        (Method::GET, format!("{base}/dashboard"), None),
        (Method::GET, format!("{base}/budgets"), None),
        (
            Method::POST,
            format!("{base}/budgets"),
            Some(r#"{"name":"monthly","limit":5000,"currency":"EUR"}"#),
        ),
        (Method::GET, format!("{base}/budgets/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/budgets/{UUID}"),
            Some(r#"{"limit":6000}"#),
        ),
        (Method::GET, format!("{base}/alerts"), None),
        (Method::GET, format!("{base}/utilization"), None),
    ]
}

// ---------------------------------------------------------------------------
// Flatten all cases
// ---------------------------------------------------------------------------

fn all_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    // Admin surface
    v.extend(admin_users_cases());
    v.extend(admin_mfa_enroll_cases());
    v.extend(admin_notifications_cases());
    v.extend(tenant_lifecycle_cases());
    // Infrastructure surface
    v.extend(infra_dashboard_cases());
    v.extend(infra_traces_cases());
    v.extend(infra_feature_flags_cases());
    v.extend(infra_jobs_cases());
    v.extend(infra_health_cases());
    // Operations surface
    v.extend(ops_deployments_cases());
    v.extend(ops_migrations_cases());
    v.extend(ops_schema_cases());
    v.extend(ops_backups_cases());
    v.extend(ops_recovery_cases());
    v.extend(ops_dr_cases());
    v.extend(ops_costs_cases());
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_batch2_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    for (method, uri, body) in all_cases() {
        let resp = app.execute(anon(method.clone(), &uri, body)).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must require auth (401), got {}",
            resp.status
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_batch2_endpoints_reject_unprivileged_user(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _user) = create_authenticated_user(&app, &TestUser::new()).await;

    for (method, uri, body) in all_cases() {
        let resp = app
            .execute(authed(&token, method.clone(), &uri, body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{method} {uri} must deny unprivileged user (403), got {}",
            resp.status
        );
    }
}

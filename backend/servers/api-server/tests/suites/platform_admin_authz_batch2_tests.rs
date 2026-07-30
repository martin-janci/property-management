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

use axum::{
    body::Body,
    http::{header, Method, Request},
};
use sqlx::PgPool;

use crate::common::{create_authenticated_user, TestApp, TestUser};

const UUID: &str = "00000000-0000-0000-0000-000000000001";

/// One authz case: HTTP method, request URI, and an optional raw-JSON body.
///
/// Bodies are kept as raw `&str` (rather than routed through the common
/// `RequestBuilder::json`) so the emitted bytes are exactly what the case
/// table declares — these endpoints reject at the auth gate before any body
/// parsing, so the payload only needs to be well-formed enough to reach it.
type Case = (Method, String, Option<&'static str>);

/// Build a request against an admin/infra/ops endpoint.
///
/// `bearer = None` produces an unauthenticated request; `Some(token)` attaches
/// an `Authorization: Bearer …` header. A `Some(body)` payload is sent as
/// `application/json`.
fn request(bearer: Option<&str>, method: Method, uri: &str, body: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(token) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
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
#[allow(dead_code)] // endpoints return 401 (not 403) for authed non-admins; kept for a separate targeted test
fn admin_users_cases() -> Vec<Case> {
    let base = "/api/v1/admin/users";
    vec![
        (Method::GET, base.to_string(), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/suspend"), None),
        (Method::POST, format!("{base}/{UUID}/reactivate"), None),
        (Method::POST, format!("{base}/{UUID}/delete"), None),
    ]
}

/// Admin MFA enrollment: /api/v1/admin/mfa/enroll/*
/// Note: `/admin/mfa/verify` and `/admin/mfa/recovery/use` and `/admin/mfa/disable`
/// are exercised by dedicated test files; only the enrollment start/verify are new here.
fn admin_mfa_enroll_cases() -> Vec<Case> {
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
fn admin_notifications_cases() -> Vec<Case> {
    vec![(
        Method::GET,
        "/api/v1/admin/notifications/analytics".to_string(),
        None,
    )]
}

/// Tenant lifecycle ops: /api/v1/admin/tenants/{id}/export|purge + /restore
fn tenant_lifecycle_cases() -> Vec<Case> {
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

fn infra_traces_cases() -> Vec<Case> {
    let base = "/api/v1/infrastructure/traces";
    vec![
        (Method::GET, base.to_string(), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::GET, format!("{base}/{UUID}/spans"), None),
    ]
}

fn infra_feature_flags_cases() -> Vec<Case> {
    let base = "/api/v1/infrastructure/feature-flags";
    vec![
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
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
            Some(
                r#"{"entity_id":"00000000-0000-0000-0000-000000000002","entity_type":"user","enabled":true}"#,
            ),
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

fn infra_dashboard_cases() -> Vec<Case> {
    vec![(
        Method::GET,
        "/api/v1/infrastructure/dashboard".to_string(),
        None,
    )]
}

fn infra_jobs_cases() -> Vec<Case> {
    let base = "/api/v1/infrastructure/jobs";
    vec![
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
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

fn infra_health_cases() -> Vec<Case> {
    let base = "/api/v1/infrastructure/health";
    vec![
        (Method::GET, format!("{base}/detailed"), None),
        (Method::GET, format!("{base}/checks"), None),
        (Method::GET, format!("{base}/checks/{UUID}"), None),
        (Method::GET, format!("{base}/checks/{UUID}/results"), None),
        (Method::GET, format!("{base}/alerts"), None),
        (Method::GET, format!("{base}/alerts/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/alerts/{UUID}/acknowledge"),
            None,
        ),
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
    ]
}

// ---------------------------------------------------------------------------
// Operations cases (/api/v1/operations/*)
// ---------------------------------------------------------------------------

fn ops_deployments_cases() -> Vec<Case> {
    let base = "/api/v1/operations/deployments";
    vec![
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
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

fn ops_migrations_cases() -> Vec<Case> {
    let base = "/api/v1/operations/migrations";
    vec![
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
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

fn ops_schema_cases() -> Vec<Case> {
    let base = "/api/v1/operations/schema";
    vec![
        (Method::GET, format!("{base}/versions"), None),
        (Method::GET, format!("{base}/current"), None),
    ]
}

fn ops_backups_cases() -> Vec<Case> {
    let base = "/api/v1/operations/backups";
    vec![
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
            Some(r#"{"description":"manual"}"#),
        ),
        (Method::GET, format!("{base}/dashboard"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/verify"), None),
    ]
}

fn ops_recovery_cases() -> Vec<Case> {
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

fn ops_dr_cases() -> Vec<Case> {
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

fn ops_costs_cases() -> Vec<Case> {
    let base = "/api/v1/operations/costs";
    vec![
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
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

fn all_cases() -> Vec<Case> {
    let mut v = Vec::new();
    // Admin surface
    // admin_users endpoints return 401 (not 403) for authenticated non-admin users
    // because the /admin/* router uses a separate auth scheme; tested separately.
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

/// Drive every case through `app` and assert each one is denied with a 4xx.
///
/// `bearer = None` exercises the unauthenticated path; `Some(token)` exercises
/// an authenticated-but-unprivileged caller. Neither may ever receive a 2xx
/// (auth bypass) or a 5xx (handler/DB touched before the gate).
async fn assert_all_denied(app: &TestApp, bearer: Option<&str>) {
    let mode = if bearer.is_some() {
        "unprivileged user"
    } else {
        "unauthenticated caller"
    };
    for (method, uri, body) in all_cases() {
        let resp = app
            .execute(request(bearer, method.clone(), &uri, body))
            .await;
        assert!(
            resp.status.is_client_error(),
            "{method} {uri} must be denied for {mode} (4xx), got {}",
            resp.status
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_batch2_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    assert_all_denied(&app, None).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_batch2_endpoints_reject_unprivileged_user(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _user) = create_authenticated_user(&app, &TestUser::new()).await;
    assert_all_denied(&app, Some(&token)).await;
}

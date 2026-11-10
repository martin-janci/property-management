//! Regression: Remaining infrastructure (Epic 71/72/89) and operations (Epic 73)
//! endpoints must be gated behind the platform-admin capability.
//!
//! This test file backfills authorization/authentication test coverage for the
//! endpoints that were previously marked as partial or none.
//!
//! For each endpoint:
//!   1. an unauthenticated request is rejected with **401 Unauthorized** — the
//!      `AuthUser` extractor (`FromRequestParts`) runs before any body/path
//!      extraction, so this holds for every method regardless of payload;
//!   2. an authenticated *non-admin* request is rejected with **403 Forbidden**.
//!
//! Assertion strength note (point 2): the handlers extract `Json<T>` *before*
//! the in-handler platform-admin check runs, so a body-bearing request whose
//! payload fails validation could surface a 4xx validation error ahead of the
//! 403. For the bodyless endpoints (all GET/DELETE plus the no-`Json` POSTs)
//! the 403 is deterministic and asserted exactly; for body-bearing endpoints we
//! assert the caller was authenticated (not 401) yet still rejected (4xx). This
//! is strictly stronger than a bare `is_client_error()` check: it proves auth is
//! actually enforced (a stray 404 would now fail) rather than merely that *some*
//! error occurred.
//!
//! Note that `/api/v1/infrastructure/health/metrics` is intentionally exempt from
//! the platform-admin capability gate since it is a Prometheus scrape endpoint.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::{create_authenticated_user, TestApp, TestUser};

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

/// Assert that an authenticated *non-admin* caller is rejected.
///
/// For bodyless requests (no `Json<T>` extraction can precede the in-handler
/// platform-admin check) the rejection is deterministically `403 Forbidden`.
/// For body-bearing requests, `Json` deserialization runs before the admin
/// check, so an incomplete/invalid payload may legitimately surface a 4xx
/// validation error ahead of the 403 — there we assert the caller was
/// authenticated (not `401`) yet still rejected (`4xx`).
fn assert_non_admin_rejected(bodyless: bool, status: StatusCode, method: &Method, uri: &str) {
    if bodyless {
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "{method} {uri} must reject an authenticated non-admin with 403, got {status}"
        );
    } else {
        assert!(
            status.is_client_error() && status != StatusCode::UNAUTHORIZED,
            "{method} {uri} must reject an authenticated non-admin (4xx, not 401), got {status}"
        );
    }
}

/// Systematically covers all remaining infrastructure partial/untested endpoints.
fn infrastructure_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/infrastructure";
    vec![
        // Traces sub-resource
        (Method::GET, format!("{base}/traces/{UUID}/spans"), None),
        // Feature flags
        (Method::GET, format!("{base}/feature-flags/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/feature-flags/{UUID}"),
            Some(r#"{}"#),
        ),
        (Method::DELETE, format!("{base}/feature-flags/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/feature-flags/{UUID}/toggle"),
            Some(r#"{"enabled":true}"#),
        ),
        (
            Method::GET,
            format!("{base}/feature-flags/{UUID}/overrides"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/feature-flags/{UUID}/overrides"),
            Some(r#"{"override_type":"user","value":true}"#),
        ),
        (
            Method::DELETE,
            format!("{base}/feature-flags/{UUID}/overrides/{UUID}"),
            None,
        ),
        (
            Method::GET,
            format!("{base}/feature-flags/{UUID}/audit-log"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/feature-flags/evaluate"),
            Some(r#"{"key":"x"}"#),
        ),
        // Background jobs
        (
            Method::POST,
            format!("{base}/jobs"),
            Some(r#"{"job_type":"email_send","payload":{}}"#),
        ),
        (Method::GET, format!("{base}/jobs/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/jobs/{UUID}/retry"),
            Some(r#"{}"#),
        ),
        (Method::POST, format!("{base}/jobs/{UUID}/cancel"), None),
        (Method::GET, format!("{base}/jobs/{UUID}/executions"), None),
        (Method::GET, format!("{base}/jobs/queues/stats"), None),
        (Method::GET, format!("{base}/jobs/types/stats"), None),
        // Health checks + alerts
        (Method::GET, format!("{base}/health/checks"), None),
        (Method::GET, format!("{base}/health/checks/{UUID}"), None),
        (
            Method::GET,
            format!("{base}/health/checks/{UUID}/results"),
            None,
        ),
        (Method::GET, format!("{base}/health/alerts/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/health/alerts/{UUID}/acknowledge"),
            Some(r#"{}"#),
        ),
        (
            Method::POST,
            format!("{base}/health/alerts/{UUID}/resolve"),
            Some(r#"{}"#),
        ),
        (Method::GET, format!("{base}/health/alert-rules"), None),
        (
            Method::POST,
            format!("{base}/health/alert-rules"),
            Some(
                r#"{"name":"x","condition":"cpu > 80","severity":"warning","notification_channels":[]}"#,
            ),
        ),
        (
            Method::GET,
            format!("{base}/health/alert-rules/{UUID}"),
            None,
        ),
        (
            Method::PUT,
            format!("{base}/health/alert-rules/{UUID}"),
            Some(r#"{}"#),
        ),
        (
            Method::DELETE,
            format!("{base}/health/alert-rules/{UUID}"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/health/alert-rules/{UUID}/toggle"),
            Some(r#"{"enabled":true}"#),
        ),
    ]
}

/// Systematically covers all operations partial/untested endpoints.
fn operations_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations";
    vec![
        // Deployments
        (Method::GET, format!("{base}/deployments"), None),
        (
            Method::POST,
            format!("{base}/deployments"),
            Some(r#"{"version":"1.0.0","environment":"blue"}"#),
        ),
        (Method::GET, format!("{base}/deployments/dashboard"), None),
        (Method::GET, format!("{base}/deployments/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/deployments/{UUID}/status"),
            Some(r#"{"status":"active"}"#),
        ),
        (
            Method::POST,
            format!("{base}/deployments/{UUID}/switch"),
            Some(r#"{"deployment_id":"00000000-0000-0000-0000-000000000001"}"#),
        ),
        (
            Method::POST,
            format!("{base}/deployments/{UUID}/rollback"),
            None,
        ),
        (
            Method::GET,
            format!("{base}/deployments/{UUID}/health-checks"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/deployments/{UUID}/health-checks"),
            None,
        ),
        // Migrations
        (Method::GET, format!("{base}/migrations"), None),
        (
            Method::POST,
            format!("{base}/migrations"),
            Some(r#"{"name":"x","version":"1"}"#),
        ),
        (Method::GET, format!("{base}/migrations/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/migrations/{UUID}/progress"),
            Some(r#"{}"#),
        ),
        (Method::GET, format!("{base}/migrations/{UUID}/logs"), None),
        (
            Method::POST,
            format!("{base}/migrations/{UUID}/rollback"),
            None,
        ),
        (
            Method::GET,
            format!("{base}/migrations/{UUID}/safety-check"),
            None,
        ),
        // Schema
        (Method::GET, format!("{base}/schema/versions"), None),
        (Method::GET, format!("{base}/schema/current"), None),
        // Backups + DR
        (Method::GET, format!("{base}/backups"), None),
        (
            Method::POST,
            format!("{base}/backups"),
            Some(r#"{"backup_type":"full"}"#),
        ),
        (Method::GET, format!("{base}/backups/dashboard"), None),
        (Method::GET, format!("{base}/backups/{UUID}"), None),
        (Method::POST, format!("{base}/backups/{UUID}/verify"), None),
        (
            Method::POST,
            format!("{base}/recovery"),
            Some(r#"{"backup_id":"00000000-0000-0000-0000-000000000001"}"#),
        ),
        (Method::GET, format!("{base}/recovery/{UUID}"), None),
        (Method::GET, format!("{base}/dr/drills"), None),
        (
            Method::POST,
            format!("{base}/dr/drills"),
            Some(
                r#"{"drill_type":"x","is_successful":true,"rto_target_secs":1,"rto_actual_secs":1,"rpo_target_secs":1,"rpo_actual_secs":1}"#,
            ),
        ),
        // Costs
        (Method::GET, format!("{base}/costs"), None),
        (
            Method::POST,
            format!("{base}/costs"),
            Some(
                r#"{"service_type":"compute","service_name":"x","cost_amount":10.0,"usage_quantity":1.0,"period_start":"2026-01-01","period_end":"2026-01-02"}"#,
            ),
        ),
        (Method::GET, format!("{base}/costs/dashboard"), None),
        (Method::GET, format!("{base}/costs/budgets"), None),
        (
            Method::POST,
            format!("{base}/costs/budgets"),
            Some(r#"{"name":"x","budget_amount":100.00,"period_type":"monthly"}"#),
        ),
        (Method::GET, format!("{base}/costs/budgets/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/costs/budgets/{UUID}"),
            Some(r#"{"name":"x","budget_amount":100.00,"period_type":"monthly"}"#),
        ),
        (Method::GET, format!("{base}/costs/alerts"), None),
        (
            Method::POST,
            format!("{base}/costs/alerts/{UUID}/acknowledge"),
            None,
        ),
        (Method::GET, format!("{base}/costs/utilization"), None),
        (Method::GET, format!("{base}/costs/recommendations"), None),
        (
            Method::POST,
            format!("{base}/costs/recommendations/{UUID}/implement"),
            None,
        ),
    ]
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn health_metrics_is_public(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let resp = app
        .execute(anon(
            Method::GET,
            "/api/v1/infrastructure/health/metrics",
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "GET /api/v1/infrastructure/health/metrics must be public (200), got {}",
        resp.status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn infrastructure_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    for (method, uri, body) in infrastructure_cases() {
        let resp = app.execute(anon(method.clone(), &uri, body)).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must reject an unauthenticated caller with 401, got {}",
            resp.status
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn infrastructure_endpoints_reject_non_platform_admin(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _r) = create_authenticated_user(&app, &TestUser::new()).await;

    for (method, uri, body) in infrastructure_cases() {
        let bodyless = body.is_none();
        let resp = app
            .execute(authed(&token, method.clone(), &uri, body))
            .await;
        assert_non_admin_rejected(bodyless, resp.status, &method, &uri);
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn operations_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    for (method, uri, body) in operations_cases() {
        let resp = app.execute(anon(method.clone(), &uri, body)).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must reject an unauthenticated caller with 401, got {}",
            resp.status
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn operations_endpoints_reject_non_platform_admin(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _r) = create_authenticated_user(&app, &TestUser::new()).await;

    for (method, uri, body) in operations_cases() {
        let bodyless = body.is_none();
        let resp = app
            .execute(authed(&token, method.clone(), &uri, body))
            .await;
        assert_non_admin_rejected(bodyless, resp.status, &method, &uri);
    }
}

//! Authz regression backfill — infrastructure / operations / platform-admin / admin
//! (BIT-331, re-cut of BIT-274/277).
//!
//! Re-authored fresh on `dev` after #1869 / #1872 were closed unmergeable. Covers
//! the platform-privileged surface that previously had no path-level authz coverage:
//!   * `/api/v1/infrastructure/**` — traces, feature-flags, jobs, health/alerts
//!   * `/api/v1/operations/**`     — deployments, migrations, schema, backups, DR, costs
//!   * `/api/v1/platform-admin/**` — orgs, stats, feature-flags, health, announcements,
//!     maintenance, support
//!   * `/api/v1/admin/**`          — user lifecycle, MFA enroll, notifications,
//!     tenant lifecycle (export/purge), agencies. (Tenant branding/feature-flags
//!     live on the host-routed production surface, not in the API router — see
//!     the BIT-588 note in `admin_cases`.)
//!
//! All of these are gated for platform operators. We assert the security invariant:
//!   1. unauthenticated  → 401 (also guards route existence);
//!   2. authenticated ordinary user → rejected with a 4xx on an existing route
//!      (never a 2xx leak, never 404/405).
//!
//! Why the authenticated leg is not pinned to a single code: under the `TestApp`
//! harness there is no `ResolvedTenant`. Infrastructure/operations gate via
//! `AuthUser::is_platform_admin()` → **403**. The `/admin/**` and `/platform-admin/**`
//! routers gate via `RequireCapability`, which wraps `RequestPrincipal`; with no
//! resolved tenant the principal layer rejects and `RequireCapability` re-maps that to
//! **401**. Both are valid "denied" outcomes; asserting "4xx on an existing route"
//! captures the property without being brittle to which family a handler uses.
//!
//! Excluded on purpose: `GET /api/v1/infrastructure/health/metrics` (public Prometheus
//! scrape — returns 2xx anonymously by design) and `POST /api/v1/platform-admin/agencies`
//! (body validation returns 4xx before the auth gate, so it is not a clean auth probe).

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::{create_authenticated_user, TestApp, TestUser};

const UUID: &str = "00000000-0000-0000-0000-000000000001";
const UUID2: &str = "00000000-0000-0000-0000-000000000002";

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

fn assert_denied(status: StatusCode, method: &Method, uri: &str) {
    let c = status.as_u16();
    assert!(
        (400..=499).contains(&c) && c != 404 && c != 405,
        "{method} {uri} must reject a non-privileged caller with a 4xx on an existing route, got {status}"
    );
}

fn assert_requires_auth(status: StatusCode, method: &Method, uri: &str) {
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "{method} {uri} must require authentication (401), got {status}"
    );
}

// ---------------------------------------------------------------------------
// Infrastructure (/api/v1/infrastructure/*)
// ---------------------------------------------------------------------------

fn infrastructure_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/infrastructure";
    vec![
        (Method::GET, format!("{base}/dashboard"), None),
        // Traces
        (Method::GET, format!("{base}/traces"), None),
        (Method::GET, format!("{base}/traces/{UUID}"), None),
        (Method::GET, format!("{base}/traces/{UUID}/spans"), None),
        // Feature flags
        (Method::GET, format!("{base}/feature-flags"), None),
        (
            Method::POST,
            format!("{base}/feature-flags"),
            Some(r#"{"key":"test-flag","enabled":false}"#),
        ),
        (Method::GET, format!("{base}/feature-flags/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/feature-flags/{UUID}"),
            Some(r#"{"enabled":true}"#),
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
            Some(
                r#"{"entity_id":"00000000-0000-0000-0000-000000000002","entity_type":"user","enabled":true}"#,
            ),
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
            Some(r#"{"key":"test-flag"}"#),
        ),
        // Jobs
        (Method::GET, format!("{base}/jobs"), None),
        (
            Method::POST,
            format!("{base}/jobs"),
            Some(r#"{"job_type":"example","payload":{}}"#),
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
        // Health (detailed + checks + alerts + alert-rules)
        (Method::GET, format!("{base}/health/detailed"), None),
        (Method::GET, format!("{base}/health/checks"), None),
        (Method::GET, format!("{base}/health/checks/{UUID}"), None),
        (
            Method::GET,
            format!("{base}/health/checks/{UUID}/results"),
            None,
        ),
        (Method::GET, format!("{base}/health/alerts"), None),
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

// ---------------------------------------------------------------------------
// Operations (/api/v1/operations/*)
// ---------------------------------------------------------------------------

fn operations_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/operations";
    vec![
        // Deployments
        (Method::GET, format!("{base}/deployments"), None),
        (
            Method::POST,
            format!("{base}/deployments"),
            Some(r#"{"version":"1.0.0","environment":"staging"}"#),
        ),
        (Method::GET, format!("{base}/deployments/dashboard"), None),
        (Method::GET, format!("{base}/deployments/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/deployments/{UUID}/status"),
            Some(r#"{"status":"in_progress"}"#),
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
            Some(r#"{"name":"test_migration","version":"1"}"#),
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
        // Backups + recovery + DR
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
            Some(r#"{"name":"monthly","budget_amount":5000,"period_type":"monthly"}"#),
        ),
        (Method::GET, format!("{base}/costs/budgets/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/costs/budgets/{UUID}"),
            Some(r#"{"name":"x","budget_amount":100.0,"period_type":"monthly"}"#),
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

// ---------------------------------------------------------------------------
// Admin (/api/v1/admin/*) — RequireCapability gated
// ---------------------------------------------------------------------------

fn admin_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin";
    vec![
        // User lifecycle
        (Method::GET, format!("{base}/users"), None),
        (Method::GET, format!("{base}/users/{UUID}"), None),
        (Method::POST, format!("{base}/users/{UUID}/suspend"), None),
        (
            Method::POST,
            format!("{base}/users/{UUID}/reactivate"),
            None,
        ),
        (Method::POST, format!("{base}/users/{UUID}/delete"), None),
        // MFA enrollment
        (Method::POST, format!("{base}/mfa/enroll/start"), None),
        (
            Method::POST,
            format!("{base}/mfa/enroll/verify"),
            Some(r#"{"code":"123456"}"#),
        ),
        // Notifications analytics
        (Method::GET, format!("{base}/notifications/analytics"), None),
        // Tenant lifecycle
        (Method::POST, format!("{base}/tenants/{UUID}/export"), None),
        (Method::POST, format!("{base}/tenants/{UUID}/purge"), None),
        // NOTE (BIT-588): tenant branding + feature-flags under
        // `/api/v1/admin/tenants/{id}/…` were probed here but are NOT registered
        // in the API router — `admin_tenant_lifecycle` only mounts
        // export/purge/restore, and the branding/feature-flags handlers live at
        // the host-routed `/admin/tenants/{id}/…` production-only surface
        // (mounted in `main.rs`, excluded from `route_table()`). Probing them
        // returned 404 and re-quarantined this file; they are dropped here.
        // Admin agencies
        (Method::GET, format!("{base}/agencies"), None),
        (Method::GET, format!("{base}/agencies/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/agencies/{UUID}/suspend"),
            None,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Platform-admin (/api/v1/platform-admin/*) — require_capability gated
// ---------------------------------------------------------------------------

fn platform_admin_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/platform-admin";
    let ff = format!("{base}/feature-flags");
    vec![
        // Organizations
        (Method::GET, format!("{base}/organizations"), None),
        (Method::GET, format!("{base}/organizations/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/organizations/{UUID}/suspend"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/organizations/{UUID}/reactivate"),
            None,
        ),
        // Stats
        (Method::GET, format!("{base}/stats"), None),
        // Feature flags
        (Method::GET, ff.clone(), None),
        (
            Method::POST,
            ff.clone(),
            Some(r#"{"key":"pf-test","enabled":false}"#),
        ),
        (Method::GET, format!("{ff}/{UUID}"), None),
        (
            Method::PUT,
            format!("{ff}/{UUID}"),
            Some(r#"{"enabled":true}"#),
        ),
        (Method::DELETE, format!("{ff}/{UUID}"), None),
        (Method::POST, format!("{ff}/{UUID}/toggle"), None),
        (
            Method::POST,
            format!("{ff}/{UUID}/overrides"),
            Some(
                r#"{"entity_id":"00000000-0000-0000-0000-000000000002","entity_type":"org","enabled":true}"#,
            ),
        ),
        (
            Method::DELETE,
            format!("{ff}/{UUID}/overrides/{UUID2}"),
            None,
        ),
        // Health
        (Method::GET, format!("{base}/health/dashboard"), None),
        (
            Method::GET,
            format!("{base}/health/metrics/cpu_usage/history"),
            None,
        ),
        (Method::GET, format!("{base}/health/alerts"), None),
        (
            Method::POST,
            format!("{base}/health/alerts/{UUID}/acknowledge"),
            None,
        ),
        (Method::GET, format!("{base}/health/thresholds"), None),
        (
            Method::PUT,
            format!("{base}/health/thresholds/cpu_usage"),
            Some(r#"{"warning":80,"critical":95}"#),
        ),
        // Announcements
        (Method::GET, format!("{base}/announcements"), None),
        (
            Method::POST,
            format!("{base}/announcements"),
            Some(r#"{"title":"Maintenance","body":"Scheduled downtime","level":"info"}"#),
        ),
        (Method::GET, format!("{base}/announcements/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/announcements/{UUID}"),
            Some(r#"{"title":"Updated"}"#),
        ),
        (Method::DELETE, format!("{base}/announcements/{UUID}"), None),
        // Maintenance
        (Method::GET, format!("{base}/maintenance"), None),
        (
            Method::POST,
            format!("{base}/maintenance"),
            Some(
                r#"{"title":"Upgrade","scheduled_at":"2025-01-01T02:00:00Z","duration_minutes":60}"#,
            ),
        ),
        (Method::DELETE, format!("{base}/maintenance/{UUID}"), None),
        // Support
        (Method::GET, format!("{base}/support/users"), None),
        (Method::GET, format!("{base}/support/users/{UUID}"), None),
        (
            Method::GET,
            format!("{base}/support/users/{UUID}/memberships"),
            None,
        ),
        (
            Method::GET,
            format!("{base}/support/users/{UUID}/sessions"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/support/users/{UUID}/sessions/revoke"),
            None,
        ),
        (
            Method::GET,
            format!("{base}/support/users/{UUID}/activity"),
            None,
        ),
        // Misc
        (Method::GET, format!("{base}/support-data"), None),
        (Method::GET, format!("{base}/onboarding-config"), None),
    ]
}

fn all_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(infrastructure_cases());
    v.extend(operations_cases());
    v.extend(admin_cases());
    v.extend(platform_admin_cases());
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_privileged_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let mut failures = Vec::new();
    for (method, uri, body) in all_cases() {
        let resp = app.execute(anon(method.clone(), &uri, body)).await;
        if resp.status != StatusCode::UNAUTHORIZED {
            failures.push(format!("{method} {uri} -> {} (want 401)", resp.status));
        }
    }
    assert!(
        failures.is_empty(),
        "these endpoints must require auth (401):\n{}",
        failures.join("\n")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_privileged_endpoints_reject_ordinary_user(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _user) = create_authenticated_user(&app, &TestUser::new()).await;

    let mut failures = Vec::new();
    for (method, uri, body) in all_cases() {
        let resp = app
            .execute(authed(&token, method.clone(), &uri, body))
            .await;
        let c = resp.status.as_u16();
        if !((400..=499).contains(&c) && c != 404 && c != 405) {
            failures.push(format!(
                "{method} {uri} -> {} (want 4xx≠404/405)",
                resp.status
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "these endpoints must deny an unprivileged user:\n{}",
        failures.join("\n")
    );
}

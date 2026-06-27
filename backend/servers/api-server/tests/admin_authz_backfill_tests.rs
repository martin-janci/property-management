//! Authz regression tests backfill for the platform-admin surface.
//!
//! Covers 37 endpoints with 74 assertions (unauthenticated -> 401, authenticated ordinary user -> 403).

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

/// admin_tenants (branding + feature-flags, mount: `/admin/tenants/{org_id}/...`)
fn tenant_branding_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = format!("/admin/tenants/{UUID}/branding");
    vec![
        (Method::GET, base.clone(), None),
        (
            Method::PUT,
            base,
            Some(r#"{"primary_color":"#fff","secondary_color":"#000","logo_url":null,"favicon_url":null,"custom_css":null}"#),
        ),
    ]
}

fn tenant_feature_flag_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = format!("/admin/tenants/{UUID}/feature-flags");
    vec![
        (Method::GET, base.clone(), None),
        (
            Method::PUT,
            base,
            Some(r#"{"key":"some-flag","enabled":true}"#),
        ),
    ]
}

/// admin_tenant_lifecycle (mount: `/api/v1/admin`)
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
        (
            Method::POST,
            "/api/v1/admin/tenants/restore".to_string(),
            None,
        ),
    ]
}

/// admin/agencies
fn agency_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/agencies";
    vec![
        (Method::GET, format!("{base}"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/suspend"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/domains"),
            Some(r#"{"domain":"example.com"}"#),
        ),
    ]
}

/// admin/audit
fn audit_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/audit";
    vec![
        (Method::GET, format!("{base}"), None),
        (Method::GET, format!("{base}/csv"), None),
    ]
}

/// admin/capabilities
fn capability_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/capabilities";
    vec![
        (Method::GET, format!("{base}/registry"), None),
        (Method::GET, format!("{base}/me"), None),
        (Method::GET, format!("{base}/users/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/users/{UUID}/grant"),
            Some(r#"{"capability":"AuditRead"}"#),
        ),
        (
            Method::DELETE,
            format!("{base}/users/{UUID}/grant/{UUID}"),
            None,
        ),
    ]
}

/// admin/impersonation
fn impersonation_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/impersonation";
    vec![
        (Method::GET, format!("{base}/active"), None),
        (
            Method::POST,
            format!("{base}/start"),
            Some(r#"{"target_user_id":"00000000-0000-0000-0000-000000000002"}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}"), None),
    ]
}

/// admin/memberships
fn membership_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/memberships";
    vec![
        (
            Method::POST,
            format!("{base}/invite"),
            Some(r#"{"email":"test@example.com","role":"member"}"#),
        ),
        (
            Method::POST,
            format!("{base}/accept"),
            Some(
                r#"{"token":"invite-token","user_id":"00000000-0000-0000-0000-000000000001","email":"test@example.com"}"#,
            ),
        ),
        (Method::DELETE, format!("{base}/{UUID}"), None),
        (Method::GET, format!("{base}/merge-collisions"), None),
    ]
}

/// admin/metrics
fn metrics_cases() -> Vec<(Method, String, Option<&'static str>)> {
    vec![(
        Method::GET,
        "/api/v1/admin/metrics/summary".to_string(),
        None,
    )]
}

/// admin/notifications
fn notifications_cases() -> Vec<(Method, String, Option<&'static str>)> {
    vec![(
        Method::GET,
        "/api/v1/admin/notifications/analytics".to_string(),
        None,
    )]
}

/// admin/principals (users.rs)
fn principals_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/principals";
    vec![
        (Method::GET, format!("{base}"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/principal-kind"),
            Some(r#"{"kind":"PlatformPrincipal"}"#),
        ),
    ]
}

/// admin/users_lifecycle
fn users_lifecycle_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/users";
    vec![
        (Method::GET, format!("{base}"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/suspend"), None),
        (Method::POST, format!("{base}/{UUID}/reactivate"), None),
        (Method::POST, format!("{base}/{UUID}/delete"), None),
    ]
}

/// admin/mfa
fn mfa_cases() -> Vec<(Method, String, Option<&'static str>)> {
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

// ---------------------------------------------------------------------------
// Helper: flatten all backfill cases
// ---------------------------------------------------------------------------

fn all_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(tenant_branding_cases());
    v.extend(tenant_feature_flag_cases());
    v.extend(tenant_lifecycle_cases());
    v.extend(agency_cases());
    v.extend(audit_cases());
    v.extend(capability_cases());
    v.extend(impersonation_cases());
    v.extend(membership_cases());
    v.extend(metrics_cases());
    v.extend(notifications_cases());
    v.extend(principals_cases());
    v.extend(users_lifecycle_cases());
    v.extend(mfa_cases());
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_endpoints_require_auth(pool: PgPool) {
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
async fn platform_admin_endpoints_reject_unprivileged_user(pool: PgPool) {
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

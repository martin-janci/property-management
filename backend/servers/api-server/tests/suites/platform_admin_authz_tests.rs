//! Authz regression tests for the platform-admin surface.
//!
//! Covers the highest-blast-radius partial endpoints that were previously
//! untested: capabilities CRUD, impersonation, agencies, memberships,
//! metrics/notifications read, principals management, tenant branding, and
//! tenant feature-flags.
//!
//! Pattern (identical to `infra_migration_platform_admin_tests.rs`):
//!   1. Unauthenticated → 401
//!   2. Authenticated ordinary user (no capabilities) → 403
//!
//! A freshly registered user (`create_authenticated_user`) never holds any
//! capability grant, so it exercises the 403 leg of every capability layer.

#![allow(dead_code)]

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

// ---------------------------------------------------------------------------
// Case tables
// ---------------------------------------------------------------------------

/// Capabilities sub-router: /api/v1/admin/capabilities/…
fn capability_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/capabilities";
    vec![
        (Method::GET, format!("{base}/registry"), None),
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

/// Impersonation sub-router: /api/v1/admin/impersonation/…
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

/// Agencies sub-router: /api/v1/admin/agencies/…
fn agency_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/agencies";
    vec![
        (Method::GET, base.to_string(), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/suspend"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/domains"),
            Some(r#"{"domain":"example.com"}"#),
        ),
    ]
}

/// Memberships sub-router: /api/v1/admin/memberships/…
/// Note: `accept` is excluded — it requires no capability (gated by principal
/// identity binding only) and would need a valid invite token to reach the
/// handler body.
fn membership_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/memberships";
    vec![
        (
            Method::POST,
            format!("{base}/invite"),
            Some(r#"{"email":"test@example.com","role":"member"}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}"), None),
        (Method::GET, format!("{base}/merge-collisions"), None),
    ]
}

/// Audit sub-router: /api/v1/admin/audit/…
fn audit_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/audit";
    vec![
        (Method::GET, base.to_string(), None),
        (Method::GET, format!("{base}/csv"), None),
    ]
}

/// Metrics sub-router: /api/v1/admin/metrics/…
fn metrics_cases() -> Vec<(Method, String, Option<&'static str>)> {
    vec![(
        Method::GET,
        "/api/v1/admin/metrics/summary".to_string(),
        None,
    )]
}

/// Principals sub-router: /api/v1/admin/principals/…
fn principals_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/admin/principals";
    vec![
        (Method::GET, base.to_string(), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/principal-kind"),
            Some(r#"{"kind":"PlatformPrincipal"}"#),
        ),
    ]
}

/// Tenant branding: /admin/tenants/{org_id}/branding
fn tenant_branding_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = format!("/admin/tenants/{UUID}/branding");
    vec![
        (Method::GET, base.clone(), None),
        (
            Method::PUT,
            base,
            Some(
                r##"{"primary_color":"#fff","secondary_color":"#000","logo_url":null,"favicon_url":null,"custom_css":null}"##,
            ),
        ),
    ]
}

/// Tenant feature-flags: /admin/tenants/{org_id}/feature-flags
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

// ---------------------------------------------------------------------------
// Helper: flatten all high-risk cases
// ---------------------------------------------------------------------------

fn all_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(capability_cases());
    v.extend(impersonation_cases());
    v.extend(agency_cases());
    v.extend(membership_cases());
    v.extend(audit_cases());
    v.extend(metrics_cases());
    v.extend(principals_cases());
    v.extend(tenant_branding_cases());
    v.extend(tenant_feature_flag_cases());
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[ignore = "BIT-351 quarantine: schema/column not implemented (BIT-565)"]
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

#[ignore = "BIT-351 quarantine: schema/column not implemented (BIT-565)"]
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

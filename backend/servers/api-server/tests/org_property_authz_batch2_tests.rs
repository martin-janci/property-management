//! Authz regression tests for org-property surface — Batch 2 (BIT-274).
//!
//! Continues from `org_property_authz_tests.rs` (batch-1).
//!
//! This batch covers:
//!   * `/api/v1/building-certifications/**` — full certification surface
//!     (certifications, credits, documents, milestones, benchmarks, costs, reminders, audit-logs)
//!   * `/api/v1/organizations/**`            — org management, members, roles, settings, branding, export, features
//!
//! Pattern:
//!   1. Unauthenticated → 401
//!   2. Authenticated ordinary user (no org membership) → 403

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use common::{create_authenticated_user, TestApp, TestUser};

const UUID: &str = "00000000-0000-0000-0000-000000000001";
const UUID2: &str = "00000000-0000-0000-0000-000000000002";
const UUID3: &str = "00000000-0000-0000-0000-000000000003";

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

/// Building certifications: /api/v1/building-certifications/*
fn building_certifications_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/building-certifications";
    let cert = format!("{base}/{UUID}");
    let credit = format!("{cert}/credits/{UUID2}");
    let doc = format!("{cert}/documents/{UUID2}");
    let milestone = format!("{cert}/milestones/{UUID2}");
    vec![
        (Method::GET, format!("{base}/dashboard"), None),
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
            Some(
                r#"{"building_id":"00000000-0000-0000-0000-000000000001","certification_type":"LEED","target_level":"Gold"}"#,
            ),
        ),
        (Method::GET, format!("{base}/expiring"), None),
        (Method::GET, cert.to_string(), None),
        (
            Method::PUT,
            cert.to_string(),
            Some(r#"{"status":"active"}"#),
        ),
        (Method::DELETE, cert.to_string(), None),
        (Method::GET, format!("{cert}/with-credits"), None),
        // Credits
        (Method::GET, format!("{cert}/credits"), None),
        (
            Method::POST,
            format!("{cert}/credits"),
            Some(r#"{"category":"energy","points":5}"#),
        ),
        (Method::GET, credit.to_string(), None),
        (Method::PUT, credit.to_string(), Some(r#"{"points":6}"#)),
        (Method::DELETE, credit.to_string(), None),
        // Documents
        (Method::GET, format!("{cert}/documents"), None),
        (
            Method::POST,
            format!("{cert}/documents"),
            Some(r#"{"title":"Certificate","url":"https://example.com/doc.pdf"}"#),
        ),
        (Method::DELETE, doc.to_string(), None),
        // Milestones
        (Method::GET, format!("{cert}/milestones"), None),
        (
            Method::POST,
            format!("{cert}/milestones"),
            Some(r#"{"title":"Phase 1","due_date":"2025-06-01"}"#),
        ),
        (
            Method::PUT,
            milestone.to_string(),
            Some(r#"{"completed":true}"#),
        ),
        (Method::DELETE, milestone.to_string(), None),
        // Benchmarks
        (Method::GET, format!("{cert}/benchmarks"), None),
        (
            Method::POST,
            format!("{cert}/benchmarks"),
            Some(r#"{"metric":"energy_use","value":120.5,"unit":"kWh/m2"}"#),
        ),
        // Costs
        (Method::GET, format!("{cert}/costs"), None),
        (
            Method::POST,
            format!("{cert}/costs"),
            Some(r#"{"description":"Audit fee","amount":2500,"currency":"EUR"}"#),
        ),
        (Method::GET, format!("{cert}/costs/total"), None),
        // Reminders
        (Method::GET, format!("{cert}/reminders"), None),
        (
            Method::POST,
            format!("{cert}/reminders"),
            Some(r#"{"message":"Renewal due","remind_at":"2025-01-01T09:00:00Z"}"#),
        ),
        // Audit logs
        (Method::GET, format!("{cert}/audit-logs"), None),
    ]
}

/// Organizations: /api/v1/organizations/*
fn organizations_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/organizations";
    vec![
        (
            Method::POST,
            base.to_string(),
            Some(r#"{"name":"Test Org","country":"SK"}"#),
        ),
        (Method::GET, base.to_string(), None),
        (Method::GET, format!("{base}/my"), None),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}"),
            Some(r#"{"name":"Updated Org"}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}"), None),
        // Members
        (Method::GET, format!("{base}/{UUID}/members"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/members"),
            Some(
                r#"{"user_id":"00000000-0000-0000-0000-000000000002","role_id":"00000000-0000-0000-0000-000000000003"}"#,
            ),
        ),
        (
            Method::PUT,
            format!("{base}/{UUID}/members/{UUID2}"),
            Some(r#"{"role_id":"00000000-0000-0000-0000-000000000004"}"#),
        ),
        (
            Method::DELETE,
            format!("{base}/{UUID}/members/{UUID2}"),
            None,
        ),
        // Roles
        (Method::GET, format!("{base}/{UUID}/roles"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/roles"),
            Some(r#"{"name":"Manager","permissions":[]}"#),
        ),
        (Method::GET, format!("{base}/{UUID}/roles/{UUID3}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/roles/{UUID3}"),
            Some(r#"{"name":"Senior Manager"}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}/roles/{UUID3}"), None),
        // Settings
        (Method::GET, format!("{base}/{UUID}/settings"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/settings"),
            Some(r#"{"timezone":"Europe/Bratislava"}"#),
        ),
        // Branding
        (Method::GET, format!("{base}/{UUID}/branding"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/branding"),
            Some(r##"{"primary_color":"#ff0000"}"##),
        ),
        // Export
        (Method::POST, format!("{base}/{UUID}/export"), None),
        // Features
        (Method::GET, format!("{base}/{UUID}/features"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/features"),
            Some(r#"{"features":[]}"#),
        ),
        (
            Method::PUT,
            format!("{base}/{UUID}/features/advanced_reporting"),
            Some(r#"{"enabled":true}"#),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Flatten all cases
// ---------------------------------------------------------------------------

fn all_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(building_certifications_cases());
    v.extend(organizations_cases());
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn org_property_batch2_endpoints_require_auth(pool: PgPool) {
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
async fn org_property_batch2_endpoints_reject_unprivileged_user(pool: PgPool) {
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

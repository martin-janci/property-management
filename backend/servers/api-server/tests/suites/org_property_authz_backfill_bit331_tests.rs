//! Authz regression backfill — org-property surface (BIT-331, re-cut of BIT-274/277).
//!
//! Re-authored fresh on `dev` after the original branches (#1869, #1872) were
//! closed unmergeable. Covers the org-property endpoints that previously had no
//! path-level authz coverage:
//!   * `/api/v1/buildings/**`               — buildings, units, unit owners, unit residents
//!   * `/api/v1/building-certifications/**` — full certification surface
//!   * `/api/v1/agencies/**`                — agency management + membership
//!   * `/api/v1/organizations/**`           — org management, members, roles, settings
//!
//! ## What we assert, and why it is robust
//!
//! These are *cross-tenant denial* regression tests. The security invariant the
//! Stable-Beta phase cares about (PAP-51) is: **an unauthorized caller must never
//! receive tenant data** — i.e. the response is a 4xx rejection on an existing
//! route, never a 2xx leak and never a 404/405 (which would mean the route under
//! test silently moved).
//!
//! We deliberately do NOT pin the exact 4xx code. Under the `TestApp` harness there
//! is no `host_tenant_middleware`, so there is no `ResolvedTenant`; the precise code
//! returned to an authenticated non-member varies by extractor family:
//!   * `ValidatedTenantExtractor` (buildings, certifications) → 403 once a tenant is
//!     pinned via `X-Tenant-ID`, otherwise 400;
//!   * `TenantExtractor` + agency-membership (agencies) → 403 once the tenant gate is
//!     cleared;
//!   * the org-scoped inline check (organizations) → 403 for a non-member.
//!
//! Pinning a single code here would be brittle and was the reason the original
//! re-cuts could not be made green without a live DB. Asserting "rejected 4xx on an
//! existing route" captures the actual security property and is stable across all
//! three families. The companion `*_require_auth` leg pins the unauthenticated case
//! and doubles as the route-existence guard.
//!
//! Endpoints that ANY authenticated user is legitimately allowed to call — top-level
//! resource *creates* (`POST /organizations`, `POST /agencies`) and self-scoped reads
//! (`GET /organizations/my`) — are intentionally excluded: they return 2xx by design
//! and are covered by their own happy-path tests, not by an authz-denial sweep.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::{
    create_authenticated_user, create_authenticated_user_with_org, TestApp, TestUser,
};

const BID: &str = "00000000-0000-0000-0000-000000000001";
const UID: &str = "00000000-0000-0000-0000-000000000002";
const PID: &str = "00000000-0000-0000-0000-000000000003";

// ---------------------------------------------------------------------------
// Request builders
// ---------------------------------------------------------------------------

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

/// Authenticated request that pins a tenant via `X-Tenant-ID` so that
/// `ValidatedTenantExtractor` / `TenantExtractor` reach the membership check
/// (403 for a non-member) rather than failing the missing-tenant gate (400).
fn authed_tenant(
    token: &str,
    org_id: &str,
    method: Method,
    uri: &str,
    body: Option<&str>,
) -> Request<Body> {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id);
    match body {
        Some(j) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

/// The route exists and rejects the caller with a 4xx — never a 2xx leak, never
/// a 404/405 (route moved). Used for the authenticated non-member leg, where the
/// exact 4xx (400/401/403) depends on the extractor family (see module docs).
fn assert_denied(status: StatusCode, method: &Method, uri: &str) {
    let c = status.as_u16();
    assert!(
        (400..=499).contains(&c) && c != 404 && c != 405,
        "{method} {uri} must reject an unauthorized caller with a 4xx on an existing route, got {status}"
    );
}

/// Unauthenticated requests must be rejected (401) — also guards route existence.
fn assert_requires_auth(status: StatusCode, method: &Method, uri: &str) {
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "{method} {uri} must require authentication (401), got {status}"
    );
}

// ---------------------------------------------------------------------------
// Case tables — tenant-scoped (membership gate fires on the X-Tenant-ID org)
// ---------------------------------------------------------------------------

fn buildings_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/buildings";
    vec![
        (
            Method::POST,
            base.to_string(),
            Some(r#"{"name":"Test Building","address":"123 Main St"}"#),
        ),
        (
            Method::POST,
            format!("{base}/bulk"),
            Some(r#"{"buildings":[]}"#),
        ),
        (Method::GET, format!("{base}/{BID}"), None),
        (
            Method::PUT,
            format!("{base}/{BID}"),
            Some(r#"{"name":"Updated"}"#),
        ),
        (Method::DELETE, format!("{base}/{BID}"), None),
        (Method::POST, format!("{base}/{BID}/restore"), None),
        (Method::GET, format!("{base}/{BID}/statistics"), None),
    ]
}

fn units_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let ubase = format!("/api/v1/buildings/{BID}/units");
    let uid = format!("{ubase}/{UID}");
    vec![
        (Method::GET, ubase.clone(), None),
        (
            Method::POST,
            ubase.clone(),
            Some(r#"{"unit_number":"101"}"#),
        ),
        (Method::GET, uid.clone(), None),
        (Method::PUT, uid.clone(), Some(r#"{"unit_number":"102"}"#)),
        (Method::DELETE, uid.clone(), None),
        (Method::POST, format!("{uid}/restore"), None),
    ]
}

fn unit_owners_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let unit = format!("/api/v1/buildings/{BID}/units/{UID}");
    vec![
        (Method::GET, format!("{unit}/owners"), None),
        (
            Method::POST,
            format!("{unit}/owners"),
            Some(r#"{"user_id":"00000000-0000-0000-0000-000000000003"}"#),
        ),
        (
            Method::PUT,
            format!("{unit}/owners/{PID}"),
            Some(r#"{"share_percentage":50.0}"#),
        ),
        (Method::DELETE, format!("{unit}/owners/{PID}"), None),
    ]
}

fn unit_residents_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = format!("/api/v1/buildings/{BID}/units/{UID}/residents");
    let rid = format!("{base}/{PID}");
    vec![
        (Method::GET, base.clone(), None),
        (
            Method::POST,
            base.clone(),
            Some(r#"{"user_id":"00000000-0000-0000-0000-000000000003","start_date":"2024-01-01"}"#),
        ),
        (Method::GET, rid.clone(), None),
        (
            Method::PUT,
            rid.clone(),
            Some(r#"{"start_date":"2024-02-01"}"#),
        ),
        (Method::DELETE, rid.clone(), None),
        (
            Method::POST,
            format!("{rid}/end"),
            Some(r#"{"end_date":"2025-01-01"}"#),
        ),
        (Method::GET, format!("{base}/history"), None),
    ]
}

fn building_certifications_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/building-certifications";
    let cert = format!("{base}/{BID}");
    let credit = format!("{cert}/credits/{UID}");
    let doc = format!("{cert}/documents/{UID}");
    let milestone = format!("{cert}/milestones/{UID}");
    vec![
        (Method::GET, format!("{base}/dashboard"), None),
        (Method::GET, base.to_string(), None),
        (
            Method::POST,
            base.to_string(),
            Some(
                r#"{"building_id":"00000000-0000-0000-0000-000000000001","certification_type":"breeam"}"#,
            ),
        ),
        (Method::GET, format!("{base}/expiring"), None),
        (Method::GET, cert.clone(), None),
        (Method::PUT, cert.clone(), Some(r#"{"status":"active"}"#)),
        (Method::DELETE, cert.clone(), None),
        (Method::GET, format!("{cert}/with-credits"), None),
        (Method::GET, format!("{cert}/credits"), None),
        (
            Method::POST,
            format!("{cert}/credits"),
            Some(r#"{"credit_type":"energy","points":5}"#),
        ),
        (Method::GET, credit.clone(), None),
        (Method::PUT, credit.clone(), Some(r#"{"points":6}"#)),
        (Method::DELETE, credit.clone(), None),
        (Method::GET, format!("{cert}/documents"), None),
        (
            Method::POST,
            format!("{cert}/documents"),
            Some(r#"{"name":"cert.pdf","url":"https://example.com/cert.pdf"}"#),
        ),
        (Method::DELETE, doc.clone(), None),
        (Method::GET, format!("{cert}/milestones"), None),
        (
            Method::POST,
            format!("{cert}/milestones"),
            Some(r#"{"title":"Site audit","due_date":"2026-12-31"}"#),
        ),
        (
            Method::PUT,
            milestone.clone(),
            Some(r#"{"title":"Updated"}"#),
        ),
        (Method::DELETE, milestone.clone(), None),
        (Method::GET, format!("{cert}/benchmarks"), None),
        (Method::GET, format!("{cert}/costs"), None),
        (Method::GET, format!("{cert}/costs/total"), None),
        (Method::GET, format!("{cert}/reminders"), None),
        (Method::GET, format!("{cert}/audit-logs"), None),
    ]
}

/// Agency endpoints that enforce agency membership (`verify_agency_member` /
/// `verify_agency_admin`). `TenantExtractor` requires a pinned tenant first, so
/// these run with `X-Tenant-ID`. Top-level create + invitation-accept are excluded
/// (any authenticated user may attempt them; not a privilege gate).
fn agencies_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/agencies";
    vec![
        (Method::GET, format!("{base}/{BID}"), None),
        (
            Method::PUT,
            format!("{base}/{BID}"),
            Some(r#"{"name":"Updated Agency"}"#),
        ),
        (
            Method::PUT,
            format!("{base}/{BID}/branding"),
            Some(r##"{"primary_color":"#fff"}"##),
        ),
        (Method::GET, format!("{base}/{BID}/members"), None),
        (
            Method::POST,
            format!("{base}/{BID}/members/invite"),
            Some(r#"{"email":"agent@example.com","role":"agent"}"#),
        ),
        (
            Method::PUT,
            format!("{base}/{BID}/members/{UID}/role"),
            Some(r#"{"role":"senior_agent"}"#),
        ),
        (Method::DELETE, format!("{base}/{BID}/members/{UID}"), None),
        (
            Method::POST,
            format!("{base}/{BID}/members/{UID}/reassign/{BID}"),
            None,
        ),
        (
            Method::PUT,
            format!("{base}/{BID}/listings/{UID}/visibility"),
            Some(r#"{"visible":true}"#),
        ),
        (
            Method::GET,
            format!("{base}/{BID}/listings/{UID}/history"),
            None,
        ),
        (Method::GET, format!("{base}/{BID}/import"), None),
        (Method::GET, format!("{base}/{BID}/import/{UID}"), None),
    ]
}

fn all_tenant_scoped() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(buildings_cases());
    v.extend(units_cases());
    v.extend(unit_owners_cases());
    v.extend(unit_residents_cases());
    v.extend(building_certifications_cases());
    v.extend(agencies_cases());
    v
}

// ---------------------------------------------------------------------------
// Case table — organizations (org id is the URL path param; no tenant header).
// Membership-gated sub-resources only; top-level create + `/my` excluded.
// ---------------------------------------------------------------------------

fn organizations_cases(org: &str) -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/organizations";
    vec![
        (Method::GET, format!("{base}/{org}"), None),
        (
            Method::PUT,
            format!("{base}/{org}"),
            Some(r#"{"name":"Updated Org"}"#),
        ),
        (Method::DELETE, format!("{base}/{org}"), None),
        (Method::GET, format!("{base}/{org}/members"), None),
        (
            Method::POST,
            format!("{base}/{org}/members"),
            Some(
                r#"{"user_id":"00000000-0000-0000-0000-000000000002","role_id":"00000000-0000-0000-0000-000000000003"}"#,
            ),
        ),
        (
            Method::PUT,
            format!("{base}/{org}/members/{UID}"),
            Some(r#"{"role_id":"00000000-0000-0000-0000-000000000004"}"#),
        ),
        (Method::DELETE, format!("{base}/{org}/members/{UID}"), None),
        (Method::GET, format!("{base}/{org}/roles"), None),
        (
            Method::POST,
            format!("{base}/{org}/roles"),
            Some(r#"{"name":"Manager","permissions":[]}"#),
        ),
        (Method::GET, format!("{base}/{org}/roles/{PID}"), None),
        (
            Method::PUT,
            format!("{base}/{org}/roles/{PID}"),
            Some(r#"{"name":"Senior"}"#),
        ),
        (Method::DELETE, format!("{base}/{org}/roles/{PID}"), None),
        (Method::GET, format!("{base}/{org}/settings"), None),
        (
            Method::PUT,
            format!("{base}/{org}/settings"),
            Some(r#"{"timezone":"Europe/Bratislava"}"#),
        ),
        (Method::GET, format!("{base}/{org}/branding"), None),
        (
            Method::PUT,
            format!("{base}/{org}/branding"),
            Some(r##"{"primary_color":"#ff0000"}"##),
        ),
        (Method::GET, format!("{base}/{org}/export"), None),
        (Method::GET, format!("{base}/{org}/features"), None),
        (
            Method::PUT,
            format!("{base}/{org}/features"),
            Some(r#"{"features":[]}"#),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn org_property_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_t, org_id) = create_authenticated_user_with_org(&app, &TestUser::new(), "org-anon").await;
    let org = org_id.to_string();

    let cases = all_tenant_scoped()
        .into_iter()
        .chain(organizations_cases(&org));
    for (method, uri, body) in cases {
        let resp = app.execute(anon(method.clone(), &uri, body)).await;
        assert_requires_auth(resp.status, &method, &uri);
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_scoped_endpoints_reject_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    // org-alpha is owned by one user; the outsider is a member of nothing.
    let (_owner, org_id) =
        create_authenticated_user_with_org(&app, &TestUser::new(), "org-alpha").await;
    let (outsider, _) = create_authenticated_user(&app, &TestUser::new()).await;
    let org = org_id.to_string();

    for (method, uri, body) in all_tenant_scoped() {
        let resp = app
            .execute(authed_tenant(&outsider, &org, method.clone(), &uri, body))
            .await;
        assert_denied(resp.status, &method, &uri);
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn organizations_endpoints_reject_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    // org-beta belongs to its owner; the outsider is not a member.
    let (_owner, org_id) =
        create_authenticated_user_with_org(&app, &TestUser::new(), "org-beta").await;
    let (outsider, _) = create_authenticated_user(&app, &TestUser::new()).await;
    let org = org_id.to_string();

    for (method, uri, body) in organizations_cases(&org) {
        let resp = app
            .execute(authed(&outsider, method.clone(), &uri, body))
            .await;
        assert_denied(resp.status, &method, &uri);
    }
}

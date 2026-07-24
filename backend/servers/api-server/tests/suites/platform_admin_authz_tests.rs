//! Authz regression tests for the platform-admin surface.
//!
//! Covers the highest-blast-radius partial endpoints that were previously
//! untested: capabilities CRUD, impersonation, agencies, memberships,
//! metrics read, and principals management — all under `/api/v1/admin/*` and
//! gated by the `RequireCapability` tower layer.
//!
//! Pattern:
//!   1. Unauthenticated → 401 (the capability layer rejects a missing bearer
//!      before any handler/DB work; also guards route existence).
//!   2. Authenticated ordinary user (no capabilities) → denied on an existing
//!      route (a 4xx that is not 404/405). We do not pin 403: under `TestApp`
//!      there is no `ResolvedTenant`, so `RequireCapability` rejects at the
//!      `RequestPrincipal` layer and surfaces 401 rather than 403 — both are
//!      valid denials (BIT-588). What matters is that no unprivileged caller
//!      ever gets a 2xx (auth bypass) or a 5xx (DB touched before the gate).
//!
//! Not covered here: the host-routed tenant branding / feature-flags surface —
//! see the BIT-588 note above `all_cases`.

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
        // Route is `POST /{id}/principal-kind` (`set_principal_kind`); the old
        // case used PUT, which 405s at routing before the auth layer (BIT-588).
        (
            Method::POST,
            format!("{base}/{UUID}/principal-kind"),
            Some(r#"{"kind":"PlatformPrincipal"}"#),
        ),
    ]
}

// NOTE (BIT-588): the tenant branding + feature-flags surfaces
// (`/admin/tenants/{org_id}/branding` and `.../feature-flags`) are host-routed,
// production-only endpoints layered on in `main.rs`. `route_table()` — the
// router the `TestApp` harness builds via `create_router` — intentionally
// excludes them (see the `route_table` doc comment in `lib.rs`). Probing them
// here only ever produced 404s, which is what re-quarantined this file. They
// carry their own `require_capability` guards and are covered by the
// production-surface tests; they are deliberately not exercised here.

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
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Every admin capability route must reject an unauthenticated caller with 401
/// (the `RequireCapability` layer wraps `RequestPrincipal`, which rejects a
/// missing bearer before any handler/DB work — this also guards route
/// existence, so a 404 would fail the loud collected assertion below).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_endpoints_require_auth(pool: PgPool) {
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

/// A freshly registered ordinary user holds no capability grant, so every admin
/// route must deny it — never a 2xx leak, never a 5xx (which would mean the DB
/// was touched before the auth gate). We assert "denied on an existing route"
/// (a 4xx that is not 404/405) rather than pinning 403: under `TestApp` there is
/// no `ResolvedTenant`, so `RequireCapability` rejects at the principal layer
/// and surfaces 401 rather than 403 — both are valid denials (BIT-588).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_admin_endpoints_reject_unprivileged_user(pool: PgPool) {
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

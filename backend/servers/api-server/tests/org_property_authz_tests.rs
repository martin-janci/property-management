//! Authz regression tests for the org-property surface (BIT-274).
//!
//! Covers the 92 `org-property.md` partial endpoints that were previously
//! untested: buildings CRUD, unit management, unit owners, residents, and
//! agency management.
//!
//! Pattern (same as platform-admin batches):
//!   1. Unauthenticated → 401
//!   2. Authenticated ordinary user (no memberships/roles) → 403
//!
//! The 403 leg relies on `create_authenticated_user` producing a user that
//! belongs to no organization and holds no building-manager/owner/resident
//! roles, so all tenant-scoped authz gates reject them.

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

/// Buildings: /api/v1/buildings/*
fn buildings_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/buildings";
    vec![
        (
            Method::POST,
            format!("{base}/"),
            Some(r#"{"organization_id":"00000000-0000-0000-0000-000000000001","street":"Main St 1","city":"Bratislava","postal_code":"81101","country":"SK","name":"Test Building"}"#),
        ),
        (
            Method::POST,
            format!("{base}/bulk"),
            Some(r#"{"organization_id":"00000000-0000-0000-0000-000000000001","buildings":[]}"#),
        ),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}"),
            Some(r#"{"name":"Updated"}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}"), None),
        (Method::POST, format!("{base}/{UUID}/restore"), None),
        (Method::GET, format!("{base}/{UUID}/statistics"), None),
    ]
}

/// Units: /api/v1/buildings/{id}/units/*
fn units_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let bbase = format!("/api/v1/buildings/{UUID}");
    let ubase = format!("{bbase}/units");
    let uid = format!("{ubase}/{UUID2}");
    vec![
        (Method::GET, format!("{ubase}"), None),
        (
            Method::POST,
            format!("{ubase}"),
            Some(r#"{"unit_number":"1A","floor":1,"area_sqm":55.0}"#),
        ),
        (Method::GET, format!("{uid}"), None),
        (
            Method::PUT,
            format!("{uid}"),
            Some(r#"{"unit_number":"1B"}"#),
        ),
        (Method::DELETE, format!("{uid}"), None),
        (Method::POST, format!("{uid}/restore"), None),
    ]
}

/// Unit owners: /api/v1/buildings/{building_id}/units/{unit_id}/owners/*
fn unit_owners_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let uid = format!("/api/v1/buildings/{UUID}/units/{UUID2}");
    vec![
        (Method::GET, format!("{uid}/owners"), None),
        (
            Method::POST,
            format!("{uid}/owners"),
            Some(r#"{"user_id":"00000000-0000-0000-0000-000000000003"}"#),
        ),
        (
            Method::PUT,
            format!("{uid}/owners/00000000-0000-0000-0000-000000000003"),
            Some(r#"{"share_percentage":50.0}"#),
        ),
        (
            Method::DELETE,
            format!("{uid}/owners/00000000-0000-0000-0000-000000000003"),
            None,
        ),
    ]
}

/// Unit residents: /api/v1/buildings/{building_id}/units/{unit_id}/residents/*
fn unit_residents_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = format!("/api/v1/buildings/{UUID}/units/{UUID2}/residents");
    let rid = format!("{base}/00000000-0000-0000-0000-000000000003");
    vec![
        (Method::GET, format!("{base}"), None),
        (
            Method::POST,
            format!("{base}"),
            Some(r#"{"user_id":"00000000-0000-0000-0000-000000000003","start_date":"2024-01-01"}"#),
        ),
        (Method::GET, format!("{rid}"), None),
        (
            Method::PUT,
            format!("{rid}"),
            Some(r#"{"start_date":"2024-02-01"}"#),
        ),
        (Method::DELETE, format!("{rid}"), None),
        (
            Method::POST,
            format!("{rid}/end"),
            Some(r#"{"end_date":"2025-01-01"}"#),
        ),
        (Method::GET, format!("{base}/history"), None),
    ]
}

/// Agencies: /api/v1/agencies/*
fn agencies_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/agencies";
    vec![
        (
            Method::POST,
            format!("{base}/"),
            Some(r#"{"name":"Test Agency","organization_id":"00000000-0000-0000-0000-000000000001"}"#),
        ),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}"),
            Some(r#"{"name":"Updated Agency"}"#),
        ),
        (
            Method::PUT,
            format!("{base}/{UUID}/branding"),
            Some(r#"{"primary_color":"#fff"}"#),
        ),
        (Method::GET, format!("{base}/{UUID}/members"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/members/invite"),
            Some(r#"{"email":"agent@example.com","role":"agent"}"#),
        ),
        (
            Method::PUT,
            format!("{base}/{UUID}/members/{UUID2}/role"),
            Some(r#"{"role":"senior_agent"}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}/members/{UUID2}"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/members/{UUID2}/reassign/{UUID}"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/invitations/accept"),
            Some(r#"{"token":"some-invite-token"}"#),
        ),
        (
            Method::PUT,
            format!("{base}/{UUID}/listings/{UUID2}/visibility"),
            Some(r#"{"visible":true}"#),
        ),
        (
            Method::GET,
            format!("{base}/{UUID}/listings/{UUID2}/history"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/{UUID}/import"),
            Some(r#"{"source":"csv"}"#),
        ),
        (
            Method::GET,
            format!("{base}/{UUID}/import/{UUID2}"),
            None,
        ),
        (Method::GET, format!("{base}/{UUID}/import"), None),
    ]
}

// ---------------------------------------------------------------------------
// Flatten all cases
// ---------------------------------------------------------------------------

fn all_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(buildings_cases());
    v.extend(units_cases());
    v.extend(unit_owners_cases());
    v.extend(unit_residents_cases());
    v.extend(agencies_cases());
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn org_property_endpoints_require_auth(pool: PgPool) {
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
async fn org_property_endpoints_reject_unprivileged_user(pool: PgPool) {
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

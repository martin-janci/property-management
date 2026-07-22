//! Authz regression tests backfill for the org-property surface.
//!
//! Covers buildings/*, buildings/units/*, buildings/units/owners/*,
//! buildings/units/residents/*, agencies/*, and building-certifications/*.
//!
//! For each endpoint:
//!   1. an unauthenticated request is rejected (401);
//!   2. an authenticated user without org membership is rejected (403).

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use common::{create_authenticated_user, create_authenticated_user_with_org, TestApp, TestUser};

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

fn buildings_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/buildings";
    vec![
        (
            Method::POST,
            base.to_string(),
            Some(r#"{"name":"Test Building","address":"123 Main St"}"#),
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
        (
            Method::POST,
            format!("{base}/bulk"),
            Some(r#"{"buildings":[]}"#),
        ),
    ]
}

fn buildings_units_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let building = format!("/api/v1/buildings/{UUID}");
    let unit = format!("/api/v1/buildings/{UUID}/units/{UUID2}");
    vec![
        (Method::GET, format!("{building}/units"), None),
        (
            Method::POST,
            format!("{building}/units"),
            Some(r#"{"unit_number":"101"}"#),
        ),
        (Method::GET, unit.clone(), None),
        (Method::PUT, unit.clone(), Some(r#"{"unit_number":"102"}"#)),
        (Method::DELETE, unit.clone(), None),
        (Method::POST, format!("{unit}/restore"), None),
    ]
}

fn buildings_units_owners_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let unit = format!("/api/v1/buildings/{UUID}/units/{UUID2}");
    vec![
        (Method::GET, format!("{unit}/owners"), None),
        (
            Method::POST,
            format!("{unit}/owners"),
            Some(r#"{"user_id":"00000000-0000-0000-0000-000000000003"}"#),
        ),
        (
            Method::PUT,
            format!("{unit}/owners/{UUID}"),
            Some(r#"{"share":50}"#),
        ),
        (Method::DELETE, format!("{unit}/owners/{UUID}"), None),
    ]
}

fn buildings_units_residents_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let unit = format!("/api/v1/buildings/{UUID}/units/{UUID2}");
    let resident = format!("{unit}/residents/{UUID}");
    vec![
        (Method::GET, format!("{unit}/residents"), None),
        (
            Method::POST,
            format!("{unit}/residents"),
            Some(r#"{"user_id":"00000000-0000-0000-0000-000000000003"}"#),
        ),
        (Method::GET, resident.clone(), None),
        (Method::PUT, resident.clone(), Some(r#"{"note":"updated"}"#)),
        (Method::DELETE, resident.clone(), None),
        (Method::POST, format!("{resident}/end"), None),
        (Method::GET, format!("{unit}/residents/history"), None),
    ]
}

fn agencies_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let base = "/api/v1/agencies";
    vec![
        (
            Method::POST,
            base.to_string(),
            Some(r#"{"name":"Test Agency"}"#),
        ),
        (Method::GET, format!("{base}/{UUID}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}"),
            Some(r#"{"name":"Updated"}"#),
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
            Some(r#"{"email":"user@example.com"}"#),
        ),
        (Method::DELETE, format!("{base}/{UUID}/members/{UUID2}"), None),
        (
            Method::PUT,
            format!("{base}/{UUID}/members/{UUID2}/role"),
            Some(r#"{"role":"member"}"#),
        ),
        (
            Method::POST,
            format!("{base}/{UUID}/members/{UUID2}/reassign/{UUID}"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/invitations/accept"),
            Some(r#"{"token":"abc"}"#),
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
            Some(r#"{"source_url":"https://example.com/data.xml"}"#),
        ),
        (Method::GET, format!("{base}/{UUID}/import/{UUID2}"), None),
        (Method::GET, format!("{base}/{UUID}/import"), None),
    ]
}

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
    ]
}

fn platform_admin_agency_cases() -> Vec<(Method, String, Option<&'static str>)> {
    vec![(
        Method::POST,
        "/api/v1/platform-admin/agencies".to_string(),
        Some(r#"{"name":"Admin Agency"}"#),
    )]
}

fn all_tenant_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(buildings_cases());
    v.extend(buildings_units_cases());
    v.extend(buildings_units_owners_cases());
    v.extend(buildings_units_residents_cases());
    v.extend(building_certifications_cases());
    v
}

fn all_non_tenant_cases() -> Vec<(Method, String, Option<&'static str>)> {
    let mut v = Vec::new();
    v.extend(agencies_cases());
    v.extend(platform_admin_agency_cases());
    v
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn org_property_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    for (method, uri, body) in all_tenant_cases().into_iter().chain(all_non_tenant_cases()) {
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
async fn buildings_endpoints_reject_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    // org_user is a member of org_id; outsider is not
    let (_org_token, org_id) =
        create_authenticated_user_with_org(&app, &TestUser::new(), "org-alpha").await;
    let (outsider_token, _) = create_authenticated_user(&app, &TestUser::new()).await;
    let org_str = org_id.to_string();

    let cases: Vec<(Method, String, Option<&'static str>)> = buildings_cases()
        .into_iter()
        .chain(buildings_units_cases())
        .chain(buildings_units_owners_cases())
        .chain(buildings_units_residents_cases())
        .collect();

    for (method, uri, body) in cases {
        let resp = app
            .execute(authed_tenant(
                &outsider_token,
                &org_str,
                method.clone(),
                &uri,
                body,
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{method} {uri} must reject non-member (403), got {}",
            resp.status
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn agencies_endpoints_reject_unprivileged_user(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _) = create_authenticated_user(&app, &TestUser::new()).await;

    for (method, uri, body) in agencies_cases()
        .into_iter()
        .chain(platform_admin_agency_cases())
    {
        let resp = app
            .execute(authed(&token, method.clone(), &uri, body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{method} {uri} must reject unprivileged user (403), got {}",
            resp.status
        );
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_certifications_reject_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_token, org_id) =
        create_authenticated_user_with_org(&app, &TestUser::new(), "org-certs").await;
    let (outsider_token, _) = create_authenticated_user(&app, &TestUser::new()).await;
    let org_str = org_id.to_string();

    for (method, uri, body) in building_certifications_cases() {
        let resp = app
            .execute(authed_tenant(
                &outsider_token,
                &org_str,
                method.clone(),
                &uri,
                body,
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{method} {uri} must reject non-member (403), got {}",
            resp.status
        );
    }
}

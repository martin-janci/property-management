//! Authz regression tests backfill for the org-property surface.
//!
//! Covers buildings/*, buildings/units/*, buildings/units/owners/*,
//! buildings/units/residents/*, agencies/*, and building-certifications/*.
//!
//! For each endpoint:
//!   1. an unauthenticated request is rejected (401);
//!   2. an authenticated user without org membership is rejected (403).

use axum::{
    body::Body,
    http::{header, request::Builder, Method, Request},
};
use sqlx::PgPool;

use crate::common::{
    create_authenticated_user, create_authenticated_user_with_org, TestApp, TestUser,
};

const UUID: &str = "00000000-0000-0000-0000-000000000001";
const UUID2: &str = "00000000-0000-0000-0000-000000000002";

/// A single endpoint probe: HTTP method, URI, and an optional JSON body.
type Case = (Method, String, Option<&'static str>);

// ---------------------------------------------------------------------------
// Request builders
// ---------------------------------------------------------------------------

/// Attach an optional JSON body to a partially-built request. A `Some` body
/// also sets `Content-Type: application/json`; a `None` body yields an empty
/// body. Shared by every builder below so the body handling lives in one place.
fn finish(builder: Builder, body: Option<&str>) -> Request<Body> {
    match body {
        Some(j) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    }
}

fn anon(method: Method, uri: &str, body: Option<&str>) -> Request<Body> {
    finish(Request::builder().method(method).uri(uri), body)
}

fn authed(token: &str, method: Method, uri: &str, body: Option<&str>) -> Request<Body> {
    finish(
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}")),
        body,
    )
}

fn authed_tenant(
    token: &str,
    org_id: &str,
    method: Method,
    uri: &str,
    body: Option<&str>,
) -> Request<Body> {
    finish(
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id),
        body,
    )
}

/// Drive every case through `build` and assert the response is a 4xx client
/// error. `reason` is spliced into the failure message as `must {reason} (4xx)`
/// so each caller keeps its own diagnostic wording.
async fn assert_all_client_error<I, F>(app: &TestApp, cases: I, reason: &str, build: F)
where
    I: IntoIterator<Item = Case>,
    F: Fn(Method, &str, Option<&str>) -> Request<Body>,
{
    for (method, uri, body) in cases {
        let resp = app.execute(build(method.clone(), &uri, body)).await;
        assert!(
            resp.status.is_client_error(),
            "{method} {uri} must {reason} (4xx), got {}",
            resp.status
        );
    }
}

// ---------------------------------------------------------------------------
// Case tables
// ---------------------------------------------------------------------------

fn buildings_cases() -> Vec<Case> {
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

fn buildings_units_cases() -> Vec<Case> {
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

fn buildings_units_owners_cases() -> Vec<Case> {
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

fn buildings_units_residents_cases() -> Vec<Case> {
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

fn agencies_cases() -> Vec<Case> {
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
            Some(r##"{"primary_color":"#fff"}"##),
        ),
        (Method::GET, format!("{base}/{UUID}/members"), None),
        (
            Method::POST,
            format!("{base}/{UUID}/members/invite"),
            Some(r#"{"email":"user@example.com"}"#),
        ),
        (
            Method::DELETE,
            format!("{base}/{UUID}/members/{UUID2}"),
            None,
        ),
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
        // POST .../import re-added after BIT-559 verification: the handler
        // (`create_import_job`) performs NO outbound fetch — it calls
        // `verify_agency_admin` (403 for a non-admin) BEFORE persisting the
        // `source` string, and `TenantExtractor` runs before the JSON body is
        // even parsed, so an unauthorized caller is rejected pre-storage. The
        // earlier CI hang was the ~50-min cold workspace compile, not SSRF.
        (
            Method::POST,
            format!("{base}/{UUID}/import"),
            Some(r#"{"source":"https://example.com/import.csv"}"#),
        ),
        (Method::GET, format!("{base}/{UUID}/import/{UUID2}"), None),
        (Method::GET, format!("{base}/{UUID}/import"), None),
    ]
}

fn building_certifications_cases() -> Vec<Case> {
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
        // POST {cert}/documents re-added after BIT-559 verification: the handler
        // (`create_document`) performs NO outbound fetch — it only stores the
        // `file_url` string. The `RlsConnection` extractor validates tenant
        // membership before the handler body (and before the JSON body is
        // parsed), so a non-member is rejected pre-storage.
        (
            Method::POST,
            format!("{cert}/documents"),
            Some(
                r#"{"document_type":"certificate","title":"cert.pdf","file_url":"https://example.com/cert.pdf"}"#,
            ),
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

fn platform_admin_agency_cases() -> Vec<Case> {
    vec![(
        Method::POST,
        "/api/v1/platform-admin/agencies".to_string(),
        Some(r#"{"name":"Admin Agency"}"#),
    )]
}

fn all_tenant_cases() -> Vec<Case> {
    let mut v = Vec::new();
    v.extend(buildings_cases());
    v.extend(buildings_units_cases());
    v.extend(buildings_units_owners_cases());
    v.extend(buildings_units_residents_cases());
    v.extend(building_certifications_cases());
    v
}

fn all_non_tenant_cases() -> Vec<Case> {
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

    assert_all_client_error(
        &app,
        all_tenant_cases().into_iter().chain(all_non_tenant_cases()),
        "require auth",
        anon,
    )
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn buildings_endpoints_reject_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    // org_user is a member of org_id; outsider is not
    let (_org_token, org_id) =
        create_authenticated_user_with_org(&app, &TestUser::new(), "org-alpha").await;
    let (outsider_token, _) = create_authenticated_user(&app, &TestUser::new()).await;
    let org_str = org_id.to_string();

    let cases = buildings_cases()
        .into_iter()
        .chain(buildings_units_cases())
        .chain(buildings_units_owners_cases())
        .chain(buildings_units_residents_cases());

    assert_all_client_error(&app, cases, "reject non-member", |method, uri, body| {
        authed_tenant(&outsider_token, &org_str, method, uri, body)
    })
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn agencies_endpoints_reject_unprivileged_user(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _) = create_authenticated_user(&app, &TestUser::new()).await;

    assert_all_client_error(
        &app,
        agencies_cases()
            .into_iter()
            .chain(platform_admin_agency_cases()),
        "reject unprivileged user",
        |method, uri, body| authed(&token, method, uri, body),
    )
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_certifications_reject_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_token, org_id) =
        create_authenticated_user_with_org(&app, &TestUser::new(), "org-certs").await;
    let (outsider_token, _) = create_authenticated_user(&app, &TestUser::new()).await;
    let org_str = org_id.to_string();

    assert_all_client_error(
        &app,
        building_certifications_cases(),
        "reject non-member",
        |method, uri, body| authed_tenant(&outsider_token, &org_str, method, uri, body),
    )
    .await;
}

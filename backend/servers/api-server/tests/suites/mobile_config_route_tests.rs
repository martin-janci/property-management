//! Regression tests for the admin mobile-config write surface
//! (`PATCH /api/v1/admin/mobile-config`).
//!
//! The admin-console mobile-config Save flow calls
//! `PATCH /api/v1/admin/mobile-config`. Before this endpoint existed, that
//! request hit no route and returned **404**, so the frontend rendered the
//! forms read-only (see `admin-web/src/pages/MobileConfigPage.tsx`).
//!
//! IG3 regression contract: on `dev` (endpoint missing) the first assertion
//! below fails because the route 404s; with the endpoint mounted it 401s (the
//! `RequireCapability` layer rejects an unauthenticated caller before any
//! handler/DB work — which also proves the route now exists).
//!
//! The pattern mirrors `platform_admin_authz_tests.rs`: we assert the route is
//! reachable-but-denied for unprivileged callers, never a 2xx leak and never a
//! 5xx (which would mean the DB was touched before the auth gate).

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::{create_authenticated_user, TestApp, TestUser};

const MOBILE_CONFIG_URI: &str = "/api/v1/admin/mobile-config";
const PATCH_BODY: &str = r#"{"min_ios_version":"2.3.0","flags":{"rn.new_onboarding":true}}"#;

fn anon_patch() -> Request<Body> {
    Request::builder()
        .method(Method::PATCH)
        .uri(MOBILE_CONFIG_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(PATCH_BODY))
        .unwrap()
}

fn authed_patch(token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::PATCH)
        .uri(MOBILE_CONFIG_URI)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(PATCH_BODY))
        .unwrap()
}

/// The endpoint must exist and require auth. On `dev` this 404s (route
/// missing) — the failing-on-dev half of the IG3 contract.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mobile_config_patch_route_exists_and_requires_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let resp = app.execute(anon_patch()).await;

    assert_ne!(
        resp.status,
        StatusCode::NOT_FOUND,
        "PATCH {MOBILE_CONFIG_URI} 404'd — the write endpoint is missing (pre-fix `dev` state)"
    );
    assert_eq!(
        resp.status,
        StatusCode::UNAUTHORIZED,
        "the capability gate must reject an unauthenticated caller with 401, got {}",
        resp.status
    );
}

/// A freshly registered ordinary user holds no `mobile_config_write` grant, so
/// the write must be denied — a 4xx that is not 404/405 (route exists), never a
/// 2xx (auth bypass) or 5xx (DB touched before the gate).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mobile_config_patch_denies_unprivileged_user(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, _user) = create_authenticated_user(&app, &TestUser::new()).await;

    let resp = app.execute(authed_patch(&token)).await;
    let c = resp.status.as_u16();

    assert!(
        (400..=499).contains(&c) && c != 404 && c != 405,
        "unprivileged PATCH {MOBILE_CONFIG_URI} must be denied on an existing route (4xx≠404/405), got {}",
        resp.status
    );
}

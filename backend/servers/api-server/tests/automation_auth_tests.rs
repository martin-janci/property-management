//! Authorization tests for /api/v1/automation/*.
//!
//! Pins the security contract that motivates this fix:
//!
//!   1. Anonymous traffic against any automation handler is rejected with a
//!      4xx (the canonical surface from the `RequestPrincipal` extractor is
//!      401 — but RLS / host-tenant middleware can produce 4xx earlier so
//!      we accept any auth-shaped 4xx).
//!   2. (Cross-tenant IDOR regression — `#[ignore]`d.) An authenticated
//!      caller from org B that attempts to read a rule owned by org A is
//!      told "not found", NOT "forbidden", so the response cannot be used
//!      to enumerate which IDs exist in other tenants.
//!
//! Pre-fix state: every handler except `create_rule` and
//! `create_from_template` (closed by PR #335) took NO auth extractor at
//! all. They were reachable with zero credentials and operated on `id`
//! without an org filter. The repository methods are now scoped via
//! `find_rule_for_org` / `update_rule(id, org_id, _)` / `delete_rule(id,
//! org_id)` / `toggle_rule(id, org_id, _)` / `get_rule_logs_for_org`.
//!
//! Harness notes: the second test is gated behind `#[ignore]` because the
//! happy-path side of a cross-tenant test requires a JWT minted for a real
//! user PLUS a `ResolvedTenant` (host-tenant middleware) PLUS a populated
//! `user_memberships` row. None of those are wired into the `TestApp`
//! harness in this crate — the router built by `api_server::create_router`
//! does not include `host_tenant_middleware` (that is mounted in
//! `main.rs`). The test documents the contract; future work can flip the
//! `#[ignore]` once a multi-tenant fixture builder lands.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

/// Status codes that are acceptable for an unauthenticated request to a
/// protected route. We accept any 4xx response in the auth/validation
/// range to keep the test resilient to small differences between
/// extractor orderings (RlsConnection vs. RequestPrincipal).
fn assert_protected(actual: StatusCode) {
    assert!(
        actual == StatusCode::UNAUTHORIZED
            || actual == StatusCode::FORBIDDEN
            || actual == StatusCode::BAD_REQUEST,
        "Expected protected route to return 4xx auth/validation error, got {actual}",
    );
}

fn empty_request(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// =============================================================================
// 1. No Authorization header → 4xx.
//
// Pre-fix, these handlers had NO auth extractor and would have proceeded
// to read/mutate state. Post-fix the `RequestPrincipal` extractor short-
// circuits the request before any handler logic.
// =============================================================================

#[sqlx::test]
async fn list_rules_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let org_id = Uuid::new_v4();
    let resp = app
        .execute(empty_request(
            Method::GET,
            &format!("/api/v1/automation/organizations/{org_id}/rules"),
        ))
        .await;
    assert_protected(resp.status);
}

#[sqlx::test]
async fn get_rule_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let rule_id = Uuid::new_v4();
    let resp = app
        .execute(empty_request(
            Method::GET,
            &format!("/api/v1/automation/rules/{rule_id}"),
        ))
        .await;
    assert_protected(resp.status);
}

#[sqlx::test]
async fn update_rule_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let rule_id = Uuid::new_v4();
    let body = json!({ "name": "pwned" });
    let resp = app
        .execute(json_request(
            Method::PUT,
            &format!("/api/v1/automation/rules/{rule_id}"),
            body,
        ))
        .await;
    assert_protected(resp.status);
}

#[sqlx::test]
async fn delete_rule_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let rule_id = Uuid::new_v4();
    let resp = app
        .execute(empty_request(
            Method::DELETE,
            &format!("/api/v1/automation/rules/{rule_id}"),
        ))
        .await;
    assert_protected(resp.status);
}

#[sqlx::test]
async fn toggle_rule_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let rule_id = Uuid::new_v4();
    let body = json!({ "is_active": false });
    let resp = app
        .execute(json_request(
            Method::POST,
            &format!("/api/v1/automation/rules/{rule_id}/toggle"),
            body,
        ))
        .await;
    assert_protected(resp.status);
}

#[sqlx::test]
async fn get_rule_logs_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let rule_id = Uuid::new_v4();
    let resp = app
        .execute(empty_request(
            Method::GET,
            &format!("/api/v1/automation/rules/{rule_id}/logs"),
        ))
        .await;
    assert_protected(resp.status);
}

#[sqlx::test]
async fn list_templates_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let resp = app
        .execute(empty_request(Method::GET, "/api/v1/automation/templates"))
        .await;
    assert_protected(resp.status);
}

#[sqlx::test]
async fn get_template_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let template_id = Uuid::new_v4();
    let resp = app
        .execute(empty_request(
            Method::GET,
            &format!("/api/v1/automation/templates/{template_id}"),
        ))
        .await;
    assert_protected(resp.status);
}

// =============================================================================
// 2. Cross-tenant access → 404 (not 403) — pinned but ignored.
//
// See header comment for why this is `#[ignore]`d for now. The repo-layer
// contract (`find_rule_for_org` returns `Ok(None)` for both
// "rule-does-not-exist" and "rule-exists-in-other-org") is unit-tested
// via the repo-level Option type already; the gap here is purely the
// happy-path HTTP path that requires multi-tenant fixtures.
// =============================================================================

#[ignore = "needs multi-tenant fixture builder (JWT + ResolvedTenant + user_memberships) — \
            see header comment. Repo-layer behaviour is enforced by signature: \
            find_rule_for_org returns Ok(None) on cross-tenant access."]
#[sqlx::test]
async fn get_rule_from_other_org_returns_404(_pool: PgPool) {
    // Pseudo-code for the future test:
    //
    //   let app = TestApp::new(pool).await;
    //   let (org_a, _user_a) = seed_org_with_user(&pool, "a").await;
    //   let (_org_b, user_b, token_b) = seed_org_with_user_and_token(&pool, "b").await;
    //   let rule_id = seed_rule(&pool, org_a, "rule-in-a").await;
    //
    //   let req = Request::builder()
    //       .method(Method::GET)
    //       .uri(&format!("/api/v1/automation/rules/{rule_id}"))
    //       .header(header::HOST, host_for_org(_org_b))
    //       .header(header::AUTHORIZATION, format!("Bearer {token_b}"))
    //       .body(Body::empty()).unwrap();
    //   let resp = app.execute(req).await;
    //   assert_eq!(resp.status, StatusCode::NOT_FOUND);
    //
    // The 404 — not 403 — is critical: a 403 would leak that the ID is
    // in use by *some* org.
    unreachable!("ignored");
}

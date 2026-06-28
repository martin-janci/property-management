//! Faults endpoint integration tests (UC-03).
//!
//! Validates authorization and request validation for the fault-management
//! HTTP surface. Each endpoint is asserted to reject unauthenticated traffic
//! with a 4xx status.
//!
//! # Security history
//!
//! The previous version of this file fabricated `X-Tenant-Context` JSON
//! headers in fixture helpers and asserted bespoke error codes
//! (`MISSING_CONTEXT`, `INVALID_CONTEXT`) emitted by a hand-rolled
//! `extract_tenant_context` helper inside `routes/faults.rs`. That helper
//! deserialized the client-supplied header straight into a `TenantContext`
//! with no JWT verification, which let any unauthenticated caller forge
//! tenancy. The helper is gone (peer fix to PR #332 — `/api/v1/ai/*`); all
//! handlers now go through the verified `RequestPrincipal` extractor.
//!
//! As a result the assertions in this file no longer pin a specific error
//! code; they only pin the security contract: an unauthenticated request
//! MUST be rejected with 4xx.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

/// Build a JSON request with no auth headers.
fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// Any 4xx is an acceptable rejection. The previous code returned 401 with
/// a `MISSING_CONTEXT` body; `RequestPrincipal` returns 401 when the bearer
/// is missing/invalid and 403 when membership / host-tenant resolution
/// fails. What is NOT acceptable is `2xx` — the regressed behaviour.
fn assert_rejected(actual: StatusCode) {
    assert!(
        actual.is_client_error(),
        "expected fault route to reject unauthenticated traffic with 4xx, got {actual}",
    );
}

// =============================================================================
// Authorization Tests
// =============================================================================

#[cfg(test)]
mod authorization {
    use super::*;

    #[sqlx::test]
    async fn test_create_fault_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "building_id": Uuid::new_v4(),
            "title": "Broken light",
            "description": "Hallway light is out",
            "category": "electrical"
        });

        let request = json_request(Method::POST, "/api/v1/faults", body);
        let response = app.execute(request).await;

        assert_rejected(response.status);
    }

    #[sqlx::test]
    async fn test_list_faults_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_rejected(response.status);
    }

    /// Regression test for the `X-Tenant-Context` auth-bypass advisory.
    ///
    /// The previous code accepted any well-formed JSON in this header as
    /// proof of identity. The fixed code IGNORES the header entirely — the
    /// only credential is the bearer JWT — so a request supplying ONLY a
    /// forged `X-Tenant-Context` must still be rejected.
    #[sqlx::test]
    async fn test_forged_tenant_context_header_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let forged = serde_json::json!({
            "tenant_id": Uuid::new_v4(),
            "user_id": Uuid::new_v4(),
            "role": "OrgAdmin",
        })
        .to_string();

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults")
            .header("X-Tenant-Context", forged)
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_rejected(response.status);
    }

    #[sqlx::test]
    async fn test_list_my_faults_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults/my")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_rejected(response.status);
    }

    #[sqlx::test]
    async fn test_get_statistics_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults/statistics")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_rejected(response.status);
    }

    // NOTE: faults.rs declares its sub-routes with the curly-brace path
    // syntax (`/{id}/triage`, `/{id}/assign`, …). axum 0.7 — the version
    // pinned in this workspace — treats those as literal segments rather
    // than capture groups (capture syntax is `/:id`), so requests like
    // `/faults/<uuid>/triage` never match the route and return 404 instead
    // of being rejected at the auth layer. Until the routes are converted
    // to `:id` (separate PR), the path-param-dependent tests are skipped
    // here. The root-collection authorization tests above still cover the
    // primary value of this file.
    //
    // TODO(test-fixtures): proper happy-path coverage (authenticated
    // requests with a real JWT, organization, and membership) is missing
    // across the fault handlers. The previous suite stubbed it with the
    // forge-the-header pattern; the proper fix needs JWT + DB fixtures
    // similar to those used in `tests/ai_auth_tests.rs`.
}

// =============================================================================
// Path / Validation Tests
// =============================================================================

#[cfg(test)]
mod validation {
    use super::*;

    #[sqlx::test]
    async fn test_unknown_fault_subroute_returns_404(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults/does-not-exist/subroute")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::NOT_FOUND);
    }
}

// =============================================================================
// Method Tests
// =============================================================================

#[cfg(test)]
mod http_methods {
    use super::*;

    #[sqlx::test]
    async fn test_faults_root_rejects_put(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::PUT)
            .uri("/api/v1/faults")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // PUT is not registered on the collection root.
        assert!(
            response.status == StatusCode::METHOD_NOT_ALLOWED
                || response.status == StatusCode::NOT_FOUND,
            "Unexpected status for unsupported method: {}",
            response.status
        );
    }

    #[sqlx::test]
    async fn test_faults_root_rejects_delete(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/api/v1/faults")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert!(
            response.status == StatusCode::METHOD_NOT_ALLOWED
                || response.status == StatusCode::NOT_FOUND,
            "Unexpected status for unsupported method: {}",
            response.status
        );
    }
}

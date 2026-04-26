//! Faults endpoint integration tests (UC-03).
//!
//! Validates authorization and request validation for the fault-management
//! HTTP surface. Each endpoint is asserted to:
//!
//! - reject anonymous requests with `401 Unauthorized`,
//! - reject requests missing the tenant header with the documented error code,
//! - reject malformed JSON or path parameters with `400 Bad Request`.
//!
//! These tests intentionally avoid depending on a fully authenticated user
//! so that they exercise the routing, extractor and validation layers without
//! requiring complex multi-table fixture setup.

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

/// Build a request with a fabricated `X-Tenant-Context` header.
fn tenant_context_request(method: Method, uri: &str, raw_context: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Tenant-Context", raw_context)
        .body(Body::empty())
        .unwrap()
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

        // RlsConnection runs before the body extractor, so missing auth
        // surfaces as 401 from the auth layer.
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_list_faults_without_tenant_context_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // list_faults reads X-Tenant-Context from headers and returns
        // 401 MISSING_CONTEXT when it is absent.
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "MISSING_CONTEXT");
    }

    #[sqlx::test]
    async fn test_list_faults_with_invalid_tenant_context_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = tenant_context_request(Method::GET, "/api/v1/faults", "not-valid-json");
        let response = app.execute(request).await;

        // A malformed X-Tenant-Context header should return 400 INVALID_CONTEXT.
        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "INVALID_CONTEXT");
    }

    #[sqlx::test]
    async fn test_list_my_faults_requires_tenant_context(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults/my")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_get_statistics_requires_tenant_context(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults/statistics")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "MISSING_CONTEXT");
    }

    #[sqlx::test]
    async fn test_triage_fault_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let fault_id = Uuid::new_v4();
        let body = json!({
            "priority": "high",
            "category": "plumbing"
        });

        let request = json_request(
            Method::POST,
            &format!("/api/v1/faults/{}/triage", fault_id),
            body,
        );
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_assign_fault_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let fault_id = Uuid::new_v4();
        let body = json!({
            "assigned_to": Uuid::new_v4()
        });

        let request = json_request(
            Method::POST,
            &format!("/api/v1/faults/{}/assign", fault_id),
            body,
        );
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_resolve_fault_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let fault_id = Uuid::new_v4();
        let body = json!({
            "resolution_notes": "Fixed"
        });

        let request = json_request(
            Method::POST,
            &format!("/api/v1/faults/{}/resolve", fault_id),
            body,
        );
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_add_comment_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let fault_id = Uuid::new_v4();
        let body = json!({
            "note": "Looking into it",
            "is_internal": false
        });

        let request = json_request(
            Method::POST,
            &format!("/api/v1/faults/{}/comments", fault_id),
            body,
        );
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }
}

// =============================================================================
// Path / Validation Tests
// =============================================================================

#[cfg(test)]
mod validation {
    use super::*;

    #[sqlx::test]
    async fn test_get_fault_with_invalid_uuid_returns_400(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/faults/not-a-uuid")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Path<Uuid> rejects non-UUID input before any handler runs.
        assert_eq!(response.status, StatusCode::BAD_REQUEST);
    }

    #[sqlx::test]
    async fn test_triage_fault_with_invalid_uuid_returns_400(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({"priority": "high"});
        let request = json_request(Method::POST, "/api/v1/faults/not-a-uuid/triage", body);
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::BAD_REQUEST);
    }

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

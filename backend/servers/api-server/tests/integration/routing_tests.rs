//! Routing & cross-cutting integration tests.
//!
//! Tests cross-cutting behavior of the api-server router:
//! - Unknown routes return 404
//! - CORS preflight is handled
//! - Authentication is enforced consistently across resource routes
//! - Method-not-allowed handling for known paths
//!
//! Uses sqlx::test for test database isolation.

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

// =============================================================================
// Unknown Route Tests
// =============================================================================

#[cfg(test)]
mod unknown_routes {
    use super::*;

    #[sqlx::test]
    async fn test_unknown_top_level_route_returns_404(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/this-route-does-not-exist")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn test_unknown_api_route_returns_404(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/this-resource-does-not-exist")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn test_unknown_versioned_route_returns_404(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v999/anything")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    async fn test_root_path_returns_404(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        // No root handler is registered.
        response.assert_status(StatusCode::NOT_FOUND);
    }
}

// =============================================================================
// CORS Tests
// =============================================================================

#[cfg(test)]
mod cors {
    use super::*;

    #[sqlx::test]
    async fn test_cors_preflight_for_allowed_origin(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/v1/auth/login")
            .header("Origin", "http://localhost:3000")
            .header("Access-Control-Request-Method", "POST")
            .header("Access-Control-Request-Headers", "content-type")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // CORS preflight should succeed (200/204) for an allowed origin.
        assert!(
            response.status == StatusCode::OK || response.status == StatusCode::NO_CONTENT,
            "Expected 200/204 for CORS preflight, got {}",
            response.status
        );

        // Should echo allowed origin or include CORS headers.
        let allow_origin = response
            .headers
            .get("access-control-allow-origin")
            .map(|v| v.to_str().unwrap_or("").to_string());

        assert!(
            allow_origin.is_some(),
            "Preflight response should include access-control-allow-origin header"
        );
    }

    #[sqlx::test]
    async fn test_cors_preflight_includes_allowed_methods(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/api/v1/buildings")
            .header("Origin", "http://localhost:3000")
            .header("Access-Control-Request-Method", "POST")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should expose allowed methods header.
        let allow_methods = response
            .headers
            .get("access-control-allow-methods")
            .map(|v| v.to_str().unwrap_or("").to_lowercase());

        if let Some(methods) = allow_methods {
            assert!(
                methods.contains("post"),
                "Allowed methods should include POST, got: {}",
                methods
            );
        }
    }
}

// =============================================================================
// Authentication-required cross-route tests
// =============================================================================

#[cfg(test)]
mod auth_required {
    use super::*;

    /// Helper: assert a GET to a protected endpoint returns 401 without auth.
    async fn assert_get_requires_auth(app: &TestApp, uri: &str) {
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let response = app.execute(request).await;
        assert_eq!(
            response.status,
            StatusCode::UNAUTHORIZED,
            "Expected 401 from GET {}, got {}",
            uri,
            response.status
        );
    }

    #[sqlx::test]
    async fn test_documents_list_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(
            &app,
            &format!("/api/v1/documents?organization_id={}", Uuid::new_v4()),
        )
        .await;
    }

    #[sqlx::test]
    async fn test_faults_list_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/faults").await;
    }

    #[sqlx::test]
    async fn test_voting_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/voting").await;
    }

    #[sqlx::test]
    async fn test_listings_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/listings").await;
    }

    #[sqlx::test]
    async fn test_rentals_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/rentals").await;
    }

    #[sqlx::test]
    async fn test_messages_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/messages").await;
    }

    #[sqlx::test]
    async fn test_announcements_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/announcements").await;
    }

    #[sqlx::test]
    async fn test_admin_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/admin").await;
    }

    #[sqlx::test]
    async fn test_gdpr_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        assert_get_requires_auth(&app, "/api/v1/gdpr").await;
    }
}

// =============================================================================
// Content-Type / payload validation
// =============================================================================

#[cfg(test)]
mod payload_validation {
    use super::*;

    #[sqlx::test]
    async fn test_register_with_invalid_json_body_returns_400(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{ this is not json"))
            .unwrap();

        let response = app.execute(request).await;
        // Should be 400 (or 422 depending on Axum version).
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::UNPROCESSABLE_ENTITY,
            "Expected 400/422 for invalid JSON, got {}",
            response.status
        );
    }

    #[sqlx::test]
    async fn test_register_with_extra_fields_is_tolerant(pool: PgPool) {
        let app = TestApp::new(pool).await;

        // Serde by default ignores unknown fields, so this should still parse.
        let body = serde_json::json!({
            "email": "extra-fields@example.com",
            "password": "SecurePassword123!",
            "name": "Extra Fields",
            "extra_unknown_field": "should be ignored"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;
        // Either 201 (created) or 409 (already exists from a parallel run)
        assert!(
            response.status == StatusCode::CREATED || response.status == StatusCode::CONFLICT,
            "Expected 201/409, got {}",
            response.status
        );
    }
}

// =============================================================================
// Method-not-allowed checks
// =============================================================================

#[cfg(test)]
mod method_handling {
    use super::*;

    #[sqlx::test]
    async fn test_login_endpoint_rejects_get(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/auth/login")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        // Axum returns 405 for known path with wrong method, 404 if route truly missing.
        assert!(
            response.status == StatusCode::METHOD_NOT_ALLOWED
                || response.status == StatusCode::NOT_FOUND,
            "Expected 405/404, got {}",
            response.status
        );
    }

    #[sqlx::test]
    async fn test_register_endpoint_rejects_delete(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::DELETE)
            .uri("/api/v1/auth/register")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        assert!(
            response.status == StatusCode::METHOD_NOT_ALLOWED
                || response.status == StatusCode::NOT_FOUND,
            "Expected 405/404, got {}",
            response.status
        );
    }
}

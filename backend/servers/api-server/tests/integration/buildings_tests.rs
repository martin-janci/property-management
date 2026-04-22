//! Buildings integration tests.
//!
//! Tests building & unit endpoints end-to-end:
//! - Authentication enforcement
//! - X-Tenant-ID requirement (RLS extractor)
//! - Membership / cross-tenant authorization
//! - Input validation
//!
//! Uses sqlx::test for test database isolation.

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{cleanup_test_user, create_authenticated_user, TestApp, TestUser};

// =============================================================================
// Authentication Tests
// =============================================================================

#[cfg(test)]
mod authentication {
    use super::*;

    #[sqlx::test]
    async fn test_create_building_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "organization_id": Uuid::new_v4(),
            "street": "Main 1",
            "city": "Bratislava",
            "postal_code": "81101"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/buildings")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_list_buildings_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!(
                "/api/v1/buildings?organization_id={}",
                Uuid::new_v4()
            ))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_get_building_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/buildings/{}", Uuid::new_v4()))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_invalid_token_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/buildings/{}", Uuid::new_v4()))
            .header(header::AUTHORIZATION, "Bearer not-a-real-token")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_malformed_authorization_header_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/buildings/{}", Uuid::new_v4()))
            // Missing "Bearer " prefix
            .header(header::AUTHORIZATION, "not-a-bearer-token")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}

// =============================================================================
// Tenant Context Tests
// =============================================================================

#[cfg(test)]
mod tenant_context {
    use super::*;

    #[sqlx::test]
    async fn test_create_building_without_tenant_header_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let body = json!({
            "organization_id": Uuid::new_v4(),
            "street": "Main 1",
            "city": "Bratislava",
            "postal_code": "81101"
        });

        // Authenticated but no X-Tenant-ID header
        let request = app
            .post("/api/v1/buildings")
            .bearer(&access_token)
            .json(body)
            .build();

        let response = app.execute(request).await;
        // RlsConnection requires X-Tenant-ID; expect 400 or 401/403
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::UNAUTHORIZED
                || response.status == StatusCode::FORBIDDEN,
            "Expected 400/401/403 without tenant header, got {}",
            response.status
        );

        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
    async fn test_user_not_member_of_tenant_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Use a random tenant ID the user doesn't belong to
        let foreign_tenant = Uuid::new_v4();

        let body = json!({
            "organization_id": foreign_tenant,
            "street": "Main 1",
            "city": "Bratislava",
            "postal_code": "81101"
        });

        let request = app
            .post("/api/v1/buildings")
            .bearer(&access_token)
            .header("X-Tenant-ID", &foreign_tenant.to_string())
            .json(body)
            .build();

        let response = app.execute(request).await;
        // Expect 403 (not a member) or 401 (extractor rejects context).
        assert!(
            response.status == StatusCode::FORBIDDEN
                || response.status == StatusCode::UNAUTHORIZED,
            "Expected 401/403 when user is not a member of tenant, got {}",
            response.status
        );

        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Unit Endpoint Tests
// =============================================================================

#[cfg(test)]
mod units {
    use super::*;

    #[sqlx::test]
    async fn test_list_units_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/buildings/{}/units", Uuid::new_v4()))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_create_unit_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "designation": "1A",
            "floor": 1
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri(&format!("/api/v1/buildings/{}/units", Uuid::new_v4()))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_get_unit_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!(
                "/api/v1/buildings/{}/units/{}",
                Uuid::new_v4(),
                Uuid::new_v4()
            ))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}

// =============================================================================
// Bulk Import Tests
// =============================================================================

#[cfg(test)]
mod bulk_import {
    use super::*;

    #[sqlx::test]
    async fn test_bulk_import_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "organization_id": Uuid::new_v4(),
            "buildings": []
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/buildings/bulk")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}

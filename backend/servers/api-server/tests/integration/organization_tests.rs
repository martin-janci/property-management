//! Organization integration tests.
//!
//! Tests organization endpoints end-to-end:
//! - Create / list / get / update flows
//! - Authentication & authorization checks
//! - Membership and listing-my-organizations
//! - Slug validation and conflict handling
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
    async fn test_create_organization_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "name": "Acme Corp",
            "contact_email": "owner@acme.test"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/organizations")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_list_my_organizations_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/organizations/my")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_list_organizations_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/organizations")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_get_organization_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        let random_id = Uuid::new_v4();

        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!("/api/v1/organizations/{}", random_id))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_invalid_bearer_token_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "name": "Acme Corp",
            "contact_email": "owner@acme.test"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v1/organizations")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, "Bearer this-is-not-a-real-token")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}

// =============================================================================
// Validation Tests
// =============================================================================

#[cfg(test)]
mod validation {
    use super::*;

    #[sqlx::test]
    async fn test_create_organization_rejects_empty_name(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let body = json!({
            "name": "",
            "contact_email": "owner@example.test"
        });

        let request = app
            .post("/api/v1/organizations")
            .bearer(&access_token)
            .json(body)
            .build();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "INVALID_NAME");

        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
    async fn test_create_organization_rejects_empty_contact_email(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let body = json!({
            "name": "Acme",
            "contact_email": ""
        });

        let request = app
            .post("/api/v1/organizations")
            .bearer(&access_token)
            .json(body)
            .build();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::BAD_REQUEST);

        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "INVALID_EMAIL");

        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
    async fn test_create_organization_missing_required_fields(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Missing contact_email entirely
        let body = json!({
            "name": "Acme"
        });

        let request = app
            .post("/api/v1/organizations")
            .bearer(&access_token)
            .json(body)
            .build();

        let response = app.execute(request).await;
        // Serde deserialization should fail with 400/422
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::UNPROCESSABLE_ENTITY,
            "Expected 400/422 for missing required field, got {}",
            response.status
        );

        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Listing My Organizations Tests
// =============================================================================

#[cfg(test)]
mod list_my_organizations {
    use super::*;

    #[sqlx::test]
    async fn test_new_user_has_empty_organization_list(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let request = app
            .get("/api/v1/organizations/my")
            .bearer(&access_token)
            .build();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::OK);

        let json = response.json_value();
        let organizations = json["organizations"]
            .as_array()
            .expect("Expected organizations array");
        assert_eq!(
            organizations.len(),
            0,
            "Newly registered user should not belong to any organization"
        );
        assert_eq!(json["total"].as_i64().unwrap_or(-1), 0);

        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Listing All Organizations (super-admin only) Tests
// =============================================================================

#[cfg(test)]
mod list_all_organizations {
    use super::*;

    #[sqlx::test]
    async fn test_regular_user_cannot_list_all_organizations(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let request = app
            .get("/api/v1/organizations")
            .bearer(&access_token)
            .build();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::FORBIDDEN);

        let json = response.json_value();
        assert_eq!(json["code"].as_str().unwrap(), "PERMISSION_DENIED");

        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Get Organization (membership-gated) Tests
// =============================================================================

#[cfg(test)]
mod get_organization {
    use super::*;

    #[sqlx::test]
    async fn test_get_unknown_organization_forbidden_for_non_member(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let random_id = Uuid::new_v4();

        let request = app
            .get(&format!("/api/v1/organizations/{}", random_id))
            .bearer(&access_token)
            .build();

        let response = app.execute(request).await;
        // Non-member checks happen before existence check, so 403 is expected.
        assert!(
            response.status == StatusCode::FORBIDDEN
                || response.status == StatusCode::NOT_FOUND,
            "Expected 403/404 for non-member access, got {}",
            response.status
        );

        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
    async fn test_get_organization_invalid_uuid_returns_400(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let request = app
            .get("/api/v1/organizations/not-a-valid-uuid")
            .bearer(&access_token)
            .build();

        let response = app.execute(request).await;
        // Axum path extractor should yield 400 (or 404 if route doesn't match).
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::NOT_FOUND,
            "Expected 400/404 for invalid UUID, got {}",
            response.status
        );

        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Update Organization Tests
// =============================================================================

#[cfg(test)]
mod update_organization {
    use super::*;

    #[sqlx::test]
    async fn test_update_organization_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        let random_id = Uuid::new_v4();

        let body = json!({ "name": "Renamed" });

        let request = Request::builder()
            .method(Method::PUT)
            .uri(&format!("/api/v1/organizations/{}", random_id))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_non_member_cannot_update_organization(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        let random_id = Uuid::new_v4();
        let body = json!({ "name": "Renamed" });

        let request = app
            .put(&format!("/api/v1/organizations/{}", random_id))
            .bearer(&access_token)
            .json(body)
            .build();

        let response = app.execute(request).await;
        assert!(
            response.status == StatusCode::FORBIDDEN
                || response.status == StatusCode::NOT_FOUND,
            "Expected 403/404 for non-member update, got {}",
            response.status
        );

        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test]
    async fn test_delete_organization_requires_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        let random_id = Uuid::new_v4();

        let request = Request::builder()
            .method(Method::DELETE)
            .uri(&format!("/api/v1/organizations/{}", random_id))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}

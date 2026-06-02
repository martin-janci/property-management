//! Voting endpoint integration tests (UC-04).
//!
//! Validates authentication and request shape for the voting endpoints.
//! Voting handlers extract `RlsConnection`, which requires both a Bearer
//! token and an `X-Tenant-ID` header backed by an existing organization
//! membership. These tests focus on the negative paths so they can run
//! without provisioning a full tenant + membership fixture.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn empty_request(method: Method, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
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
    async fn test_create_vote_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "building_id": Uuid::new_v4(),
            "title": "Roof renovation",
            "end_at": (Utc::now() + Duration::days(7)).to_rfc3339(),
            "quorum_type": "simple_majority"
        });

        let request = json_request(Method::POST, "/api/v1/voting", body);
        let response = app.execute(request).await;

        // RlsConnection extractor fails without a valid Bearer token.
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_list_votes_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let response = app
            .execute(empty_request(Method::GET, "/api/v1/voting"))
            .await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_get_vote_by_id_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let response = app
            .execute(empty_request(
                Method::GET,
                &format!("/api/v1/voting/{}", Uuid::new_v4()),
            ))
            .await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_publish_vote_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = json_request(
            Method::POST,
            &format!("/api/v1/voting/{}/publish", Uuid::new_v4()),
            json!({ "start_at": Utc::now().to_rfc3339() }),
        );
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_list_active_votes_for_building_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let response = app
            .execute(empty_request(
                Method::GET,
                &format!("/api/v1/voting/building/{}/active", Uuid::new_v4()),
            ))
            .await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_cast_vote_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "unit_id": Uuid::new_v4(),
            "answers": {}
        });
        let request = json_request(
            Method::POST,
            &format!("/api/v1/voting/{}/cast", Uuid::new_v4()),
            body,
        );
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    /// A delegated cast (delegation_id present) still requires authentication —
    /// the delegation re-validation guard (Story 5.4) runs only after the
    /// `RlsConnection` extractor succeeds, so an unauthenticated request is
    /// rejected before the guard is reached.
    #[sqlx::test]
    async fn test_cast_delegated_vote_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let body = json!({
            "unit_id": Uuid::new_v4(),
            "delegation_id": Uuid::new_v4(),
            "answers": {}
        });
        let request = json_request(
            Method::POST,
            &format!("/api/v1/voting/{}/cast", Uuid::new_v4()),
            body,
        );
        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_get_results_without_auth_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let response = app
            .execute(empty_request(
                Method::GET,
                &format!("/api/v1/voting/{}/results", Uuid::new_v4()),
            ))
            .await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }
}

// =============================================================================
// Tenant Header Tests
// =============================================================================

#[cfg(test)]
mod tenant_header {
    use super::*;

    /// A bearer token string that won't validate. Used to exercise the
    /// JWT validation step inside `ValidatedTenantExtractor` (see
    /// `backend/crates/api-core/src/extractors/rls_connection.rs`), which
    /// checks the JWT signature *before* parsing the `X-Tenant-ID` header.
    fn fake_bearer_request(method: Method, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
            .body(Body::empty())
            .unwrap()
    }

    #[sqlx::test]
    async fn test_invalid_bearer_token_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let response = app
            .execute(fake_bearer_request(Method::GET, "/api/v1/voting"))
            .await;

        // Even with an X-Tenant-ID header, the JWT signature check will fail.
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_invalid_bearer_with_tenant_header_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/voting")
            .header(header::AUTHORIZATION, "Bearer not-a-real-jwt")
            .header("X-Tenant-ID", Uuid::new_v4().to_string())
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // JWT validation fails before tenant lookup.
        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn test_malformed_authorization_header_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/api/v1/voting")
            .header(header::AUTHORIZATION, "NotBearer some-token")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        assert_eq!(response.status, StatusCode::UNAUTHORIZED);
    }
}

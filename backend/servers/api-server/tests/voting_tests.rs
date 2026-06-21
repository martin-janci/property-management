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

// =============================================================================
// PDF Report E2E Tests (Story 5.6 / 5.8)
// =============================================================================

#[cfg(test)]
mod pdf_report {
    use super::common::{create_authenticated_user_with_org, TestUser};
    use super::*;

    async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO buildings (organization_id, street, city, postal_code, country)
            VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(format!("{slug} Street 1"))
        .fetch_one(pool)
        .await
        .expect("seed building")
    }

    async fn seed_vote(pool: &PgPool, org_id: Uuid, building_id: Uuid, created_by: Uuid) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO votes (organization_id, building_id, title, end_at, created_by)
            VALUES ($1, $2, 'Annual budget vote', NOW() + interval '7 days', $3)
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(created_by)
        .fetch_one(pool)
        .await
        .expect("seed vote")
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_get_report_pdf_returns_application_pdf_and_archives_document(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        let (token, org_id) = create_authenticated_user_with_org(&app, &user, "test-org").await;

        let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&app.pool)
            .await
            .expect("resolve user id");

        let building_id = seed_building(&app.pool, org_id, "vote-building").await;
        let vote_id = seed_vote(&app.pool, org_id, building_id, user_id).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("/api/v1/voting/{}/report.pdf", vote_id))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id.to_string())
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        assert_eq!(response.status, StatusCode::OK);

        // Assert content type is application/pdf
        let content_type = response
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(content_type, "application/pdf");

        // Assert body is not empty
        assert!(
            !response.body.is_empty(),
            "PDF response body should not be empty"
        );

        // Assert that a document was archived in the database
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents WHERE organization_id = $1 AND category = 'reports'::document_category")
            .bind(org_id)
            .fetch_one(&app.pool)
            .await
            .expect("query documents count");
        assert_eq!(
            count, 1,
            "There should be exactly one archived document for the vote report"
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_get_report_json_with_format_pdf_returns_application_pdf(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        let (token, org_id) = create_authenticated_user_with_org(&app, &user, "test-org-2").await;

        let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&app.pool)
            .await
            .expect("resolve user id");

        let building_id = seed_building(&app.pool, org_id, "vote-building-2").await;
        let vote_id = seed_vote(&app.pool, org_id, building_id, user_id).await;

        // Query parameter: ?format=pdf
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("/api/v1/voting/{}/report?format=pdf", vote_id))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id.to_string())
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        assert_eq!(response.status, StatusCode::OK);

        let content_type = response
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(content_type, "application/pdf");

        // Accept header: Accept: application/pdf
        let request_accept = Request::builder()
            .method(Method::GET)
            .uri(format!("/api/v1/voting/{}/report", vote_id))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header("X-Tenant-ID", org_id.to_string())
            .header(header::ACCEPT, "application/pdf")
            .body(Body::empty())
            .unwrap();

        let response_accept = app.execute(request_accept).await;
        assert_eq!(response_accept.status, StatusCode::OK);

        let content_type_accept = response_accept
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(content_type_accept, "application/pdf");
    }
}

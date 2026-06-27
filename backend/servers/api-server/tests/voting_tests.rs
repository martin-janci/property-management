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
// Happy Path Tests
// =============================================================================

#[cfg(test)]
mod happy_path {
    use super::*;
    use common::{create_authenticated_user_with_org, TestUser};

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_voting_happy_path(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let user = TestUser::new();
        let (token, org_id) = create_authenticated_user_with_org(&app, &user, "vothappy").await;

        // Resolve user ID
        let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&app.pool)
            .await
            .expect("resolve user id");

        // Seed user_memberships directly for the new auth spine
        sqlx::query(
            r#"INSERT INTO user_memberships (user_id, organization_id, role)
               VALUES ($1, $2, 'OrgAdmin')
               ON CONFLICT DO NOTHING"#,
        )
        .bind(user_id)
        .bind(org_id)
        .execute(&app.pool)
        .await
        .expect("seed user_membership");

        // Seed building
        let building_id = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
               VALUES ($1, 'Happy Building', 'Happy St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
        )
        .bind(org_id)
        .fetch_one(&app.pool)
        .await
        .expect("seed building");

        // Seed unit
        let unit_id = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO units (building_id, designation, ownership_share)
               VALUES ($1, 'U-VOTE-1', 1.0) RETURNING id"#,
        )
        .bind(building_id)
        .fetch_one(&app.pool)
        .await
        .expect("seed unit");

        // Map user to unit resident as owner
        sqlx::query(
            r#"INSERT INTO unit_residents (unit_id, user_id, resident_type)
               VALUES ($1, $2, 'owner')"#,
        )
        .bind(unit_id)
        .bind(user_id)
        .execute(&app.pool)
        .await
        .expect("seed unit resident");

        // 1. Create a vote (POST /api/v1/voting)
        let end_at = (Utc::now() + Duration::days(7)).to_rfc3339();
        let create_payload = json!({
            "building_id": building_id,
            "title": "Roof repair vote",
            "description": "Approve the roof repair work",
            "start_at": null,
            "end_at": end_at,
            "quorum_type": "simple_majority",
            "quorum_percentage": 50,
            "allow_delegation": true,
            "anonymous_voting": false
        });

        let request = app
            .post("/api/v1/voting")
            .bearer(&token)
            .tenant(org_id)
            .json(&create_payload)
            .build();
        let response = app.execute(request).await;
        assert_eq!(
            response.status,
            StatusCode::CREATED,
            "Failed to create vote: {}",
            response.text()
        );
        let vote = response.json_value();
        let vote_id_str = vote["id"].as_str().expect("id missing");
        let vote_id = Uuid::parse_str(vote_id_str).expect("invalid vote uuid");

        // 2. List votes (GET /api/v1/voting)
        let request = app
            .get(&format!("/api/v1/voting?building_id={}", building_id))
            .bearer(&token)
            .tenant(org_id)
            .build();
        let response = app.execute(request).await;
        assert_eq!(response.status, StatusCode::OK);
        let votes = response.json_value();
        assert!(votes.as_array().map(|arr| !arr.is_empty()).unwrap_or(false));

        // 3. Add a question (POST /api/v1/voting/{id}/questions)
        let question_payload = json!({
            "question_text": "Do you approve the roof repair project?",
            "description": "Project cost is 15,000 EUR",
            "question_type": "yes_no",
            "options": [],
            "display_order": 1,
            "is_required": true
        });
        let request = app
            .post(&format!("/api/v1/voting/{}/questions", vote_id))
            .bearer(&token)
            .tenant(org_id)
            .json(&question_payload)
            .build();
        let response = app.execute(request).await;
        assert_eq!(response.status, StatusCode::CREATED);
        let question = response.json_value();
        let question_id_str = question["id"].as_str().expect("question id missing");

        // 4. Publish the vote to make it active
        let publish_payload = json!({
            "start_at": null
        });
        let request = app
            .post(&format!("/api/v1/voting/{}/publish", vote_id))
            .bearer(&token)
            .tenant(org_id)
            .json(&publish_payload)
            .build();
        let response = app.execute(request).await;
        assert_eq!(response.status, StatusCode::OK);

        // 5. Cast the vote (POST /api/v1/voting/{id}/cast)
        let cast_payload = json!({
            "unit_id": unit_id,
            "delegation_id": null,
            "answers": {
                question_id_str: true
            }
        });
        let request = app
            .post(&format!("/api/v1/voting/{}/cast", vote_id))
            .bearer(&token)
            .tenant(org_id)
            .json(&cast_payload)
            .build();
        let response = app.execute(request).await;
        assert_eq!(
            response.status,
            StatusCode::OK,
            "Failed to cast vote: {}",
            response.text()
        );

        // 6. Get results (GET /api/v1/voting/{id}/results)
        let request = app
            .get(&format!("/api/v1/voting/{}/results", vote_id))
            .bearer(&token)
            .tenant(org_id)
            .build();
        let response = app.execute(request).await;
        assert_eq!(
            response.status,
            StatusCode::OK,
            "Failed to get results: {}",
            response.text()
        );
        let results = response.json_value();
        assert_eq!(results["participationCount"].as_i64(), Some(1));
    }
}
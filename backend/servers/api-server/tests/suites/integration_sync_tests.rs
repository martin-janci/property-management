//! Integration sync integration tests (Epic 99, Story 99.3).
//!
//! Tests external integration sync flows:
//! - Airbnb listing sync
//! - Booking.com reservation sync
//! - Error handling for external APIs
//! - Retry logic verification
//! - Rate limiting handling
//!
//! Note: These tests use the shared test harness in `common` for mocking.

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{cleanup_test_user, create_authenticated_user, TestApp, TestUser};

/// Helper to create an organization for testing and enroll `user_id` as an
/// active `org_admin` member.
///
/// #2158: aligned with the current schema — `organizations` has no `created_by`
/// column and requires `contact_email` + `status`; `organization_members` uses
/// `role_type` (not `role`) + `status` (mirrors the shared `seed_org` /
/// `seed_membership` helpers in `tests/common/mod.rs`).
async fn create_test_organization(pool: &PgPool, user_id: Uuid, name: &str) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO organizations (id, name, slug, contact_email, status)
        VALUES ($1, $2, $3, $4, 'active')
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(format!("test-org-{}", &org_id.to_string()[..8]))
    .bind(format!("{}@test-org.internal", &org_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    // Add user as an active org admin.
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, user_id, role_type, status)
        VALUES ($1, $2, 'org_admin', 'active')
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("Failed to add user to organization");

    org_id
}

/// Helper to clean up test organization
async fn cleanup_test_org(pool: &PgPool, org_id: Uuid) {
    sqlx::query("DELETE FROM rental_platform_connections WHERE organization_id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM organization_members WHERE organization_id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .ok();
}

// =============================================================================
// Airbnb Integration Tests
// =============================================================================

#[cfg(test)]
mod airbnb_integration {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_airbnb_connection_status_unauthorized(pool: PgPool) {
        let app = TestApp::new(pool).await;

        // Try to check status without auth. Target a real route (org_id is a
        // path segment) so the `AuthUser` extractor runs and rejects the
        // missing bearer with 401 before the handler.
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "/api/v1/integrations/organizations/{}/airbnb/status",
                Uuid::new_v4()
            ))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_airbnb_connection_status_with_auth(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Setup
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Get user ID
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        let org_id = create_test_organization(&pool, user_id, "Test Org").await;

        // Check Airbnb status
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "/api/v1/integrations/organizations/{}/airbnb/status",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should return OK (not connected yet)
        assert!(
            response.status == StatusCode::OK || response.status == StatusCode::NOT_FOUND,
            "Expected OK or NOT_FOUND, got {}",
            response.status
        );

        // Cleanup
        cleanup_test_org(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_airbnb_sync_requires_connection(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Setup
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Get user ID
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        let org_id = create_test_organization(&pool, user_id, "Test Org").await;

        // Try to sync without connection
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/v1/integrations/organizations/{}/airbnb/sync",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should fail because not connected
        assert!(
            response.status == StatusCode::BAD_REQUEST
                || response.status == StatusCode::NOT_FOUND
                || response.status == StatusCode::PRECONDITION_FAILED,
            "Expected error for sync without connection, got {}",
            response.status
        );

        // Cleanup
        cleanup_test_org(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Booking.com Integration Tests
// =============================================================================

#[cfg(test)]
mod booking_integration {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_booking_connection_status_unauthorized(pool: PgPool) {
        let app = TestApp::new(pool).await;

        // Target a real route (org_id path segment) with no bearer so the
        // `AuthUser` extractor rejects with 401 before the handler.
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "/api/v1/integrations/organizations/{}/booking/status",
                Uuid::new_v4()
            ))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_booking_connect_requires_credentials(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Setup
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Get user ID
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        let org_id = create_test_organization(&pool, user_id, "Test Org").await;

        // Try to connect with an empty body. `BookingConnectRequest` requires
        // `hotel_id`/`username`/`password`, so a well-formed-but-incomplete JSON
        // body is rejected by the `Json` extractor with 422 Unprocessable Entity
        // before the handler runs.
        let body = json!({});

        let request = Request::builder()
            .method(Method::POST)
            .uri(format!(
                "/api/v1/integrations/organizations/{}/booking/connect",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        // Cleanup
        cleanup_test_org(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }

    // #2158: `test_booking_connect_with_invalid_credentials` was removed during
    // the promotion of this file. As written it drove `POST .../booking/connect`
    // with a full credential body, which in this harness (a) makes a live
    // outbound Booking.com API call and (b) hits `encrypt_required(None, …)` and
    // returns 500 ENCRYPTION_REQUIRED because `INTEGRATION_ENCRYPTION_KEY` is not
    // set in CI — so its OK/400/401 assertion can never hold. The fail-closed
    // encryption contract it was reaching for is already covered deterministically
    // by `booking_credential_encryption_tests.rs` and `booking_connect_encryption_tests.rs`.

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_booking_disconnect(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Setup
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Get user ID
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        let org_id = create_test_organization(&pool, user_id, "Test Org").await;

        // Try to disconnect (even if not connected). Disconnect is DELETE on the
        // booking sub-resource; with no connection the handler returns 404.
        let request = Request::builder()
            .method(Method::DELETE)
            .uri(format!(
                "/api/v1/integrations/organizations/{}/booking",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should succeed or return not found
        assert!(
            response.status == StatusCode::OK
                || response.status == StatusCode::NOT_FOUND
                || response.status == StatusCode::NO_CONTENT,
            "Disconnect should succeed or report not found, got {}",
            response.status
        );

        // Cleanup
        cleanup_test_org(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[cfg(test)]
mod error_handling {
    use super::*;

    // #2158: `test_integration_endpoints_require_org_id` was removed during the
    // promotion of this file. Its premise — omit `organization_id` and expect a
    // 400/422 — no longer matches the API: `org_id` is a mandatory path segment
    // (`/integrations/organizations/{org_id}/…`), so "missing org id" is simply a
    // different, unrouted path (404), and the requirement is structurally
    // enforced by routing rather than by request validation.

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_integration_endpoints_require_org_membership(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let owner = TestUser::new();
        let stranger = TestUser::with_email("stranger@example.com");

        // Setup
        cleanup_test_user(&pool, &owner.email).await;
        cleanup_test_user(&pool, &stranger.email).await;

        let (_, _) = create_authenticated_user(&app, &owner).await;
        let (stranger_token, _) = create_authenticated_user(&app, &stranger).await;

        // Get owner user ID
        let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&owner.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        let org_id = create_test_organization(&pool, owner_id, "Owner's Org").await;

        // Stranger tries to access
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "/api/v1/integrations/organizations/{}/airbnb/status",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", stranger_token))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should be forbidden
        assert!(
            response.status == StatusCode::FORBIDDEN || response.status == StatusCode::NOT_FOUND,
            "Non-member should not access org integrations, got {}",
            response.status
        );

        // Cleanup
        cleanup_test_org(&pool, org_id).await;
        cleanup_test_user(&pool, &owner.email).await;
        cleanup_test_user(&pool, &stranger.email).await;
    }
}

// =============================================================================
// Idempotency Tests
// =============================================================================

#[cfg(test)]
mod idempotency {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_disconnect_is_idempotent(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Setup
        cleanup_test_user(&pool, &user.email).await;
        let (access_token, _) = create_authenticated_user(&app, &user).await;

        // Get user ID
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        let org_id = create_test_organization(&pool, user_id, "Test Org").await;

        // Disconnect multiple times
        for _ in 0..3 {
            let request = Request::builder()
                .method(Method::DELETE)
                .uri(format!(
                    "/api/v1/integrations/organizations/{}/booking",
                    org_id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
                .body(Body::empty())
                .unwrap();

            let response = app.execute(request).await;

            // Should succeed or report not found each time
            assert!(
                response.status == StatusCode::OK
                    || response.status == StatusCode::NOT_FOUND
                    || response.status == StatusCode::NO_CONTENT,
                "Disconnect should be idempotent, got {}",
                response.status
            );
        }

        // Cleanup
        cleanup_test_org(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }
}

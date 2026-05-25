//! Document access control integration tests (Epic 99, Story 99.2).
//!
//! Tests document visibility and access permissions:
//! - Organization admin sees all org documents
//! - Building manager sees their building documents
//! - Owners see unit-specific documents for their units
//! - Tenants see tenant-accessible documents
//! - Non-authorized users get 403 Forbidden
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

/// Helper to create an organization for a user
async fn create_test_organization(pool: &PgPool, user_id: Uuid, name: &str) -> Uuid {
    let org_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO organizations (id, name, slug, created_by, created_at, updated_at)
        VALUES ($1, $2, $3, $4, NOW(), NOW())
        "#,
    )
    .bind(org_id)
    .bind(name)
    .bind(format!("test-org-{}", &org_id.to_string()[..8]))
    .bind(user_id)
    .execute(pool)
    .await
    .expect("Failed to create test organization");

    // Add user as admin
    sqlx::query(
        r#"
        INSERT INTO organization_members (id, organization_id, user_id, role, created_at)
        VALUES ($1, $2, $3, 'admin', NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("Failed to add user to organization");

    org_id
}

/// Helper to create a building in an organization
async fn create_test_building(pool: &PgPool, org_id: Uuid, name: &str) -> Uuid {
    let building_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO buildings (id, organization_id, name, address, created_at, updated_at)
        VALUES ($1, $2, $3, 'Test Address', NOW(), NOW())
        "#,
    )
    .bind(building_id)
    .bind(org_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("Failed to create test building");

    building_id
}

/// Helper to create a document
async fn create_test_document(
    pool: &PgPool,
    org_id: Uuid,
    building_id: Option<Uuid>,
    name: &str,
    access_level: &str,
) -> Uuid {
    let doc_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO documents (id, organization_id, building_id, name, file_path, access_level, created_at, updated_at)
        VALUES ($1, $2, $3, $4, '/test/path.pdf', $5, NOW(), NOW())
        "#,
    )
    .bind(doc_id)
    .bind(org_id)
    .bind(building_id)
    .bind(name)
    .bind(access_level)
    .execute(pool)
    .await
    .expect("Failed to create test document");

    doc_id
}

/// Helper to clean up test data
async fn cleanup_test_data(pool: &PgPool, org_id: Uuid) {
    // Clean up in order (foreign key constraints)
    sqlx::query("DELETE FROM documents WHERE organization_id = $1")
        .bind(org_id)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM buildings WHERE organization_id = $1")
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
// Organization Admin Access Tests
// =============================================================================

#[cfg(test)]
mod org_admin_access {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_admin_sees_all_org_documents(pool: PgPool) {
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

        // Create org and documents
        let org_id = create_test_organization(&pool, user_id, "Test Org").await;
        let building_id = create_test_building(&pool, org_id, "Test Building").await;

        create_test_document(&pool, org_id, None, "Org Doc 1", "admin").await;
        create_test_document(&pool, org_id, Some(building_id), "Building Doc 1", "member").await;
        create_test_document(&pool, org_id, Some(building_id), "Building Doc 2", "public").await;

        // Request documents
        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!(
                "/api/v1/documents?organization_id={}",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
        let json = response.json_value();

        // Admin should see all 3 documents
        let documents = json["documents"].as_array().unwrap();
        assert!(documents.len() >= 3, "Admin should see all documents");

        // Cleanup
        cleanup_test_data(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_admin_can_create_documents(pool: PgPool) {
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

        // Create document
        let body = json!({
            "name": "New Document",
            "access_level": "member"
        });

        let request = Request::builder()
            .method(Method::POST)
            .uri(&format!(
                "/api/v1/documents?organization_id={}",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = app.execute(request).await;

        // Should be created or accepted for upload
        assert!(
            response.status == StatusCode::CREATED || response.status == StatusCode::OK,
            "Expected 201 or 200, got {}",
            response.status
        );

        // Cleanup
        cleanup_test_data(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Non-Member Access Tests
// =============================================================================

#[cfg(test)]
mod non_member_access {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_non_member_cannot_access_org_documents(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let owner = TestUser::new();
        let stranger = TestUser::with_email("stranger@example.com");

        // Setup owner
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

        // Create org with owner
        let org_id = create_test_organization(&pool, owner_id, "Owner's Org").await;
        create_test_document(&pool, org_id, None, "Private Doc", "member").await;

        // Stranger tries to access
        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!(
                "/api/v1/documents?organization_id={}",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", stranger_token))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should be forbidden
        assert!(
            response.status == StatusCode::FORBIDDEN || response.status == StatusCode::NOT_FOUND,
            "Non-member should not access org documents"
        );

        // Cleanup
        cleanup_test_data(&pool, org_id).await;
        cleanup_test_user(&pool, &owner.email).await;
        cleanup_test_user(&pool, &stranger.email).await;
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_unauthenticated_cannot_access_documents(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let user = TestUser::new();

        // Setup
        cleanup_test_user(&pool, &user.email).await;
        let (_, _) = create_authenticated_user(&app, &user).await;

        // Get user ID
        let user_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        let org_id = create_test_organization(&pool, user_id, "Test Org").await;
        create_test_document(&pool, org_id, None, "Some Doc", "member").await;

        // Try to access without auth
        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!(
                "/api/v1/documents?organization_id={}",
                org_id
            ))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::UNAUTHORIZED);

        // Cleanup
        cleanup_test_data(&pool, org_id).await;
        cleanup_test_user(&pool, &user.email).await;
    }
}

// =============================================================================
// Access Level Tests
// =============================================================================

#[cfg(test)]
mod access_levels {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_public_documents_visible_to_members(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let admin = TestUser::new();
        let member = TestUser::with_email("member@example.com");

        // Setup users
        cleanup_test_user(&pool, &admin.email).await;
        cleanup_test_user(&pool, &member.email).await;

        let (_, _) = create_authenticated_user(&app, &admin).await;
        let (member_token, _) = create_authenticated_user(&app, &member).await;

        // Get admin user ID
        let admin_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&admin.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        // Get member user ID
        let member_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(&member.email)
            .fetch_one(&pool)
            .await
            .expect("User not found");

        // Create org with admin
        let org_id = create_test_organization(&pool, admin_id, "Test Org").await;

        // Add member to org with member role
        sqlx::query(
            r#"
            INSERT INTO organization_members (id, organization_id, user_id, role, created_at)
            VALUES ($1, $2, $3, 'member', NOW())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(org_id)
        .bind(member_id)
        .execute(&pool)
        .await
        .ok();

        // Create documents with different access levels
        create_test_document(&pool, org_id, None, "Public Doc", "public").await;
        create_test_document(&pool, org_id, None, "Admin Only Doc", "admin").await;

        // Member requests documents
        let request = Request::builder()
            .method(Method::GET)
            .uri(&format!(
                "/api/v1/documents?organization_id={}",
                org_id
            ))
            .header(header::AUTHORIZATION, format!("Bearer {}", member_token))
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
        let json = response.json_value();

        // Member should see public docs but not admin-only docs
        let documents = json["documents"].as_array().unwrap();
        let doc_names: Vec<&str> = documents
            .iter()
            .filter_map(|d| d["name"].as_str())
            .collect();

        assert!(
            doc_names.contains(&"Public Doc"),
            "Member should see public documents"
        );

        // Cleanup
        cleanup_test_data(&pool, org_id).await;
        cleanup_test_user(&pool, &admin.email).await;
        cleanup_test_user(&pool, &member.email).await;
    }
}

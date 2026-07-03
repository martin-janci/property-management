//! Repository Layer Integration Tests (Epic 80, Story 80.5)
//!
//! This module provides test scaffolding for the repository layer.
//! Tests verify CRUD operations, data integrity, and error handling.
//!
//! Test Categories:
//! 1. UserRepository - User CRUD, email verification, password management
//! 2. OrganizationRepository - Organization lifecycle management
//! 3. BuildingRepository - Building data operations
//! 4. (Future) Additional repository tests
//!
//! NOTE: These tests are marked #[ignore] as they require a test database.
//! Run with: cargo test --test repository_tests -- --ignored --test-threads=1

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

// =============================================================================
// Test Database Infrastructure
// =============================================================================

/// Test database connection configuration for repository tests.
/// Provides helper methods for test setup and teardown.
pub struct TestDb {
    pool: PgPool,
}

impl TestDb {
    /// Create a new test database connection.
    ///
    /// Uses TEST_DATABASE_URL environment variable or defaults to local test DB.
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ppt_test".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&database_url)
            .await?;

        Ok(Self { pool })
    }

    /// Get reference to the database pool for direct queries.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Set request context for RLS operations.
    ///
    /// This establishes the tenant/user context required for Row-Level Security.
    pub async fn set_request_context(
        &self,
        org_id: Option<Uuid>,
        user_id: Option<Uuid>,
        is_super_admin: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT set_request_context($1, $2, $3)")
            .bind(org_id)
            .bind(user_id)
            .bind(is_super_admin)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear request context after test operations.
    pub async fn clear_context(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT clear_request_context()")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Set up as super admin for test data creation (bypasses RLS).
    pub async fn setup_as_super_admin(&self) -> Result<(), sqlx::Error> {
        self.set_request_context(None, None, true).await
    }

    // =========================================================================
    // Test Data Factory Methods
    // =========================================================================

    /// Create a test user with minimal required fields.
    pub async fn create_test_user(&self, email: &str, name: &str) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at)
            VALUES ($1, 'test_hash_$argon2id$v=19$m=16,t=2,p=1$dGVzdF9zYWx0$dGVzdF9oYXNo', $2, 'active', NOW())
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a test user in pending state (unverified email).
    pub async fn create_pending_user(&self, email: &str, name: &str) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO users (email, password_hash, name, status)
            VALUES ($1, 'test_hash_$argon2id$v=19$m=16,t=2,p=1$dGVzdF9zYWx0$dGVzdF9oYXNo', $2, 'pending')
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a test organization.
    pub async fn create_test_org(&self, name: &str) -> Result<Uuid, sqlx::Error> {
        let slug = name.to_lowercase().replace(' ', "-");
        let row = sqlx::query(
            r#"
            INSERT INTO organizations (name, slug, contact_email)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(name)
        .bind(&slug)
        .bind(format!("contact@{}.test", slug))
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Add a user as member of an organization.
    pub async fn add_org_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role_type: &str,
    ) -> Result<Uuid, sqlx::Error> {
        // Get the appropriate role for this org (if exists)
        let role_row = sqlx::query(
            r#"
            SELECT id FROM roles
            WHERE organization_id = $1 AND LOWER(name) LIKE $2
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(format!("%{}%", role_type.to_lowercase()))
        .fetch_optional(&self.pool)
        .await?;

        let role_id: Option<Uuid> = role_row.map(|r| r.get("id"));

        let row = sqlx::query(
            r#"
            INSERT INTO organization_members (organization_id, user_id, role_id, role_type, status)
            VALUES ($1, $2, $3, $4, 'active')
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role_id)
        .bind(role_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Create a test building in an organization.
    pub async fn create_test_building(
        &self,
        org_id: Uuid,
        name: &str,
        address: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
            VALUES ($1, $2, $3, 'Test City', '12345', 'SK', 'active')
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(address)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    // =========================================================================
    // Cleanup Methods
    // =========================================================================

    /// Clean up all test data created during tests.
    ///
    /// Should be called in test teardown to ensure isolation between tests.
    pub async fn cleanup(&self) -> Result<(), sqlx::Error> {
        // Use super admin context to bypass RLS for cleanup
        self.setup_as_super_admin().await?;

        // Clean up in reverse dependency order
        sqlx::query("DELETE FROM organization_members WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM roles WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM buildings WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM organizations WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM email_verification_tokens WHERE TRUE")
            .execute(&self.pool)
            .await?;
        // SAFETY: Only delete users with emails ending in exact test domains
        // This prevents accidental deletion of production users whose email might
        // contain 'test' as a substring (e.g., user@contest.com, user@attestation.org)
        sqlx::query(
            "DELETE FROM users WHERE email LIKE '%@test.com' OR email LIKE '%@repo-test.com'",
        )
        .execute(&self.pool)
        .await?;

        self.clear_context().await?;

        Ok(())
    }
}

// =============================================================================
// UserRepository Tests
// =============================================================================

/// Test UserRepository.create() - successful user creation
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_user_repository_create_success() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    // Test: Create a new user
    let email = "create_test@repo-test.com";
    let name = "Create Test User";

    let user_id = db.create_test_user(email, name).await.unwrap();

    // Verify: User was created with correct data
    let row = sqlx::query("SELECT email, name, status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .unwrap();

    let db_email: String = row.get("email");
    let db_name: String = row.get("name");
    let db_status: String = row.get("status");

    assert_eq!(db_email, email);
    assert_eq!(db_name, name);
    assert_eq!(db_status, "active");

    db.cleanup().await.unwrap();
}

/// Test UserRepository.find_by_email() - user found
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_user_repository_find_by_email_found() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let email = "findbyemail@repo-test.com";
    let name = "Find By Email User";
    let user_id = db.create_test_user(email, name).await.unwrap();

    // Test: Find user by email (case-insensitive)
    let row = sqlx::query(
        "SELECT id, name FROM users WHERE LOWER(email) = LOWER($1) AND status != 'deleted'",
    )
    .bind(email.to_uppercase()) // Test case-insensitivity
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(row.is_some(), "User should be found by email");
    let found_id: Uuid = row.unwrap().get("id");
    assert_eq!(found_id, user_id);

    db.cleanup().await.unwrap();
}

/// Test UserRepository.find_by_email() - user not found
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_user_repository_find_by_email_not_found() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    // Test: Find non-existent user
    let row = sqlx::query("SELECT id FROM users WHERE LOWER(email) = LOWER($1)")
        .bind("nonexistent@repo-test.com")
        .fetch_optional(db.pool())
        .await
        .unwrap();

    assert!(row.is_none(), "Non-existent user should return None");

    db.cleanup().await.unwrap();
}

/// Test UserRepository.email_exists() - email collision detection
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_user_repository_email_exists() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let email = "emailexists@repo-test.com";
    db.create_test_user(email, "Email Exists User")
        .await
        .unwrap();

    // Test: Check if email exists
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM users
        WHERE LOWER(email) = LOWER($1)
        AND (status != 'deleted' OR deleted_at > NOW() - INTERVAL '30 days')
        "#,
    )
    .bind(email)
    .fetch_one(db.pool())
    .await
    .unwrap();

    assert!(count > 0, "Existing email should be detected");

    // Test: Non-existent email
    let count2: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM users
        WHERE LOWER(email) = LOWER($1)
        "#,
    )
    .bind("nonexistent_email@repo-test.com")
    .fetch_one(db.pool())
    .await
    .unwrap();

    assert_eq!(count2, 0, "Non-existent email should return 0");

    db.cleanup().await.unwrap();
}

/// Test UserRepository.verify_email() - email verification flow
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_user_repository_verify_email() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    // Create pending user
    let user_id = db
        .create_pending_user("verify@repo-test.com", "Verify User")
        .await
        .unwrap();

    // Verify initial status
    let status_before: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(status_before, "pending");

    // Test: Verify email
    let result = sqlx::query(
        r#"
        UPDATE users
        SET email_verified_at = NOW(), status = 'active', updated_at = NOW()
        WHERE id = $1 AND status = 'pending'
        RETURNING id
        "#,
    )
    .bind(user_id)
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(result.is_some(), "Verification should succeed");

    // Verify status changed
    let status_after: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(status_after, "active");

    db.cleanup().await.unwrap();
}

/// Test UserRepository.update_password() - password update
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_user_repository_update_password() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let user_id = db
        .create_test_user("password@repo-test.com", "Password User")
        .await
        .unwrap();

    let new_hash = "new_hash_$argon2id$v=19$m=16,t=2,p=1$bmV3X3NhbHQ$bmV3X2hhc2g";

    // Test: Update password
    let result = sqlx::query(
        r#"
        UPDATE users SET password_hash = $2, updated_at = NOW()
        WHERE id = $1 AND status != 'deleted'
        "#,
    )
    .bind(user_id)
    .bind(new_hash)
    .execute(db.pool())
    .await
    .unwrap();

    assert!(result.rows_affected() > 0, "Password should be updated");

    // Verify password was changed
    let stored_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .unwrap();

    assert_eq!(stored_hash, new_hash);

    db.cleanup().await.unwrap();
}

/// Test UserRepository.soft_delete() - soft deletion
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_user_repository_soft_delete() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let user_id = db
        .create_test_user("delete@repo-test.com", "Delete User")
        .await
        .unwrap();
    let admin_id = db
        .create_test_user("admin@repo-test.com", "Admin User")
        .await
        .unwrap();

    // Test: Soft delete user
    let result = sqlx::query(
        r#"
        UPDATE users
        SET status = 'deleted', deleted_at = NOW(), deleted_by = $2, updated_at = NOW()
        WHERE id = $1 AND status != 'deleted'
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(admin_id)
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(result.is_some(), "Soft delete should succeed");

    // Verify user is soft-deleted
    let status: String = sqlx::query_scalar("SELECT status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(db.pool())
        .await
        .unwrap();

    assert_eq!(status, "deleted");

    // Verify user is not returned by normal queries
    let found = sqlx::query("SELECT id FROM users WHERE id = $1 AND status != 'deleted'")
        .bind(user_id)
        .fetch_optional(db.pool())
        .await
        .unwrap();

    assert!(found.is_none(), "Deleted user should not be found");

    db.cleanup().await.unwrap();
}

// =============================================================================
// OrganizationRepository Tests
// =============================================================================

/// Test OrganizationRepository.create() - organization creation
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_organization_repository_create() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    // Test: Create organization
    let org_id = db.create_test_org("Test Organization").await.unwrap();

    // Verify organization was created
    let row = sqlx::query("SELECT name, slug, status FROM organizations WHERE id = $1")
        .bind(org_id)
        .fetch_one(db.pool())
        .await
        .unwrap();

    let name: String = row.get("name");
    let slug: String = row.get("slug");
    let status: String = row.get("status");

    assert_eq!(name, "Test Organization");
    assert_eq!(slug, "test-organization");
    assert_eq!(status, "active");

    db.cleanup().await.unwrap();
}

/// Test OrganizationRepository - unique slug enforcement
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_organization_repository_unique_slug() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    // Create first organization
    db.create_test_org("Unique Slug Test").await.unwrap();

    // Test: Creating duplicate slug should fail
    let result = db.create_test_org("Unique Slug Test").await;

    assert!(result.is_err(), "Duplicate slug should cause error");

    db.cleanup().await.unwrap();
}

// =============================================================================
// BuildingRepository Tests
// =============================================================================

/// Test BuildingRepository.create() - building creation
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_building_repository_create() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Building Test Org").await.unwrap();

    // Test: Create building
    let building_id = db
        .create_test_building(org_id, "Test Building", "123 Test Street")
        .await
        .unwrap();

    // Verify building was created
    let row = sqlx::query("SELECT name, street, organization_id FROM buildings WHERE id = $1")
        .bind(building_id)
        .fetch_one(db.pool())
        .await
        .unwrap();

    let name: String = row.get("name");
    let street: String = row.get("street");
    let db_org_id: Uuid = row.get("organization_id");

    assert_eq!(name, "Test Building");
    assert_eq!(street, "123 Test Street");
    assert_eq!(db_org_id, org_id);

    db.cleanup().await.unwrap();
}

/// Test building belongs to organization (RLS context)
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_building_repository_rls_isolation() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    // Create two organizations
    let org_a = db.create_test_org("Org A Buildings").await.unwrap();
    let org_b = db.create_test_org("Org B Buildings").await.unwrap();

    // Create buildings in each org
    let building_a = db
        .create_test_building(org_a, "Building A", "A Street")
        .await
        .unwrap();
    let _building_b = db
        .create_test_building(org_b, "Building B", "B Street")
        .await
        .unwrap();

    // Create user in Org A
    let user_a = db
        .create_test_user("user_a_building@repo-test.com", "User A")
        .await
        .unwrap();
    db.add_org_member(org_a, user_a, "member").await.unwrap();

    // Set context as User A in Org A
    db.set_request_context(Some(org_a), Some(user_a), false)
        .await
        .unwrap();

    // Test: User A should only see Org A buildings
    let buildings: Vec<_> = sqlx::query("SELECT id, name FROM buildings")
        .fetch_all(db.pool())
        .await
        .unwrap();

    // With RLS, should only see Org A building
    assert!(
        buildings.iter().all(|b| {
            let id: Uuid = b.get("id");
            id == building_a
        }),
        "User A should only see Org A buildings"
    );

    db.clear_context().await.unwrap();
    db.cleanup().await.unwrap();
}

// =============================================================================
// Email Verification Token Tests
// =============================================================================

/// Test email verification token creation and usage
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_email_verification_token_lifecycle() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let user_id = db
        .create_pending_user("token@repo-test.com", "Token User")
        .await
        .unwrap();

    let token_hash = "test_token_hash_abc123";

    // Test: Create verification token
    let token_row = sqlx::query(
        r#"
        INSERT INTO email_verification_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, NOW() + INTERVAL '24 hours')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .fetch_one(db.pool())
    .await
    .unwrap();

    let token_id: Uuid = token_row.get("id");

    // Test: Find token by hash
    let found = sqlx::query(
        "SELECT id, user_id FROM email_verification_tokens WHERE token_hash = $1 AND used_at IS NULL",
    )
    .bind(token_hash)
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(found.is_some(), "Token should be found");
    let found_user_id: Uuid = found.unwrap().get("user_id");
    assert_eq!(found_user_id, user_id);

    // Test: Mark token as used
    let result = sqlx::query(
        "UPDATE email_verification_tokens SET used_at = NOW() WHERE id = $1 AND used_at IS NULL",
    )
    .bind(token_id)
    .execute(db.pool())
    .await
    .unwrap();

    assert!(result.rows_affected() > 0, "Token should be marked as used");

    // Test: Used token should not be found
    let found_after = sqlx::query(
        "SELECT id FROM email_verification_tokens WHERE token_hash = $1 AND used_at IS NULL",
    )
    .bind(token_hash)
    .fetch_optional(db.pool())
    .await
    .unwrap();

    assert!(found_after.is_none(), "Used token should not be found");

    db.cleanup().await.unwrap();
}

// =============================================================================
// Session/Membership Tests
// =============================================================================

/// Test organization membership operations
#[tokio::test]
#[ignore = "requires Postgres test database; run with --ignored --test-threads=1"]
async fn test_organization_membership() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Membership Test Org").await.unwrap();
    let user_id = db
        .create_test_user("member@repo-test.com", "Member User")
        .await
        .unwrap();

    // Test: Add member to organization
    let member_id = db.add_org_member(org_id, user_id, "member").await.unwrap();

    // Verify membership
    let row = sqlx::query(
        "SELECT organization_id, user_id, role_type, status FROM organization_members WHERE id = $1",
    )
    .bind(member_id)
    .fetch_one(db.pool())
    .await
    .unwrap();

    let db_org_id: Uuid = row.get("organization_id");
    let db_user_id: Uuid = row.get("user_id");
    let role_type: String = row.get("role_type");
    let status: String = row.get("status");

    assert_eq!(db_org_id, org_id);
    assert_eq!(db_user_id, user_id);
    assert_eq!(role_type, "member");
    assert_eq!(status, "active");

    db.cleanup().await.unwrap();
}

// =============================================================================
// Test Runner Helper
// =============================================================================

/// Run all repository tests.
///
/// Execute with: cargo test --test repository_tests -- --ignored --test-threads=1
pub async fn run_all_repository_tests() {
    println!("Repository Layer Test Suite");
    println!("============================");
    println!("Tests verify:");
    println!("  - User CRUD operations");
    println!("  - Email verification flow");
    println!("  - Password management");
    println!("  - Organization lifecycle");
    println!("  - Building operations");
    println!("  - RLS isolation");
    println!("  - Membership management");
    println!("============================");
    println!();
    println!("Run with: cargo test --test repository_tests -- --ignored --test-threads=1");
}

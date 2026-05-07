//! RLS Smoke Tests - Fast tenant isolation verification for CI
//!
//! These tests run on every PR to catch basic RLS regressions quickly.
//! For comprehensive testing, see `rls_penetration_tests.rs` (runs weekly).
//!
//! Schema mapping (kept here so the test stays in sync with migration drift):
//! - `buildings`     → columns are `street`, `city`, `postal_code`, `country`
//!   (NOT `address_line1`; that name was used by an earlier migration that
//!   was renamed in `00007_create_buildings.sql`).
//! - `organization_members` → role-as-string lives in `role_type` (default
//!   `'member'`); `role_id` is a separate UUID FK to the `roles` table for
//!   custom-role lookups (we don't need that here).

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

struct TestDb {
    pool: PgPool,
}

impl TestDb {
    async fn new() -> Result<Self, sqlx::Error> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ppt_test".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&database_url)
            .await?;

        // RLS is enforced for non-owner non-superuser roles WITHOUT needing
        // `FORCE ROW LEVEL SECURITY`. The CI workflow creates a dedicated
        // `rls_test_runner` role (non-superuser, granted only the privileges
        // it needs) and `TEST_DATABASE_URL` connects as that role.
        //
        // If you run this test locally and `TEST_DATABASE_URL` points at a
        // superuser, RLS will be silently bypassed and the assertions will
        // all fail — that's a configuration issue, not a test bug. See
        // .github/workflows/backend.yml's "Create non-superuser test role"
        // step for the exact role/grants.

        Ok(Self { pool })
    }

    async fn set_request_context(
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

    async fn clear_context(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT clear_request_context()")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_test_org(&self, name: &str) -> Result<Uuid, sqlx::Error> {
        let slug = format!(
            "{}-{}",
            name.to_lowercase().replace(' ', "-"),
            Uuid::new_v4()
        );
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

    async fn create_test_user(&self, email: &str, name: &str) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at)
            VALUES ($1, 'test_hash', $2, 'active', NOW())
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    async fn cleanup(&self) {
        // FORCE RLS is on, so DELETEs need super-admin context to bypass the
        // per-row policy. Without this, cleanup is a silent no-op and leftover
        // rows from a previous run leak into subsequent tests (which then see
        // unexpected building counts and assert-fail).
        let _ = self.set_request_context(None, None, true).await;
        // Clean up in reverse order of dependencies.
        let _ = sqlx::query("DELETE FROM buildings WHERE name LIKE 'Smoke%'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM organization_members WHERE user_id IN (SELECT id FROM users WHERE email LIKE 'smoke%@test.com')")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM users WHERE email LIKE 'smoke%@test.com'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM organizations WHERE name LIKE 'Smoke%'")
            .execute(&self.pool)
            .await;
        let _ = self.clear_context().await;
    }
}

/// Core smoke test: Verify tenant A cannot see tenant B's buildings
#[tokio::test]
#[ignore] // Requires database with migrations - run with --ignored flag
async fn smoke_test_cross_tenant_isolation() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    // Create two tenants
    let org_a = db
        .create_test_org("Smoke Org A")
        .await
        .expect("Failed to create org A");
    let org_b = db
        .create_test_org("Smoke Org B")
        .await
        .expect("Failed to create org B");

    // Create users for each tenant
    let user_a = db
        .create_test_user("smoke_user_a@test.com", "User A")
        .await
        .expect("Failed to create user A");
    let user_b = db
        .create_test_user("smoke_user_b@test.com", "User B")
        .await
        .expect("Failed to create user B");

    // organization_members has RLS with a WITH CHECK predicate
    // (`organization_id = get_current_org_id() OR is_super_admin()`).
    // Setup needs super-admin context to bypass it before flipping back to a
    // per-tenant context for the actual isolation assertions.
    db.set_request_context(None, None, true)
        .await
        .expect("Failed to set super-admin context for setup");

    // Add users to their respective orgs
    sqlx::query("INSERT INTO organization_members (organization_id, user_id, role_type) VALUES ($1, $2, 'manager')")
        .bind(org_a)
        .bind(user_a)
        .execute(&db.pool)
        .await
        .expect("Failed to add user A to org A");

    sqlx::query("INSERT INTO organization_members (organization_id, user_id, role_type) VALUES ($1, $2, 'manager')")
        .bind(org_b)
        .bind(user_b)
        .execute(&db.pool)
        .await
        .expect("Failed to add user B to org B");

    // Create a building for each org (bypassing RLS with super admin)
    db.set_request_context(Some(org_a), Some(user_a), true)
        .await
        .expect("Failed to set context");
    sqlx::query("INSERT INTO buildings (organization_id, name, street, city, postal_code, country) VALUES ($1, 'Smoke Building A', 'Addr A', 'City A', '00000', 'Country A')")
        .bind(org_a)
        .execute(&db.pool)
        .await
        .expect("Failed to create building A");

    db.set_request_context(Some(org_b), Some(user_b), true)
        .await
        .expect("Failed to set context");
    sqlx::query("INSERT INTO buildings (organization_id, name, street, city, postal_code, country) VALUES ($1, 'Smoke Building B', 'Addr B', 'City B', '00000', 'Country B')")
        .bind(org_b)
        .execute(&db.pool)
        .await
        .expect("Failed to create building B");

    // CRITICAL TEST: User A with context for Org A should NOT see Org B's building
    db.set_request_context(Some(org_a), Some(user_a), false)
        .await
        .expect("Failed to set context for user A");

    let visible_buildings: Vec<String> =
        sqlx::query_scalar("SELECT name FROM buildings WHERE name LIKE 'Smoke%'")
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query buildings");

    // User A should only see Building A
    assert_eq!(
        visible_buildings.len(),
        1,
        "User A should only see 1 building, but saw: {:?}",
        visible_buildings
    );
    assert!(
        visible_buildings.contains(&"Smoke Building A".to_string()),
        "User A should see Building A"
    );
    assert!(
        !visible_buildings.contains(&"Smoke Building B".to_string()),
        "User A should NOT see Building B"
    );

    // Verify User B sees only their building too
    db.set_request_context(Some(org_b), Some(user_b), false)
        .await
        .expect("Failed to set context for user B");

    let visible_buildings_b: Vec<String> =
        sqlx::query_scalar("SELECT name FROM buildings WHERE name LIKE 'Smoke%'")
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query buildings for B");

    assert_eq!(
        visible_buildings_b.len(),
        1,
        "User B should only see 1 building"
    );
    assert!(
        visible_buildings_b.contains(&"Smoke Building B".to_string()),
        "User B should see Building B"
    );

    db.clear_context().await.expect("Failed to clear context");
    db.cleanup().await;

    println!("✓ Cross-tenant isolation verified");
}

/// Smoke test: Verify no context means no data access
#[tokio::test]
#[ignore] // Requires database with migrations - run with --ignored flag
async fn smoke_test_null_context_blocks_access() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    // Create a tenant and building
    let org = db
        .create_test_org("Smoke Null Context Org")
        .await
        .expect("Failed to create org");
    let user = db
        .create_test_user("smoke_null@test.com", "Null User")
        .await
        .expect("Failed to create user");

    // Create building as super admin
    db.set_request_context(Some(org), Some(user), true)
        .await
        .expect("Failed to set context");
    sqlx::query("INSERT INTO buildings (organization_id, name, street, city, postal_code, country) VALUES ($1, 'Smoke Null Building', 'Addr', 'City', '00000', 'Country')")
        .bind(org)
        .execute(&db.pool)
        .await
        .expect("Failed to create building");

    // Clear context - should see nothing
    db.clear_context().await.expect("Failed to clear context");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM buildings WHERE name LIKE 'Smoke%'")
        .fetch_one(&db.pool)
        .await
        .expect("Failed to count buildings");

    assert_eq!(
        count, 0,
        "With null context, should see 0 buildings, but saw {}",
        count
    );

    db.cleanup().await;

    println!("✓ Null context blocks access verified");
}

/// Smoke test: Verify context clearing works
#[tokio::test]
#[ignore] // Requires database with migrations - run with --ignored flag
async fn smoke_test_context_clearing() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    let org = db
        .create_test_org("Smoke Context Clear Org")
        .await
        .expect("Failed to create org");
    let user = db
        .create_test_user("smoke_clear@test.com", "Clear User")
        .await
        .expect("Failed to create user");

    // Create building as super admin
    db.set_request_context(Some(org), Some(user), true)
        .await
        .expect("Failed to set context");
    sqlx::query("INSERT INTO buildings (organization_id, name, street, city, postal_code, country) VALUES ($1, 'Smoke Clear Building', 'Addr', 'City', '00000', 'Country')")
        .bind(org)
        .execute(&db.pool)
        .await
        .expect("Failed to create building");

    // Set context - should see building
    db.set_request_context(Some(org), Some(user), false)
        .await
        .expect("Failed to set context");
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM buildings WHERE name LIKE 'Smoke%'")
            .fetch_one(&db.pool)
            .await
            .expect("Failed to count");
    assert_eq!(count_before, 1, "With context, should see 1 building");

    // Clear context - should see nothing
    db.clear_context().await.expect("Failed to clear context");
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM buildings WHERE name LIKE 'Smoke%'")
            .fetch_one(&db.pool)
            .await
            .expect("Failed to count");
    assert_eq!(
        count_after, 0,
        "After clearing context, should see 0 buildings"
    );

    db.cleanup().await;

    println!("✓ Context clearing verified");
}

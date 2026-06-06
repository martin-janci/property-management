//! Announcement Integration Tests (Epic 6, Story 6.1)
//!
//! Tests for the announcements domain:
//! - CRUD operations
//! - Status transitions (draft -> scheduled -> published -> archived)
//! - Targeting (all, building, units, roles)
//! - Read tracking and acknowledgment
//! - RLS isolation

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

/// Test database connection configuration
struct TestDb {
    pool: PgPool,
}

impl TestDb {
    async fn new() -> Result<Self, sqlx::Error> {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ppt_test".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&database_url)
            .await?;

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

    async fn create_test_announcement(
        &self,
        org_id: Uuid,
        author_id: Uuid,
        title: &str,
        content: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO announcements (organization_id, author_id, title, content, target_type, target_ids, status)
            VALUES ($1, $2, $3, $4, 'all', '[]', 'draft')
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(author_id)
        .bind(title)
        .bind(content)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    async fn cleanup(&self) -> Result<(), sqlx::Error> {
        self.set_request_context(None, None, true).await?;

        // Clean up in correct order to respect foreign keys
        sqlx::query("DELETE FROM announcement_reads WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM announcement_attachments WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM announcements WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM organization_members WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM roles WHERE TRUE")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM organizations WHERE name LIKE '%Test%'")
            .execute(&self.pool)
            .await?;
        // SAFETY: Only delete users with emails ending in @test.com (exact test domain)
        // This prevents accidental deletion of production users whose email might
        // contain 'test' as a substring (e.g., user@contest.com, user@attestation.org)
        sqlx::query("DELETE FROM users WHERE email LIKE '%@test.com'")
            .execute(&self.pool)
            .await?;

        self.clear_context().await?;
        Ok(())
    }

    async fn setup_as_super_admin(&self) -> Result<(), sqlx::Error> {
        self.set_request_context(None, None, true).await
    }
}

// ==================== Basic CRUD Tests ====================

/// Test creating an announcement
#[tokio::test]
#[ignore]
async fn test_create_announcement() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Announcement Test Org").await.unwrap();
    let user_id = db
        .create_test_user("author@test.com", "Test Author")
        .await
        .unwrap();

    let announcement_id = db
        .create_test_announcement(org_id, user_id, "Test Title", "Test Content")
        .await
        .unwrap();

    // Verify announcement was created
    let row = sqlx::query("SELECT * FROM announcements WHERE id = $1")
        .bind(announcement_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

    let title: String = row.get("title");
    let status: String = row.get("status");

    assert_eq!(title, "Test Title");
    assert_eq!(status, "draft");

    db.cleanup().await.unwrap();
}

/// Test announcement status transitions
#[tokio::test]
#[ignore]
async fn test_announcement_status_transitions() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Status Test Org").await.unwrap();
    let user_id = db
        .create_test_user("status@test.com", "Status Author")
        .await
        .unwrap();

    let announcement_id = db
        .create_test_announcement(org_id, user_id, "Status Test", "Content")
        .await
        .unwrap();

    // Transition: draft -> published
    sqlx::query(
        "UPDATE announcements SET status = 'published', published_at = NOW() WHERE id = $1",
    )
    .bind(announcement_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let row = sqlx::query("SELECT status FROM announcements WHERE id = $1")
        .bind(announcement_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

    let status: String = row.get("status");
    assert_eq!(status, "published");

    // Transition: published -> archived
    sqlx::query("UPDATE announcements SET status = 'archived' WHERE id = $1")
        .bind(announcement_id)
        .execute(&db.pool)
        .await
        .unwrap();

    let row = sqlx::query("SELECT status FROM announcements WHERE id = $1")
        .bind(announcement_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

    let status: String = row.get("status");
    assert_eq!(status, "archived");

    db.cleanup().await.unwrap();
}

/// Test scheduled publishing
#[tokio::test]
#[ignore]
async fn test_scheduled_announcement() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Schedule Test Org").await.unwrap();
    let user_id = db
        .create_test_user("schedule@test.com", "Schedule Author")
        .await
        .unwrap();

    let announcement_id = db
        .create_test_announcement(org_id, user_id, "Scheduled Test", "Content")
        .await
        .unwrap();

    // Set scheduled time in the past to simulate publishing
    let past_time = chrono::Utc::now() - chrono::Duration::hours(1);
    sqlx::query("UPDATE announcements SET status = 'scheduled', scheduled_at = $1 WHERE id = $2")
        .bind(past_time)
        .bind(announcement_id)
        .execute(&db.pool)
        .await
        .unwrap();

    // Query for announcements that should be published
    let due_announcements: Vec<_> = sqlx::query(
        "SELECT * FROM announcements WHERE status = 'scheduled' AND scheduled_at <= NOW()",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();

    assert_eq!(
        due_announcements.len(),
        1,
        "Should find one due announcement"
    );

    db.cleanup().await.unwrap();
}

// ==================== Read Tracking Tests ====================

/// Test marking announcement as read
#[tokio::test]
#[ignore]
async fn test_mark_announcement_read() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Read Test Org").await.unwrap();
    let author_id = db
        .create_test_user("read_author@test.com", "Read Author")
        .await
        .unwrap();
    let reader_id = db
        .create_test_user("reader@test.com", "Reader")
        .await
        .unwrap();

    let announcement_id = db
        .create_test_announcement(org_id, author_id, "Read Test", "Content")
        .await
        .unwrap();

    // Mark as read
    sqlx::query(
        r#"
        INSERT INTO announcement_reads (announcement_id, user_id, read_at)
        VALUES ($1, $2, NOW())
        "#,
    )
    .bind(announcement_id)
    .bind(reader_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Verify read record exists
    let read_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = $1 AND user_id = $2",
    )
    .bind(announcement_id)
    .bind(reader_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(read_count, 1);

    db.cleanup().await.unwrap();
}

/// Test acknowledging announcement
#[tokio::test]
#[ignore]
async fn test_acknowledge_announcement() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Ack Test Org").await.unwrap();
    let author_id = db
        .create_test_user("ack_author@test.com", "Ack Author")
        .await
        .unwrap();
    let reader_id = db
        .create_test_user("acknowledger@test.com", "Acknowledger")
        .await
        .unwrap();

    let announcement_id = db
        .create_test_announcement(org_id, author_id, "Ack Test", "Content")
        .await
        .unwrap();

    // Set acknowledgment required
    sqlx::query("UPDATE announcements SET acknowledgment_required = true WHERE id = $1")
        .bind(announcement_id)
        .execute(&db.pool)
        .await
        .unwrap();

    // Mark as read and acknowledged
    sqlx::query(
        r#"
        INSERT INTO announcement_reads (announcement_id, user_id, read_at, acknowledged_at)
        VALUES ($1, $2, NOW(), NOW())
        "#,
    )
    .bind(announcement_id)
    .bind(reader_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Verify acknowledgment
    let ack_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM announcement_reads WHERE announcement_id = $1 AND acknowledged_at IS NOT NULL",
    )
    .bind(announcement_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(ack_count, 1);

    db.cleanup().await.unwrap();
}

// ==================== Attachment Tests ====================

/// Test adding attachment to announcement
#[tokio::test]
#[ignore]
async fn test_announcement_attachments() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Attach Test Org").await.unwrap();
    let user_id = db
        .create_test_user("attach@test.com", "Attach Author")
        .await
        .unwrap();

    let announcement_id = db
        .create_test_announcement(org_id, user_id, "Attachment Test", "Content")
        .await
        .unwrap();

    // Add attachment
    sqlx::query(
        r#"
        INSERT INTO announcement_attachments (announcement_id, file_key, file_name, file_type, file_size)
        VALUES ($1, 'test-key-123', 'document.pdf', 'application/pdf', 1024)
        "#,
    )
    .bind(announcement_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Verify attachment
    let attachments: Vec<_> =
        sqlx::query("SELECT * FROM announcement_attachments WHERE announcement_id = $1")
            .bind(announcement_id)
            .fetch_all(&db.pool)
            .await
            .unwrap();

    assert_eq!(attachments.len(), 1);

    let file_name: String = attachments[0].get("file_name");
    assert_eq!(file_name, "document.pdf");

    db.cleanup().await.unwrap();
}

// ==================== RLS Isolation Tests ====================

/// Test that announcements are isolated by organization
#[tokio::test]
#[ignore]
async fn test_announcement_rls_isolation() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    // Create two organizations
    let org_a_id = db.create_test_org("Announcement Org A").await.unwrap();
    let org_b_id = db.create_test_org("Announcement Org B").await.unwrap();

    let user_a_id = db
        .create_test_user("ann_user_a@test.com", "User A")
        .await
        .unwrap();
    let user_b_id = db
        .create_test_user("ann_user_b@test.com", "User B")
        .await
        .unwrap();

    // Create announcements in each org
    let ann_a_id = db
        .create_test_announcement(org_a_id, user_a_id, "Org A Announcement", "Content A")
        .await
        .unwrap();
    let ann_b_id = db
        .create_test_announcement(org_b_id, user_b_id, "Org B Announcement", "Content B")
        .await
        .unwrap();

    // Acquire a connection for RLS testing
    let mut conn = db.pool.acquire().await.unwrap();

    // Set context as User A in Org A
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_a_id)
        .bind(user_a_id)
        .bind(false)
        .execute(&mut *conn)
        .await
        .unwrap();

    // User A should only see Org A announcements
    let visible_announcements: Vec<_> = sqlx::query("SELECT id FROM announcements")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    let visible_ids: Vec<Uuid> = visible_announcements.iter().map(|r| r.get("id")).collect();

    assert!(
        visible_ids.contains(&ann_a_id),
        "User A should see Org A announcement"
    );
    assert!(
        !visible_ids.contains(&ann_b_id),
        "User A should NOT see Org B announcement"
    );

    drop(conn);
    db.clear_context().await.unwrap();
    db.cleanup().await.unwrap();
}

// ==================== Targeting Tests ====================

/// Test announcement targeting validation
#[tokio::test]
#[ignore]
async fn test_announcement_targeting() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Target Test Org").await.unwrap();
    let user_id = db
        .create_test_user("target@test.com", "Target Author")
        .await
        .unwrap();

    // Create announcement targeting specific building
    let building_id = Uuid::new_v4();
    let target_ids = serde_json::json!([building_id.to_string()]);

    let row = sqlx::query(
        r#"
        INSERT INTO announcements (organization_id, author_id, title, content, target_type, target_ids, status)
        VALUES ($1, $2, 'Building Announcement', 'Content', 'building', $3, 'draft')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(&target_ids)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let announcement_id: Uuid = row.get("id");

    // Verify targeting
    let ann = sqlx::query("SELECT target_type, target_ids FROM announcements WHERE id = $1")
        .bind(announcement_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

    let target_type: String = ann.get("target_type");
    assert_eq!(target_type, "building");

    db.cleanup().await.unwrap();
}

/// Test pinning announcement
#[tokio::test]
#[ignore]
async fn test_pin_announcement() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Pin Test Org").await.unwrap();
    let user_id = db
        .create_test_user("pin@test.com", "Pin Author")
        .await
        .unwrap();

    let announcement_id = db
        .create_test_announcement(org_id, user_id, "Pin Test", "Content")
        .await
        .unwrap();

    // Pin the announcement
    sqlx::query(
        "UPDATE announcements SET pinned = true, pinned_at = NOW(), pinned_by = $1 WHERE id = $2",
    )
    .bind(user_id)
    .bind(announcement_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Verify pinned state
    let ann = sqlx::query("SELECT pinned FROM announcements WHERE id = $1")
        .bind(announcement_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();

    let pinned: bool = ann.get("pinned");
    assert!(pinned);

    db.cleanup().await.unwrap();
}

/// Issue #972.7: announcements pinned longer than the cutoff are auto-unpinned
/// by `auto_unpin_expired`, while recently-pinned ones are left untouched.
#[tokio::test]
#[ignore]
async fn test_auto_unpin_expired() {
    use db::repositories::AnnouncementRepository;

    let db = TestDb::new().await.expect("Failed to connect to test DB");
    db.cleanup().await.unwrap();
    db.setup_as_super_admin().await.unwrap();

    let org_id = db.create_test_org("Auto Unpin Org").await.unwrap();
    let user_id = db
        .create_test_user("unpin@test.com", "Unpin Author")
        .await
        .unwrap();

    // Stale pin: pinned 40 days ago — should be unpinned with a 30-day cutoff.
    let stale_id = db
        .create_test_announcement(org_id, user_id, "Stale Pin", "Content")
        .await
        .unwrap();
    sqlx::query(
        "UPDATE announcements SET pinned = true, pinned_at = NOW() - INTERVAL '40 days', pinned_by = $1 WHERE id = $2",
    )
    .bind(user_id)
    .bind(stale_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Fresh pin: pinned 1 day ago — should remain pinned.
    let fresh_id = db
        .create_test_announcement(org_id, user_id, "Fresh Pin", "Content")
        .await
        .unwrap();
    sqlx::query(
        "UPDATE announcements SET pinned = true, pinned_at = NOW() - INTERVAL '1 day', pinned_by = $1 WHERE id = $2",
    )
    .bind(user_id)
    .bind(fresh_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let repo = AnnouncementRepository::new(db.pool.clone());
    let unpinned = repo
        .auto_unpin_expired(chrono::Duration::days(30))
        .await
        .unwrap();

    assert_eq!(unpinned, vec![stale_id], "only the stale pin should unpin");

    // Stale announcement is fully unpinned (pinned flag + audit columns cleared).
    let stale = sqlx::query("SELECT pinned, pinned_at, pinned_by FROM announcements WHERE id = $1")
        .bind(stale_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    let stale_pinned: bool = stale.get("pinned");
    let stale_pinned_at: Option<chrono::DateTime<chrono::Utc>> = stale.get("pinned_at");
    let stale_pinned_by: Option<Uuid> = stale.get("pinned_by");
    assert!(!stale_pinned, "stale announcement should be unpinned");
    assert!(stale_pinned_at.is_none(), "pinned_at should be cleared");
    assert!(stale_pinned_by.is_none(), "pinned_by should be cleared");

    // Fresh announcement is untouched.
    let fresh_pinned: bool = sqlx::query("SELECT pinned FROM announcements WHERE id = $1")
        .bind(fresh_id)
        .fetch_one(&db.pool)
        .await
        .unwrap()
        .get("pinned");
    assert!(fresh_pinned, "fresh announcement should stay pinned");

    db.cleanup().await.unwrap();
}

// ==================== RLS Table Coverage Test ====================

/// Verify announcements tables have RLS enabled
#[tokio::test]
#[ignore]
async fn test_announcement_rls_coverage() {
    let db = TestDb::new().await.expect("Failed to connect to test DB");

    let announcement_tables = vec![
        "announcements",
        "announcement_attachments",
        "announcement_reads",
    ];

    for table_name in &announcement_tables {
        // Check if table has RLS enabled
        let rls_enabled: bool = sqlx::query_scalar(
            r#"
            SELECT rowsecurity
            FROM pg_tables
            WHERE schemaname = 'public' AND tablename = $1
            "#,
        )
        .bind(table_name)
        .fetch_one(&db.pool)
        .await
        .unwrap_or(false);

        assert!(rls_enabled, "Table {} should have RLS enabled", table_name);

        // Check if table has at least one policy
        let policy_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM pg_policies
            WHERE schemaname = 'public' AND tablename = $1
            "#,
        )
        .bind(table_name)
        .fetch_one(&db.pool)
        .await
        .unwrap_or(0);

        assert!(
            policy_count > 0,
            "Table {} should have at least one RLS policy",
            table_name
        );
    }
}

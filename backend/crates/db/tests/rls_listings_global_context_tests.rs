#![allow(clippy::doc_overindented_list_items)]

//! Phase 4: 4-context RLS policy on `listings`.
//!
//! This file is the canonical verification of invariant **I-D** ("a listing
//! exists once; agency view and global view are filters of the same row").
//!
//! Migration `00136_listings_global_read_policy.sql` replaces the old
//! `listings_tenant_isolation` policy with a 4-branch one:
//!
//!   1. super-admin        — `is_super_admin()` bypass.
//!   2. agency-staff       — `organization_id = get_current_org_id()`.
//!   3. agency-subdomain   — same RLS predicate as #2; the differentiator
//!                           lives at the application layer.
//!   4. PlatformHost       — `is_published AND is_global_read_context()`,
//!                           with `app.global_read = on` AND
//!                           `app.current_org_id` UNSET.
//!
//! Plus a hard property: WITH CHECK omits the global branch — PlatformHost
//! cannot WRITE listings.
//!
//! These tests run under a non-superuser DB role (the CI workflow creates
//! `rls_test_runner`); see `rls_smoke_tests.rs` for the same `TEST_DATABASE_URL`
//! convention.

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

        Ok(Self { pool })
    }

    /// Set the legacy tenant trio (org / user / super-admin).
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

    /// Phase 4: opt the connection into the global-read context.
    async fn set_global_read(&self, enabled: bool) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT set_global_read_context($1)")
            .bind(enabled)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Resets every RLS-relevant session setting (Phase 4: also `app.global_read`).
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

    /// Insert a listing under super-admin context. Returns the listing id.
    async fn insert_listing(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        title: &str,
        is_published: bool,
    ) -> Result<Uuid, sqlx::Error> {
        let row = sqlx::query(
            r#"
            INSERT INTO listings (
                organization_id, created_by, status, transaction_type, title,
                property_type, street, city, postal_code, country, price, currency,
                is_published, published_at
            ) VALUES (
                $1, $2, 'draft', 'sale', $3,
                'apartment', '1 Smoke St', 'Smoke City', '00000', 'SK', 100000, 'EUR',
                $4, CASE WHEN $4 THEN NOW() ELSE NULL END
            )
            RETURNING id
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(title)
        .bind(is_published)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.get("id"))
    }

    async fn cleanup(&self) {
        let _ = self.set_request_context(None, None, true).await;
        let _ = self.set_global_read(false).await;
        let _ = sqlx::query("DELETE FROM listings WHERE title LIKE 'Smoke4%'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM organization_members WHERE user_id IN (SELECT id FROM users WHERE email LIKE 'smoke4_%@test.com')")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM users WHERE email LIKE 'smoke4_%@test.com'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM organizations WHERE name LIKE 'Smoke4%'")
            .execute(&self.pool)
            .await;
        let _ = self.clear_context().await;
    }
}

/// The headline test: four contexts on the same `listings` table.
///
/// Setup: 2 listings per org × 2 orgs (one published + one draft each) = 4 rows.
///
/// Expected visibility:
///   * super-admin             → 4 rows.
///   * agency A staff           → 2 (both A's, regardless of is_published).
///   * agency B staff           → 2 (both B's).
///   * PlatformHost (global)    → 2 (the one published row from each of A, B).
#[tokio::test]
#[ignore = "requires database with migrations; run with --ignored"]
async fn four_context_listings_visibility() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    // Bootstrap as super-admin so the create_default_roles trigger on
    // `INSERT INTO organizations` can populate `roles` (RLS-protected).
    db.set_request_context(None, None, true)
        .await
        .expect("setup: super-admin context");

    let org_a = db
        .create_test_org("Smoke4 Org A")
        .await
        .expect("create org A");
    let org_b = db
        .create_test_org("Smoke4 Org B")
        .await
        .expect("create org B");

    let user_a = db
        .create_test_user("smoke4_a@test.com", "User A")
        .await
        .expect("create user A");
    let user_b = db
        .create_test_user("smoke4_b@test.com", "User B")
        .await
        .expect("create user B");

    // Insert under super-admin so listings policy USING/CHECK is bypassed
    // for setup. Two per org: one published, one draft.
    let _a_pub = db
        .insert_listing(org_a, user_a, "Smoke4 A Published", true)
        .await
        .expect("insert A published");
    let _a_draft = db
        .insert_listing(org_a, user_a, "Smoke4 A Draft", false)
        .await
        .expect("insert A draft");
    let _b_pub = db
        .insert_listing(org_b, user_b, "Smoke4 B Published", true)
        .await
        .expect("insert B published");
    let _b_draft = db
        .insert_listing(org_b, user_b, "Smoke4 B Draft", false)
        .await
        .expect("insert B draft");

    // ------------------------------------------------------------------
    // Context 1: super-admin sees all 4.
    // ------------------------------------------------------------------
    db.set_request_context(None, None, true)
        .await
        .expect("super-admin context");
    db.set_global_read(false).await.expect("global_read off");
    let titles: Vec<String> =
        sqlx::query_scalar("SELECT title FROM listings WHERE title LIKE 'Smoke4%' ORDER BY title")
            .fetch_all(&db.pool)
            .await
            .expect("super-admin select");
    assert_eq!(
        titles.len(),
        4,
        "super-admin should see 4 listings, saw: {titles:?}"
    );

    // ------------------------------------------------------------------
    // Context 2 / 3: agency-A sees its own 2 (published + draft).
    // ------------------------------------------------------------------
    db.set_request_context(Some(org_a), Some(user_a), false)
        .await
        .expect("agency-A context");
    db.set_global_read(false).await.expect("global_read off");
    let titles_a: Vec<String> =
        sqlx::query_scalar("SELECT title FROM listings WHERE title LIKE 'Smoke4%' ORDER BY title")
            .fetch_all(&db.pool)
            .await
            .expect("agency-A select");
    assert_eq!(titles_a.len(), 2, "agency-A should see 2: {titles_a:?}");
    assert!(titles_a.contains(&"Smoke4 A Published".to_string()));
    assert!(titles_a.contains(&"Smoke4 A Draft".to_string()));
    assert!(!titles_a.contains(&"Smoke4 B Published".to_string()));
    assert!(!titles_a.contains(&"Smoke4 B Draft".to_string()));

    // Same RLS predicate covers context 3 (agency-subdomain public read of
    // own org). The differentiator is at the app layer, not RLS.
    db.set_request_context(Some(org_b), Some(user_b), false)
        .await
        .expect("agency-B context");
    db.set_global_read(false).await.expect("global_read off");
    let titles_b: Vec<String> =
        sqlx::query_scalar("SELECT title FROM listings WHERE title LIKE 'Smoke4%' ORDER BY title")
            .fetch_all(&db.pool)
            .await
            .expect("agency-B select");
    assert_eq!(titles_b.len(), 2, "agency-B should see 2: {titles_b:?}");
    assert!(titles_b.contains(&"Smoke4 B Published".to_string()));
    assert!(titles_b.contains(&"Smoke4 B Draft".to_string()));
    assert!(!titles_b.contains(&"Smoke4 A Published".to_string()));

    // ------------------------------------------------------------------
    // Context 4: PlatformHost — current_org_id UNSET, global_read = on.
    // Should see ONLY the published rows (one each from A, B).
    // ------------------------------------------------------------------
    db.set_request_context(None, None, false)
        .await
        .expect("platform-host: clear org");
    db.set_global_read(true)
        .await
        .expect("platform-host: global_read on");
    let titles_global: Vec<String> =
        sqlx::query_scalar("SELECT title FROM listings WHERE title LIKE 'Smoke4%' ORDER BY title")
            .fetch_all(&db.pool)
            .await
            .expect("platform-host select");
    assert_eq!(
        titles_global.len(),
        2,
        "PlatformHost should see only published, saw: {titles_global:?}"
    );
    assert!(titles_global.contains(&"Smoke4 A Published".to_string()));
    assert!(titles_global.contains(&"Smoke4 B Published".to_string()));
    assert!(!titles_global.contains(&"Smoke4 A Draft".to_string()));
    assert!(!titles_global.contains(&"Smoke4 B Draft".to_string()));

    db.cleanup().await;
}

/// PlatformHost is read-only: WITH CHECK omits the global branch, so any
/// attempted write fails. This is the schema-level guarantee that the global
/// portal cannot be a write surface.
#[tokio::test]
#[ignore = "requires database with migrations; run with --ignored"]
async fn platform_host_cannot_write_listings() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    db.set_request_context(None, None, true)
        .await
        .expect("setup: super-admin");
    let org_a = db
        .create_test_org("Smoke4 Org WriteCheck A")
        .await
        .expect("create org A");
    let user_a = db
        .create_test_user("smoke4_writecheck_a@test.com", "User A")
        .await
        .expect("create user A");
    let listing_id = db
        .insert_listing(org_a, user_a, "Smoke4 WriteCheck Published", true)
        .await
        .expect("insert listing");

    // Switch to PlatformHost.
    db.set_request_context(None, None, false)
        .await
        .expect("clear org");
    db.set_global_read(true).await.expect("global_read on");

    // Sanity: we CAN read it.
    let title: Option<String> = sqlx::query_scalar("SELECT title FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_optional(&db.pool)
        .await
        .expect("read under platform-host");
    assert_eq!(
        title.as_deref(),
        Some("Smoke4 WriteCheck Published"),
        "platform-host should be able to READ a published listing"
    );

    // Attempting an UPDATE: WITH CHECK has no global branch and
    // get_current_org_id() returns NULL, so the new row must violate the
    // policy and the UPDATE must fail (or affect zero rows under USING).
    //
    // The exact failure mode depends on PostgreSQL: with FORCE RLS, an
    // UPDATE that the USING clause permits but WITH CHECK rejects raises
    // SQLSTATE 42501 "new row violates row-level security policy". An
    // UPDATE that USING also rejects simply affects 0 rows. Either is a
    // pass for "platform-host can't write." We check both.
    let update = sqlx::query("UPDATE listings SET title = 'Smoke4 PWNED' WHERE id = $1")
        .bind(listing_id)
        .execute(&db.pool)
        .await;
    let blocked = match update {
        Err(_) => true,
        Ok(result) => result.rows_affected() == 0,
    };
    assert!(
        blocked,
        "PlatformHost UPDATE on listings must be blocked (WITH CHECK denies it)"
    );

    // Confirm by reading the title back as super-admin: it must be unchanged.
    db.set_request_context(None, None, true)
        .await
        .expect("verify: super-admin");
    db.set_global_read(false).await.expect("global_read off");
    let title_after: String = sqlx::query_scalar("SELECT title FROM listings WHERE id = $1")
        .bind(listing_id)
        .fetch_one(&db.pool)
        .await
        .expect("verify read");
    assert_eq!(
        title_after, "Smoke4 WriteCheck Published",
        "title must not have changed"
    );

    // Same for INSERT — try to insert a listing whose org_id is org_a from
    // platform-host context; WITH CHECK requires (super-admin OR org_id =
    // current_org_id), and current_org_id is NULL, so this must fail.
    db.set_request_context(None, None, false)
        .await
        .expect("clear org");
    db.set_global_read(true).await.expect("global_read on");
    let insert = sqlx::query(
        r#"
        INSERT INTO listings (
            organization_id, created_by, status, transaction_type, title,
            property_type, street, city, postal_code, country, price, currency,
            is_published
        ) VALUES (
            $1, $2, 'draft', 'sale', 'Smoke4 PlatformHostInsert',
            'apartment', '2 Smoke St', 'Smoke City', '00000', 'SK', 200000, 'EUR',
            TRUE
        )
        "#,
    )
    .bind(org_a)
    .bind(user_a)
    .execute(&db.pool)
    .await;
    assert!(
        insert.is_err(),
        "PlatformHost INSERT on listings must fail (WITH CHECK denies it)"
    );

    db.cleanup().await;
}

/// Defense for leak #2: clear_request_context() must reset `app.global_read`
/// so a PlatformHost connection returned to the pool cannot leak the global
/// view onto a subsequent agency-scoped request that lands on the same
/// connection.
#[tokio::test]
#[ignore = "requires database with migrations; run with --ignored"]
async fn clear_request_context_resets_global_read() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    db.set_request_context(None, None, true)
        .await
        .expect("setup: super-admin");
    let org_a = db
        .create_test_org("Smoke4 Org BleedCheck A")
        .await
        .expect("create org A");
    let org_b = db
        .create_test_org("Smoke4 Org BleedCheck B")
        .await
        .expect("create org B");
    let user_a = db
        .create_test_user("smoke4_bleed_a@test.com", "User A")
        .await
        .expect("user A");
    let user_b = db
        .create_test_user("smoke4_bleed_b@test.com", "User B")
        .await
        .expect("user B");
    let _ = db
        .insert_listing(org_a, user_a, "Smoke4 BleedCheck A Published", true)
        .await
        .expect("insert");
    let _ = db
        .insert_listing(org_b, user_b, "Smoke4 BleedCheck B Published", true)
        .await
        .expect("insert");

    // Step 1: PlatformHost session — sees both published rows.
    db.set_request_context(None, None, false)
        .await
        .expect("clear org");
    db.set_global_read(true).await.expect("global_read on");
    let count_global: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM listings WHERE title LIKE 'Smoke4 BleedCheck%'")
            .fetch_one(&db.pool)
            .await
            .expect("count under global");
    assert_eq!(count_global, 2, "global view should see 2 published");

    // Step 2: clear_request_context() drops every flag.
    db.clear_context().await.expect("clear");

    // Step 3: a fresh tenant-scoped context for org A — must NOT see B's row.
    // If clear_request_context didn't reset app.global_read, app.global_read
    // would still be 'on' here and the policy's third OR-branch would expose
    // B's row.
    db.set_request_context(Some(org_a), Some(user_a), false)
        .await
        .expect("agency A context");
    let titles: Vec<String> = sqlx::query_scalar(
        "SELECT title FROM listings WHERE title LIKE 'Smoke4 BleedCheck%' ORDER BY title",
    )
    .fetch_all(&db.pool)
    .await
    .expect("post-clear select");
    assert_eq!(
        titles.len(),
        1,
        "after clear, agency-A must see only its own row, saw: {titles:?}"
    );
    assert_eq!(titles[0], "Smoke4 BleedCheck A Published");

    db.cleanup().await;
}

/// Migration 00135 backfill check: rows with `status='active'` carry
/// `is_published=TRUE`. We don't depend on existing data here — we INSERT
/// our own row with status='active' and assert the column was set by the
/// trigger... wait, there's no trigger for that. The migration backfill
/// only ran ONCE on 00135 apply. New rows default `is_published=FALSE`.
///
/// So this test verifies the *default*: a new listing created today defaults
/// to is_published=FALSE regardless of status. The `/global-publish` endpoint
/// is the only path that flips it. This is the binary-first sequencing per
/// ROADMAP Phase 4.
#[tokio::test]
#[ignore = "requires database with migrations; run with --ignored"]
async fn new_listing_defaults_to_not_published() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    db.set_request_context(None, None, true)
        .await
        .expect("setup: super-admin");
    let org_a = db
        .create_test_org("Smoke4 Org Default")
        .await
        .expect("create org");
    let user_a = db
        .create_test_user("smoke4_default@test.com", "User")
        .await
        .expect("create user");

    let row = sqlx::query(
        r#"
        INSERT INTO listings (
            organization_id, created_by, status, transaction_type, title,
            property_type, street, city, postal_code, country, price, currency
        ) VALUES (
            $1, $2, 'active', 'sale', 'Smoke4 Default Active',
            'apartment', '3 Smoke St', 'Smoke City', '00000', 'SK', 300000, 'EUR'
        )
        RETURNING is_published
        "#,
    )
    .bind(org_a)
    .bind(user_a)
    .fetch_one(&db.pool)
    .await
    .expect("insert active");

    let is_published: bool = row.get("is_published");
    assert!(
        !is_published,
        "even a status='active' listing must default to is_published=FALSE — \
         publish-to-global is an explicit opt-in via /global-publish"
    );

    db.cleanup().await;
}

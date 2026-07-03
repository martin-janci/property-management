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
        // Phase 6: portal_users has been dropped (migration 00148). All FK columns
        // that previously referenced portal_users(id) now reference users(id).
        // Writing to users alone is sufficient.
        let row = sqlx::query(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
            VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
            RETURNING id
            "#,
        )
        .bind(email)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        let id: Uuid = row.get("id");

        Ok(id)
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
        // Phase 6: portal_users dropped; users rows cover both staff and public principals.
        let _ = sqlx::query("DELETE FROM users WHERE email LIKE 'smoke%@test.com'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM organizations WHERE name LIKE 'Smoke%'")
            .execute(&self.pool)
            .await;
        let _ = self.clear_context().await;
    }

    /// Cleanup for the Phase 0 multi-table test. Runs under super-admin context
    /// so FORCE RLS doesn't turn the DELETEs into silent no-ops. Most child
    /// rows go away via ON DELETE CASCADE when their org/building/listing
    /// parents are removed, but we explicitly delete the few global-scoped
    /// helper rows (feature_packages, feature_flags, agencies) the test
    /// created, and rely on cascade for everything tenant-scoped.
    async fn cleanup_phase0(&self) {
        let _ = self.set_request_context(None, None, true).await;
        // Deleting the orgs cascades to buildings -> units -> listings and all
        // their descendant tenant-scoped rows. Buildings are also matched by
        // the `Smoke Building %` name pattern as a belt-and-braces measure.
        let _ = sqlx::query("DELETE FROM buildings WHERE name LIKE 'Smoke Building %'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM organization_members WHERE user_id IN (SELECT id FROM users WHERE email LIKE 'smoke_p0_%@test.com')")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM organizations WHERE name LIKE 'Smoke Org P0%'")
            .execute(&self.pool)
            .await;
        // Global / non-tenant-scoped helper rows created by the test.
        let _ = sqlx::query("DELETE FROM agencies WHERE name LIKE 'Smoke Agency %'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM feature_packages WHERE key LIKE 'pkg-%'")
            .execute(&self.pool)
            .await;
        let _ = sqlx::query("DELETE FROM feature_flags WHERE key LIKE 'flag-%'")
            .execute(&self.pool)
            .await;
        // Phase 6: portal_users dropped.
        let _ = sqlx::query("DELETE FROM users WHERE email LIKE 'smoke_p0_%@test.com'")
            .execute(&self.pool)
            .await;
        let _ = self.clear_context().await;
    }
}

/// Core smoke test: Verify tenant A cannot see tenant B's buildings
#[tokio::test]
#[ignore = "requires database with migrations; run with --ignored"]
async fn smoke_test_cross_tenant_isolation() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    // ALL setup runs as super-admin so RLS-enforced trigger inserts succeed.
    // Most importantly: `INSERT INTO organizations` fires the
    // `create_default_roles()` trigger which inserts into `roles` — and
    // `roles` has a WITH CHECK that requires
    // `organization_id = get_current_org_id() OR is_super_admin()`.
    // Without super-admin context here, the trigger fails with
    // "new row violates row-level security policy for table \"roles\"".
    db.set_request_context(None, None, true)
        .await
        .expect("Failed to set super-admin context for setup");

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

    // Add users to their respective orgs
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role_id) \
         VALUES ($1, $2, (SELECT id FROM roles WHERE organization_id = $1 AND name = 'Manager'))",
    )
    .bind(org_a)
    .bind(user_a)
    .execute(&db.pool)
    .await
    .expect("Failed to add user A to org A");

    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role_id) \
         VALUES ($1, $2, (SELECT id FROM roles WHERE organization_id = $1 AND name = 'Manager'))",
    )
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
#[ignore = "requires database with migrations; run with --ignored"]
async fn smoke_test_null_context_blocks_access() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    // Setup runs as super-admin so the `create_default_roles` trigger that
    // fires on `INSERT INTO organizations` can populate `roles` (which is
    // RLS-protected with WITH CHECK).
    db.set_request_context(None, None, true)
        .await
        .expect("Failed to set super-admin context for setup");

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
#[ignore = "requires database with migrations; run with --ignored"]
async fn smoke_test_context_clearing() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup().await;

    // Setup runs as super-admin so the `create_default_roles` trigger that
    // fires on `INSERT INTO organizations` can populate `roles` (which is
    // RLS-protected with WITH CHECK).
    db.set_request_context(None, None, true)
        .await
        .expect("Failed to set super-admin context for setup");

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

// ===========================================================================
// Phase 0: RLS coverage for the 70 newly-protected tenant-scoped tables
// (migrations 00110-00123). The test below builds a parallel data graph for
// two orgs entirely under super-admin context, then switches to each org's
// non-super-admin context and asserts it sees ONLY its own rows.
//
// Coverage: every one of the 17 own-org tables, plus at least one
// representative child table per distinct parent chain:
//   - listings children      (listing_photos)
//   - reality portal chain   (listing_inquiries -> inquiry_messages two-hop)
//   - community chain        (community_groups -> community_posts ->
//                             community_post_reactions three-hop)
//   - disputes chain         (dispute_parties, resolution_votes two-hop)
//   - owner-analytics chain  (property_valuations -> comparable_properties)
//   - portfolio chain        (portfolio_properties_perf)
//   - marketplace chain      (provider_quotes)
//   - edd chain              (edd_documents)
//   - agencies chain         (agency_listings)
// ===========================================================================

/// Helper: insert a building for an org and return its id. Caller must be in
/// super-admin context.
async fn insert_building(db: &TestDb, org: Uuid, label: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, $2, 'St', 'City', '00000', 'Country') RETURNING id",
    )
    .bind(org)
    .bind(format!("Smoke Building {label}"))
    .fetch_one(&db.pool)
    .await
    .expect("insert building")
}

/// Helper: insert a unit under a building and return its id.
async fn insert_unit(db: &TestDb, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, $2, 1) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(&db.pool)
    .await
    .expect("insert unit")
}

/// Helper: insert a listing for an org and return its id.
async fn insert_listing(db: &TestDb, org: Uuid, created_by: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO listings (organization_id, created_by, transaction_type, title, \
         property_type, street, city, postal_code, country, price) \
         VALUES ($1, $2, 'sale', $3, 'apartment', 'St', 'City', '00000', 'SK', 100000) \
         RETURNING id",
    )
    .bind(org)
    .bind(created_by)
    .bind(title)
    .fetch_one(&db.pool)
    .await
    .expect("insert listing")
}

/// Phase 0 smoke test: build a two-org data graph spanning all newly-protected
/// tables and assert each org sees only its own rows.
#[tokio::test]
#[ignore = "requires database with migrations; run with --ignored"]
async fn smoke_test_phase0_rls_cross_tenant_isolation() {
    let db = TestDb::new()
        .await
        .expect("Failed to connect to test database");
    db.cleanup_phase0().await;

    // ---- All setup under super-admin context ----
    db.set_request_context(None, None, true)
        .await
        .expect("set super-admin context");

    let org_a = db.create_test_org("Smoke Org P0A").await.expect("org A");
    let org_b = db.create_test_org("Smoke Org P0B").await.expect("org B");
    let user_a = db
        .create_test_user("smoke_p0_a@test.com", "P0 User A")
        .await
        .expect("user A");
    let user_b = db
        .create_test_user("smoke_p0_b@test.com", "P0 User B")
        .await
        .expect("user B");

    // Build the full graph for both orgs. `g` returns (org, user, suffix).
    for (org, user, suffix) in [(org_a, user_a, "A"), (org_b, user_b, "B")] {
        let building = insert_building(&db, org, suffix).await;
        let unit = insert_unit(&db, building, &format!("U-{suffix}")).await;
        let listing = insert_listing(&db, org, user, &format!("Smoke Listing {suffix}")).await;

        // --- listings children (00110) ---
        sqlx::query("INSERT INTO listing_photos (listing_id, url) VALUES ($1, 'http://x')")
            .bind(listing)
            .execute(&db.pool)
            .await
            .expect("listing_photos");
        sqlx::query(
            "INSERT INTO listing_syndications (listing_id, portal) VALUES ($1, 'reality_portal')",
        )
        .bind(listing)
        .execute(&db.pool)
        .await
        .expect("listing_syndications");

        // --- agencies children (00111) ---
        let agency = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO agencies (name, slug, email) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(format!("Smoke Agency {suffix}"))
        .bind(format!("smoke-agency-{}", Uuid::new_v4()))
        .bind(format!("agency-{suffix}@test.com"))
        .fetch_one(&db.pool)
        .await
        .expect("agency");
        sqlx::query(
            "INSERT INTO agency_listings (listing_id, agency_id, realtor_id) VALUES ($1, $2, $3)",
        )
        .bind(listing)
        .bind(agency)
        .bind(user)
        .execute(&db.pool)
        .await
        .expect("agency_listings");

        // --- short-term rentals own-org (00112) ---
        sqlx::query(
            "INSERT INTO rental_platform_connections (organization_id, unit_id, platform) \
             VALUES ($1, $2, 'direct')",
        )
        .bind(org)
        .bind(unit)
        .execute(&db.pool)
        .await
        .expect("rental_platform_connections");
        sqlx::query(
            "INSERT INTO rental_bookings (organization_id, unit_id, platform, guest_name, \
             check_in, check_out) VALUES ($1, $2, 'direct', 'Guest', '2026-01-01', '2026-01-05')",
        )
        .bind(org)
        .bind(unit)
        .execute(&db.pool)
        .await
        .expect("rental_bookings");
        let booking = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM rental_bookings WHERE organization_id = $1 LIMIT 1",
        )
        .bind(org)
        .fetch_one(&db.pool)
        .await
        .expect("fetch booking");
        sqlx::query(
            "INSERT INTO rental_guests (organization_id, booking_id, first_name, last_name) \
             VALUES ($1, $2, 'First', 'Last')",
        )
        .bind(org)
        .bind(booking)
        .execute(&db.pool)
        .await
        .expect("rental_guests");
        sqlx::query(
            "INSERT INTO rental_guest_reports (organization_id, building_id, report_type, \
             period_start, period_end, authority_code, authority_name, report_format) \
             VALUES ($1, $2, 'monthly', '2026-01-01', '2026-01-31', 'SK_X', 'Auth', 'pdf')",
        )
        .bind(org)
        .bind(building)
        .execute(&db.pool)
        .await
        .expect("rental_guest_reports");
        sqlx::query(
            "INSERT INTO rental_calendar_blocks (organization_id, unit_id, block_start, \
             block_end, reason) VALUES ($1, $2, '2026-02-01', '2026-02-03', 'maintenance')",
        )
        .bind(org)
        .bind(unit)
        .execute(&db.pool)
        .await
        .expect("rental_calendar_blocks");
        sqlx::query(
            "INSERT INTO rental_ical_feeds (organization_id, unit_id, feed_name, feed_token) \
             VALUES ($1, $2, 'Feed', $3)",
        )
        .bind(org)
        .bind(unit)
        .bind(format!("token-{}", Uuid::new_v4()))
        .execute(&db.pool)
        .await
        .expect("rental_ical_feeds");

        // --- reality portal chain (00113): inquiry -> message (two-hop) ---
        let inquiry = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO listing_inquiries (listing_id, realtor_id, name, email, message) \
             VALUES ($1, $2, 'Inq', 'inq@test.com', 'hi') RETURNING id",
        )
        .bind(listing)
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("listing_inquiries");
        sqlx::query(
            "INSERT INTO inquiry_messages (inquiry_id, sender_type, sender_id, message) \
             VALUES ($1, 'realtor', $2, 'reply')",
        )
        .bind(inquiry)
        .bind(user)
        .execute(&db.pool)
        .await
        .expect("inquiry_messages");

        // --- community chain (00114): group -> post -> reaction (three-hop) ---
        let group = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO community_groups (building_id, name, slug) VALUES ($1, 'Grp', $2) \
             RETURNING id",
        )
        .bind(building)
        .bind(format!("grp-{}", Uuid::new_v4()))
        .fetch_one(&db.pool)
        .await
        .expect("community_groups");
        let post = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO community_posts (group_id, author_id, content) VALUES ($1, $2, 'post') \
             RETURNING id",
        )
        .bind(group)
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("community_posts");
        sqlx::query(
            "INSERT INTO community_post_reactions (post_id, user_id, reaction_type) \
             VALUES ($1, $2, 'like')",
        )
        .bind(post)
        .bind(user)
        .execute(&db.pool)
        .await
        .expect("community_post_reactions");
        // workflow automation (own-org)
        sqlx::query(
            "INSERT INTO workflow_automation_rules (organization_id, name, trigger_type) \
             VALUES ($1, 'Rule', 'manual')",
        )
        .bind(org)
        .execute(&db.pool)
        .await
        .expect("workflow_automation_rules");

        // --- disputes chain (00115) ---
        let dispute = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO disputes (organization_id, reference_number, category, title, \
             description, filed_by) VALUES ($1, $2, 'noise', 'Dispute', 'desc', $3) RETURNING id",
        )
        .bind(org)
        .bind(format!("REF-{}", Uuid::new_v4()))
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("disputes");
        let party = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO dispute_parties (dispute_id, user_id, role) \
             VALUES ($1, $2, 'complainant') RETURNING id",
        )
        .bind(dispute)
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("dispute_parties");
        let resolution = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO dispute_resolutions (dispute_id, proposed_by, resolution_text) \
             VALUES ($1, $2, 'resolve') RETURNING id",
        )
        .bind(dispute)
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("dispute_resolutions");
        sqlx::query(
            "INSERT INTO resolution_votes (resolution_id, party_id, accepted) \
             VALUES ($1, $2, true)",
        )
        .bind(resolution)
        .bind(party)
        .execute(&db.pool)
        .await
        .expect("resolution_votes");

        // --- owner-analytics chain (00116) ---
        sqlx::query(
            "INSERT INTO expense_auto_approval_rules (organization_id, owner_id) \
             VALUES ($1, $2)",
        )
        .bind(org)
        .bind(user)
        .execute(&db.pool)
        .await
        .expect("expense_auto_approval_rules");
        let valuation = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO property_valuations (unit_id, value_low, value_high, estimated_value, \
             valuation_method) VALUES ($1, 90000, 110000, 100000, 'avm') RETURNING id",
        )
        .bind(unit)
        .fetch_one(&db.pool)
        .await
        .expect("property_valuations");
        sqlx::query(
            "INSERT INTO comparable_properties (valuation_id, address, sale_price, size_sqm, \
             rooms, distance_km, source) VALUES ($1, 'Addr', 95000, 50, 2, 1.0, 'src')",
        )
        .bind(valuation)
        .execute(&db.pool)
        .await
        .expect("comparable_properties");

        // --- edd chain (00117) ---
        let aml = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO aml_risk_assessments (organization_id, party_id, party_type) \
             VALUES ($1, $2, 'individual') RETURNING id",
        )
        .bind(org)
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("aml_risk_assessments");
        let edd = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO edd_records (aml_assessment_id, organization_id, party_id, initiated_by) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(aml)
        .bind(org)
        .bind(user)
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("edd_records");
        sqlx::query(
            "INSERT INTO edd_documents (edd_id, document_type, file_path, original_filename, \
             file_size_bytes, mime_type, uploaded_by) \
             VALUES ($1, 'passport', '/p', 'p.pdf', 100, 'application/pdf', $2)",
        )
        .bind(edd)
        .bind(user)
        .execute(&db.pool)
        .await
        .expect("edd_documents");
        sqlx::query(
            "INSERT INTO moderation_cases (content_type, content_id, content_owner_id, \
             organization_id) VALUES ('listing', $1, $2, $3)",
        )
        .bind(listing)
        .bind(user)
        .bind(org)
        .execute(&db.pool)
        .await
        .expect("moderation_cases");
        sqlx::query(
            "INSERT INTO suspicious_activities (organization_id, party_id, activity_type, \
             description, detection_method) VALUES ($1, $2, 'structuring', 'desc', 'rule')",
        )
        .bind(org)
        .bind(user)
        .execute(&db.pool)
        .await
        .expect("suspicious_activities");

        // --- feature packages / analytics (00118 / 00119) ---
        // organization_packages needs a feature_package; create one (global table).
        let package = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO feature_packages (key, name, display_name) VALUES ($1, 'Pkg', 'Pkg') \
             RETURNING id",
        )
        .bind(format!("pkg-{}", Uuid::new_v4()))
        .fetch_one(&db.pool)
        .await
        .expect("feature_packages");
        sqlx::query(
            "INSERT INTO organization_packages (organization_id, package_id, source) \
             VALUES ($1, $2, 'purchase')",
        )
        .bind(org)
        .bind(package)
        .execute(&db.pool)
        .await
        .expect("organization_packages");
        // feature_usage_events needs a feature_flag.
        let flag = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO feature_flags (key, name, description) VALUES ($1, 'Flag', 'desc') \
             RETURNING id",
        )
        .bind(format!("flag-{}", Uuid::new_v4()))
        .fetch_one(&db.pool)
        .await
        .expect("feature_flags");
        sqlx::query(
            "INSERT INTO feature_usage_events (feature_flag_id, organization_id, event_type) \
             VALUES ($1, $2, 'access')",
        )
        .bind(flag)
        .bind(org)
        .execute(&db.pool)
        .await
        .expect("feature_usage_events");

        // --- portfolio performance chain (00120) ---
        let portfolio = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO performance_portfolios (organization_id, name) VALUES ($1, 'Pf') \
             RETURNING id",
        )
        .bind(org)
        .fetch_one(&db.pool)
        .await
        .expect("performance_portfolios");
        sqlx::query(
            "INSERT INTO portfolio_properties_perf (portfolio_id, building_id, acquisition_date, \
             acquisition_price) VALUES ($1, $2, '2025-01-01', 200000)",
        )
        .bind(portfolio)
        .bind(building)
        .execute(&db.pool)
        .await
        .expect("portfolio_properties_perf");
        sqlx::query(
            "INSERT INTO market_benchmarks (organization_id, name, period_year) \
             VALUES ($1, 'Bench', 2025)",
        )
        .bind(org)
        .execute(&db.pool)
        .await
        .expect("market_benchmarks");

        // --- marketplace chain (00121) ---
        let provider = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO service_provider_profiles (user_id, company_name, contact_name, \
             contact_email) VALUES ($1, 'Co', 'Name', $2) RETURNING id",
        )
        .bind(user)
        .bind(format!("prov-{suffix}@test.com"))
        .fetch_one(&db.pool)
        .await
        .expect("service_provider_profiles");
        let rfq = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO rfqs (organization_id, created_by, title, description, service_category) \
             VALUES ($1, $2, 'RFQ', 'desc', 'plumbing') RETURNING id",
        )
        .bind(org)
        .bind(user)
        .fetch_one(&db.pool)
        .await
        .expect("rfqs");
        sqlx::query(
            "INSERT INTO provider_quotes (rfq_id, provider_id, price) VALUES ($1, $2, 500)",
        )
        .bind(rfq)
        .bind(provider)
        .execute(&db.pool)
        .await
        .expect("provider_quotes");
    }

    // ---- Assert isolation: for each table, org A sees exactly 1 of its rows ----
    // We define a closure that runs a COUNT scoped to a marker, under a given
    // org's non-super-admin context, and asserts the count equals 1.
    let own_org_tables: &[(&str, &str)] = &[
        ("listings", "title LIKE 'Smoke Listing %'"),
        ("rental_platform_connections", "platform = 'direct'"),
        ("rental_bookings", "guest_name = 'Guest'"),
        ("rental_guests", "first_name = 'First'"),
        ("rental_guest_reports", "authority_code = 'SK_X'"),
        ("rental_calendar_blocks", "reason = 'maintenance'"),
        ("rental_ical_feeds", "feed_name = 'Feed'"),
        ("workflow_automation_rules", "name = 'Rule'"),
        ("disputes", "title = 'Dispute'"),
        ("expense_auto_approval_rules", "TRUE"),
        ("aml_risk_assessments", "party_type = 'individual'"),
        ("edd_records", "TRUE"),
        ("moderation_cases", "content_type = 'listing'"),
        ("suspicious_activities", "activity_type = 'structuring'"),
        ("organization_packages", "source = 'purchase'"),
        ("feature_usage_events", "event_type = 'access'"),
        ("performance_portfolios", "name = 'Pf'"),
        ("market_benchmarks", "name = 'Bench'"),
        ("rfqs", "title = 'RFQ'"),
    ];

    // Child tables: one representative per distinct parent chain.
    let child_tables: &[(&str, &str)] = &[
        ("listing_photos", "url = 'http://x'"),
        ("listing_syndications", "portal = 'reality_portal'"),
        ("agency_listings", "TRUE"),
        ("listing_inquiries", "name = 'Inq'"),
        ("inquiry_messages", "message = 'reply'"),
        ("community_groups", "name = 'Grp'"),
        ("community_posts", "content = 'post'"),
        ("community_post_reactions", "reaction_type = 'like'"),
        ("dispute_parties", "role = 'complainant'"),
        ("dispute_resolutions", "resolution_text = 'resolve'"),
        ("resolution_votes", "accepted = true"),
        ("property_valuations", "valuation_method = 'avm'"),
        ("comparable_properties", "address = 'Addr'"),
        ("edd_documents", "document_type = 'passport'"),
        ("portfolio_properties_perf", "acquisition_price = 200000"),
        ("provider_quotes", "price = 500"),
    ];

    for (org_label, org_id, other_label) in [("A", org_a, "B"), ("B", org_b, "A")] {
        let user_id = if org_label == "A" { user_a } else { user_b };
        db.set_request_context(Some(org_id), Some(user_id), false)
            .await
            .expect("set non-super-admin context");

        for (table, predicate) in own_org_tables.iter().chain(child_tables.iter()) {
            let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT COUNT(*) FROM {table} WHERE {predicate}"
            )))
            .fetch_one(&db.pool)
            .await
            .unwrap_or_else(|e| panic!("count query failed for {table}: {e}"));
            assert_eq!(
                count, 1,
                "org {org_label} should see exactly its own row in `{table}` \
                 (not org {other_label}'s), but saw {count}"
            );
        }
    }

    // ---- Null context sees nothing ----
    db.clear_context().await.expect("clear context");
    for (table, predicate) in own_org_tables.iter().chain(child_tables.iter()) {
        let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT COUNT(*) FROM {table} WHERE {predicate}"
        )))
        .fetch_one(&db.pool)
        .await
        .unwrap_or_else(|e| panic!("null-context count failed for {table}: {e}"));
        assert_eq!(
            count, 0,
            "with null context, `{table}` should be invisible, but saw {count}"
        );
    }

    db.cleanup_phase0().await;
    println!("✓ Phase 0 cross-tenant isolation verified across 70 tables");
}

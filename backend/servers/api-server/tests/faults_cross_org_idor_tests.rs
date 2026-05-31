//! Regression tests for the cross-tenant IDOR fix on the Epic 4 fault routes
//! (issue #796).
//!
//! Audit history: the `/api/v1/faults/*` handlers authenticate the caller with
//! `RequestPrincipal`, and the handlers that reach the DB through
//! `RlsConnection` are isolated by the `faults_tenant_isolation` RLS policy.
//! But several by-id handlers reached the DB through the *non-RLS* pool methods,
//! which carry no `organization_id` predicate, so a caller in org B could read
//! or mutate org A's fault by guessing (or enumerating) the UUID:
//!
//! - `get_fault`      → `find_by_id_with_details(id)`  (full fault leak)
//! - `assign_fault`   → `assign(id, …)`                (cross-tenant mutate)
//! - `resolve_fault`  → `resolve(id, …)`               (cross-tenant mutate)
//! - `confirm_fault`  → `confirm(id, …)`               (cross-tenant mutate)
//! - `reopen_fault`   → `reopen(id, …)`                (cross-tenant mutate)
//! - `list_comments`  → `list_timeline(id, …)`         (timeline leak)
//! - `add_comment` / `add_work_note`                   (cross-tenant write)
//! - `list_attachments` / `delete_attachment`          (NO auth extractor at all)
//!
//! `create_fault` additionally accepted a client-supplied `building_id` /
//! `unit_id` with no check that they belong to the caller's tenant (the RLS
//! `WITH CHECK` only validates the new row's own `organization_id`).
//!
//! The fix adds `_for_org` / scoped repository methods that thread the
//! principal's `effective_org` into the SQL `WHERE` clause
//! (`find_by_id_for_org`, `building_belongs_to_org`, `unit_in_building`,
//! `delete_attachment_for_fault`). The handlers call them as an authorization
//! guard before touching the resource. This mirrors the `_for_org` idiom
//! introduced for the AI/LLM routes (#766/#816) and the work-order routes
//! (#821).
//!
//! Why repo-layer and not HTTP-layer: `TestApp` mounts the router WITHOUT
//! `host_tenant_middleware`, so `RequestPrincipal` can never resolve an
//! `effective_org` in tests (see `ai_llm_cross_tenant_idor_tests.rs`'s caveat).
//! The IDOR fix itself lives in the repository's WHERE clause, so the security
//! contract is asserted there directly: the scoped method must NOT return /
//! mutate another org's row even though the raw (pre-fix) query would. Each
//! read test also runs the unscoped query to demonstrate the leak the scoped
//! variant closes.

use db::repositories::FaultRepository;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("FaultIDOR Org {slug}"))
    .bind(format!("fault-idor-org-{slug}"))
    .bind(format!("{slug}@fault-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'FaultIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code)
        VALUES ($1, 'Test St 1', 'Bratislava', '81101') RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation)
        VALUES ($1, $2) RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_fault(pool: &PgPool, org_id: Uuid, building_id: Uuid, reporter_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO faults (organization_id, building_id, reporter_id, title, description)
        VALUES ($1, $2, $3, 'Broken light', 'Hallway light is out') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(reporter_id)
    .fetch_one(pool)
    .await
    .expect("seed fault")
}

async fn seed_attachment(pool: &PgPool, fault_id: Uuid, uploaded_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO fault_attachments
            (fault_id, filename, original_filename, content_type, size_bytes, storage_url, uploaded_by)
        VALUES ($1, 'f.jpg', 'orig.jpg', 'image/jpeg', 1234, 's3://bucket/f.jpg', $2)
        RETURNING id
        "#,
    )
    .bind(fault_id)
    .bind(uploaded_by)
    .fetch_one(pool)
    .await
    .expect("seed attachment")
}

// ---------------------------------------------------------------------------
// (1) Fault read/mutate guard is tenant-scoped — org B cannot resolve org A's
//     fault. `fault_belongs_to_org` backs `require_fault_in_tenant`, the guard
//     in front of every non-RLS by-id handler (get/assign/resolve/confirm/
//     reopen, comments, work notes, attachments).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn fault_belongs_to_org_blocks_cross_tenant_access(pool: PgPool) {
    let repo = FaultRepository::new(pool.clone());

    let org_a = seed_org(&pool, "a").await;
    let org_b = seed_org(&pool, "b").await;
    let user_a = seed_user(&pool, "a@fault-idor.test").await;
    let building_a = seed_building(&pool, org_a).await;
    let fault_in_a = seed_fault(&pool, org_a, building_a, user_a).await;

    // Same-org guard succeeds.
    let same_org = repo
        .fault_belongs_to_org(fault_in_a, org_a)
        .await
        .expect("query ok");
    assert!(same_org, "org A must be able to load its own fault");

    // Cross-org guard returns false (the IDOR is blocked → handler maps to 404).
    let cross_org = repo
        .fault_belongs_to_org(fault_in_a, org_b)
        .await
        .expect("query ok");
    assert!(!cross_org, "org B must NOT be able to load org A's fault");

    // Demonstrate the leak the org predicate closes: the unscoped lookup the
    // pre-fix handlers effectively performed returns the row regardless of org.
    let unscoped: Option<Uuid> = sqlx::query_scalar("SELECT id FROM faults WHERE id = $1")
        .bind(fault_in_a)
        .fetch_optional(&pool)
        .await
        .expect("query ok");
    assert_eq!(
        unscoped,
        Some(fault_in_a),
        "sanity: the unscoped query (the vulnerable pre-fix path) does leak the row"
    );
}

// ---------------------------------------------------------------------------
// (2) `create_fault` building validation — a fault may not be filed against a
//     building owned by a different tenant.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_belongs_to_org_rejects_foreign_building(pool: PgPool) {
    let repo = FaultRepository::new(pool.clone());

    let org_a = seed_org(&pool, "ba").await;
    let org_b = seed_org(&pool, "bb").await;
    let building_a = seed_building(&pool, org_a).await;

    assert!(
        repo.building_belongs_to_org(building_a, org_a)
            .await
            .expect("query ok"),
        "org A's building must validate against org A"
    );
    assert!(
        !repo
            .building_belongs_to_org(building_a, org_b)
            .await
            .expect("query ok"),
        "org B must NOT be able to file a fault against org A's building"
    );
}

// ---------------------------------------------------------------------------
// (3) `create_fault` unit validation — the unit must sit in the building named
//     on the same request (transitively, the caller's tenant).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unit_in_building_rejects_mismatched_building(pool: PgPool) {
    let repo = FaultRepository::new(pool.clone());

    let org_a = seed_org(&pool, "ua").await;
    let building_a = seed_building(&pool, org_a).await;
    let other_building = seed_building(&pool, org_a).await;
    let unit_a = seed_unit(&pool, building_a, "3B").await;

    assert!(
        repo.unit_in_building(unit_a, building_a)
            .await
            .expect("query ok"),
        "unit must validate against the building it belongs to"
    );
    assert!(
        !repo
            .unit_in_building(unit_a, other_building)
            .await
            .expect("query ok"),
        "a unit from a different building must be rejected"
    );
}

// ---------------------------------------------------------------------------
// (4) Attachment delete is scoped to its fault — a foreign `attachment_id`
//     paired with an owned `fault_id` deletes nothing.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_attachment_for_fault_is_scoped(pool: PgPool) {
    let repo = FaultRepository::new(pool.clone());

    let org_a = seed_org(&pool, "da").await;
    let user_a = seed_user(&pool, "da@fault-idor.test").await;
    let building_a = seed_building(&pool, org_a).await;
    let fault_one = seed_fault(&pool, org_a, building_a, user_a).await;
    let fault_two = seed_fault(&pool, org_a, building_a, user_a).await;
    let attachment_on_one = seed_attachment(&pool, fault_one, user_a).await;

    // Deleting via the wrong fault id removes nothing and leaves the row.
    let mismatched = repo
        .delete_attachment_for_fault(attachment_on_one, fault_two)
        .await
        .expect("query ok");
    assert!(
        !mismatched,
        "an attachment may not be deleted through a fault that does not own it"
    );
    let still_there: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fault_attachments WHERE id = $1")
            .bind(attachment_on_one)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        still_there, 1,
        "the attachment row must survive a scoped miss"
    );

    // Deleting via the owning fault id succeeds and removes the row.
    let deleted = repo
        .delete_attachment_for_fault(attachment_on_one, fault_one)
        .await
        .expect("query ok");
    assert!(deleted, "the owning fault may delete its attachment");
    let gone: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM fault_attachments WHERE id = $1")
        .bind(attachment_on_one)
        .fetch_one(&pool)
        .await
        .expect("query ok");
    assert_eq!(
        gone, 0,
        "the attachment row must be gone after a scoped delete"
    );
}

//! IDOR regression tests for `PUT /api/v1/inquiries/{id}/read` (PR #497).
//!
//! # Background
//!
//! Before PR #497 the `mark_as_read` route handler called
//! `mark_inquiry_read(id)` — a plain `UPDATE … WHERE id = $1` that had **no
//! ownership check**. Any authenticated realtor who knew (or guessed) a UUID
//! could silently mark a rival's inquiry as read. PR #497 replaced that call
//! with `mark_inquiry_read_for_realtor(id, realtor_id)`, which adds:
//!
//! ```sql
//! SELECT EXISTS(SELECT 1 FROM listing_inquiries
//!               WHERE id = $1 AND realtor_id = $2)
//! ```
//!
//! …and only updates when the caller is the owner.
//!
//! A companion IDOR in `respond_to_inquiry` (same handler file) was fixed in
//! PR #508: the repo now returns `Option<InquiryMessage>` and the handler maps
//! `None → 404`. Both IDOR fixes are exercised by the `db` crate tests in
//! `backend/crates/db/tests/inquiry_mark_read_idor_tests.rs`; this file covers
//! the repository-layer contracts that the route handlers depend on.
//!
//! # Test strategy
//!
//! We test the repository layer directly via [`RealityPortalRepository`],
//! which is the immediate target of the fix. Three invariants are exercised:
//!
//! | Case | Actor | Expected result |
//! |------|-------|-----------------|
//! | C1   | Realtor B marks realtor A's inquiry read | returns `false` → HTTP 404; `read_at` remains `NULL` |
//! | C2   | Realtor A marks own inquiry read | returns `true` → HTTP 204; `read_at` is set |
//! | C3   | Realtor A re-marks (idempotent) | returns `true` → HTTP 204 (already read is not an error) |
//!
//! Tests use `#[sqlx::test(migrator = "db::MIGRATOR")]` for an isolated,
//! migrated database — the same harness as every other integration test in
//! this suite.

#![allow(dead_code)]

use db::repositories::RealityPortalRepository;
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Seed helpers
// ============================================================================

/// Insert a minimal `organizations` row (required by `listings` FK).
async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("IDOR Org {slug}"))
    .bind(format!("idor-org-{slug}"))
    .bind(format!("{slug}@idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed_org failed")
}

/// Insert a minimal `users` row with `principal_kind = 'public'`.
///
/// Realtors in the Reality Portal are public-principal users; using 'public'
/// here keeps the seed faithful to production data.
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users
            (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'IDOR Test User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed_user failed")
}

/// Insert a minimal `listings` row owned by `created_by`.
///
/// The listing must exist because `listing_inquiries.listing_id` references it
/// with ON DELETE CASCADE.
async fn seed_listing(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'IDOR Test Listing', 'apartment', 100000.00, 'EUR',
                'Test Street 1', 'Bratislava', '81101', 'SK')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed_listing failed")
}

/// Insert a `listing_inquiries` row addressed to `realtor_id`.
///
/// Returns the inquiry id. `read_at` is intentionally `NULL` so each test
/// starts from the "unread" baseline.
async fn seed_inquiry(pool: &PgPool, listing_id: Uuid, realtor_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, name, email, message, inquiry_type, preferred_contact)
        VALUES ($1, $2, 'Buyer', 'buyer@idor.test', 'Is this still available?', 'info', 'email')
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(realtor_id)
    .fetch_one(pool)
    .await
    .expect("seed_inquiry failed")
}

/// Read the current `read_at` value for an inquiry (returns `None` when unset).
async fn get_read_at(pool: &PgPool, inquiry_id: Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT read_at FROM listing_inquiries WHERE id = $1",
    )
    .bind(inquiry_id)
    .fetch_one(pool)
    .await
    .expect("get_read_at failed")
}

// ============================================================================
// C1 — Realtor B cannot mark realtor A's inquiry as read (IDOR guard)
// ============================================================================

/// C1: `mark_inquiry_read_for_realtor(inquiry_owned_by_a, b_id)` must
/// return `false`, and `read_at` must stay `NULL`.
///
/// Before PR #497 this would silently return `Ok(())` and set `read_at`, which
/// is the IDOR. After the fix the ownership EXISTS-check returns no row for
/// `realtor_id = b_id`, so the function short-circuits and returns `false`
/// without touching the row.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn realtor_b_cannot_mark_realtor_a_inquiry_read(pool: PgPool) {
    let org = seed_org(&pool, "idor-c1").await;
    let realtor_a = seed_user(&pool, "realtor-a-c1@idor.test").await;
    let realtor_b = seed_user(&pool, "realtor-b-c1@idor.test").await;
    let listing = seed_listing(&pool, org, realtor_a).await;
    let inquiry_id = seed_inquiry(&pool, listing, realtor_a).await;

    let repo = RealityPortalRepository::new(pool.clone());

    // B attempts to mark A's inquiry read — must be rejected (returns false).
    let owned = repo
        .mark_inquiry_read_for_realtor(inquiry_id, realtor_b)
        .await
        .expect("mark_inquiry_read_for_realtor returned Err");

    assert!(
        !owned,
        "C1: mark_inquiry_read_for_realtor must return false when the inquiry \
         belongs to a different realtor (IDOR guard not triggered)"
    );

    // The row must not have been mutated — read_at must still be NULL.
    let read_at = get_read_at(&pool, inquiry_id).await;
    assert!(
        read_at.is_none(),
        "C1: read_at must remain NULL after a cross-realtor mark-read attempt; \
         got {:?} — the IDOR update ran despite the ownership check",
        read_at,
    );
}

// ============================================================================
// C2 — Realtor A marks their own inquiry as read → true, read_at is set
// ============================================================================

/// C2: `mark_inquiry_read_for_realtor(inquiry_owned_by_a, a_id)` must
/// return `true`, and `read_at` must be set to a non-NULL timestamp.
///
/// This is the happy-path: the legitimate owner can mark their own inquiry read.
/// The handler maps `true` → HTTP 204.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn realtor_a_can_mark_own_inquiry_read(pool: PgPool) {
    let org = seed_org(&pool, "idor-c2").await;
    let realtor_a = seed_user(&pool, "realtor-a-c2@idor.test").await;
    let listing = seed_listing(&pool, org, realtor_a).await;
    let inquiry_id = seed_inquiry(&pool, listing, realtor_a).await;

    let repo = RealityPortalRepository::new(pool.clone());

    let owned = repo
        .mark_inquiry_read_for_realtor(inquiry_id, realtor_a)
        .await
        .expect("mark_inquiry_read_for_realtor returned Err");

    assert!(
        owned,
        "C2: mark_inquiry_read_for_realtor must return true when the calling \
         realtor owns the inquiry"
    );

    // read_at must now be populated.
    let read_at = get_read_at(&pool, inquiry_id).await;
    assert!(
        read_at.is_some(),
        "C2: read_at must be set after the owner marks their inquiry read; \
         got NULL — the UPDATE did not execute"
    );
}

// ============================================================================
// C3 — Idempotent re-mark: calling a second time still returns true
// ============================================================================

/// C3: Calling `mark_inquiry_read_for_realtor` a second time on an already-read
/// inquiry must still return `true` (idempotent 204, not 404 or 500).
///
/// The implementation uses `WHERE read_at IS NULL` to guard the UPDATE, so the
/// second call is a no-op for the data row. But the ownership EXISTS-check
/// (against `listing_inquiries WHERE id = $1 AND realtor_id = $2`) still
/// succeeds — the function must therefore return `true` (owned) rather than
/// `false` (not-found/not-owned).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn idempotent_re_mark_returns_true(pool: PgPool) {
    let org = seed_org(&pool, "idor-c3").await;
    let realtor_a = seed_user(&pool, "realtor-a-c3@idor.test").await;
    let listing = seed_listing(&pool, org, realtor_a).await;
    let inquiry_id = seed_inquiry(&pool, listing, realtor_a).await;

    let repo = RealityPortalRepository::new(pool.clone());

    // First mark — sets read_at.
    let first = repo
        .mark_inquiry_read_for_realtor(inquiry_id, realtor_a)
        .await
        .expect("first mark returned Err");
    assert!(first, "C3: first mark must return true");

    let read_at_after_first = get_read_at(&pool, inquiry_id).await;
    assert!(
        read_at_after_first.is_some(),
        "C3: read_at must be set after the first mark"
    );

    // Second mark — inquiry is already read; ownership check still passes.
    let second = repo
        .mark_inquiry_read_for_realtor(inquiry_id, realtor_a)
        .await
        .expect("second mark returned Err");

    assert!(
        second,
        "C3: idempotent re-mark must return true (owned), not false (404). \
         The handler must return 204, not 404, when the inquiry is already read."
    );

    // read_at must not have been reset to NULL.
    let read_at_after_second = get_read_at(&pool, inquiry_id).await;
    assert!(
        read_at_after_second.is_some(),
        "C3: read_at must remain set after the idempotent re-mark"
    );
}

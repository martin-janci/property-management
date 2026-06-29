//! Pagination regression tests for the realtor inquiries list (#774 item 3).
//!
//! # Background
//!
//! `GET /api/v1/realtors/inquiries` (and the mirror `GET /api/v1/inquiries`)
//! returned `total = inquiries.len()` — i.e. the size of the *current page*,
//! not the real total. On every non-final page the client saw a `total` equal
//! to `limit`, so `total / limit` pagination silently broke.
//!
//! The fix wires the route to [`RealityPortalRepository::count_realtor_inquiries`],
//! which counts with the same status filter as
//! [`RealityPortalRepository::get_realtor_inquiries`]. These tests exercise the
//! repository contract the route now depends on:
//!
//! | Case | Setup | Expected |
//! |------|-------|----------|
//! | C1 | 5 inquiries, page of 2 | page len 2, **count 5** |
//! | C2 | status filter | count matches the filtered subset, not the page |
//!
//! Uses `#[sqlx::test(migrator = "db::MIGRATOR")]` — the same harness as the
//! IDOR suite in this directory.

#![allow(dead_code)]

use db::models::CreateListingInquiry;
use db::repositories::RealityPortalRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Insert a minimal `organizations` row (required by `listings` FK).
async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Pag Org {slug}"))
    .bind(format!("pag-org-{slug}"))
    .bind(format!("{slug}@pag.test"))
    .fetch_one(pool)
    .await
    .expect("seed_org failed")
}

/// Insert a minimal public-principal `users` row (a Reality Portal realtor).
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users
            (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Pag Test User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed_user failed")
}

/// Insert a minimal active `listings` row owned by `created_by`.
async fn seed_listing(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings
            (organization_id, created_by, status, transaction_type,
             title, property_type, price, currency,
             street, city, postal_code, country)
        VALUES ($1, $2, 'active', 'sale',
                'Pag Test Listing', 'apartment', 100000.00, 'EUR',
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

/// Create N inquiries addressed to `realtor_id` via the repository (the same
/// path production uses), each with the given inquiry_type/status default.
async fn seed_inquiries(
    repo: &RealityPortalRepository,
    listing_id: Uuid,
    realtor_id: Uuid,
    n: usize,
) {
    for i in 0..n {
        repo.create_inquiry(
            listing_id,
            realtor_id,
            None,
            CreateListingInquiry {
                name: format!("Buyer {i}"),
                email: format!("buyer{i}@pag.test"),
                phone: None,
                message: "Is this still available?".to_string(),
                inquiry_type: Some("info".to_string()),
                preferred_contact: Some("email".to_string()),
                preferred_time: None,
            },
        )
        .await
        .expect("create_inquiry failed");
    }
}

// ============================================================================
// C1 — count reflects the full total, not the current page length
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn count_returns_full_total_not_page_len(pool: PgPool) {
    let org = seed_org(&pool, "pag-c1").await;
    let realtor = seed_user(&pool, "realtor-c1@pag.test").await;
    let listing = seed_listing(&pool, org, realtor).await;

    let repo = RealityPortalRepository::new(pool.clone());
    seed_inquiries(&repo, listing, realtor, 5).await;

    // First page of 2.
    let page = repo
        .get_realtor_inquiries(realtor, None, 2, 0)
        .await
        .expect("get_realtor_inquiries failed");
    assert_eq!(page.len(), 2, "C1: page should hold exactly `limit` rows");

    // The real total must be 5, NOT the page length of 2 (the old bug).
    let total = repo
        .count_realtor_inquiries(realtor, None)
        .await
        .expect("count_realtor_inquiries failed");
    assert_eq!(
        total, 5,
        "C1: count must report the full total (5), not the page length (2)"
    );
}

// ============================================================================
// C2 — count respects the same status filter as the page query
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn count_respects_status_filter(pool: PgPool) {
    let org = seed_org(&pool, "pag-c2").await;
    let realtor = seed_user(&pool, "realtor-c2@pag.test").await;
    let listing = seed_listing(&pool, org, realtor).await;

    let repo = RealityPortalRepository::new(pool.clone());
    seed_inquiries(&repo, listing, realtor, 3).await;

    // Flip one inquiry to 'read' so the status partitions the set.
    let first = repo
        .get_realtor_inquiries(realtor, None, 1, 0)
        .await
        .expect("get_realtor_inquiries failed");
    let target = first[0].id;
    let owned = repo
        .mark_inquiry_read_for_realtor(target, realtor)
        .await
        .expect("mark_inquiry_read_for_realtor failed");
    assert!(owned, "C2: realtor owns the inquiry");

    // Unfiltered total is still 3.
    let total_all = repo
        .count_realtor_inquiries(realtor, None)
        .await
        .expect("count all failed");
    assert_eq!(total_all, 3, "C2: unfiltered count must be 3");

    // 'new' count drops to 2; 'read' count is 1.
    let total_new = repo
        .count_realtor_inquiries(realtor, Some("new".to_string()))
        .await
        .expect("count new failed");
    let total_read = repo
        .count_realtor_inquiries(realtor, Some("read".to_string()))
        .await
        .expect("count read failed");
    assert_eq!(total_new, 2, "C2: 'new' count must be 2 after one was read");
    assert_eq!(total_read, 1, "C2: 'read' count must be 1");
}

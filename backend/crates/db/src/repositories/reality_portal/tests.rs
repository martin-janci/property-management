//! Cross-realtor IDOR-guard tests for `respond_to_inquiry` (PR #498).
//!
//! Live-DB integration tests; tagged `#[ignore]`. Run with:
//! `cargo test -p db -- --ignored --test-threads=1`.

/// Cross-realtor IDOR guard on `respond_to_inquiry`.
///
/// These tests verify that the ownership check added in PR #498 is
/// preserved.  They run against a live database and are therefore tagged
/// `#[ignore]`; run them with:
///
/// ```bash
/// cargo test -p db -- --ignored --test-threads=1
/// ```
///
/// # What is tested
///
/// 1. `realtor_a_can_respond_to_own_inquiry` — the happy path.
///    Realtor A owns inquiry #1; calling `respond_to_inquiry` with
///    realtor_a's id returns `Ok(Some(_))`.
///
/// 2. `realtor_b_cannot_respond_to_realtor_a_inquiry` — the IDOR guard.
///    Realtor B calls `respond_to_inquiry` with inquiry #1 (owned by A).
///    The method MUST return `Ok(None)`.  If it returns `Ok(Some(_))` the
///    test fails, proving an IDOR regression.
///
/// Both tests also verify that no `inquiry_messages` row is written when
/// ownership fails.
use super::*;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

async fn test_pool() -> DbPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ppt_test".to_string());
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .expect("test db pool")
}

/// Seed a minimal user (no org required — portal users live outside orgs).
async fn seed_portal_user(pool: &DbPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Test Realtor', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed portal user")
}

/// Seed a minimal listing owned by `created_by`.
async fn seed_listing(pool: &DbPool, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings (
            created_by, title, description, property_type, transaction_type,
            price, currency, street, city, postal_code, country, status
        )
        VALUES (
            $1, 'Test Listing', 'desc', 'apartment', 'sale',
            100000, 'EUR', 'Test St 1', 'Bratislava', '81101', 'SK', 'active'
        )
        RETURNING id
        "#,
    )
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

/// Seed a listing inquiry owned by `realtor_id`.
async fn seed_inquiry(pool: &DbPool, listing_id: Uuid, realtor_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, name, email, message, inquiry_type, preferred_contact)
        VALUES ($1, $2, 'Inquirer', 'inquirer@test.sk', 'Hello', 'info', 'email')
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(realtor_id)
    .fetch_one(pool)
    .await
    .expect("seed inquiry")
}

async fn cleanup(pool: &DbPool, user_emails: &[&str]) {
    // Clean up in reverse dependency order.
    sqlx::query("DELETE FROM inquiry_messages WHERE TRUE")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM listing_inquiries WHERE TRUE")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM listings WHERE title = 'Test Listing'")
        .execute(pool)
        .await
        .ok();
    for email in user_emails {
        sqlx::query("DELETE FROM users WHERE email = $1")
            .bind(email)
            .execute(pool)
            .await
            .ok();
    }
}

#[tokio::test]
#[ignore]
async fn realtor_a_can_respond_to_own_inquiry() {
    let pool = test_pool().await;
    let emails = ["realtor_a_own@test.sk"];
    cleanup(&pool, &emails).await;

    let realtor_a = seed_portal_user(&pool, emails[0]).await;
    let listing = seed_listing(&pool, realtor_a).await;
    let inquiry_id = seed_inquiry(&pool, listing, realtor_a).await;

    let repo = RealityPortalRepository::new(pool.clone());
    let result = repo
        .respond_to_inquiry(inquiry_id, realtor_a, "Thank you for your interest!")
        .await
        .expect("repo call failed");

    assert!(
        result.is_some(),
        "Realtor A must be able to respond to their own inquiry"
    );

    cleanup(&pool, &emails).await;
}

#[tokio::test]
#[ignore]
async fn realtor_b_cannot_respond_to_realtor_a_inquiry() {
    let pool = test_pool().await;
    let emails = ["realtor_a_idor@test.sk", "realtor_b_idor@test.sk"];
    cleanup(&pool, &emails).await;

    let realtor_a = seed_portal_user(&pool, emails[0]).await;
    let realtor_b = seed_portal_user(&pool, emails[1]).await;
    let listing = seed_listing(&pool, realtor_a).await;
    let inquiry_id = seed_inquiry(&pool, listing, realtor_a).await;

    let repo = RealityPortalRepository::new(pool.clone());

    // Realtor B attempts to respond to realtor A's inquiry (IDOR attempt).
    let result = repo
        .respond_to_inquiry(inquiry_id, realtor_b, "Hijacked response!")
        .await
        .expect("repo call failed");

    assert!(
        result.is_none(),
        "Realtor B MUST NOT be able to respond to Realtor A's inquiry (IDOR)"
    );

    // Verify no message was inserted.
    let msg_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM inquiry_messages WHERE inquiry_id = $1")
            .bind(inquiry_id)
            .fetch_one(&pool)
            .await
            .expect("count query failed");

    assert_eq!(
        msg_count, 0,
        "No inquiry_messages row must be written when ownership check fails"
    );

    cleanup(&pool, &emails).await;
}

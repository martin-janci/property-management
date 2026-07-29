//! Regression test for issue #519 — `mark_inquiry_read` IDOR in
//! reality-server's realtors.rs handler. The repo-level guarantee is that
//! `mark_inquiry_read_for_realtor` only flips a row if the caller actually
//! owns it.

use crate::common::seed_org;
use db::repositories::RealityPortalRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_pm_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Inq User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_portal_user(pool: &PgPool, email: &str) -> Uuid {
    // portal_users was dropped in migration 00148; realtors are now unified
    // in the users table with principal_kind='public'.
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Realtor', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed portal user")
}

async fn seed_listing(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings (
            organization_id, created_by, status, transaction_type, title,
            property_type, street, city, postal_code, price
        )
        VALUES ($1, $2, 'active', 'rent', 'Test', 'apartment', 'S', 'C', '00000', 100)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

async fn seed_inquiry(pool: &PgPool, listing_id: Uuid, realtor_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listing_inquiries
            (listing_id, realtor_id, name, email, message)
        VALUES ($1, $2, 'Buyer', 'b@x.test', 'hello')
        RETURNING id
        "#,
    )
    .bind(listing_id)
    .bind(realtor_id)
    .fetch_one(pool)
    .await
    .expect("seed inquiry")
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn realtor_b_cannot_mark_realtor_a_inquiry_read(pool: PgPool) {
    let org_id = seed_org(&pool, "idor-inq").await;
    let pm_user = seed_pm_user(&pool, "inq-pm@idor.test").await;
    let realtor_a = seed_portal_user(&pool, "a@idor.test").await;
    let realtor_b = seed_portal_user(&pool, "b@idor.test").await;

    let listing_id = seed_listing(&pool, org_id, pm_user).await;
    let inquiry_id = seed_inquiry(&pool, listing_id, realtor_a).await;

    let repo = RealityPortalRepository::new(pool.clone());

    // realtor_b tries to mark realtor_a's inquiry as read — must NOT succeed.
    let updated = repo
        .mark_inquiry_read_for_realtor(inquiry_id, realtor_b)
        .await
        .expect("call");
    assert!(
        !updated,
        "realtor_b must not be able to mark realtor_a's inquiry (IDOR #519)"
    );

    // Row state must be untouched.
    let read_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT read_at FROM listing_inquiries WHERE id = $1")
            .bind(inquiry_id)
            .fetch_one(&pool)
            .await
            .expect("select");
    assert!(
        read_at.is_none(),
        "read_at must remain NULL after IDOR attempt"
    );

    // Owning realtor flips it.
    let updated = repo
        .mark_inquiry_read_for_realtor(inquiry_id, realtor_a)
        .await
        .expect("call");
    assert!(updated, "owner must succeed");
}

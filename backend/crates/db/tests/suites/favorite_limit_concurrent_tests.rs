//! TOCTOU guard for the per-user favorites cap (code-review finding:
//! reality-server favorites limit check and insert were not atomic).
//!
//! Many concurrent `add_favorite_capped` calls for distinct listings must never
//! leave the user with more than `max` favorites.

use db::repositories::{FavoriteError, RealityPortalRepository};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::seed_org;

async fn seed_portal_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Favorite User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed portal user")
}

async fn seed_listing(pool: &PgPool, org_id: Uuid, created_by: Uuid, title: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO listings (
            organization_id, created_by, transaction_type, title, property_type,
            street, city, postal_code, country, price, status
        )
        VALUES (
            $1, $2, 'sale', $3, 'apartment',
            'Main St', 'Bratislava', '81101', 'SK', 100000, 'active'
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .bind(title)
    .fetch_one(pool)
    .await
    .expect("seed listing")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn concurrent_add_favorite_never_exceeds_cap(pool: PgPool) {
    const MAX: i64 = 5;
    const ATTEMPTS: usize = 20;

    let repo = RealityPortalRepository::new(pool.clone());
    let org_id = seed_org(&pool, "fav-cap-org").await;
    let user_id = seed_portal_user(&pool, "fav-cap@test.sk").await;

    let mut listings = Vec::new();
    for i in 0..ATTEMPTS {
        listings.push(seed_listing(&pool, org_id, user_id, &format!("Cap flat {i}")).await);
    }

    let handles: Vec<_> = listings
        .into_iter()
        .map(|listing_id| {
            let repo = repo.clone();
            tokio::spawn(async move {
                repo.add_favorite_capped(user_id, listing_id, None, MAX)
                    .await
            })
        })
        .collect();

    let mut ok = 0;
    for h in handles {
        match h.await.expect("join") {
            Ok(_) => ok += 1,
            Err(FavoriteError::LimitReached { .. }) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM portal_favorites WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, MAX, "cap must hold under concurrency (rows stored)");
    assert_eq!(ok as i64, MAX, "exactly MAX inserts may succeed");
}

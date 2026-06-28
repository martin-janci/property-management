//! Favorite alert worker tests (BIT-138).
//!
//! Verifies the worker can queue favorite price-change and back-on-market
//! alerts even though `portal_favorites` and `listing_price_history` are under
//! FORCE RLS. The worker must iterate orgs and set tenant context explicitly.

#![allow(dead_code)]

use db::models::listing_status;
use db::repositories::RealityPortalRepository;
use reality_server::services::{FavoriteAlertConfig, FavoriteAlertWorker};
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Org {slug}"))
    .bind(slug)
    .bind(format!("{slug}@example.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

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
async fn worker_queues_price_and_back_on_market_alerts(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let worker = FavoriteAlertWorker::new(
        pool.clone(),
        FavoriteAlertConfig {
            enabled: true,
            poll_interval_secs: 3600,
        },
    );

    let org_id = seed_org(&pool, "favorite-alerts-org").await;
    let user_id = seed_portal_user(&pool, "favorite-alerts@test.sk").await;
    let listing_id = seed_listing(&pool, org_id, user_id, "Riverfront flat").await;

    repo.add_favorite(user_id, listing_id, None)
        .await
        .expect("add favorite");

    // Price drop: trigger history row and queue one alert.
    sqlx::query("UPDATE listings SET price = 90000 WHERE id = $1")
        .bind(listing_id)
        .execute(&pool)
        .await
        .expect("drop price");

    // First status transition updates the snapshot only — no alert while the
    // listing moves off market.
    sqlx::query("UPDATE listings SET status = $2 WHERE id = $1")
        .bind(listing_id)
        .bind(listing_status::PAUSED)
        .execute(&pool)
        .await
        .expect("pause listing");

    worker.run_once().await;

    let first_pass = repo
        .get_favorite_alerts(user_id, 20, 0)
        .await
        .expect("list alerts after first pass");
    assert_eq!(
        first_pass.len(),
        1,
        "first pass should queue only the price-change alert"
    );
    assert_eq!(first_pass[0].alert_type, "price_change");
    assert_eq!(
        repo.count_pending_favorite_alerts(user_id).await.unwrap(),
        1
    );

    // Second transition back to active should enqueue a back-on-market alert.
    sqlx::query("UPDATE listings SET status = $2 WHERE id = $1")
        .bind(listing_id)
        .bind(listing_status::ACTIVE)
        .execute(&pool)
        .await
        .expect("reactivate listing");

    worker.run_once().await;

    let alerts = repo
        .get_favorite_alerts(user_id, 20, 0)
        .await
        .expect("list alerts");
    assert_eq!(alerts.len(), 2, "expected one price and one status alert");
    assert!(
        alerts.iter().any(|a| a.alert_type == "price_change"),
        "price alert missing from queue"
    );
    assert!(
        alerts.iter().any(|a| {
            a.alert_type == "back_on_market"
                && a.previous_status.as_deref() == Some(listing_status::PAUSED)
                && a.new_status.as_deref() == Some(listing_status::ACTIVE)
        }),
        "back-on-market alert missing from queue"
    );
}

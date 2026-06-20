//! Saved-search alert *delivery* tests (Story 16.3, issue #983).
//!
//! The background matching engine (`SavedSearchAlertWorker`) enqueues
//! `search_alert_queue` rows; these tests cover the in-app delivery surface that
//! lets a portal user retrieve and acknowledge their alerts:
//! `get_search_alerts` / `count_pending_search_alerts` /
//! `mark_search_alert_read` / `mark_all_search_alerts_read`.
//!
//! Run against a live Postgres (sqlx spins up a per-test database):
//! `DATABASE_URL=... cargo test -p db --test search_alert_delivery_tests`.

use db::models::CreatePortalSavedSearch;
use db::repositories::RealityPortalRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed a portal user and return its id. Portal users live in `users`
/// (`principal_kind = 'public'`) since migration 00148 dropped `portal_users` and
/// re-pointed `portal_saved_searches` / `search_alert_queue` at `users(id)`.
async fn seed_portal_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Alert Tester', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({email}): {e}"))
}

async fn seed_saved_search(repo: &RealityPortalRepository, user_id: Uuid, name: &str) -> Uuid {
    repo.create_saved_search(
        user_id,
        CreatePortalSavedSearch {
            name: name.to_string(),
            criteria: serde_json::json!({ "city": "Bratislava" }),
            alerts_enabled: true,
            alert_frequency: "daily".to_string(),
        },
    )
    .await
    .expect("create saved search")
    .id
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_alert_delivery_lifecycle(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let user = seed_portal_user(&pool, "alerts-lifecycle@test.sk").await;
    let search_id = seed_saved_search(&repo, user, "Bratislava flats").await;

    let l1 = Uuid::new_v4();
    let l2 = Uuid::new_v4();
    repo.enqueue_search_alert(&pool, search_id, user, &[l1, l2], "new_listing")
        .await
        .expect("enqueue alert");

    // List: one pending alert, joined to its search name, carrying the matches.
    let alerts = repo
        .get_search_alerts(user, 100)
        .await
        .expect("list alerts");
    assert_eq!(alerts.len(), 1, "expected exactly one alert");
    let alert = &alerts[0];
    assert_eq!(alert.saved_search_name, "Bratislava flats");
    assert_eq!(alert.matching_listing_ids, vec![l1, l2]);
    assert_eq!(alert.alert_type, "new_listing");
    assert_eq!(alert.status, "pending");
    assert!(alert.processed_at.is_none());
    assert_eq!(
        repo.count_pending_search_alerts(user).await.unwrap(),
        1,
        "one unread alert"
    );

    // Acknowledge it: pending -> sent.
    assert!(
        repo.mark_search_alert_read(alert.id, user).await.unwrap(),
        "marking an owned pending alert returns true"
    );
    assert_eq!(
        repo.count_pending_search_alerts(user).await.unwrap(),
        0,
        "no unread alerts after ack"
    );
    let after = repo.get_search_alerts(user, 100).await.unwrap();
    assert_eq!(after[0].status, "sent");
    assert!(after[0].processed_at.is_some());

    // Idempotent: a second ack of an already-sent alert reports no change.
    assert!(
        !repo.mark_search_alert_read(alert.id, user).await.unwrap(),
        "re-acking an already-sent alert returns false"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn search_alert_read_is_owner_scoped(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let owner = seed_portal_user(&pool, "alerts-owner@test.sk").await;
    let attacker = seed_portal_user(&pool, "alerts-attacker@test.sk").await;
    let search_id = seed_saved_search(&repo, owner, "Owner search").await;
    repo.enqueue_search_alert(&pool, search_id, owner, &[Uuid::new_v4()], "new_listing")
        .await
        .expect("enqueue alert");
    let alert_id = repo.get_search_alerts(owner, 100).await.unwrap()[0].id;

    // The attacker cannot ack the owner's alert (no IDOR) and sees none of it.
    assert!(
        !repo
            .mark_search_alert_read(alert_id, attacker)
            .await
            .unwrap(),
        "a non-owner must not be able to mark another user's alert read"
    );
    assert!(
        repo.get_search_alerts(attacker, 100)
            .await
            .unwrap()
            .is_empty(),
        "a user only sees their own alerts"
    );
    assert_eq!(
        repo.count_pending_search_alerts(owner).await.unwrap(),
        1,
        "the owner's alert is still pending after the failed cross-user ack"
    );

    // mark-all is owner-scoped too.
    assert_eq!(
        repo.mark_all_search_alerts_read(attacker).await.unwrap(),
        0,
        "mark-all touches nothing for a user with no alerts"
    );
    assert_eq!(
        repo.mark_all_search_alerts_read(owner).await.unwrap(),
        1,
        "mark-all clears the owner's pending alert"
    );
    assert_eq!(repo.count_pending_search_alerts(owner).await.unwrap(), 0);
}

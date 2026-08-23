//! Regression test for issue #2831 — the quiet-hours drain double-delivered
//! held notifications under more than one api-server replica.
//!
//! The drain worker runs inside every api-server process. Before the fix it
//! selected due rows with a plain `SELECT ... WHERE released_at IS NULL`
//! (`get_notifications_to_release`), which claimed nothing: two replicas polling
//! the same cadence both read the SAME due rows, both delivered them, and both
//! marked them released. Every held notification was delivered once per replica.
//!
//! The fix replaces the read with an atomic claim
//! (`claim_notifications_to_release`): a single
//! `UPDATE ... WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING *`
//! stamps `claimed_at` and returns only the rows this caller won, so a due row
//! is handed to at most one replica. These tests drive the real repository
//! method against Postgres and assert that guarantee directly.

use std::collections::HashSet;

use chrono::{Duration, Utc};
use db::models::CreateHeldNotification;
use db::repositories::GranularNotificationRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed an `active` user and return its id (held_notifications.user_id FKs to
/// users(id)).
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Drain User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a held notification whose quiet-hours window has already ended, so the
/// drain claim query considers it due for release.
async fn seed_due_held(repo: &GranularNotificationRepository, user_id: Uuid) -> Uuid {
    repo.create_held_notification(CreateHeldNotification {
        user_id,
        event_type: "fault".to_string(),
        title: "Held during quiet hours".to_string(),
        body: Some("body".to_string()),
        data: None,
        channels: vec!["push".to_string()],
        // release_at in the past => due now.
        release_at: Utc::now() - Duration::hours(1),
        is_priority: false,
    })
    .await
    .expect("seed held notification")
    .id
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn claim_hands_each_due_row_to_at_most_one_replica(pool: PgPool) {
    let repo = GranularNotificationRepository::new(pool.clone());
    let user_id = seed_user(&pool, "drain-claim@users.test").await;

    let id_a = seed_due_held(&repo, user_id).await;
    let id_b = seed_due_held(&repo, user_id).await;

    // Replica 1 polls first and claims the due rows.
    let replica1 = repo
        .claim_notifications_to_release(500, 300)
        .await
        .expect("replica 1 claim");
    // Replica 2 polls immediately after, within the claim lease.
    let replica2 = repo
        .claim_notifications_to_release(500, 300)
        .await
        .expect("replica 2 claim");

    let claimed1: HashSet<Uuid> = replica1.iter().map(|h| h.id).collect();
    let claimed2: HashSet<Uuid> = replica2.iter().map(|h| h.id).collect();

    // The regression: on `main` both replicas got both rows (double delivery).
    // With the claim, replica 1 owns both and replica 2 sees nothing to do.
    assert_eq!(
        claimed1,
        HashSet::from([id_a, id_b]),
        "the first replica to poll claims every due row"
    );
    assert!(
        claimed2.is_empty(),
        "a second replica within the lease must not re-claim already-claimed rows; got {claimed2:?}"
    );

    // No row is ever claimed by both replicas — the at-most-once invariant.
    assert!(
        claimed1.is_disjoint(&claimed2),
        "no held notification may be claimed by two replicas at once"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn recording_a_partial_attempt_clears_the_claim_for_prompt_retry(pool: PgPool) {
    let repo = GranularNotificationRepository::new(pool.clone());
    let user_id = seed_user(&pool, "drain-retry@users.test").await;
    let id = seed_due_held(&repo, user_id).await;

    // Tick 1: a replica claims the row.
    let first = repo
        .claim_notifications_to_release(500, 300)
        .await
        .expect("first claim");
    assert_eq!(
        first.iter().map(|h| h.id).collect::<Vec<_>>(),
        vec![id],
        "the due row is claimed on the first poll"
    );

    // A sibling channel failed, so the worker persists per-channel progress and
    // keeps the row held. record_held_attempt must clear the claim so the row is
    // re-claimable on the very next tick instead of waiting out the lease.
    repo.record_held_attempt(id, &["push".to_string()], 1)
        .await
        .expect("record partial attempt");

    // Next tick, even with a long lease, the row is claimable again because its
    // claim was cleared (not because the lease expired).
    let retry = repo
        .claim_notifications_to_release(500, 3600)
        .await
        .expect("retry claim");
    assert_eq!(
        retry.iter().map(|h| h.id).collect::<Vec<_>>(),
        vec![id],
        "clearing the claim on a partial-failure retry lets the next tick re-claim promptly"
    );
    // The per-channel progress is preserved so the retry won't re-deliver push.
    assert_eq!(retry[0].delivered_channels, vec!["push".to_string()]);
    assert_eq!(retry[0].attempts, 1);
}

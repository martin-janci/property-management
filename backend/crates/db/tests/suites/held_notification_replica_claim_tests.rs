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

/// Two api-server replicas running the drain worker *concurrently* against the
/// same held-notification batch must deliver each row at most once.
///
/// The sibling `claim_hands_each_due_row_to_at_most_one_replica` drives the two
/// claims sequentially (replica 1 wins every row before replica 2 polls), which
/// proves the claim *excludes* an already-claimed row but never actually races
/// two claimers on the queue. This test does: both replicas run the real drain
/// loop — claim a small batch under `FOR UPDATE SKIP LOCKED`, "deliver" it
/// (mark released, so it leaves the candidate set), repeat until the queue
/// drains — on separate pool connections at the same time, contending for the
/// same rows across many rounds.
///
/// The invariants encode at-most-once delivery under >1 replica:
///   * no held notification is delivered by BOTH replicas (disjoint sets) — the
///     regression: on `main`, a plain `SELECT ... WHERE released_at IS NULL`
///     handed every due row to *both* loops, so this intersection was non-empty;
///   * every due row is delivered exactly once overall (the atomic claim must
///     not drop a row either — at-least-once, no loss);
///   * the total number of deliveries across the two replicas equals the number
///     of seeded rows — no duplicate, no gap.
///
/// A deliberately long claim lease (never expires during the test) means the
/// ONLY way the peer replica could pick up a row this replica already claimed is
/// if the claim were non-atomic — so any double delivery here is a real
/// concurrency defect, not a lease-expiry re-claim.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn two_concurrent_replicas_deliver_each_held_row_at_most_once(pool: PgPool) {
    let repo = GranularNotificationRepository::new(pool.clone());
    let user_id = seed_user(&pool, "drain-race@users.test").await;

    // Enough due rows that two claimers each draining in small rounds genuinely
    // interleave on the shared queue rather than one finishing before the other
    // starts.
    const N: usize = 40;
    let mut seeded: HashSet<Uuid> = HashSet::with_capacity(N);
    for _ in 0..N {
        seeded.insert(seed_due_held(&repo, user_id).await);
    }

    // The drain loop each replica runs: claim a small batch (forces several
    // rounds and real interleaving), deliver-then-release each claimed row
    // exactly as `QuietHoursDrainWorker` does on a clean delivery, repeat until
    // the queue is empty. Returns the ids this replica delivered.
    async fn drain_replica(pool: PgPool) -> Vec<Uuid> {
        let repo = GranularNotificationRepository::new(pool);
        let mut delivered = Vec::new();
        loop {
            // Small batch limit; a 1-hour lease that never lapses in-test, so a
            // claimed-but-not-yet-released row can never be re-claimed by the peer.
            let claimed = repo
                .claim_notifications_to_release(7, 3600)
                .await
                .expect("claim due held notifications");
            if claimed.is_empty() {
                break;
            }
            for held in claimed {
                repo.mark_notification_released(held.id)
                    .await
                    .expect("mark released");
                delivered.push(held.id);
            }
        }
        delivered
    }

    let replica_a = tokio::spawn(drain_replica(pool.clone()));
    let replica_b = tokio::spawn(drain_replica(pool.clone()));
    let delivered_a = replica_a.await.expect("replica A join");
    let delivered_b = replica_b.await.expect("replica B join");

    let set_a: HashSet<Uuid> = delivered_a.iter().copied().collect();
    let set_b: HashSet<Uuid> = delivered_b.iter().copied().collect();

    // No replica delivered the same row twice within its own loop (each claim
    // batch returns distinct rows and each is released once).
    assert_eq!(
        set_a.len(),
        delivered_a.len(),
        "replica A delivered a row more than once"
    );
    assert_eq!(
        set_b.len(),
        delivered_b.len(),
        "replica B delivered a row more than once"
    );

    // Core at-most-once invariant across replicas: no row delivered by both.
    assert!(
        set_a.is_disjoint(&set_b),
        "a held notification was delivered by BOTH replicas (double delivery): {:?}",
        set_a.intersection(&set_b).collect::<Vec<_>>()
    );

    // Exactly-once overall: the atomic claim must not drop a row either.
    let union: HashSet<Uuid> = set_a.union(&set_b).copied().collect();
    assert_eq!(
        union, seeded,
        "every due row must be delivered exactly once across the two replicas"
    );
    assert_eq!(
        delivered_a.len() + delivered_b.len(),
        N,
        "total deliveries across both replicas must equal the {N} held rows — \
         at-most-once with no loss"
    );

    // DB ground truth: every held row is released exactly once.
    let released: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM held_notifications WHERE user_id = $1 AND released_at IS NOT NULL",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("count released rows");
    assert_eq!(
        released, N as i64,
        "all held rows must be released exactly once"
    );
}

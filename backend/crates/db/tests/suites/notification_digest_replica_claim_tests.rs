//! Regression test for the inactivity-digest worker double-sending digests
//! under more than one api-server replica (Story 2B.3).
//!
//! The `NotificationDigestWorker` runs inside every api-server process. Before
//! the fix its per-tick sweep claimed nothing: two replicas polling the same
//! cadence both selected the same inactivity candidates and both
//! `create_digest` + emailed them (duplicate rows *and* duplicate emails), and
//! both fetched the same unsent rows via `get_unsent_digests` and re-emailed
//! them. Every eligible user was digested once per replica.
//!
//! The fix guards the whole sweep with a single cross-replica Postgres advisory
//! lock (`notification_digest_sweep`). Exactly one replica claims it per tick
//! and runs the sweep; peers skip. These tests drive the real repository claim
//! methods against Postgres and assert that guarantee directly — mirroring the
//! held-notification replica-claim suite.

use std::sync::Arc;

use db::repositories::GranularNotificationRepository;
use sqlx::PgPool;
use tokio::sync::Barrier;

/// The sweep lock is mutually exclusive across replicas: while one replica
/// holds it a second claim is refused, and once the holder releases it the
/// lease becomes available again.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn digest_sweep_lock_is_held_by_at_most_one_replica(pool: PgPool) {
    // Two independent repository handles over the same pool stand in for two
    // api-server replicas: each `try_claim_digest_sweep` call takes its own
    // pooled connection (its own Postgres session), exactly as two processes
    // would.
    let replica_a = GranularNotificationRepository::new(pool.clone());
    let replica_b = GranularNotificationRepository::new(pool.clone());

    // Replica A wins the sweep lease for this tick.
    let claim_a = replica_a
        .try_claim_digest_sweep()
        .await
        .expect("replica A claim query");
    assert!(
        claim_a.is_some(),
        "the first replica must win the uncontended sweep lock"
    );
    let lock_conn = claim_a.unwrap();

    // Replica B polling the same tick is refused — so it cannot run the sweep
    // and cannot send a second copy of every digest.
    let claim_b = replica_b
        .try_claim_digest_sweep()
        .await
        .expect("replica B claim query");
    assert!(
        claim_b.is_none(),
        "a second replica must NOT win the sweep lock while a peer holds it \
         (this is the double-send the fix prevents)"
    );

    // Replica A finishes its sweep and releases the lease.
    replica_a
        .release_digest_sweep(lock_conn)
        .await
        .expect("release sweep lock");

    // A later tick (either replica) can now claim it again.
    let claim_next = replica_b
        .try_claim_digest_sweep()
        .await
        .expect("post-release claim query");
    assert!(
        claim_next.is_some(),
        "the sweep lock must be reclaimable once the holder releases it"
    );
    replica_b
        .release_digest_sweep(claim_next.unwrap())
        .await
        .expect("release sweep lock again");
}

/// Two replicas racing to start the same tick: at most one wins the sweep. The
/// loser backing off (skipping its tick) is exactly what keeps digests from
/// being emailed twice.
///
/// Deterministic: `pg_try_advisory_lock` is atomic, so two concurrent sessions
/// never both acquire the same key. A barrier holds the winner's lease until
/// *both* replicas have attempted their claim, so the loser can never slip in
/// after an early release and spuriously "win" too.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn concurrent_replicas_grant_the_sweep_to_at_most_one(pool: PgPool) {
    let barrier = Arc::new(Barrier::new(2));

    async fn attempt(pool: PgPool, barrier: Arc<Barrier>) -> bool {
        let repo = GranularNotificationRepository::new(pool);
        let claim = repo
            .try_claim_digest_sweep()
            .await
            .expect("claim sweep lock");
        let won = claim.is_some();
        // Neither replica releases until both have attempted their claim.
        barrier.wait().await;
        if let Some(conn) = claim {
            repo.release_digest_sweep(conn)
                .await
                .expect("release sweep lock");
        }
        won
    }

    let a = tokio::spawn(attempt(pool.clone(), barrier.clone()));
    let b = tokio::spawn(attempt(pool.clone(), barrier.clone()));
    let won_a = a.await.expect("join replica A");
    let won_b = b.await.expect("join replica B");

    // Exactly one replica ran the sweep this tick; never both.
    assert!(
        won_a ^ won_b,
        "exactly one replica must run the sweep (won_a={won_a}, won_b={won_b})"
    );
}

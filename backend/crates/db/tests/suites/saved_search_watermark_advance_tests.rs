//! Repository round-trip guard for the saved-search alert watermark-advance
//! atomicity contract (#983 code-review finding).
//!
//! Why this test exists
//! --------------------
//! `SavedSearchAlertWorker` used to enqueue a `search_alert_queue` row and then
//! advance the search's `last_matched_at` / `match_count` watermark as two
//! separate autocommitted statements, discarding the advance error with
//! `let _ = …`. When the enqueue committed but the advance failed, the alert
//! stayed `pending` in the queue while the watermark never moved — so the next
//! due run re-found the same listings and enqueued a **duplicate** alert.
//!
//! `RealityPortalRepository::enqueue_and_advance_saved_search` fixes that by
//! committing both writes in a single transaction. This test guards that
//! contract directly against the DB, independent of the worker's poll loop:
//!  - on success both writes land, and
//!  - when the advance fails the enqueue is rolled back, so a subsequent
//!    successful run enqueues exactly **one** alert (no duplicate) rather than
//!    leaving an orphaned queue row behind.
//!
//! `#[sqlx::test]` connects as the Postgres SUPERUSER (bypassing RLS), and
//! `portal_saved_searches` / `search_alert_queue` are not RLS-gated anyway, so
//! no tenant context is needed for this behavioral check.

use db::models::CreatePortalSavedSearch;
use db::repositories::RealityPortalRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_portal_user(pool: &PgPool, email: &str) -> Uuid {
    // portal_users was merged into `users` (migration 00148); portal users are
    // rows with principal_kind='public'.
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Watermark Tester', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({email}): {e}"))
}

async fn seed_saved_search(repo: &RealityPortalRepository, user_id: Uuid) -> Uuid {
    repo.create_saved_search(
        user_id,
        CreatePortalSavedSearch {
            name: "Bratislava flats".to_string(),
            criteria: serde_json::json!({ "city": "Bratislava" }),
            alerts_enabled: true,
            alert_frequency: "daily".to_string(),
        },
    )
    .await
    .expect("create saved search")
    .id
}

async fn match_count(pool: &PgPool, search_id: Uuid) -> i64 {
    // `match_count` is `bigint` (i64) since migration 00232 (#2814) — was
    // `int4` before, which overflowed and permanently wedged high-traffic
    // saved searches once the alert-advance became atomic.
    sqlx::query_scalar::<_, i64>("SELECT match_count FROM portal_saved_searches WHERE id = $1")
        .bind(search_id)
        .fetch_one(pool)
        .await
        .expect("read match_count")
}

async fn last_matched_at_is_set(pool: &PgPool, search_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT last_matched_at IS NOT NULL FROM portal_saved_searches WHERE id = $1",
    )
    .bind(search_id)
    .fetch_one(pool)
    .await
    .expect("read last_matched_at")
}

/// On success, the enqueue and the watermark advance both land: one pending
/// alert is queued, `match_count` grows by the number of matches, and
/// `last_matched_at` is stamped.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn enqueue_and_advance_commits_both_writes(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let user = seed_portal_user(&pool, "watermark-ok@test.sk").await;
    let search = seed_saved_search(&repo, user).await;

    let l1 = Uuid::new_v4();
    let l2 = Uuid::new_v4();
    repo.enqueue_and_advance_saved_search(search, user, &[l1, l2])
        .await
        .expect("enqueue + advance should commit");

    assert_eq!(
        repo.count_pending_search_alerts(user).await.unwrap(),
        1,
        "exactly one alert should be queued",
    );
    assert_eq!(
        match_count(&pool, search).await,
        2_i64,
        "match_count must advance by the number of matched listings",
    );
    assert!(
        last_matched_at_is_set(&pool, search).await,
        "the watermark timestamp must be stamped after a completed scan",
    );
}

/// A no-match run advances only the watermark (no queue row), so the cadence
/// window still restarts.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn enqueue_and_advance_with_no_matches_only_advances_watermark(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let user = seed_portal_user(&pool, "watermark-nomatch@test.sk").await;
    let search = seed_saved_search(&repo, user).await;

    repo.enqueue_and_advance_saved_search(search, user, &[])
        .await
        .expect("no-match advance should commit");

    assert_eq!(
        repo.count_pending_search_alerts(user).await.unwrap(),
        0,
        "a no-match run must not queue an alert",
    );
    assert!(
        last_matched_at_is_set(&pool, search).await,
        "a no-match run must still advance the watermark",
    );
}

/// Regression (#983): if the watermark advance fails, the enqueue must roll back
/// so no orphaned `pending` alert is left behind. We force the advance to fail
/// by pushing `match_count` to `i64::MAX`, so the `match_count + N` update
/// overflows `bigint` and aborts the transaction. The pre-enqueued alert row
/// must NOT survive, and a subsequent successful run must then queue exactly one
/// alert — not two — proving a failed advance never causes a duplicate send.
///
/// Note: pre-#2814 the column was `int4` and this test primed to `i32::MAX`.
/// Since migration 00232 widened it to `bigint`, `i32::MAX + N` fits fine —
/// we still exercise the same rollback contract by climbing to `i64::MAX`.
/// The realism of the prime doesn't matter; what matters is that a DB-side
/// arithmetic failure inside the tx rolls both writes back.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn failed_watermark_advance_rolls_back_enqueue(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let user = seed_portal_user(&pool, "watermark-fail@test.sk").await;
    let search = seed_saved_search(&repo, user).await;

    // Force the watermark advance (`match_count = match_count + N`) to overflow
    // bigint and abort the transaction.
    sqlx::query("UPDATE portal_saved_searches SET match_count = $2 WHERE id = $1")
        .bind(search)
        .bind(i64::MAX)
        .execute(&pool)
        .await
        .expect("prime match_count to i64::MAX");

    let listing = Uuid::new_v4();
    let err = repo
        .enqueue_and_advance_saved_search(search, user, &[listing])
        .await
        .expect_err("advance overflow must surface as an error, not be swallowed");
    // Sanity: it is a DB error (the overflow), not some other failure.
    let _ = err;

    assert_eq!(
        repo.count_pending_search_alerts(user).await.unwrap(),
        0,
        "a failed watermark advance must roll back the enqueue — no orphaned alert",
    );
    assert_eq!(
        match_count(&pool, search).await,
        i64::MAX,
        "the watermark must be unchanged after the aborted transaction",
    );

    // Reset the counter and re-run: the window is retried and enqueues exactly
    // one alert. On the pre-fix (non-atomic) behavior the rolled-back run would
    // have left an orphan and this would observe two.
    sqlx::query("UPDATE portal_saved_searches SET match_count = 0 WHERE id = $1")
        .bind(search)
        .execute(&pool)
        .await
        .expect("reset match_count");

    repo.enqueue_and_advance_saved_search(search, user, &[listing])
        .await
        .expect("retry run should commit");

    assert_eq!(
        repo.count_pending_search_alerts(user).await.unwrap(),
        1,
        "the retry must queue exactly one alert — a failed advance must not duplicate",
    );
}

/// Regression (#2814): a saved search whose accumulated `match_count` is
/// already past the old `int4` ceiling MUST continue to advance its watermark
/// and deliver alerts. Before migration 00232 widened the column to `bigint`,
/// `match_count + N` overflowed `integer` and the atomic-run contract
/// (introduced by PR #2812) rolled the whole run back — so the search stopped
/// delivering alerts forever with no self-recovery. This test locks the fix
/// in: prime `match_count` just past `i32::MAX`, run one more advance, and
/// prove the run commits (watermark timestamp stamped, count grows, one alert
/// queued) rather than being rolled back.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn watermark_advances_past_i32_max_boundary(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let user = seed_portal_user(&pool, "watermark-past-i32-max@test.sk").await;
    let search = seed_saved_search(&repo, user).await;

    // Prime to a value that would have overflowed the pre-#2814 `int4` column
    // on any non-trivial advance. `bigint` swallows this comfortably.
    let primed: i64 = i32::MAX as i64;
    sqlx::query("UPDATE portal_saved_searches SET match_count = $2 WHERE id = $1")
        .bind(search)
        .bind(primed)
        .execute(&pool)
        .await
        .expect("prime match_count past i32::MAX");

    let listing = Uuid::new_v4();
    repo.enqueue_and_advance_saved_search(search, user, &[listing])
        .await
        .expect(
            "advance past i32::MAX must commit — the pre-widening column overflowed here and \
             permanently wedged the search",
        );

    assert_eq!(
        repo.count_pending_search_alerts(user).await.unwrap(),
        1,
        "the run must queue exactly one alert once the ceiling is gone",
    );
    assert_eq!(
        match_count(&pool, search).await,
        primed + 1,
        "match_count must grow past i32::MAX now that the column is bigint",
    );
    assert!(
        last_matched_at_is_set(&pool, search).await,
        "the watermark timestamp must be stamped so the cadence window restarts",
    );
}

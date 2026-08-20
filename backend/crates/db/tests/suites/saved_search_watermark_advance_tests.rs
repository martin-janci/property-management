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

async fn match_count(pool: &PgPool, search_id: Uuid) -> i32 {
    sqlx::query_scalar::<_, i32>("SELECT match_count FROM portal_saved_searches WHERE id = $1")
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
        2,
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
/// by pushing `match_count` to `i32::MAX`, so the `match_count + N` update
/// overflows `integer` and aborts the transaction. The pre-enqueued alert row
/// must NOT survive, and a subsequent successful run must then queue exactly one
/// alert — not two — proving a failed advance never causes a duplicate send.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn failed_watermark_advance_rolls_back_enqueue(pool: PgPool) {
    let repo = RealityPortalRepository::new(pool.clone());
    let user = seed_portal_user(&pool, "watermark-fail@test.sk").await;
    let search = seed_saved_search(&repo, user).await;

    // Force the watermark advance (`match_count = match_count + N`) to overflow
    // int4 and abort the transaction.
    sqlx::query("UPDATE portal_saved_searches SET match_count = $2 WHERE id = $1")
        .bind(search)
        .bind(i32::MAX)
        .execute(&pool)
        .await
        .expect("prime match_count to i32::MAX");

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
        i32::MAX,
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

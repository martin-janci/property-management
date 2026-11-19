//! DB-backed regression tests for the vote-reminder dedup helpers
//! (`VoteRepository::find_votes_due_for_reminder`, `mark_vote_reminder_sent`).
//!
//! IG3 evidence for the vote-reminder spam fix: `send_vote_reminders` ran on a
//! ~60s scheduler tick and re-selected every active vote ending inside the
//! reminder window on every tick, re-notifying all non-voters each time. There
//! was no guard, so owners were spammed with a reminder every minute for the
//! whole window. The fix adds `votes.last_reminder_at` plus a ~5-minute dedup
//! predicate (mirroring `invoices.last_payment_reminder_at`, #1790).
//!
//! These tests seed real votes around the reminder window and call the actual
//! repo methods. They FAIL on pre-fix code, where the `last_reminder_at`
//! column and `mark_vote_reminder_sent` don't exist (won't compile) and the
//! vote would be returned on every sweep.
//!
//! Run against a live Postgres (sqlx spins up a per-test database):
//! `DATABASE_URL=... cargo test -p db --test suite_3 vote_reminder_dedup`.

use db::repositories::VoteRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $2, $3, 'active') RETURNING id",
    )
    .bind(format!("Org {slug}"))
    .bind(slug)
    .bind(format!("{slug}@vote-reminder.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', 'Creator', 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, 'B', 'Addr', 'City', '00000', 'SK') RETURNING id",
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Active vote ending `hours_from_now` from now (negative = already ended).
async fn seed_active_vote(
    pool: &PgPool,
    org: Uuid,
    building: Uuid,
    creator: Uuid,
    title: &str,
    hours_from_now: i64,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO votes \
            (organization_id, building_id, title, start_at, end_at, status, \
             created_by, published_by, published_at) \
         VALUES ($1, $2, $3, NOW() - INTERVAL '1 hour', \
                 NOW() + make_interval(mins => $4), 'active', $5, $5, NOW()) \
         RETURNING id",
    )
    .bind(org)
    .bind(building)
    .bind(title)
    .bind((hours_from_now * 60) as i32)
    .bind(creator)
    .fetch_one(pool)
    .await
    .expect("seed vote")
}

fn ids(votes: &[db::models::Vote]) -> Vec<Uuid> {
    votes.iter().map(|v| v.id).collect()
}

/// IG3 (core): once an in-window active vote is stamped via
/// `mark_vote_reminder_sent`, it must drop out of the reminder sweep for the
/// ~5-minute cooldown — proving the persistent dedup that stops the ~60s tick
/// from re-spamming non-voters for the whole window. On pre-fix code this test
/// can't compile (no `last_reminder_at` column / `mark_vote_reminder_sent`),
/// and behaviourally the vote would be returned on every call.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reminder_is_not_resent_after_stamp(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());
    let org = seed_org(&pool, "dedup").await;
    let user = seed_user(&pool, "dedup@vote-reminder.test").await;
    let building = seed_building(&pool, org).await;
    // Ends in 12 hours → inside the 1-day reminder window.
    let vote = seed_active_vote(&pool, org, building, user, "V-DEDUP", 12).await;

    // First sweep: the vote is due for a reminder.
    let first = repo.find_votes_due_for_reminder(1).await.unwrap();
    assert!(
        ids(&first).contains(&vote),
        "an in-window active vote must be returned before any reminder is sent"
    );

    // Reminder dispatched → stamp it.
    repo.mark_vote_reminder_sent(vote).await.unwrap();

    // Next sweep within the cooldown: it must NOT be returned again.
    let second = repo.find_votes_due_for_reminder(1).await.unwrap();
    assert!(
        !ids(&second).contains(&vote),
        "a just-reminded vote must be skipped within the ~5-minute dedup window"
    );
}

/// IG3 (boundary): the sweep only returns active votes ending strictly in the
/// future AND inside the `days_before` window. A vote that already ended and a
/// vote ending past the window are both excluded.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reminder_window_boundary(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());
    let org = seed_org(&pool, "window").await;
    let user = seed_user(&pool, "window@vote-reminder.test").await;
    let building = seed_building(&pool, org).await;

    // Ends in 6 hours → inside the 1-day window.
    let in_window = seed_active_vote(&pool, org, building, user, "V-IN", 6).await;
    // Ends in 48 hours → past the 1-day window.
    let past_window = seed_active_vote(&pool, org, building, user, "V-FAR", 48).await;
    // Already ended 1 hour ago → never reminded.
    let ended = seed_active_vote(&pool, org, building, user, "V-ENDED", -1).await;

    let due = ids(&repo.find_votes_due_for_reminder(1).await.unwrap());
    assert!(due.contains(&in_window), "a vote ending in 6h is in-window");
    assert!(
        !due.contains(&past_window),
        "a vote ending in 48h is past the 1-day window"
    );
    assert!(
        !due.contains(&ended),
        "an already-ended vote must never be reminded"
    );
}

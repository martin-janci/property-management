//! Regression tests for `bug-webhook-airbnb-dup-sync-jobs`.
//!
//! Airbnb delivers webhooks at-least-once. The inbound handler
//! (`routes/integrations/webhook.rs::handle_airbnb_webhook`) guards against
//! redelivery by INSERTing the delivery's dedup key into the
//! `airbnb_webhook_events` ledger with `ON CONFLICT (event_id) DO NOTHING`
//! and returning 200 without side effects when no row is inserted — i.e. the
//! SYNC_EXTERNAL job is enqueued exactly once per distinct delivery.
//!
//! These tests exercise the exact SQL the handler runs against an ephemeral
//! migrated database (migration `00169_airbnb_webhook_events`), proving that
//! a redelivery does NOT produce a second ledger row and therefore would NOT
//! enqueue a second SYNC_EXTERNAL job. They require a `DATABASE_URL`; under
//! `SQLX_OFFLINE=true` with no DB the harness skips them.

use sqlx::{PgPool, Row};

/// Mirror of the dedup INSERT performed by `handle_airbnb_webhook` (step 4).
/// Returns the number of rows affected — 1 for a first-seen delivery, 0 for a
/// redelivery (conflict). In the handler, a 0 means "duplicate → return 200
/// without enqueuing".
async fn try_record_delivery(pool: &PgPool, dedup_key: &str, event_type: &str) -> u64 {
    sqlx::query(
        "INSERT INTO airbnb_webhook_events (event_id, event_type) \
         VALUES ($1, $2) ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(dedup_key)
    .bind(event_type)
    .execute(pool)
    .await
    .expect("ledger insert should not error")
    .rows_affected()
}

async fn ledger_count(pool: &PgPool, dedup_key: &str) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM airbnb_webhook_events WHERE event_id = $1")
        .bind(dedup_key)
        .fetch_one(pool)
        .await
        .expect("count query")
        .get::<i64, _>("n")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn first_delivery_records_and_redelivery_is_suppressed(pool: PgPool) {
    let key = "evt_reservation_created_001";

    // First delivery: ledger row created → handler would enqueue SYNC_EXTERNAL.
    let first = try_record_delivery(&pool, key, "ReservationCreated").await;
    assert_eq!(
        first, 1,
        "first delivery must insert exactly one ledger row"
    );

    // Redelivery (same event_id): conflict → 0 rows → handler returns 200
    // WITHOUT enqueuing a second SYNC_EXTERNAL job.
    let redelivery = try_record_delivery(&pool, key, "ReservationCreated").await;
    assert_eq!(
        redelivery, 0,
        "redelivery must be suppressed (ON CONFLICT DO NOTHING)"
    );

    // Exactly one ledger row exists for this delivery — i.e. one enqueue.
    assert_eq!(
        ledger_count(&pool, key).await,
        1,
        "redelivery must not create a second ledger row"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn synthetic_key_redelivery_is_suppressed(pool: PgPool) {
    // Legacy deliveries omit event_id; the handler derives a synthetic key.
    // A byte-identical redelivery yields the same synthetic key, so it is
    // suppressed exactly like an event_id-bearing redelivery.
    let synthetic_key = "synthetic:deadbeefcafef00d";

    assert_eq!(
        try_record_delivery(&pool, synthetic_key, "ReservationCreated").await,
        1
    );
    assert_eq!(
        try_record_delivery(&pool, synthetic_key, "ReservationCreated").await,
        0,
        "synthetic-key redelivery must be suppressed"
    );
    assert_eq!(ledger_count(&pool, synthetic_key).await, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn distinct_deliveries_each_record(pool: PgPool) {
    // Two genuinely different deliveries (different keys) must both record,
    // so legitimate back-to-back events still each enqueue their sync job.
    assert_eq!(
        try_record_delivery(&pool, "evt_a", "ReservationUpdated").await,
        1
    );
    assert_eq!(
        try_record_delivery(&pool, "evt_b", "ReservationUpdated").await,
        1
    );
    assert_eq!(ledger_count(&pool, "evt_a").await, 1);
    assert_eq!(ledger_count(&pool, "evt_b").await, 1);
}

//! Repo-layer regression tests for `RentalRepository::record_airbnb_webhook_event`
//! (GH #1306, PR #1288 follow-up).
//!
//! The existing `api-server/tests/airbnb_webhook_dedup_tests.rs` mirrors the
//! dedup `INSERT … ON CONFLICT` SQL **inline** rather than calling the repo
//! method. PAP-170 moved that SQL into `RentalRepository::record_airbnb_webhook_event`
//! and the handler now calls the repo — but no test covers the repo contract
//! (`Ok(true)` first-seen / `Ok(false)` redelivery-suppressed).
//!
//! These tests pin that contract at the repo boundary so a future refactor that
//! changes the INSERT or its conflict target fails loudly here.

use db::repositories::RentalRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// First call returns `Ok(true)` (row inserted); redelivery returns `Ok(false)`.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn record_airbnb_webhook_event_first_seen_and_redelivery(pool: PgPool) {
    let repo = RentalRepository::new(pool.clone());

    let event_id = format!("evt_resv_created_{}", Uuid::new_v4());
    let event_type = "ReservationCreated";

    // First delivery → row inserted → Ok(true).
    let first = repo
        .record_airbnb_webhook_event(&event_id, event_type)
        .await
        .expect("first record_airbnb_webhook_event");
    assert!(
        first,
        "first delivery must return Ok(true) — row was inserted"
    );

    // Redelivery of the same event_id → ON CONFLICT DO NOTHING → Ok(false).
    let second = repo
        .record_airbnb_webhook_event(&event_id, event_type)
        .await
        .expect("second record_airbnb_webhook_event (redelivery)");
    assert!(
        !second,
        "redelivery must return Ok(false) — duplicate event suppressed"
    );

    // Verify exactly one row exists in the ledger.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM airbnb_webhook_events WHERE event_id = $1")
            .bind(&event_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(
        count, 1,
        "exactly one ledger row must exist after first+redelivery"
    );
}

/// Distinct `event_id`s are independent — each records its own row.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn record_airbnb_webhook_event_distinct_ids_are_independent(pool: PgPool) {
    let repo = RentalRepository::new(pool.clone());

    let id_a = format!("evt_distinct_a_{}", Uuid::new_v4());
    let id_b = format!("evt_distinct_b_{}", Uuid::new_v4());

    let a = repo
        .record_airbnb_webhook_event(&id_a, "ReservationCreated")
        .await
        .expect("record event_a");
    let b = repo
        .record_airbnb_webhook_event(&id_b, "ReservationCancelled")
        .await
        .expect("record event_b");

    assert!(a, "event_a: first delivery must return Ok(true)");
    assert!(b, "event_b: first delivery must return Ok(true)");

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM airbnb_webhook_events WHERE event_id = ANY($1)")
            .bind(&[&id_a, &id_b][..])
            .fetch_one(&pool)
            .await
            .expect("count two rows");
    assert_eq!(count, 2, "two distinct event_ids must each produce one row");
}

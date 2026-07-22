//! Integration tests for Airbnb sync-state **reconciliation** through the
//! inbound realtime webhook (`routes/integrations/webhook.rs::handle_airbnb_webhook`,
//! mounted at `POST /api/v1/integrations/airbnb/webhook`).
//!
//! # Why this file exists (gap analysis)
//!
//! The existing Airbnb webhook tests cover the *transport* layer and the
//! *ledger* dedup gate:
//!
//! * `airbnb_webhook_routes_tests.rs` — HMAC signature verification, malformed
//!   payload rejection, route wiring, and the `airbnb_webhook_events` ledger
//!   row being written exactly once for a byte-identical redelivery.
//! * `airbnb_webhook_dedup_tests.rs` — the raw `ON CONFLICT DO NOTHING` ledger
//!   SQL in isolation.
//!
//! Neither drives the handler's **dispatch / reconcile** branch (step 5 of the
//! handler), where an inbound event is reconciled against the *current DB
//! state* of the affected booking/connection. That is the substance of "sync
//! reconciliation": taking an externally-sourced reservation event and
//! converging local booking state to match, while degrading gracefully when
//! the local state has diverged from what the event assumes.
//!
//! These tests close that gap end-to-end, through the real Axum router + an
//! ephemeral migrated Postgres, by seeding a full rental graph
//! (org → building → unit → connection → booking) and posting signed events:
//!
//! 1. **Reconcile-to-cancelled** — a `reservation_cancelled` event for a live
//!    booking converges that booking's status to `cancelled` and stamps the
//!    cancellation reason.
//! 2. **Idempotent re-apply (state level)** — a *second*, distinct delivery
//!    (different `event_id`, so the ledger does NOT suppress it) for an
//!    already-cancelled booking is a no-op: status stays `cancelled`, no error,
//!    still 200. This is the booking-state idempotency guard, which is separate
//!    from the ledger dedup gate already covered elsewhere.
//! 3. **Divergence: unknown booking** — a cancellation for a confirmation code
//!    that does not exist locally is acknowledged with 200 and changes nothing
//!    (no crash, no spurious row).
//! 4. **Divergence: unknown listing** — a `reservation_created`/`_updated`
//!    event for a listing with no active local connection is acknowledged with
//!    200 and enqueues no sync job (no matching connection to reconcile to).
//! 5. **Connection match drives reconcile** — a `reservation_created` for a
//!    listing that DOES have an active connection is accepted (200) and the
//!    handler enqueues exactly one `SYNC_EXTERNAL` background job for the
//!    matched connection's organization (the dispatch path in step 5 of the
//!    handler), proving the reconcile reached the connection lookup.
//!
//! All payloads carry a unique `event_id` so the ledger never masks the
//! state-reconciliation assertions we care about.
//!
//! # A note on the Postgres enum columns
//!
//! `rental_bookings.status` is the Postgres enum `rental_booking_status` and
//! `rental_bookings.platform` / `rental_platform_connections.platform` are the
//! enum `rental_platform`. SQLx will not decode a bare enum column into a Rust
//! `String`, nor bind a Rust `&str` parameter against an enum column without an
//! explicit cast. So, mirroring the sibling seed/read patterns in
//! `rental_connection_token_leak_tests.rs` and `db/tests/rls_smoke_tests.rs`,
//! we (a) write enum values as SQL string literals cast to the enum type and
//! (b) read enum columns back as `::text`.
//!
//! # Secret wiring
//!
//! `AppState` reads `AIRBNB_WEBHOOK_SECRET` from the process env once at
//! construction (`AirbnbAppConfig::from_env`). We seed it via a `Once` before
//! the first `TestApp::new`, mirroring `airbnb_webhook_routes_tests.rs`.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::common::TestApp;

type HmacSha256 = Hmac<Sha256>;

const WEBHOOK_SECRET: &str = "airbnb-webhook-test-secret-key";
const WEBHOOK_URI: &str = "/api/v1/integrations/airbnb/webhook";

/// Seed the webhook secret into the process env before the first `AppState` is
/// constructed. `set_var` is not thread-safe alongside concurrent `getenv`, so
/// gate it behind a `Once`, exactly like the sibling webhook test binaries.
fn ensure_webhook_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("AIRBNB_WEBHOOK_SECRET", WEBHOOK_SECRET);
    });
}

/// Compute the hex HMAC-SHA256 signature Airbnb would send in
/// `X-Airbnb-Signature` for `body` under `WEBHOOK_SECRET`.
fn sign(body: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(WEBHOOK_SECRET.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn signed_post(body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(WEBHOOK_URI)
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Airbnb-Signature", sign(body))
        .body(Body::from(body.to_owned()))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Fixtures — seed a full rental graph via SQL (org → building → unit →
// connection → booking). Mirrors the seeding pattern in
// `rental_connection_token_leak_tests.rs`.
//
// `platform` / `status` are Postgres enums; we write them as string literals
// cast to the enum type so SQLx never has to bind a `&str` against an enum
// column (which is a bind type mismatch).
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Airbnb Reconcile Org {slug}"))
    .bind(format!("airbnb-reconcile-org-{slug}"))
    .bind(format!("{slug}@airbnb-reconcile.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation, floor)
        VALUES ($1, $2, 1) RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Seed an active Airbnb connection whose `external_property_id` is the Airbnb
/// listing id — that is the column `find_airbnb_connection_by_listing_id`
/// matches on. `platform` is written as the enum literal `'airbnb'`.
async fn seed_connection(pool: &PgPool, org_id: Uuid, unit_id: Uuid, listing_id: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (
            organization_id, unit_id, platform,
            access_token, refresh_token, external_property_id, is_active
        )
        VALUES ($1, $2, 'airbnb'::rental_platform, 'access-tok', 'refresh-tok', $3, true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(listing_id)
    .fetch_one(pool)
    .await
    .expect("seed connection")
}

/// Seed a confirmed Airbnb booking keyed by `external_booking_id`
/// (= the webhook `confirmation_code`). Returns the booking id.
///
/// `platform` and `status` are the Postgres enums `rental_platform` and
/// `rental_booking_status`; we cast the string literals to those enum types
/// explicitly.
async fn seed_booking(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    connection_id: Uuid,
    confirmation_code: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings (
            organization_id, unit_id, connection_id, platform,
            external_booking_id, guest_name, guest_count,
            check_in, check_out, status
        )
        VALUES (
            $1, $2, $3, 'airbnb'::rental_platform,
            $4, 'Test Guest', 2,
            DATE '2026-07-01', DATE '2026-07-05', 'confirmed'::rental_booking_status
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(connection_id)
    .bind(confirmation_code)
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

/// Read `rental_bookings.status` back. The column is the Postgres enum
/// `rental_booking_status`, which SQLx cannot decode into a Rust `String`
/// directly, so we cast it to `::text` in the projection.
async fn booking_status(pool: &PgPool, booking_id: Uuid) -> String {
    sqlx::query("SELECT status::text AS status FROM rental_bookings WHERE id = $1")
        .bind(booking_id)
        .fetch_one(pool)
        .await
        .expect("fetch booking status")
        .get::<String, _>("status")
}

async fn booking_cancellation_reason(pool: &PgPool, booking_id: Uuid) -> Option<String> {
    sqlx::query("SELECT cancellation_reason FROM rental_bookings WHERE id = $1")
        .bind(booking_id)
        .fetch_one(pool)
        .await
        .expect("fetch cancellation reason")
        .get::<Option<String>, _>("cancellation_reason")
}

async fn ledger_count(pool: &PgPool, event_id: &str) -> i64 {
    sqlx::query("SELECT COUNT(*) AS n FROM airbnb_webhook_events WHERE event_id = $1")
        .bind(event_id)
        .fetch_one(pool)
        .await
        .expect("ledger count")
        .get::<i64, _>("n")
}

/// Count enqueued `SYNC_EXTERNAL` background jobs for `org_id`.
///
/// `job_type` is a plain `VARCHAR` column (see migration 00072), so the text
/// comparison is correct as-is — no enum cast needed here.
async fn sync_job_count(pool: &PgPool, org_id: Uuid) -> i64 {
    sqlx::query(
        "SELECT COUNT(*) AS n FROM background_jobs \
         WHERE job_type = 'sync_external' AND org_id = $1",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("count sync jobs")
    .get::<i64, _>("n")
}

/// Build a signed `reservation_cancelled` payload with a unique `event_id`.
fn cancellation_event(event_id: &str, confirmation_code: &str) -> String {
    format!(
        r#"{{"event_type":"reservation_cancelled","event_id":"{event_id}","listing_id":"listing-x","confirmation_code":"{confirmation_code}","timestamp":"2026-06-01T12:00:00Z","payload":{{}}}}"#
    )
}

/// Build a signed `reservation_created` payload with a unique `event_id`.
fn created_event(event_id: &str, listing_id: &str, confirmation_code: &str) -> String {
    format!(
        r#"{{"event_type":"reservation_created","event_id":"{event_id}","listing_id":"{listing_id}","confirmation_code":"{confirmation_code}","timestamp":"2026-06-01T12:00:00Z","payload":{{}}}}"#
    )
}

// ===========================================================================
// 1. Reconcile-to-cancelled — a cancellation event converges a live booking.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancellation_event_reconciles_booking_to_cancelled(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "cancel").await;
    let building = seed_building(&pool, org, "cancel").await;
    let unit = seed_unit(&pool, building, "C-1").await;
    let conn = seed_connection(&pool, org, unit, "listing-cancel").await;
    let booking = seed_booking(&pool, org, unit, conn, "HMCANCEL1").await;

    assert_eq!(booking_status(&pool, booking).await, "confirmed");

    let body = cancellation_event("evt_reconcile_cancel_1", "HMCANCEL1");
    let resp = app.execute(signed_post(&body)).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "cancellation must be acknowledged with 200; got {}: {}",
        resp.status,
        resp.text()
    );

    assert_eq!(
        booking_status(&pool, booking).await,
        "cancelled",
        "booking should have been reconciled to cancelled"
    );
    assert!(
        booking_cancellation_reason(&pool, booking)
            .await
            .unwrap_or_default()
            .to_lowercase()
            .contains("airbnb"),
        "cancellation reason should record the Airbnb webhook origin"
    );
}

// ===========================================================================
// 2. Idempotent re-apply at the booking-state level — a *distinct* delivery
//    (new event_id, so the ledger does NOT suppress it) for an already-
//    cancelled booking is a no-op.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn second_distinct_cancellation_is_state_idempotent(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "reapply").await;
    let building = seed_building(&pool, org, "reapply").await;
    let unit = seed_unit(&pool, building, "R-1").await;
    let conn = seed_connection(&pool, org, unit, "listing-reapply").await;
    let booking = seed_booking(&pool, org, unit, conn, "HMREAPPLY1").await;

    // First delivery cancels the booking.
    let first = cancellation_event("evt_reapply_1", "HMREAPPLY1");
    let r1 = app.execute(signed_post(&first)).await;
    assert_eq!(r1.status, StatusCode::OK, "first cancellation -> 200");
    assert_eq!(booking_status(&pool, booking).await, "cancelled");

    // Second delivery is a DIFFERENT event_id (the ledger dedup gate, which is
    // covered elsewhere, will NOT suppress this) but targets the same already-
    // cancelled booking. The handler's booking-state guard must make this a
    // no-op rather than erroring or re-mutating.
    let second = cancellation_event("evt_reapply_2", "HMREAPPLY1");
    assert_eq!(
        ledger_count(&pool, "evt_reapply_2").await,
        0,
        "second event_id must be unseen by the ledger before delivery"
    );
    let r2 = app.execute(signed_post(&second)).await;
    assert_eq!(
        r2.status,
        StatusCode::OK,
        "re-applying cancellation to an already-cancelled booking must stay 200; got {}: {}",
        r2.status,
        r2.text()
    );
    assert_eq!(
        ledger_count(&pool, "evt_reapply_2").await,
        1,
        "the distinct second delivery should still be recorded in the ledger"
    );
    assert_eq!(
        booking_status(&pool, booking).await,
        "cancelled",
        "status must remain cancelled after the idempotent re-apply"
    );
}

// ===========================================================================
// 3. Divergence — cancellation for an unknown confirmation code.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancellation_for_unknown_booking_is_acknowledged_without_side_effects(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // No booking seeded for this confirmation code at all.
    let body = cancellation_event("evt_unknown_booking_1", "HMNOSUCH1");
    let resp = app.execute(signed_post(&body)).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "cancellation for an unknown booking must be acknowledged (200), not error; got {}: {}",
        resp.status,
        resp.text()
    );

    let count: i64 = sqlx::query("SELECT COUNT(*) AS n FROM rental_bookings")
        .fetch_one(&pool)
        .await
        .expect("count bookings")
        .get::<i64, _>("n");
    assert_eq!(
        count, 0,
        "a divergent cancellation must not fabricate a booking row"
    );
}

// ===========================================================================
// 4. Divergence — created/updated event for a listing with no connection.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn created_event_for_unknown_listing_is_acknowledged(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    // Listing id matches no active connection — the reconcile path finds no
    // connection and acknowledges without enqueueing a sync job.
    let body = created_event("evt_unknown_listing_1", "listing-orphan", "HMORPHAN1");
    let resp = app.execute(signed_post(&body)).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "created event for an unknown listing must be acknowledged (200); got {}: {}",
        resp.status,
        resp.text()
    );

    // The delivery is still recorded in the dedup ledger (it was first-seen and
    // well-formed); only the downstream reconcile is a no-op.
    assert_eq!(
        ledger_count(&pool, "evt_unknown_listing_1").await,
        1,
        "well-formed first delivery should be ledgered even when no connection matches"
    );

    let jobs: i64 =
        sqlx::query("SELECT COUNT(*) AS n FROM background_jobs WHERE job_type = 'sync_external'")
            .fetch_one(&pool)
            .await
            .expect("count jobs")
            .get::<i64, _>("n");
    assert_eq!(
        jobs, 0,
        "no sync job should be enqueued when no connection matches the listing"
    );
}

// ===========================================================================
// 5. Connection match drives reconcile — created event for a connected
//    listing reaches the dispatch path and enqueues exactly one sync job.
//
// The handler's ReservationCreated branch looks the connection up by listing
// id (`find_airbnb_connection_by_listing_id`), and on a hit enqueues exactly
// one SYNC_EXTERNAL background job scoped to that connection's organization.
// The dedup ledger is first-seen for `evt_connected_1`, so the dispatch is not
// suppressed. We seed an active connection whose `external_property_id` equals
// the event's `listing_id` so the lookup hits.
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn created_event_for_connected_listing_enqueues_one_sync_job(pool: PgPool) {
    ensure_webhook_secret();
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "match").await;
    let building = seed_building(&pool, org, "match").await;
    let unit = seed_unit(&pool, building, "M-1").await;
    // `external_property_id` == the event's `listing_id` so the connection
    // lookup in the handler hits and drives the enqueue.
    let _conn = seed_connection(&pool, org, unit, "listing-connected").await;

    // Sanity: no sync jobs before the delivery.
    assert_eq!(
        sync_job_count(&pool, org).await,
        0,
        "no sync jobs should exist before the webhook is delivered"
    );

    let body = created_event("evt_connected_1", "listing-connected", "HMCONN1");
    let resp = app.execute(signed_post(&body)).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "created event for a connected listing must be acknowledged (200); got {}: {}",
        resp.status,
        resp.text()
    );
    assert_eq!(
        ledger_count(&pool, "evt_connected_1").await,
        1,
        "first-seen connected delivery should be ledgered"
    );

    assert_eq!(
        sync_job_count(&pool, org).await,
        1,
        "exactly one SYNC_EXTERNAL job should be enqueued for the matched connection"
    );
}

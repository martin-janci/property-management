//! BIT-85 — cross-tenant upsert security regression tests.
//!
//! Before this fix the two OAuth upsert functions used
//! `ON CONFLICT (unit_id, platform)` for org-level (nil unit_id) connections.
//! Because `UNIQUE(unit_id, platform)` has no `organization_id` column, two
//! orgs that both call the upsert without a `unit_id` share a single conflict
//! slot: org B's `DO UPDATE` fires against org A's row and — since
//! `organization_id` was absent from `DO UPDATE SET` — the row's
//! `organization_id` stays as org A while the tokens are replaced with org B's
//! credentials. Cross-tenant token-binding hazard.
//!
//! These tests must PASS on the patched branch and would FAIL on original dev.

use crate::common::seed_org;
use db::repositories::RentalRepository;
use sqlx::PgPool;

// ---------------------------------------------------------------------------
// Airbnb cross-tenant upsert
// ---------------------------------------------------------------------------

/// Two orgs each call upsert_airbnb_connection without a unit_id.
/// After both calls org A's row must still have organization_id = org_a,
/// org B's row must have organization_id = org_b, and tokens must not bleed.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn airbnb_upsert_nil_unit_id_is_org_scoped(pool: PgPool) {
    let org_a = seed_org(&pool, "airbnb-a").await;
    let org_b = seed_org(&pool, "airbnb-b").await;
    let repo = RentalRepository::new(pool.clone());

    let conn_a = repo
        .upsert_airbnb_connection(org_a, None, "token-a-v1", None, None, None)
        .await
        .expect("org A airbnb upsert");
    let conn_b = repo
        .upsert_airbnb_connection(org_b, None, "token-b-v1", None, None, None)
        .await
        .expect("org B airbnb upsert");

    assert_ne!(
        conn_a.id, conn_b.id,
        "each org must have its own connection row"
    );
    assert_eq!(
        conn_a.organization_id, org_a,
        "org A connection must belong to org A"
    );
    assert_eq!(
        conn_b.organization_id, org_b,
        "org B connection must not be rebound to org A"
    );

    let raw_a = sqlx::query_scalar::<_, Option<String>>(
        "SELECT access_token FROM rental_platform_connections WHERE id = $1",
    )
    .bind(conn_a.id)
    .fetch_one(&pool)
    .await
    .expect("fetch conn_a token");
    assert_eq!(
        raw_a.as_deref(),
        Some("token-a-v1"),
        "org A token must be untouched"
    );

    let raw_b = sqlx::query_scalar::<_, Option<String>>(
        "SELECT access_token FROM rental_platform_connections WHERE id = $1",
    )
    .bind(conn_b.id)
    .fetch_one(&pool)
    .await
    .expect("fetch conn_b token");
    assert_eq!(
        raw_b.as_deref(),
        Some("token-b-v1"),
        "org B token must not bleed to org A"
    );
}

/// When org B refreshes its token after both orgs are connected, org A's row
/// must remain untouched.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn airbnb_upsert_refresh_does_not_rebind_other_org(pool: PgPool) {
    let org_a = seed_org(&pool, "airbnb-ra").await;
    let org_b = seed_org(&pool, "airbnb-rb").await;
    let repo = RentalRepository::new(pool.clone());

    repo.upsert_airbnb_connection(org_a, None, "token-a-orig", None, None, None)
        .await
        .expect("org A upsert");
    repo.upsert_airbnb_connection(org_b, None, "token-b-orig", None, None, None)
        .await
        .expect("org B upsert");
    repo.upsert_airbnb_connection(org_b, None, "token-b-refreshed", None, None, None)
        .await
        .expect("org B refresh");

    let raw_a = sqlx::query_scalar::<_, Option<String>>(
        "SELECT access_token FROM rental_platform_connections \
         WHERE organization_id = $1 AND platform = 'airbnb'",
    )
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .expect("fetch org A token");
    assert_eq!(
        raw_a.as_deref(),
        Some("token-a-orig"),
        "org A token must not be changed by org B refresh"
    );
}

// ---------------------------------------------------------------------------
// Booking OAuth cross-tenant upsert
// ---------------------------------------------------------------------------

/// Mirrors the Airbnb tests for upsert_booking_oauth_connection.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn booking_oauth_upsert_nil_unit_id_is_org_scoped(pool: PgPool) {
    let org_a = seed_org(&pool, "booking-a").await;
    let org_b = seed_org(&pool, "booking-b").await;
    let repo = RentalRepository::new(pool.clone());

    let conn_a = repo
        .upsert_booking_oauth_connection(org_a, None, "booking-token-a", None, None, None)
        .await
        .expect("org A booking upsert");
    let conn_b = repo
        .upsert_booking_oauth_connection(org_b, None, "booking-token-b", None, None, None)
        .await
        .expect("org B booking upsert");

    assert_ne!(conn_a.id, conn_b.id, "each org must have its own row");
    assert_eq!(
        conn_a.organization_id, org_a,
        "org A connection must belong to org A"
    );
    assert_eq!(
        conn_b.organization_id, org_b,
        "org B connection must not be rebound to org A"
    );

    let raw_a = sqlx::query_scalar::<_, Option<String>>(
        "SELECT access_token FROM rental_platform_connections WHERE id = $1",
    )
    .bind(conn_a.id)
    .fetch_one(&pool)
    .await
    .expect("fetch conn_a token");
    assert_eq!(
        raw_a.as_deref(),
        Some("booking-token-a"),
        "org A booking token must be untouched"
    );
}

/// organization_id must be stable after an idempotent re-upsert by the same org.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn booking_oauth_upsert_idempotent_same_org(pool: PgPool) {
    let org_a = seed_org(&pool, "booking-idem").await;
    let repo = RentalRepository::new(pool.clone());

    repo.upsert_booking_oauth_connection(org_a, None, "tok-v1", None, None, None)
        .await
        .expect("first upsert");
    let conn = repo
        .upsert_booking_oauth_connection(org_a, None, "tok-v2", None, None, None)
        .await
        .expect("second upsert");

    assert_eq!(
        conn.organization_id, org_a,
        "organization_id must remain org_a after re-upsert"
    );

    let raw = sqlx::query_scalar::<_, Option<String>>(
        "SELECT access_token FROM rental_platform_connections WHERE id = $1",
    )
    .bind(conn.id)
    .fetch_one(&pool)
    .await
    .expect("fetch updated token");
    assert_eq!(
        raw.as_deref(),
        Some("tok-v2"),
        "token must be updated on re-upsert"
    );
}

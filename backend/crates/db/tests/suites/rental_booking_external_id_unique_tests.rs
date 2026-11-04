//! Regression tests for the `rental_bookings` external-id uniqueness rule.
//!
//! Migration 00051 declared `UNIQUE NULLS NOT DISTINCT (platform,
//! external_booking_id)`, which treated every NULL external id as equal: only ONE
//! booking per platform without an external id could exist system-wide, so a
//! second manual booking (in any organisation) failed with 23505. Migration
//! 00239 replaces it with a partial unique index on
//! `(organization_id, platform, external_booking_id) WHERE external_booking_id
//! IS NOT NULL`.

use sqlx::PgPool;
use uuid::Uuid;

const UNIQUE_VIOLATION: &str = "23505";

async fn super_admin(pool: &PgPool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");
}

/// Seed an organisation with one building and unit; returns `(org_id, unit_id)`.
async fn seed_org_unit(pool: &PgPool, tag: &str) -> (Uuid, Uuid) {
    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Booking Unique Org {tag}"))
    .bind(format!("booking-unique-org-{tag}"))
    .bind(format!("booking-unique-{tag}@test.local"))
    .fetch_one(pool)
    .await
    .expect("seed org");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Booking Unique Building', 'Street 1', 'City', '00000', 'Country', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, '1A') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit");

    (org_id, unit_id)
}

async fn insert_booking(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    platform: &str,
    external_id: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings
            (organization_id, unit_id, platform, external_booking_id, guest_name, check_in, check_out)
        VALUES ($1, $2, $3::rental_platform, $4, 'Guest', DATE '2026-03-01', DATE '2026-03-04')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(platform)
    .bind(external_id)
    .fetch_one(pool)
    .await
}

fn sqlstate(err: &sqlx::Error) -> Option<String> {
    err.as_database_error()
        .and_then(|e| e.code())
        .map(|c| c.into_owned())
}

/// Several bookings without an external id on the same platform must coexist,
/// both inside one organisation and across organisations.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bookings_without_external_id_are_unconstrained(pool: PgPool) {
    super_admin(&pool).await;
    let (org_a, unit_a) = seed_org_unit(&pool, "null-a").await;
    let (org_b, unit_b) = seed_org_unit(&pool, "null-b").await;

    for (org, unit) in [(org_a, unit_a), (org_a, unit_a), (org_b, unit_b)] {
        insert_booking(&pool, org, unit, "direct", None)
            .await
            .expect("manual booking without external id must not hit a unique violation");
    }

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rental_bookings \
         WHERE platform = 'direct' AND external_booking_id IS NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 3);
}

/// A platform's external id stays unique within an organisation, but two
/// organisations may each hold a booking with the same platform reference.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn external_id_unique_per_org_and_platform(pool: PgPool) {
    super_admin(&pool).await;
    let (org_a, unit_a) = seed_org_unit(&pool, "ext-a").await;
    let (org_b, unit_b) = seed_org_unit(&pool, "ext-b").await;

    insert_booking(&pool, org_a, unit_a, "airbnb", Some("HM123"))
        .await
        .expect("first external booking");

    let dup = insert_booking(&pool, org_a, unit_a, "airbnb", Some("HM123"))
        .await
        .expect_err("duplicate external id in the same org must be rejected");
    assert_eq!(sqlstate(&dup).as_deref(), Some(UNIQUE_VIOLATION));

    insert_booking(&pool, org_a, unit_a, "booking", Some("HM123"))
        .await
        .expect("same external id on a different platform is a different booking");
    insert_booking(&pool, org_b, unit_b, "airbnb", Some("HM123"))
        .await
        .expect("same external id in another organisation is allowed");
}

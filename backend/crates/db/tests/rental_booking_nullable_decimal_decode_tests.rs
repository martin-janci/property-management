//! Regression test for GH #1422 — `rental_bookings` nullable-Decimal decode.
//!
//! `RentalBooking::{total_amount, platform_fee, cleaning_fee}` are
//! `Option<Decimal>` over nullable `DECIMAL(12,2)` columns
//! (`00051_create_short_term_rentals.sql`). They used to carry
//! `#[sqlx(try_from = "Decimal")]`, which decodes the column as a *non-null*
//! `Decimal` first — so a NULL value produced a runtime "unexpected null"
//! decode error instead of `None`. Any `rental_bookings` row with a NULL
//! amount/fee then failed to materialize into `RentalBooking`.
//!
//! This test inserts a booking row with all three financial fields NULL and
//! decodes it into `RentalBooking`, asserting they round-trip as `None`.
//! Restoring the `#[sqlx(try_from = "Decimal")]` attributes makes the decode
//! error, so this fails on `main`.
//!
//! NOTE: the projection casts the `platform`/`status` PG enums to `text` (the
//! model decodes them as `String`). That mirrors the explicit projections used
//! across `RentalRepository`; the repo's bare `SELECT *` / `RETURNING *`
//! booking readers still lack those casts and carry the same enum-decode gap —
//! tracked separately in GH #1363.

use db::models::RentalBooking;
use sqlx::PgPool;
use uuid::Uuid;

const BOOKING_COLUMNS: &str = r#"
    id, organization_id, unit_id, connection_id,
    platform::text AS platform, external_booking_id, external_booking_url,
    guest_name, guest_email, guest_phone, guest_count,
    check_in, check_out, check_in_time, check_out_time,
    total_amount, currency, platform_fee, cleaning_fee,
    status::text AS status, cancelled_at, cancellation_reason,
    guest_notes, internal_notes, synced_at, raw_data,
    created_at, updated_at
"#;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Rental {slug}"))
    .bind(slug)
    .bind(format!("{slug}@rental.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_unit(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country, name)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .bind(format!("{slug} Building"))
    .fetch_one(pool)
    .await
    .expect("seed building");

    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building_id)
    .bind("STR-1")
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

#[sqlx::test]
async fn rental_booking_decodes_null_financial_fields(pool: PgPool) {
    let org = seed_org(&pool, "null_fees").await;
    let unit = seed_unit(&pool, org, "null_fees").await;

    // Insert a booking with NULL total_amount / platform_fee / cleaning_fee.
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings (
            organization_id, unit_id, platform, guest_name, guest_count,
            check_in, check_out, total_amount, platform_fee, cleaning_fee
        )
        VALUES ($1, $2, 'direct', 'Null Fee Guest', 1,
                '2026-07-01', '2026-07-03', NULL, NULL, NULL)
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .fetch_one(&pool)
    .await
    .expect("insert booking with NULL financial fields");

    // Decode the row into the model — this is the path that used to error on the
    // non-null `try_from = "Decimal"` decode of a NULL column.
    let query = format!("SELECT {BOOKING_COLUMNS} FROM rental_bookings WHERE id = $1");
    let booking = sqlx::query_as::<_, RentalBooking>(&query)
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("RentalBooking decodes NULL total_amount/platform_fee/cleaning_fee as None");

    assert_eq!(booking.total_amount, None);
    assert_eq!(booking.platform_fee, None);
    assert_eq!(booking.cleaning_fee, None);

    // Sanity: a NON-null fee still decodes correctly after dropping `try_from`.
    sqlx::query("UPDATE rental_bookings SET cleaning_fee = 49.50 WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("set a non-null fee");

    let booking = sqlx::query_as::<_, RentalBooking>(&query)
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("RentalBooking decodes a non-null fee");

    assert_eq!(
        booking.cleaning_fee,
        Some("49.50".parse().expect("decimal literal"))
    );
    assert_eq!(booking.total_amount, None);
}

//! Regression test for issue #1422 — `rental_bookings.{total_amount, platform_fee,
//! cleaning_fee}` are NULLABLE `DECIMAL(12,2)` columns (migration 00051), but the
//! `RentalBooking` model decoded them with `#[sqlx(try_from = "Decimal")]`, which
//! forces a NON-NULL `Decimal` decode first and fails with "unexpected null" on
//! any booking row whose amount/fee is NULL. `find_booking_by_id` does a bare
//! `SELECT * FROM rental_bookings`, so such a row could never be loaded.
//!
//! This test inserts a booking with all three financial columns left NULL and
//! loads it through the repository, so it fails on `main` (decode error) and
//! passes once the `try_from` attributes are dropped (plain `Option<Decimal>`
//! decodes nullable NUMERIC natively).

use db::repositories::RentalRepository;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_booking_by_id_decodes_null_fees(pool: PgPool) {
    // Super-admin context satisfies the rental_bookings RLS policies for seeding
    // and for the unscoped by-id read below.
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(&pool)
        .await
        .expect("set super-admin context");

    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ('Rental Null Org', 'rental-null-org', 'rental@null.test', 'active')
        RETURNING id
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("seed org");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Rental Null Building', 'Street 1', 'City', '00000', 'Country', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, '1A') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&pool)
    .await
    .expect("seed unit");

    // Booking with total_amount / platform_fee / cleaning_fee all left NULL —
    // the exact row shape that triggers the decode failure.
    let booking_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings
            (organization_id, unit_id, platform, guest_name, check_in, check_out)
        VALUES ($1, $2, 'airbnb', 'Null Fee Guest', DATE '2026-01-01', DATE '2026-01-05')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(&pool)
    .await
    .expect("seed booking");

    let repo = RentalRepository::new(pool.clone());
    let booking = repo
        .find_booking_by_id(booking_id)
        .await
        .expect("find_booking_by_id must decode a NULL-fee booking (issue #1422)")
        .expect("booking row exists");

    assert_eq!(booking.total_amount, None);
    assert_eq!(booking.platform_fee, None);
    assert_eq!(booking.cleaning_fee, None);
}

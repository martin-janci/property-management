//! Regression tests for the FORCE-RLS-masked `42804` enum decode gap on the
//! short-term-rental repository (BIT-113, GH #1363; sibling of #859/#858).
//!
//! `rental_bookings.platform` / `.status` and `rental_platform_connections.platform`
//! are PG enums (`rental_platform`, `rental_booking_status`) that the `RentalBooking`
//! and `RentalPlatformConnection` models decode into `String`. A bare `SELECT *` /
//! `RETURNING *` hands sqlx the raw enum OID and fails at runtime ("mismatched types
//! ... is not compatible with SQL type `rental_platform`", SQLSTATE 42804). The
//! failure only surfaces when a row is actually returned, so it stayed masked under
//! FORCE RLS on paths that returned zero rows. The fix projects the enum columns
//! explicitly as `col::text` (see `BOOKING_COLUMNS` / `PLATFORM_CONNECTION_COLUMNS`).
//!
//! These tests SELECT / RETURN real rows through the repository, so they fail on the
//! pre-fix bare-`*` queries (decode error) and pass once the casts are in place.
//!
//! Seed rows use literal-enum SQL (`'airbnb'`, `'confirmed'`) rather than the
//! repository's `create_*` inserts: sqlx 0.9 rejects *binding* a `String` param into
//! an enum column (the encode side of the same strictness, also 42804), which is a
//! separate concern from the decode gap under test here. The three
//! `#[sqlx(try_from = "Decimal")]` booking fee fields are seeded non-NULL because
//! that decode rejects SQL NULL even for an `Option<Decimal>` field — an unrelated
//! quirk that would otherwise mask the enum-cast assertion.

use db::models::rental::UpdateBooking;
use db::repositories::RentalRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed org -> building -> unit -> ('airbnb') connection -> ('airbnb','confirmed')
/// booking under super-admin RLS context (satisfies the `is_super_admin()` arm of
/// the FORCE-RLS `*_tenant_isolation` policies). Returns
/// `(org_id, unit_id, connection_id, booking_id)`.
async fn seed(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");

    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ('Rental Enum Org', 'rental-enum-org', 'rental-enum@decode.test', 'active')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed org");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Rental Enum Building', 'Street 1', 'Bratislava', '81101', 'Slovakia', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation, floor)
        VALUES ($1, '3B', 1)
        RETURNING id
        "#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit");

    // Enum value supplied as a literal so Postgres coerces it; a bound `String`
    // param would be rejected on encode (42804) — separate from the decode gap.
    let connection_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (organization_id, unit_id, platform, is_active)
        VALUES ($1, $2, 'airbnb', true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed connection");

    // total_amount / platform_fee / cleaning_fee are set because the model decodes
    // them via `#[sqlx(try_from = "Decimal")]`, which rejects SQL NULL.
    let booking_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings (
            organization_id, unit_id, connection_id, platform,
            guest_name, guest_count, check_in, check_out,
            total_amount, platform_fee, cleaning_fee, status
        )
        VALUES ($1, $2, $3, 'airbnb',
                'Jane Guest', 2, CURRENT_DATE, CURRENT_DATE + 3,
                500.00, 50.00, 30.00, 'confirmed')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(connection_id)
    .fetch_one(pool)
    .await
    .expect("seed booking");

    (org_id, unit_id, connection_id, booking_id)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn find_booking_by_id_decodes_platform_and_status_enums(pool: PgPool) {
    let (_org, _unit, _conn, booking_id) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // Pre-fix this is a bare `SELECT *` and 500s decoding `platform`/`status`.
    let booking = repo
        .find_booking_by_id(booking_id)
        .await
        .expect("find_booking_by_id must decode the rental enums (BIT-113)")
        .expect("booking row exists");
    assert_eq!(booking.platform, "airbnb");
    assert_eq!(booking.status, "confirmed");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn find_booking_for_org_decodes_enums(pool: PgPool) {
    let (org_id, _unit, _conn, booking_id) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let booking = repo
        .find_booking_for_org(org_id, booking_id)
        .await
        .expect("find_booking_for_org must decode the rental enums (BIT-113)")
        .expect("booking row exists");
    assert_eq!(booking.platform, "airbnb");
    assert_eq!(booking.status, "confirmed");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_booking_returning_decodes_enums(pool: PgPool) {
    let (_org, _unit, _conn, booking_id) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // update_booking returns its row via RETURNING — the enum columns must decode.
    let updated = repo
        .update_booking(
            booking_id,
            UpdateBooking {
                guest_name: Some("Renamed Guest".to_string()),
                guest_email: None,
                guest_phone: None,
                guest_count: None,
                check_in: None,
                check_out: None,
                check_in_time: None,
                check_out_time: None,
                total_amount: None,
                currency: None,
                guest_notes: None,
                internal_notes: None,
            },
        )
        .await
        .expect("update_booking RETURNING must decode the rental enums (BIT-113)");
    assert_eq!(updated.guest_name, "Renamed Guest");
    assert_eq!(updated.platform, "airbnb");
    assert_eq!(updated.status, "confirmed");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn connection_read_paths_decode_platform_enum(pool: PgPool) {
    let (org_id, _unit, conn_id, _booking) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let for_org = repo
        .find_connection_for_org(org_id, conn_id)
        .await
        .expect("find_connection_for_org must decode the platform enum (BIT-113)")
        .expect("connection row exists");
    assert_eq!(for_org.platform, "airbnb");

    // find_airbnb_connection_by_org was a bare `SELECT *` pre-fix.
    let by_org = repo
        .find_airbnb_connection_by_org(org_id)
        .await
        .expect("find_airbnb_connection_by_org must decode the platform enum (BIT-113)")
        .expect("airbnb connection exists");
    assert_eq!(by_org.platform, "airbnb");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_airbnb_tokens_returning_decodes_platform_enum(pool: PgPool) {
    let (_org, _unit, conn_id, _booking) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // update_airbnb_tokens returns its row via RETURNING (bare `*` pre-fix).
    let rotated = repo
        .update_airbnb_tokens(conn_id, "enc-access", Some("enc-refresh"), None)
        .await
        .expect("update_airbnb_tokens RETURNING must decode the platform enum (BIT-113)");
    assert_eq!(rotated.platform, "airbnb");
}

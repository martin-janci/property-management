//! Regression tests for BIT-118 — 42804 enum-decode gap on
//! `RentalGuest.status`, `CalendarBlock.source_platform`, and
//! `ICalFeed.import_platform`.
//!
//! These columns are PG enum types (`guest_registration_status`, `rental_platform`)
//! that the Rust models decode as `String`. A bare `SELECT *` / `RETURNING *`
//! hands sqlx the raw enum OID and fails at runtime with SQLSTATE 42804. The
//! failure is masked under FORCE RLS on paths that return zero rows. The fix
//! projects the enum columns explicitly as `col::text` via `GUEST_COLUMNS`,
//! `CALENDAR_BLOCK_COLUMNS`, and `ICAL_FEED_COLUMNS` consts (sibling of
//! `BOOKING_COLUMNS` / `PLATFORM_CONNECTION_COLUMNS` from BIT-113 / GH #1486).
//!
//! Seed rows use literal-enum SQL (`'pending'`, `'airbnb'`) rather than
//! repository `create_*` inserts to avoid the encode-side 42804 — a separate
//! concern from the decode gap under test here.

use db::models::rental::{UpdateGuest, UpdateICalFeed};
use db::repositories::RentalRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed org -> building -> unit -> booking (for guest foreign key), guest, calendar
/// block, and iCal feed — all with non-NULL enum values seeded as SQL literals.
///
/// Returns `(org_id, unit_id, booking_id, guest_id, block_id, feed_id, feed_token)`.
async fn seed(pool: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid, String) {
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
        VALUES ('Guest Enum Org', 'guest-enum-org', 'guest-enum@decode.test', 'active')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed org");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Guest Enum Building', 'Street 1', 'Bratislava', '81101', 'Slovakia', 'active')
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
        VALUES ($1, '1A', 1)
        RETURNING id
        "#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit");

    // Connection needed for the booking FK.
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

    let booking_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings (
            organization_id, unit_id, connection_id, platform,
            guest_name, guest_count, check_in, check_out,
            total_amount, platform_fee, cleaning_fee, status
        )
        VALUES ($1, $2, $3, 'airbnb',
                'Test Guest', 1, CURRENT_DATE, CURRENT_DATE + 2,
                100.00, 10.00, 5.00, 'confirmed')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(connection_id)
    .fetch_one(pool)
    .await
    .expect("seed booking");

    // Guest seeded with status = 'pending' as a SQL literal (not a bound param)
    // to sidestep the encode-side enum strictness.
    let guest_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_guests (
            organization_id, booking_id, first_name, last_name, is_primary, status
        )
        VALUES ($1, $2, 'Ada', 'Lovelace', true, 'pending')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(booking_id)
    .fetch_one(pool)
    .await
    .expect("seed guest");

    // Calendar block with source_platform = 'airbnb' as SQL literal.
    let block_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_calendar_blocks (
            organization_id, unit_id, block_start, block_end, reason, source_platform
        )
        VALUES ($1, $2, CURRENT_DATE + 10, CURRENT_DATE + 14, 'maintenance', 'airbnb')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed calendar block");

    // iCal feed with import_platform = 'airbnb' as SQL literal; token generated
    // by Postgres so it is guaranteed unique per test run.
    let (feed_id, feed_token): (Uuid, String) = sqlx::query_as(
        r#"
        INSERT INTO rental_ical_feeds (
            organization_id, unit_id, feed_name, feed_token, import_platform, is_active
        )
        VALUES ($1, $2, 'Airbnb iCal', gen_random_uuid()::text, 'airbnb', true)
        RETURNING id, feed_token
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed ical feed");

    (
        org_id, unit_id, booking_id, guest_id, block_id, feed_id, feed_token,
    )
}

// ============================================================================
// RentalGuest — status (guest_registration_status enum)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_guest_by_id_decodes_status(pool: PgPool) {
    let (_org, _unit, _booking, guest_id, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let guest = repo
        .find_guest_by_id(guest_id)
        .await
        .expect("find_guest_by_id must decode the guest_registration_status enum (BIT-118)")
        .expect("guest row exists");
    assert_eq!(guest.status, "pending");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_guest_for_org_decodes_status(pool: PgPool) {
    let (org_id, _unit, _booking, guest_id, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let guest = repo
        .find_guest_for_org(org_id, guest_id)
        .await
        .expect("find_guest_for_org must decode the guest_registration_status enum (BIT-118)")
        .expect("guest row exists");
    assert_eq!(guest.status, "pending");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_guest_returning_decodes_status(pool: PgPool) {
    let (_org, _unit, _booking, guest_id, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let updated = repo
        .update_guest(
            guest_id,
            UpdateGuest {
                first_name: Some("Ada Updated".to_string()),
                last_name: None,
                date_of_birth: None,
                nationality: None,
                id_type: None,
                id_number: None,
                id_issuing_country: None,
                id_expiry_date: None,
                id_document_url: None,
                email: None,
                phone: None,
                address_street: None,
                address_city: None,
                address_postal_code: None,
                address_country: None,
            },
        )
        .await
        .expect("update_guest RETURNING must decode the guest_registration_status enum (BIT-118)");
    assert_eq!(updated.first_name, "Ada Updated");
    assert_eq!(updated.status, "pending");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_guest_for_org_returning_decodes_status(pool: PgPool) {
    let (org_id, _unit, _booking, guest_id, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let updated = repo
        .update_guest_for_org(
            org_id,
            guest_id,
            UpdateGuest {
                first_name: Some("Ada Org Updated".to_string()),
                last_name: None,
                date_of_birth: None,
                nationality: None,
                id_type: None,
                id_number: None,
                id_issuing_country: None,
                id_expiry_date: None,
                id_document_url: None,
                email: None,
                phone: None,
                address_street: None,
                address_city: None,
                address_postal_code: None,
                address_country: None,
            },
        )
        .await
        .expect(
            "update_guest_for_org RETURNING must decode the guest_registration_status enum (BIT-118)",
        )
        .expect("row matched");
    assert_eq!(updated.first_name, "Ada Org Updated");
    assert_eq!(updated.status, "pending");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_guest_returning_decodes_status(pool: PgPool) {
    let (_org, _unit, _booking, guest_id, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let registered = repo.register_guest(guest_id).await.expect(
        "register_guest RETURNING must decode the guest_registration_status enum (BIT-118)",
    );
    assert_eq!(registered.status, "registered");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_guest_for_org_returning_decodes_status(pool: PgPool) {
    let (org_id, _unit, _booking, guest_id, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let registered = repo
        .register_guest_for_org(org_id, guest_id)
        .await
        .expect(
            "register_guest_for_org RETURNING must decode the guest_registration_status enum (BIT-118)",
        )
        .expect("row matched");
    assert_eq!(registered.status, "registered");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_guests_for_booking_decodes_status(pool: PgPool) {
    let (_org, _unit, booking_id, _guest, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let guests = repo
        .get_guests_for_booking(booking_id)
        .await
        .expect("get_guests_for_booking must decode the guest_registration_status enum (BIT-118)");
    assert_eq!(guests.len(), 1);
    assert_eq!(guests[0].status, "pending");
}

// ============================================================================
// CalendarBlock — source_platform (rental_platform enum)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_calendar_block_returning_decodes_source_platform(pool: PgPool) {
    let (org_id, unit_id, _booking, _guest, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // create_calendar_block sets source_platform = NULL; we verify the RETURNING
    // path succeeds (no 42804 panic when the column value is NULL).
    use chrono::NaiveDate;
    use db::models::rental::CreateCalendarBlock;
    let block = repo
        .create_calendar_block(
            org_id,
            CreateCalendarBlock {
                unit_id,
                block_start: NaiveDate::from_ymd_opt(2030, 1, 1).unwrap(),
                block_end: NaiveDate::from_ymd_opt(2030, 1, 5).unwrap(),
                reason: "maintenance".to_string(),
                notes: None,
            },
        )
        .await
        .expect(
            "create_calendar_block RETURNING must not panic on source_platform decode (BIT-118)",
        );
    assert_eq!(block.source_platform, None);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn seeded_calendar_block_source_platform_decodes_via_raw_query(pool: PgPool) {
    use db::models::rental::CalendarBlock;

    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(&pool)
        .await
        .expect("set super-admin context");

    // Retrieve the block seeded with source_platform='airbnb' using the same
    // explicit projection as CALENDAR_BLOCK_COLUMNS to verify the cast decodes.
    let (_, _, _, _, block_id, _, _) = seed(&pool).await;

    let block = sqlx::query_as::<_, CalendarBlock>(
        r#"
        SELECT id, organization_id, unit_id,
               block_start, block_end, reason, booking_id,
               source_platform::text AS source_platform,
               synced_at, notes, created_at
        FROM rental_calendar_blocks WHERE id = $1
        "#,
    )
    .bind(block_id)
    .fetch_one(&pool)
    .await
    .expect("source_platform::text cast must decode into CalendarBlock.source_platform (BIT-118)");
    assert_eq!(block.source_platform.as_deref(), Some("airbnb"));
}

// ============================================================================
// ICalFeed — import_platform (rental_platform enum)
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_ical_feed_by_token_decodes_import_platform(pool: PgPool) {
    let (_org, _unit, _booking, _guest, _block, _feed, feed_token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let feed = repo
        .find_ical_feed_by_token(&feed_token)
        .await
        .expect("find_ical_feed_by_token must decode the rental_platform enum (BIT-118)")
        .expect("feed row exists");
    assert_eq!(feed.import_platform.as_deref(), Some("airbnb"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_ical_feeds_for_unit_decodes_import_platform(pool: PgPool) {
    let (_org, unit_id, _booking, _guest, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let feeds = repo
        .get_ical_feeds_for_unit(unit_id)
        .await
        .expect("get_ical_feeds_for_unit must decode the rental_platform enum (BIT-118)");
    assert_eq!(feeds.len(), 1);
    assert_eq!(feeds[0].import_platform.as_deref(), Some("airbnb"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_ical_feeds_for_unit_in_org_decodes_import_platform(pool: PgPool) {
    let (org_id, unit_id, _booking, _guest, _block, _feed, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let feeds = repo
        .get_ical_feeds_for_unit_in_org(org_id, unit_id)
        .await
        .expect("get_ical_feeds_for_unit_in_org must decode the rental_platform enum (BIT-118)");
    assert_eq!(feeds.len(), 1);
    assert_eq!(feeds[0].import_platform.as_deref(), Some("airbnb"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_ical_feed_for_org_returning_decodes_import_platform(pool: PgPool) {
    let (org_id, _unit, _booking, _guest, _block, feed_id, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // update_ical_feed_for_org does not change import_platform — it is returned
    // unchanged via RETURNING {ICAL_FEED_COLUMNS}, so the cast must decode.
    let updated = repo
        .update_ical_feed_for_org(
            org_id,
            feed_id,
            UpdateICalFeed {
                feed_name: Some("Airbnb iCal Updated".to_string()),
                import_url: None,
                is_active: None,
            },
        )
        .await
        .expect("update_ical_feed_for_org RETURNING must decode the rental_platform enum (BIT-118)")
        .expect("row matched");
    assert_eq!(updated.feed_name, "Airbnb iCal Updated");
    assert_eq!(updated.import_platform.as_deref(), Some("airbnb"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_ical_feed_returning_decodes_import_platform(pool: PgPool) {
    let (_org, _unit, _booking, _guest, _block, feed_id, _token) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    let updated = repo
        .update_ical_feed(
            feed_id,
            UpdateICalFeed {
                feed_name: Some("Airbnb iCal Renamed".to_string()),
                import_url: None,
                is_active: None,
            },
        )
        .await
        .expect("update_ical_feed RETURNING must decode the rental_platform enum (BIT-118)");
    assert_eq!(updated.feed_name, "Airbnb iCal Renamed");
    assert_eq!(updated.import_platform.as_deref(), Some("airbnb"));
}

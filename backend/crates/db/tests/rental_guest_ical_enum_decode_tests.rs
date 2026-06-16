//! Regression test for BIT-118 — sqlx 0.9 refuses to decode a Postgres
//! ENUM column straight into a Rust `String` field. This covers the remaining
//! rental models: `RentalGuest.status` (enum `guest_registration_status`),
//! `CalendarBlock.source_platform` (enum `rental_platform`), and
//! `ICalFeed.import_platform` (enum `rental_platform`).
//!
//! The fix casts these enum columns to text in the repository projections.

use db::models::rental::{
    CreateCalendarBlock, CreateGuest, CreateICalFeed, UpdateGuest, UpdateICalFeed,
};
use db::repositories::RentalRepository;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::NaiveDate;

/// Seed the minimal hierarchy needed for rental tests.
/// Returns `(org_id, unit_id, booking_id)`.
async fn seed(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
    // Super-admin context to bypass RLS during seeding
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");

    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO organizations (name, slug) VALUES ('Rental Test Org', 'rental-test-org') RETURNING id"
    )
    .fetch_one(pool)
    .await
    .expect("seed org");

    let building_id: Uuid = sqlx::query_scalar(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, 'Rental Building', 'Street', 'City', '12345', 'Country') RETURNING id"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit_id: Uuid = sqlx::query_scalar(
        "INSERT INTO units (building_id, name, unit_number) VALUES ($1, 'Unit 101', '101') RETURNING id"
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit");

    let booking_id: Uuid = sqlx::query_scalar(
        "INSERT INTO rental_bookings (organization_id, unit_id, platform, guest_name, check_in, check_out, status) \
         VALUES ($1, $2, 'airbnb', 'Primary Guest', '2026-06-01', '2026-06-05', 'confirmed') RETURNING id"
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed booking");

    (org_id, unit_id, booking_id)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn guest_read_paths_decode_status_enum(pool: PgPool) {
    let (org_id, _unit_id, booking_id) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // Seed a guest with 'pending' status via direct SQL (to avoid create_guest's RETURNING decode)
    let guest_id: Uuid = sqlx::query_scalar(
        "INSERT INTO rental_guests (organization_id, booking_id, first_name, last_name, status) \
         VALUES ($1, $2, 'John', 'Doe', 'pending') RETURNING id"
    )
    .bind(org_id)
    .bind(booking_id)
    .fetch_one(&pool)
    .await
    .expect("seed guest");

    // find_guest_by_id
    let guest = repo.find_guest_by_id(guest_id)
        .await
        .expect("find_guest_by_id must decode status enum")
        .expect("guest exists");
    assert_eq!(guest.status, "pending");

    // find_guest_for_org
    let guest = repo.find_guest_for_org(org_id, guest_id)
        .await
        .expect("find_guest_for_org must decode status enum")
        .expect("guest exists");
    assert_eq!(guest.status, "pending");

    // get_guests_for_booking
    let guests = repo.get_guests_for_booking(booking_id)
        .await
        .expect("get_guests_for_booking must decode status enum");
    assert_eq!(guests.len(), 1);
    assert_eq!(guests[0].status, "pending");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn guest_mutation_paths_decode_status_enum(pool: PgPool) {
    let (org_id, _unit_id, booking_id) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // create_guest (RETURNING)
    let guest = repo.create_guest(org_id, CreateGuest {
        booking_id,
        first_name: "Jane".to_string(),
        last_name: "Smith".to_string(),
        date_of_birth: None,
        nationality: None,
        id_type: None,
        id_number: None,
        id_issuing_country: None,
        id_expiry_date: None,
        email: None,
        phone: None,
        address_street: None,
        address_city: None,
        address_postal_code: None,
        address_country: None,
        is_primary: false,
    })
    .await
    .expect("create_guest RETURNING must decode status enum");
    assert_eq!(guest.status, "pending");

    // update_guest (RETURNING)
    let updated = repo.update_guest(guest.id, UpdateGuest {
        first_name: Some("Jane Updated".to_string()),
        ..Default::default()
    })
    .await
    .expect("update_guest RETURNING must decode status enum");
    assert_eq!(updated.status, "pending");

    // register_guest (RETURNING)
    let registered = repo.register_guest(guest.id)
        .await
        .expect("register_guest RETURNING must decode status enum");
    assert_eq!(registered.status, "registered");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn calendar_block_mutation_decodes_platform_enum(pool: PgPool) {
    let (org_id, unit_id, _booking_id) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // create_calendar_block (RETURNING)
    // Note: source_platform is usually set by sync, but here it starts as NULL.
    // We want to make sure the projection still works.
    let block = repo.create_calendar_block(org_id, CreateCalendarBlock {
        unit_id,
        block_start: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
        block_end: NaiveDate::from_ymd_opt(2026, 7, 5).unwrap(),
        reason: "maintenance".to_string(),
        notes: None,
    })
    .await
    .expect("create_calendar_block RETURNING must decode source_platform enum");
    assert!(block.source_platform.is_none());

    // Seed a block with a platform directly and then we can check a read if we had one that returns CalendarBlock.
    // get_calendar_events returns CalendarEvent, not CalendarBlock, but it also casts source_platform.
    let events = repo.get_calendar_events(unit_id, NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(), NaiveDate::from_ymd_opt(2026, 7, 5).unwrap())
        .await
        .expect("get_calendar_events must decode source_platform enum");
    assert_eq!(events.len(), 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ical_feed_read_mutation_paths_decode_platform_enum(pool: PgPool) {
    let (org_id, unit_id, _booking_id) = seed(&pool).await;
    let repo = RentalRepository::new(pool.clone());

    // create_ical_feed (RETURNING)
    let feed = repo.create_ical_feed(org_id, CreateICalFeed {
        unit_id,
        feed_name: "My Feed".to_string(),
        import_url: Some("http://example.com/ical".to_string()),
        import_platform: Some("airbnb".to_string()),
    })
    .await
    .expect("create_ical_feed RETURNING must decode import_platform enum");
    assert_eq!(feed.import_platform, Some("airbnb".to_string()));

    // find_ical_feed_by_token
    let found = repo.find_ical_feed_by_token(&feed.feed_token)
        .await
        .expect("find_ical_feed_by_token must decode import_platform enum")
        .expect("feed exists");
    assert_eq!(found.import_platform, Some("airbnb".to_string()));

    // get_ical_feeds_for_unit
    let feeds = repo.get_ical_feeds_for_unit(unit_id)
        .await
        .expect("get_ical_feeds_for_unit must decode import_platform enum");
    assert_eq!(feeds.len(), 1);
    assert_eq!(feeds[0].import_platform, Some("airbnb".to_string()));

    // update_ical_feed_for_org (RETURNING)
    let updated = repo.update_ical_feed_for_org(org_id, feed.id, UpdateICalFeed {
        feed_name: Some("Updated Feed".to_string()),
        ..Default::default()
    })
    .await
    .expect("update_ical_feed_for_org RETURNING must decode import_platform enum")
    .expect("feed exists");
    assert_eq!(updated.import_platform, Some("airbnb".to_string()));
}

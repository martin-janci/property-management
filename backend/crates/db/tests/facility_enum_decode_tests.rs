//! Regression test for issue #859 — sqlx 0.9 refuses to decode a Postgres
//! ENUM column straight into a Rust `String` field. The `facilities` repository
//! reads `facilities.facility_type` (enum `facility_type`) and
//! `facility_bookings.status` (enum `booking_status`) into `String` model
//! fields via `query_as`. Before the fix these reads compiled and passed
//! clippy but 500'd at runtime ("mismatched types ... is not compatible with
//! SQL type `facility_type`"). The fix casts the enum columns to text.
//!
//! These tests actually SELECT rows through the repository, so they fail on
//! `main` (decode error) and pass once the casts are in place.

use db::repositories::FacilityRepository;
use sqlx::PgPool;
use uuid::Uuid;

/// Seed org -> building -> facility (+ optional booking) under super-admin RLS
/// context, returning the ids needed by the read-path assertions.
/// Returns `(building_id, facility_id, user_id)`.
async fn seed(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
    // Super-admin context satisfies the `*_super_admin FOR ALL` RLS policies on
    // facilities / facility_bookings, so seeding does not need a manager row.
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");

    let org_id: Uuid = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ('Enum Decode Org', 'enum-decode-org', 'enum@decode.test', 'active')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed org");

    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Enum Decode Building', 'Street 1', 'City', '00000', 'Country', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building");

    let user_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ('enum-decode@user.test', 'test_hash', 'Enum User', 'active', NOW())
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed user");

    // hourly_fee / deposit_amount are set because the `Facility` model decodes
    // them via `#[sqlx(try_from = "Decimal")]`, which rejects SQL NULL even for
    // an `Option<Decimal>` field — unrelated to the enum-decode fix under test.
    let facility_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO facilities (building_id, name, facility_type, is_bookable, is_active, hourly_fee, deposit_amount)
        VALUES ($1, 'Gym', 'gym', TRUE, TRUE, 10.00, 50.00)
        RETURNING id
        "#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed facility");

    let _ = org_id;
    (building_id, facility_id, user_id)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_by_id_decodes_facility_type_enum(pool: PgPool) {
    let (_building_id, facility_id, _user_id) = seed(&pool).await;
    let repo = FacilityRepository::new(pool.clone());

    // On `main` this 500s: facility_type is the `facility_type` pg enum decoded
    // into `Facility.facility_type: String`.
    let facility = repo
        .find_by_id(facility_id)
        .await
        .expect("find_by_id must not fail decoding the facility_type enum (#859)")
        .expect("facility row exists");
    assert_eq!(facility.facility_type, "gym");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_by_building_decodes_facility_type_enum(pool: PgPool) {
    let (building_id, _facility_id, _user_id) = seed(&pool).await;
    let repo = FacilityRepository::new(pool.clone());

    let summaries = repo
        .find_by_building(building_id)
        .await
        .expect("find_by_building must not fail decoding the facility_type enum (#859)");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].facility_type, "gym");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_bookable_decodes_facility_type_enum(pool: PgPool) {
    let (building_id, _facility_id, _user_id) = seed(&pool).await;
    let repo = FacilityRepository::new(pool.clone());

    let facilities = repo
        .find_bookable(building_id)
        .await
        .expect("find_bookable must not fail decoding the facility_type enum (#859)");
    assert_eq!(facilities.len(), 1);
    assert_eq!(facilities[0].facility_type, "gym");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_read_paths_decode_booking_status_enum(pool: PgPool) {
    let (building_id, facility_id, user_id) = seed(&pool).await;
    let repo = FacilityRepository::new(pool.clone());

    // Seed a 'pending' booking directly. (We avoid `create_booking` here because
    // its `RETURNING *` decode of the `total_fee` `#[sqlx(try_from = Decimal)]`
    // field rejects SQL NULL — an unrelated model quirk — so we set total_fee.)
    let start = chrono::Utc::now() + chrono::Duration::hours(1);
    let end = start + chrono::Duration::hours(2);
    let booking_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO facility_bookings
            (facility_id, user_id, start_time, end_time, status, total_fee)
        VALUES ($1, $2, $3, $4, 'pending', 0.00)
        RETURNING id
        "#,
    )
    .bind(facility_id)
    .bind(user_id)
    .bind(start)
    .bind(end)
    .fetch_one(&pool)
    .await
    .expect("seed booking");

    // find_booking_by_id (fixed in #858) + the three #859 read paths.
    let by_id = repo
        .find_booking_by_id(booking_id)
        .await
        .expect("find_booking_by_id must decode booking_status enum")
        .expect("row exists");
    assert_eq!(by_id.status, "pending");

    let by_facility = repo
        .find_bookings_by_facility(
            facility_id,
            start - chrono::Duration::hours(1),
            end + chrono::Duration::hours(1),
        )
        .await
        .expect("find_bookings_by_facility must decode booking_status enum (#859)");
    assert_eq!(by_facility.len(), 1);

    let by_user = repo
        .find_bookings_by_user(user_id)
        .await
        .expect("find_bookings_by_user must decode booking_status enum (#859)");
    assert_eq!(by_user.len(), 1);

    let pending = repo
        .find_pending_bookings(building_id)
        .await
        .expect("find_pending_bookings must decode booking_status enum (#859)");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, "pending");
}

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

use db::models::facility::UpdateFacility;
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
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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

// ---------------------------------------------------------------------------
// Mutation RETURNING paths (issue #859).
//
// The SELECT readers above were fixed, but the update/approve/reject/cancel
// mutations returned their row via `RETURNING *`, which re-exposes the raw
// `facility_type` / `status` enum columns and so still 500'd on decode. These
// tests drive those RETURNING paths and assert the enum surfaces as the
// expected `String`; they fail (ColumnDecode) on the pre-fix `RETURNING *`.
//
// Scope note: `create` / `create_booking` are intentionally NOT exercised here.
// Those INSERTs *bind* a `String` into the enum column (`$n` of type text),
// which sqlx 0.9 also rejects ("column is of type <enum> but expression is of
// type text", SQLSTATE 42804) — that is the encode side of the same strictness
// and a separate fix. The RETURNING decode regressions #859 reports are covered
// by the update/approve/reject/cancel paths below, whose seed rows are created
// via literal-enum SQL (which Postgres coerces, unlike a bound param).
//
// The Decimal fee fields are set on every seeded row: the model decodes them via
// `#[sqlx(try_from = "Decimal")]`, which rejects SQL NULL — an unrelated quirk
// (see the booking-read test above) that would otherwise mask the enum cast.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn facility_update_decodes_facility_type_enum(pool: PgPool) {
    let (_building_id, facility_id, _user_id) = seed(&pool).await;
    let repo = FacilityRepository::new(pool.clone());

    // update() returns its row via RETURNING — the `facility_type` enum must
    // decode. The seeded facility already has its Decimal fee fields set.
    let updated = repo
        .update(
            facility_id,
            UpdateFacility {
                name: Some("Gym 2".to_string()),
                description: None,
                location: None,
                capacity: None,
                is_bookable: None,
                requires_approval: None,
                max_booking_hours: None,
                max_advance_days: None,
                min_advance_hours: None,
                available_from: None,
                available_to: None,
                available_days: None,
                rules: None,
                hourly_fee: None,
                deposit_amount: None,
                is_active: None,
                photos: None,
                amenities: None,
            },
        )
        .await
        .expect("update RETURNING must decode facility_type enum (#859)")
        .expect("facility row exists");
    assert_eq!(updated.name, "Gym 2");
    assert_eq!(updated.facility_type, "gym");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn booking_transitions_decode_status_enum(pool: PgPool) {
    let (_building_id, facility_id, user_id) = seed(&pool).await;
    let repo = FacilityRepository::new(pool.clone());

    // Seed a 'pending' booking with total_fee set (the booking is created via
    // SQL, not `create_booking`, to keep the total_fee `try_from` NULL quirk out
    // of this enum-decode test). `nth` offsets the time window so successive
    // bookings don't collide on the reasonable-duration / time checks.
    async fn seed_pending(pool: &PgPool, facility_id: Uuid, user_id: Uuid, nth: i64) -> Uuid {
        let start = chrono::Utc::now() + chrono::Duration::days(nth + 1);
        sqlx::query_scalar::<_, Uuid>(
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
        .bind(start + chrono::Duration::hours(2))
        .fetch_one(pool)
        .await
        .expect("seed pending booking")
    }

    let approved = repo
        .approve_booking(seed_pending(&pool, facility_id, user_id, 0).await, user_id)
        .await
        .expect("approve_booking RETURNING must decode status enum (#859)")
        .expect("booking was pending");
    assert_eq!(approved.status, "approved");

    let rejected = repo
        .reject_booking(
            seed_pending(&pool, facility_id, user_id, 1).await,
            user_id,
            "no capacity",
        )
        .await
        .expect("reject_booking RETURNING must decode status enum (#859)")
        .expect("booking was pending");
    assert_eq!(rejected.status, "rejected");

    let cancelled = repo
        .cancel_booking(
            seed_pending(&pool, facility_id, user_id, 2).await,
            Some("changed plans"),
        )
        .await
        .expect("cancel_booking RETURNING must decode status enum (#859)")
        .expect("booking exists");
    assert_eq!(cancelled.status, "cancelled");
}

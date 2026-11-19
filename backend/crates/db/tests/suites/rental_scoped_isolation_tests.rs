//! Cross-tenant isolation regression tests for the org-scoped rental booking /
//! guest / connection repository methods.
//!
//! Background: alongside the hardened `*_for_org` methods (#804), the
//! `RentalRepository` carried a parallel family of UNSCOPED helpers
//! (`find_booking_by_id`, `update_booking`, `find_guest_by_id`, `update_guest`,
//! `register_guest`, `delete_guest`, `get_booking_with_guests`) that looked up /
//! mutated a row by its UUID alone, with NO `organization_id` predicate. The
//! production routes had already migrated to the `*_for_org` variants, so the
//! unscoped ones were dead — but they remained a latent IDOR footgun: any future
//! handler reaching for the obvious `find_guest_by_id` name would silently read
//! another tenant's guest PII (id number, date of birth, …) by UUID. Those dead
//! methods have been removed; these tests lock in that the surviving scoped
//! methods are the only path AND that every one of them enforces the org guard.
//!
//! The seed runs under the super-admin RLS context, so RLS is bypassed and the
//! ONLY thing that can keep org B out of org A's rows is the app-level
//! `AND organization_id = $n` predicate in each query — which is exactly what is
//! under test here. A different-org caller must get `None` / `false`, never the
//! row.

use db::models::rental::{UpdateBooking, UpdateGuest};
use db::repositories::RentalRepository;
use sqlx::PgPool;
use uuid::Uuid;

struct Fixture {
    org: Uuid,
    booking: Uuid,
    guest: Uuid,
    connection: Uuid,
}

/// Seed `org -> building -> unit -> booking -> primary guest` plus one
/// token-bearing `rental_platform_connections` row, under the super-admin
/// context, and return the ids. `tag` keeps slugs/emails unique.
async fn seed(pool: &PgPool, tag: &str) -> Fixture {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Option::<Uuid>::None)
        .bind(true)
        .execute(pool)
        .await
        .expect("set super-admin context");

    let org = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Rental Iso Org {tag}"))
    .bind(format!("rental-iso-{tag}"))
    .bind(format!("rental-iso-{tag}@isolation.test"))
    .fetch_one(pool)
    .await
    .expect("seed org");

    let building = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'Iso Building', 'Street 1', 'City', '00000', 'Country', 'active')
        RETURNING id
        "#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, '1A') RETURNING id"#,
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("seed unit");

    let booking = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings
            (organization_id, unit_id, platform, guest_name, check_in, check_out)
        VALUES ($1, $2, 'airbnb', 'Iso Guest', DATE '2026-01-01', DATE '2026-01-05')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .fetch_one(pool)
    .await
    .expect("seed booking");

    let guest = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_guests
            (organization_id, booking_id, first_name, last_name, is_primary, status)
        VALUES ($1, $2, 'Ada', 'Lovelace', true, 'pending')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(booking)
    .fetch_one(pool)
    .await
    .expect("seed guest");

    // A platform connection carrying OAuth credentials. This is the highest
    // value IDOR target in the rental domain: a leaked row exposes the
    // `access_token` / `refresh_token` for the org's Airbnb account.
    let connection = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections
            (organization_id, unit_id, platform, access_token, refresh_token)
        VALUES ($1, $2, 'airbnb', 'secret-access-token', 'secret-refresh-token')
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(unit)
    .fetch_one(pool)
    .await
    .expect("seed connection");

    Fixture {
        org,
        booking,
        guest,
        connection,
    }
}

fn empty_booking_update() -> UpdateBooking {
    UpdateBooking {
        guest_name: Some("Hijacked".to_string()),
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
    }
}

fn empty_guest_update() -> UpdateGuest {
    UpdateGuest {
        first_name: Some("Hijacked".to_string()),
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
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_booking_for_org_rejects_foreign_org(pool: PgPool) {
    let a = seed(&pool, "fb-a").await;
    let b = seed(&pool, "fb-b").await;
    let repo = RentalRepository::new(pool.clone());

    // Org B must not read org A's booking by UUID.
    assert!(
        repo.find_booking_for_org(b.org, a.booking)
            .await
            .expect("query ok")
            .is_none(),
        "cross-tenant find_booking_for_org must return None"
    );
    // Sanity: the owning org still reads it.
    assert!(repo
        .find_booking_for_org(a.org, a.booking)
        .await
        .expect("query ok")
        .is_some());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_booking_with_guests_for_org_rejects_foreign_org(pool: PgPool) {
    let a = seed(&pool, "bwg-a").await;
    let b = seed(&pool, "bwg-b").await;
    let repo = RentalRepository::new(pool.clone());

    assert!(
        repo.get_booking_with_guests_for_org(b.org, a.booking)
            .await
            .expect("query ok")
            .is_none(),
        "cross-tenant get_booking_with_guests_for_org must return None (no PII leak)"
    );
    assert!(repo
        .get_booking_with_guests_for_org(a.org, a.booking)
        .await
        .expect("query ok")
        .is_some());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_booking_for_org_rejects_foreign_org(pool: PgPool) {
    let a = seed(&pool, "ub-a").await;
    let b = seed(&pool, "ub-b").await;
    let repo = RentalRepository::new(pool.clone());

    assert!(
        repo.update_booking_for_org(b.org, a.booking, empty_booking_update())
            .await
            .expect("query ok")
            .is_none(),
        "cross-tenant update_booking_for_org must match no row"
    );
    // The booking's guest_name must be untouched by the foreign write.
    let name: String = sqlx::query_scalar("SELECT guest_name FROM rental_bookings WHERE id = $1")
        .bind(a.booking)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "Iso Guest", "foreign update must not mutate the row");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_connection_for_org_rejects_foreign_org(pool: PgPool) {
    // `find_connection_for_org` is the ONLY surviving by-id connection lookup
    // (the unkeyed `find_connection_by_id` was removed with the legacy OAuth
    // callbacks). It backs `GET /api/v1/rentals/connections/{id}`, so this test
    // locks in that a caller from org B cannot read org A's connection — and,
    // critically, its `access_token` / `refresh_token` — by guessing the UUID.
    let a = seed(&pool, "fc-a").await;
    let b = seed(&pool, "fc-b").await;
    let repo = RentalRepository::new(pool.clone());

    // Org B must not read org A's connection by UUID (no OAuth-token leak).
    assert!(
        repo.find_connection_for_org(b.org, a.connection)
            .await
            .expect("query ok")
            .is_none(),
        "cross-tenant find_connection_for_org must return None (no OAuth-token leak)"
    );
    // Sanity: the owning org still reads its own connection.
    assert!(repo
        .find_connection_for_org(a.org, a.connection)
        .await
        .expect("query ok")
        .is_some());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_guest_for_org_rejects_foreign_org(pool: PgPool) {
    let a = seed(&pool, "fg-a").await;
    let b = seed(&pool, "fg-b").await;
    let repo = RentalRepository::new(pool.clone());

    assert!(
        repo.find_guest_for_org(b.org, a.guest)
            .await
            .expect("query ok")
            .is_none(),
        "cross-tenant find_guest_for_org must return None (no guest PII leak)"
    );
    assert!(repo
        .find_guest_for_org(a.org, a.guest)
        .await
        .expect("query ok")
        .is_some());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_guest_for_org_rejects_foreign_org(pool: PgPool) {
    let a = seed(&pool, "ug-a").await;
    let b = seed(&pool, "ug-b").await;
    let repo = RentalRepository::new(pool.clone());

    assert!(
        repo.update_guest_for_org(b.org, a.guest, empty_guest_update())
            .await
            .expect("query ok")
            .is_none(),
        "cross-tenant update_guest_for_org must match no row"
    );
    let first: String = sqlx::query_scalar("SELECT first_name FROM rental_guests WHERE id = $1")
        .bind(a.guest)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(first, "Ada", "foreign update must not mutate the guest");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn register_guest_for_org_rejects_foreign_org(pool: PgPool) {
    let a = seed(&pool, "rg-a").await;
    let b = seed(&pool, "rg-b").await;
    let repo = RentalRepository::new(pool.clone());

    assert!(
        repo.register_guest_for_org(b.org, a.guest)
            .await
            .expect("query ok")
            .is_none(),
        "cross-tenant register_guest_for_org must match no row"
    );
    let status: String = sqlx::query_scalar("SELECT status::text FROM rental_guests WHERE id = $1")
        .bind(a.guest)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "pending", "foreign register must not flip status");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_guest_for_org_rejects_foreign_org(pool: PgPool) {
    let a = seed(&pool, "dg-a").await;
    let b = seed(&pool, "dg-b").await;
    let repo = RentalRepository::new(pool.clone());

    assert!(
        !repo
            .delete_guest_for_org(b.org, a.guest)
            .await
            .expect("query ok"),
        "cross-tenant delete_guest_for_org must delete nothing"
    );
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM rental_guests WHERE id = $1)")
            .bind(a.guest)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(exists, "foreign delete must not remove the guest");
}

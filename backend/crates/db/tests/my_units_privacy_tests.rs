//! Privacy tests for the resident-facing "My Unit" view (Epic 3, Story 3.6 /
//! BIT-197). These prove the privacy filter is enforced **server-side in the
//! query layer**, not merely hidden in the UI.
//!
//! `UnitResidentRepository::find_my_units(user_id)` backs `GET
//! /api/v1/users/me/units`. The done-criteria for the feature require:
//!   - a resident sees only their own unit(s) and association status, and
//!   - other residents'/owners' PII is never exposed.
//!
//! The strongest proof of the second point is structural + behavioural: when
//! two residents share the same unit, `find_my_units` resolved for one resident
//! returns ONLY that caller's association — the other resident's residency row
//! (and the `MyUnitRow` projection carries no field for other residents at all)
//! never appears. We also confirm the query is strictly scoped by `user_id`
//! (no cross-resident bleed) and that ended residencies are excluded.
//!
//! DB-gated (`#[sqlx::test(migrator = "db::MIGRATOR")]`): CI provisions Postgres and runs these.

use db::repositories::UnitResidentRepository;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn insert_org(conn: &mut PgConnection, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $2, $3, 'active') RETURNING id",
    )
    .bind(format!("BIT-197 {slug}"))
    .bind(slug)
    .bind(format!("{slug}@bit197.test"))
    .fetch_one(conn)
    .await
    .expect("insert org")
}

async fn insert_user(conn: &mut PgConnection, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', $2, 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .bind(format!("User {email}"))
    .fetch_one(conn)
    .await
    .expect("insert user")
}

async fn insert_building(conn: &mut PgConnection, org: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, $2, 'Addr', 'City', '00000', 'Country') RETURNING id",
    )
    .bind(org)
    .bind(name)
    .fetch_one(conn)
    .await
    .expect("insert building")
}

async fn insert_unit(conn: &mut PgConnection, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(conn)
    .await
    .expect("insert unit")
}

/// Active residency (`end_date IS NULL`). Returns the `unit_residents` id.
async fn insert_resident(
    conn: &mut PgConnection,
    unit: Uuid,
    user: Uuid,
    resident_type: &str,
    is_primary: bool,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type, is_primary) \
         VALUES ($1, $2, $3::resident_type, $4) RETURNING id",
    )
    .bind(unit)
    .bind(user)
    .bind(resident_type)
    .bind(is_primary)
    .fetch_one(conn)
    .await
    .expect("insert resident")
}

/// Ended residency (`end_date` in the past) — should be excluded from the
/// resident-facing view.
async fn insert_ended_resident(conn: &mut PgConnection, unit: Uuid, user: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type, start_date, end_date) \
         VALUES ($1, $2, 'tenant'::resident_type, CURRENT_DATE - 365, CURRENT_DATE - 30) \
         RETURNING id",
    )
    .bind(unit)
    .bind(user)
    .fetch_one(conn)
    .await
    .expect("insert ended resident")
}

/// Two residents share one unit. `find_my_units(alice)` must return ONLY
/// Alice's own association — Bob's residency is never surfaced, proving other
/// residents' PII is filtered server-side.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_my_units_returns_only_callers_association(pool: PgPool) {
    let mut conn = pool.acquire().await.expect("acquire");
    let org = insert_org(&mut conn, "shared").await;
    let building = insert_building(&mut conn, org, "B1").await;
    let unit = insert_unit(&mut conn, building, "1A").await;

    let alice = insert_user(&mut conn, "alice@bit197.test").await;
    let bob = insert_user(&mut conn, "bob@bit197.test").await;
    let alice_residency = insert_resident(&mut conn, unit, alice, "owner", true).await;
    let bob_residency = insert_resident(&mut conn, unit, bob, "tenant", false).await;

    let repo = UnitResidentRepository::new(pool.clone());

    let alice_units = repo
        .find_my_units(alice)
        .await
        .expect("alice find_my_units");
    assert_eq!(alice_units.len(), 1, "Alice should see exactly her unit");
    let row = &alice_units[0];
    assert_eq!(row.unit_id, unit);
    assert_eq!(
        row.resident_id, alice_residency,
        "the association returned must be Alice's own residency, not Bob's"
    );
    assert_ne!(
        row.resident_id, bob_residency,
        "Bob's residency must never appear in Alice's My Unit view"
    );
    assert_eq!(row.resident_type, "owner");
    assert!(row.is_primary);

    // Symmetric: Bob sees only his own association for the same shared unit.
    let bob_units = repo.find_my_units(bob).await.expect("bob find_my_units");
    assert_eq!(bob_units.len(), 1);
    assert_eq!(bob_units[0].resident_id, bob_residency);
    assert_eq!(bob_units[0].resident_type, "tenant");
}

/// A user with no residency (and other tenants' units in the DB) sees nothing.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_my_units_is_scoped_strictly_by_user(pool: PgPool) {
    let mut conn = pool.acquire().await.expect("acquire");
    let org = insert_org(&mut conn, "scope").await;
    let building = insert_building(&mut conn, org, "B1").await;
    let unit = insert_unit(&mut conn, building, "2B").await;

    let resident = insert_user(&mut conn, "resident@bit197.test").await;
    let stranger = insert_user(&mut conn, "stranger@bit197.test").await;
    insert_resident(&mut conn, unit, resident, "owner", true).await;

    let repo = UnitResidentRepository::new(pool.clone());

    let stranger_units = repo.find_my_units(stranger).await.expect("stranger");
    assert!(
        stranger_units.is_empty(),
        "a non-resident must not see any units, even ones that exist in the DB"
    );

    let resident_units = repo.find_my_units(resident).await.expect("resident");
    assert_eq!(resident_units.len(), 1);
    assert_eq!(resident_units[0].unit_id, unit);
}

/// Ended residencies (moved-out) are excluded; only active associations show.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_my_units_excludes_ended_residencies(pool: PgPool) {
    let mut conn = pool.acquire().await.expect("acquire");
    let org = insert_org(&mut conn, "ended").await;
    let building = insert_building(&mut conn, org, "B1").await;
    let active_unit = insert_unit(&mut conn, building, "3C").await;
    let old_unit = insert_unit(&mut conn, building, "old").await;

    let user = insert_user(&mut conn, "mover@bit197.test").await;
    insert_resident(&mut conn, active_unit, user, "tenant", true).await;
    insert_ended_resident(&mut conn, old_unit, user).await;

    let repo = UnitResidentRepository::new(pool.clone());

    let units = repo.find_my_units(user).await.expect("find_my_units");
    assert_eq!(
        units.len(),
        1,
        "only the active residency should be returned"
    );
    assert_eq!(units[0].unit_id, active_unit);
}

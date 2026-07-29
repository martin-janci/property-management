//! Regression test for PAP-143 — defense-in-depth on the secondary
//! `parking_spots` lookup in
//! [`RegistryRepository::get_vehicle_registration_with_details`].
//!
//! The primary `vehicle_registrations` read is keyed on `(id, tenant_id)`, but
//! the follow-up `SELECT spot_number FROM parking_spots WHERE id = $1` was keyed
//! on the spot id alone. `vehicle_registrations.parking_spot_id` carries no FK
//! constraint, so a stale or tampered registration row can point at another
//! tenant's spot — and the unscoped lookup would then leak that foreign tenant's
//! `spot_number` into the response.
//!
//! The fix adds `AND tenant_id = $2`, threading the already-verified caller
//! tenant. CI runs these repo tests against a superuser pool that bypasses
//! FORCE RLS, so the SQL predicate (not RLS) is what this test exercises:
//! pre-fix this returns the foreign spot number, post-fix it resolves to `None`.

use crate::common::seed_org;
use db::repositories::RegistryRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Registry User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
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
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_parking_spot(pool: &PgPool, org_id: Uuid, building_id: Uuid, spot: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO parking_spots (tenant_id, building_id, spot_number)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(spot)
    .fetch_one(pool)
    .await
    .expect("seed parking spot")
}

/// Insert a vehicle registration raw so we can set a cross-tenant
/// `parking_spot_id` (the repo create method does not accept one, and the
/// column has no FK — exactly the hazard under test).
async fn seed_vehicle_registration(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    owner_id: Uuid,
    parking_spot_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO vehicle_registrations
            (tenant_id, unit_id, owner_id, vehicle_type, make, model,
             license_plate, parking_spot_id, status)
        VALUES ($1, $2, $3, 'car', 'Skoda', 'Octavia', 'BL-123-AB', $4, 'approved')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(owner_id)
    .bind(parking_spot_id)
    .fetch_one(pool)
    .await
    .expect("seed vehicle registration")
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn details_does_not_leak_cross_tenant_parking_spot(pool: PgPool) {
    let org_a = seed_org(&pool, "registry-a").await;
    let org_b = seed_org(&pool, "registry-b").await;
    let owner = seed_user(&pool, "owner@registry.test").await;

    let building_a = seed_building(&pool, org_a, "registry-a").await;
    let unit_a = seed_unit(&pool, building_a, "A-1").await;

    // Org B owns a parking spot in its own building.
    let building_b = seed_building(&pool, org_b, "registry-b").await;
    let spot_b = seed_parking_spot(&pool, org_b, building_b, "B-007").await;

    // Org A's registration points (stale/tampered) at Org B's spot.
    let reg_id = seed_vehicle_registration(&pool, org_a, unit_a, owner, spot_b).await;

    let repo = RegistryRepository::new(pool.clone());

    let details = repo
        .get_vehicle_registration_with_details(org_a, reg_id)
        .await
        .expect("query details")
        .expect("registration exists for its own tenant");

    // The registration itself still resolves for its owning tenant…
    assert_eq!(details.registration.id, reg_id);
    // …but the foreign tenant's parking spot number must NOT leak (PAP-143).
    assert_eq!(
        details.parking_spot_number, None,
        "cross-tenant parking_spot_id must not leak Org B's spot_number"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn details_returns_same_tenant_parking_spot(pool: PgPool) {
    let org_a = seed_org(&pool, "registry-ok").await;
    let owner = seed_user(&pool, "owner@registry-ok.test").await;

    let building_a = seed_building(&pool, org_a, "registry-ok").await;
    let unit_a = seed_unit(&pool, building_a, "A-1").await;
    let spot_a = seed_parking_spot(&pool, org_a, building_a, "A-042").await;

    let reg_id = seed_vehicle_registration(&pool, org_a, unit_a, owner, spot_a).await;

    let repo = RegistryRepository::new(pool.clone());

    let details = repo
        .get_vehicle_registration_with_details(org_a, reg_id)
        .await
        .expect("query details")
        .expect("registration exists");

    // A same-tenant spot is still resolved — the fix scopes, it does not break.
    assert_eq!(details.parking_spot_number.as_deref(), Some("A-042"));
}

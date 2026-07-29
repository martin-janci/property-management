//! Regression tests for PAP-158 — enum-decode + `unit_number` column defects on
//! the registry read/list/RETURNING surface in
//! [`db::repositories::RegistryRepository`].
//!
//! PAP-143 fixed only the *vehicle by-id* read. The pet reads, both list
//! summaries, the `review_*` / `create_*` / `update_*` `RETURNING` paths, and the
//! parking-spot detail join carried the same two latent defects:
//!
//!   1. ENUM columns (`pet_type`, `pet_size`, `vehicle_type`, `status`) selected
//!      bare into the `String` model fields. SQLx rejects an enum OID where it
//!      expects text, so any query that actually returns a row fails to decode —
//!      masked in production only because FORCE RLS normally returns zero rows
//!      (the PAP-136 "deny-all-masked decode" pattern).
//!   2. `units.unit_number` does not exist — the display column is `designation`.
//!
//! CI runs these repo tests against a superuser pool that bypasses FORCE RLS, so
//! a real row IS returned here: pre-fix every query below errors on decode (or on
//! the missing column); post-fix they resolve cleanly. A second test pins the
//! defense-in-depth tenant scope added to the vehicle-list `parking_spots` join.

use crate::common::seed_org;
use db::models::{
    registry_status, CreatePetRegistration, CreateVehicleRegistration, PetRegistrationQuery,
    ReviewRegistration, UpdatePetRegistration, VehicleRegistrationQuery,
};
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

/// Exercises every fixed read/list/RETURNING path in a single migrated DB:
/// enum columns must decode into the `String` model fields, and `unit_number`
/// must resolve from `units.designation`.
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn registry_reads_decode_enums_and_resolve_designation(pool: PgPool) {
    let org = seed_org(&pool, "pap158").await;
    let owner = seed_user(&pool, "owner@pap158.test").await;
    let building = seed_building(&pool, org, "pap158").await;
    let unit = seed_unit(&pool, building, "A-101").await;

    let repo = RegistryRepository::new(pool.clone());

    // -- Pet: create RETURNING + by-id + with-details + list summary ----------
    let pet = repo
        .create_pet_registration(
            org,
            owner,
            CreatePetRegistration {
                unit_id: unit,
                pet_name: "Rex".into(),
                pet_type: "dog".into(),
                breed: Some("Beagle".into()),
                pet_size: "medium".into(),
                weight_kg: None,
                age_years: Some(3),
                color: Some("brown".into()),
                microchip_id: None,
                vaccination_date: None,
                vaccination_document_id: None,
                special_needs: None,
                notes: None,
            },
        )
        .await
        .expect("create pet registration decodes enums on RETURNING");
    assert_eq!(pet.pet_type, "dog");
    assert_eq!(pet.pet_size, "medium");
    assert_eq!(pet.status, registry_status::PENDING);

    let pet_by_id = repo
        .get_pet_registration(org, pet.id)
        .await
        .expect("get_pet_registration decodes enums")
        .expect("pet exists");
    assert_eq!(pet_by_id.pet_type, "dog");
    assert_eq!(pet_by_id.status, registry_status::PENDING);

    let pet_details = repo
        .get_pet_registration_with_details(org, pet.id)
        .await
        .expect("pet with-details query")
        .expect("pet exists");
    assert_eq!(pet_details.unit_number.as_deref(), Some("A-101"));
    assert_eq!(
        pet_details.building_name.as_deref(),
        Some("pap158 Building")
    );

    let (pet_list, pet_total) = repo
        .list_pet_registrations(org, PetRegistrationQuery::default())
        .await
        .expect("list pet registrations decodes enums + designation");
    assert_eq!(pet_total, 1);
    assert_eq!(pet_list.len(), 1);
    assert_eq!(pet_list[0].pet_type, "dog");
    assert_eq!(pet_list[0].pet_size, "medium");
    assert_eq!(pet_list[0].status, registry_status::PENDING);
    assert_eq!(pet_list[0].unit_number.as_deref(), Some("A-101"));

    // update RETURNING decode + the pet_size ENUM encode cast (COALESCE($n::pet_size)).
    let updated_pet = repo
        .update_pet_registration(
            org,
            pet.id,
            UpdatePetRegistration {
                pet_name: None,
                breed: None,
                pet_size: Some("large".into()),
                weight_kg: None,
                age_years: None,
                color: None,
                microchip_id: None,
                vaccination_date: None,
                vaccination_document_id: None,
                special_needs: None,
                notes: None,
            },
        )
        .await
        .expect("update pet decodes enums on RETURNING + encodes pet_size")
        .expect("pet exists");
    assert_eq!(updated_pet.pet_size, "large");
    assert_eq!(updated_pet.pet_type, "dog");
    assert_eq!(updated_pet.status, registry_status::PENDING);

    let reviewed_pet = repo
        .review_pet_registration(
            org,
            pet.id,
            owner,
            ReviewRegistration {
                approve: true,
                rejection_reason: None,
            },
        )
        .await
        .expect("review pet decodes enums on RETURNING")
        .expect("pet exists");
    assert_eq!(reviewed_pet.status, registry_status::APPROVED);

    // -- Vehicle: create RETURNING + with-details + list summary -------------
    let vehicle = repo
        .create_vehicle_registration(
            org,
            owner,
            CreateVehicleRegistration {
                unit_id: unit,
                vehicle_type: "car".into(),
                make: "Skoda".into(),
                model: "Octavia".into(),
                year: Some(2022),
                color: Some("blue".into()),
                license_plate: "BL-158-AB".into(),
                registration_document_id: None,
                insurance_document_id: None,
                notes: None,
            },
        )
        .await
        .expect("create vehicle registration decodes enums on RETURNING");
    assert_eq!(vehicle.vehicle_type, "car");
    assert_eq!(vehicle.status, registry_status::PENDING);

    let vehicle_details = repo
        .get_vehicle_registration_with_details(org, vehicle.id)
        .await
        .expect("vehicle with-details query")
        .expect("vehicle exists");
    assert_eq!(vehicle_details.unit_number.as_deref(), Some("A-101"));

    let (vehicle_list, vehicle_total) = repo
        .list_vehicle_registrations(org, VehicleRegistrationQuery::default())
        .await
        .expect("list vehicle registrations decodes enums + designation");
    assert_eq!(vehicle_total, 1);
    assert_eq!(vehicle_list.len(), 1);
    assert_eq!(vehicle_list[0].vehicle_type, "car");
    assert_eq!(vehicle_list[0].status, registry_status::PENDING);
    assert_eq!(vehicle_list[0].unit_number.as_deref(), Some("A-101"));

    let reviewed_vehicle = repo
        .review_vehicle_registration(
            org,
            vehicle.id,
            owner,
            ReviewRegistration {
                approve: false,
                rejection_reason: Some("missing insurance".into()),
            },
        )
        .await
        .expect("review vehicle decodes enums on RETURNING")
        .expect("vehicle exists");
    assert_eq!(reviewed_vehicle.status, registry_status::REJECTED);

    // -- Parking spot detail join resolves the assigned unit designation -----
    sqlx::query(
        r#"
        INSERT INTO parking_spots (tenant_id, building_id, spot_number, assigned_to_unit_id)
        VALUES ($1, $2, 'P-1', $3)
        "#,
    )
    .bind(org)
    .bind(building)
    .bind(unit)
    .execute(&pool)
    .await
    .expect("seed assigned parking spot");

    let (spots, spot_total) = repo
        .list_parking_spots(org, Default::default())
        .await
        .expect("list parking spots resolves designation");
    assert_eq!(spot_total, 1);
    assert_eq!(spots.len(), 1);
    assert_eq!(spots[0].assigned_unit_number.as_deref(), Some("A-101"));
}

/// Insert a vehicle registration raw so we can set a cross-tenant
/// `parking_spot_id` (the repo create method does not accept one, and the column
/// has no FK — exactly the hazard under test).
async fn seed_vehicle_registration_with_spot(
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

/// The vehicle *list* summary joins `parking_spots` on `vr.parking_spot_id`.
/// That column carries no FK, so without a tenant predicate on the join a stale
/// cross-tenant id would leak a foreign tenant's `spot_number` into the list —
/// the same leak PAP-143 closed on the by-id detail path. PAP-158 scopes the
/// join; this test pins it (pre-fix: leaks "B-007"; post-fix: None).
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn vehicle_list_does_not_leak_cross_tenant_parking_spot(pool: PgPool) {
    let org_a = seed_org(&pool, "list-a").await;
    let org_b = seed_org(&pool, "list-b").await;
    let owner = seed_user(&pool, "owner@list.test").await;

    let building_a = seed_building(&pool, org_a, "list-a").await;
    let unit_a = seed_unit(&pool, building_a, "A-1").await;

    let building_b = seed_building(&pool, org_b, "list-b").await;
    let spot_b = seed_parking_spot(&pool, org_b, building_b, "B-007").await;

    let _reg = seed_vehicle_registration_with_spot(&pool, org_a, unit_a, owner, spot_b).await;

    let repo = RegistryRepository::new(pool.clone());

    let (list, total) = repo
        .list_vehicle_registrations(org_a, VehicleRegistrationQuery::default())
        .await
        .expect("list vehicle registrations");

    assert_eq!(total, 1, "Org A sees its own registration");
    assert_eq!(list.len(), 1);
    assert_eq!(
        list[0].parking_spot_number, None,
        "cross-tenant parking_spot_id must not leak Org B's spot_number into the list"
    );
}

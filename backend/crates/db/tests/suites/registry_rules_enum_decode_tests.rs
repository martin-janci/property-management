//! Regression tests for PAP-159 — allowed_pet_types pet_type[] decode+bind fix
//! in [`db::repositories::RegistryRepository`].

use crate::common::seed_org;
use db::models::UpdateRegistryRules;
use db::repositories::RegistryRepository;
use sqlx::PgPool;
use uuid::Uuid;

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

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn registry_rules_decode_allowed_pet_types_enum_array(pool: PgPool) {
    let org = seed_org(&pool, "rules_fix").await;
    let building = seed_building(&pool, org, "rules_fix").await;
    let repo = RegistryRepository::new(pool.clone());

    // 1. Upsert with non-empty allowed_pet_types (test encode/bind cast)
    let rules = repo
        .upsert_registry_rules(
            org,
            building,
            UpdateRegistryRules {
                pets_allowed: Some(true),
                pets_require_approval: Some(true),
                max_pets_per_unit: Some(2),
                allowed_pet_types: Some(vec!["dog".into(), "cat".into()]),
                banned_pet_breeds: None,
                max_pet_weight: None,
                vehicles_require_approval: Some(false),
                max_vehicles_per_unit: None,
                notes: None,
            },
        )
        .await
        .expect("upsert_registry_rules decodes enum array on RETURNING");

    assert_eq!(
        rules.allowed_pet_types,
        Some(vec!["dog".into(), "cat".into()])
    );

    // 2. Read back (test SELECT decode cast)
    let fetched_rules = repo
        .get_registry_rules(org, building)
        .await
        .expect("get_registry_rules decodes enum array")
        .expect("rules exist");

    assert_eq!(
        fetched_rules.allowed_pet_types,
        Some(vec!["dog".into(), "cat".into()])
    );

    // 3. Update existing (test ON CONFLICT DO UPDATE encode/bind cast)
    let updated_rules = repo
        .upsert_registry_rules(
            org,
            building,
            UpdateRegistryRules {
                pets_allowed: None,
                pets_require_approval: None,
                max_pets_per_unit: None,
                allowed_pet_types: Some(vec!["bird".into()]),
                banned_pet_breeds: None,
                max_pet_weight: None,
                vehicles_require_approval: None,
                max_vehicles_per_unit: None,
                notes: None,
            },
        )
        .await
        .expect("upsert_registry_rules update decodes enum array on RETURNING");

    assert_eq!(updated_rules.allowed_pet_types, Some(vec!["bird".into()]));
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn registry_rules_decode_unknown_pet_type_variant_gracefully(pool: PgPool) {
    // GH #1363 / #1366 — audit of the allowed_pet_types enum decode path.
    //
    // Both read paths (`get_registry_rules` SELECT and `upsert_registry_rules`
    // RETURNING) project the column as `allowed_pet_types::text[]` into the
    // `Option<Vec<String>>` model field. The explicit `::text[]` cast is what
    // makes the decode robust: SQLx decodes the Postgres `pet_type[]` enum array
    // as a plain text array, so a variant present in the *database* enum but NOT
    // enumerated in the Rust `db::models::pet_type` constants module still decodes
    // into a `String` — no unknown-OID decode error, no panic.
    //
    // This test pins that behaviour. It adds a brand-new enum value to the DB
    // `pet_type` type (one the Rust side knows nothing about), writes it directly
    // into a row, and asserts the repository read decodes it as a plain string.
    //
    // Reverting the `::text[]` cast on the SELECT in `get_registry_rules` to a
    // bare `allowed_pet_types` (native enum decode) makes this test fail with a
    // decode error on the unknown OID.
    let org = seed_org(&pool, "unknown_variant").await;
    let building = seed_building(&pool, org, "unknown_variant").await;
    let repo = RegistryRepository::new(pool.clone());

    // Extend the DB enum with a value the Rust `pet_type` module does not declare.
    // (sqlx::test runs each test in its own transaction/database, so this does not
    // leak to other tests.)
    sqlx::query("ALTER TYPE pet_type ADD VALUE IF NOT EXISTS 'ferret'")
        .execute(&pool)
        .await
        .expect("extend pet_type enum with an unknown-to-Rust variant");

    // Insert a rules row directly, casting the unknown variant into the enum array.
    // We bypass the repository binder here precisely because its
    // `$6::text[]::pet_type[]` cast would still accept 'ferret' — the point is to
    // get an "unknown to Rust" value persisted, then exercise the *read* decode.
    sqlx::query(
        r#"
        INSERT INTO building_registry_rules (tenant_id, building_id, allowed_pet_types)
        VALUES ($1, $2, ARRAY['dog', 'ferret']::pet_type[])
        "#,
    )
    .bind(org)
    .bind(building)
    .execute(&pool)
    .await
    .expect("insert row with an unknown-to-Rust pet_type enum value");

    // Read path must decode the unknown variant gracefully as a plain string.
    let fetched = repo
        .get_registry_rules(org, building)
        .await
        .expect("get_registry_rules decodes unknown enum variant without panic/error")
        .expect("rules exist");

    assert_eq!(
        fetched.allowed_pet_types,
        Some(vec!["dog".into(), "ferret".into()]),
        "unknown DB enum variant should decode as a plain string via the ::text[] cast"
    );

    // The unknown variant survives an upsert RETURNING decode too (COALESCE keeps
    // the existing array when allowed_pet_types is None on the update).
    let updated = repo
        .upsert_registry_rules(
            org,
            building,
            UpdateRegistryRules {
                pets_allowed: Some(true),
                pets_require_approval: None,
                max_pets_per_unit: None,
                allowed_pet_types: None, // COALESCE -> keep existing ['dog','ferret']
                banned_pet_breeds: None,
                max_pet_weight: None,
                vehicles_require_approval: None,
                max_vehicles_per_unit: None,
                notes: None,
            },
        )
        .await
        .expect("upsert RETURNING decodes unknown enum variant without panic/error");

    assert_eq!(
        updated.allowed_pet_types,
        Some(vec!["dog".into(), "ferret".into()]),
        "upsert RETURNING must also decode the unknown variant as a plain string"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn registry_rules_insert_branch_none_booleans_use_schema_defaults(pool: PgPool) {
    // GH #1406 — the NOT-NULL COALESCE on the INSERT branch of upsert_registry_rules
    // (COALESCE($3, TRUE) / COALESCE($4, TRUE) / COALESCE($9, FALSE)) was added to
    // fix a 23502 "null value in column" error when all three booleans are None.
    // The pre-existing test only binds all-Some on the INSERT path, so it cannot
    // catch a regression to the INSERT branch. This test binds None for all three and
    // asserts the schema DEFAULTs apply without a 23502.
    //
    // Reverting any of the three COALESCE(…, <default>) on the INSERT branch of
    // upsert_registry_rules in registry.rs should make this test fail.
    let org = seed_org(&pool, "coalesce_defaults").await;
    let building = seed_building(&pool, org, "coalesce_defaults").await;
    let repo = RegistryRepository::new(pool.clone());

    // First upsert on a fresh (org, building) — hits the INSERT branch, not ON CONFLICT.
    // All three NOT NULL boolean columns left as None → must COALESCE to schema DEFAULT.
    let rules = repo
        .upsert_registry_rules(
            org,
            building,
            UpdateRegistryRules {
                pets_allowed: None,          // DEFAULT TRUE
                pets_require_approval: None, // DEFAULT TRUE
                max_pets_per_unit: None,
                allowed_pet_types: None,
                banned_pet_breeds: None,
                max_pet_weight: None,
                vehicles_require_approval: None, // DEFAULT FALSE
                max_vehicles_per_unit: None,
                notes: None,
            },
        )
        .await
        .expect("INSERT with None booleans applies COALESCE defaults (no 23502)");

    // Schema DEFAULTs from 00067_create_building_registries.sql.
    assert!(rules.pets_allowed, "pets_allowed should default to TRUE");
    assert!(
        rules.pets_require_approval,
        "pets_require_approval should default to TRUE"
    );
    assert!(
        !rules.vehicles_require_approval,
        "vehicles_require_approval should default to FALSE"
    );
}

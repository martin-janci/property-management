//! Regression tests for PAP-159 — allowed_pet_types pet_type[] decode+bind fix
//! in [`db::repositories::RegistryRepository`].

use db::models::UpdateRegistryRules;
use db::repositories::RegistryRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Rules {slug}"))
    .bind(slug)
    .bind(format!("{slug}@rules.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
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

#[sqlx::test]
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
                pets_allowed: None,            // DEFAULT TRUE
                pets_require_approval: None,   // DEFAULT TRUE
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

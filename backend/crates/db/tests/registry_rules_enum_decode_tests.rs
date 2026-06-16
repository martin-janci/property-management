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

/// #1406: the upsert INSERT branch must COALESCE NULL booleans to the column
/// schema defaults. `pets_allowed` / `pets_require_approval` /
/// `vehicles_require_approval` are `NOT NULL` (migration 00067) but the request
/// type carries `Option<bool>`; PR #1379 wrapped the INSERT binds in
/// `COALESCE($n, <default>)`. Without it, a first upsert (INSERT, no existing
/// row) with `None` booleans binds SQL NULL and fails with `23502 not-null
/// violation`. This test creates rules with all three booleans `None` and
/// asserts the row is created with the schema defaults (TRUE / TRUE / FALSE).
#[sqlx::test]
async fn registry_rules_insert_branch_coalesces_null_booleans_to_defaults(pool: PgPool) {
    let org = seed_org(&pool, "rules_defaults").await;
    let building = seed_building(&pool, org, "rules_defaults").await;
    let repo = RegistryRepository::new(pool.clone());

    // First upsert for this (org, building) → INSERT branch, all booleans None.
    let rules = repo
        .upsert_registry_rules(
            org,
            building,
            UpdateRegistryRules {
                pets_allowed: None,
                pets_require_approval: None,
                max_pets_per_unit: None,
                allowed_pet_types: None,
                banned_pet_breeds: None,
                max_pet_weight: None,
                vehicles_require_approval: None,
                max_vehicles_per_unit: None,
                notes: None,
            },
        )
        .await
        .expect("INSERT-branch upsert must COALESCE NULL booleans to defaults (no 23502)");

    // Schema defaults from 00067_create_building_registries.sql.
    assert!(rules.pets_allowed, "pets_allowed default is TRUE");
    assert!(
        rules.pets_require_approval,
        "pets_require_approval default is TRUE"
    );
    assert!(
        !rules.vehicles_require_approval,
        "vehicles_require_approval default is FALSE"
    );
}

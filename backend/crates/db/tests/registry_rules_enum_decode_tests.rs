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

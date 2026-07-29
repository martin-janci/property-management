//! Regression tests for the SQL-injection fix on Epic 134 predictive
//! maintenance equipment listing (#790, round 12).
//!
//! Audit history: `PredictiveMaintenanceRepository::list_equipment` built its
//! `ORDER BY` clause with `format!("name {}", sort_order)`, interpolating the
//! caller-supplied `sort_order` query value straight into the SQL string
//! (wrapped in `sqlx::AssertSqlSafe`). A request such as
//! `?sort_order=ASC; DROP TABLE equipment_registry; --` or
//! `?sort_order=(SELECT ...)` would be spliced into the executed statement.
//!
//! The fix ALLOWLISTS both the sort column and the direction: `sort_by` maps
//! through a `match` to one of three fixed column literals, and `sort_order`
//! maps to the literal `ASC` or `DESC` (defaulting to `ASC`). Nothing from the
//! raw request value can reach the SQL text any more, so a malicious value is
//! silently treated as the default and the query returns normal results
//! without error.

use crate::common::seed_org;
use db::models::predictive_maintenance::{CreateEquipment, EquipmentQuery};
use db::repositories::PredictiveMaintenanceRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'PM Sort User', 'active', NOW())
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
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_equipment(
    repo: &PredictiveMaintenanceRepository,
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
    building_id: Uuid,
    name: &str,
) -> Uuid {
    let eq = repo
        .create_equipment(
            pool,
            org_id,
            user_id,
            CreateEquipment {
                building_id,
                name: name.to_string(),
                equipment_type: "hvac".to_string(),
                category: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                location_description: None,
                floor_number: None,
                unit_id: None,
                installation_date: None,
                warranty_expiry_date: None,
                expected_lifespan_years: None,
                replacement_cost: None,
                notes: None,
                specifications: None,
            },
        )
        .await
        .expect("create equipment");
    eq.id
}

/// A malicious `sort_order` (classic stacked-statement injection) must NOT
/// alter the query: the call returns the seeded rows normally and the table
/// still exists afterwards.
#[sqlx::test]
async fn malicious_sort_order_does_not_inject(pool: PgPool) {
    let repo = PredictiveMaintenanceRepository::new(pool.clone());

    let org = seed_org(&pool, "inj-order").await;
    let user = seed_user(&pool, "inj-order@pm-sort.test").await;
    let building = seed_building(&pool, org, "inj-order").await;
    seed_equipment(&repo, &pool, org, user, building, "Boiler").await;
    seed_equipment(&repo, &pool, org, user, building, "Chiller").await;

    let query = EquipmentQuery {
        sort_by: Some("name".to_string()),
        // Attempt a stacked DROP via the order direction.
        sort_order: Some("ASC; DROP TABLE equipment_registry; --".to_string()),
        ..Default::default()
    };

    let result = repo.list_equipment(&pool, org, query).await;

    // The injection must be neutralised — the query succeeds and returns the
    // two seeded rows, sorted ascending by name (the allowlisted default).
    let equipment = result.expect("list_equipment must succeed, not error/inject");
    assert_eq!(equipment.len(), 2, "both seeded rows must be returned");
    assert_eq!(equipment[0].name, "Boiler");
    assert_eq!(equipment[1].name, "Chiller");

    // The table must still exist — proves no DROP executed.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM equipment_registry")
        .fetch_one(&pool)
        .await
        .expect("equipment_registry table must still exist");
    assert_eq!(count, 2, "rows must be intact after injection attempt");
}

/// A malicious `sort_by` (subquery injection) must fall back to the default
/// column and return normal results without error.
#[sqlx::test]
async fn malicious_sort_by_does_not_inject(pool: PgPool) {
    let repo = PredictiveMaintenanceRepository::new(pool.clone());

    let org = seed_org(&pool, "inj-by").await;
    let user = seed_user(&pool, "inj-by@pm-sort.test").await;
    let building = seed_building(&pool, org, "inj-by").await;
    seed_equipment(&repo, &pool, org, user, building, "Alpha").await;

    let query = EquipmentQuery {
        // Subquery / column injection attempt.
        sort_by: Some("(SELECT password_hash FROM users LIMIT 1)".to_string()),
        sort_order: Some("DESC".to_string()),
        ..Default::default()
    };

    let equipment = repo
        .list_equipment(&pool, org, query)
        .await
        .expect("list_equipment must succeed despite malicious sort_by");
    assert_eq!(equipment.len(), 1);
    assert_eq!(equipment[0].name, "Alpha");
}

/// Sanity: the legitimate `desc` ordering is still honoured after the fix.
#[sqlx::test]
async fn legitimate_desc_ordering_still_works(pool: PgPool) {
    let repo = PredictiveMaintenanceRepository::new(pool.clone());

    let org = seed_org(&pool, "ok-desc").await;
    let user = seed_user(&pool, "ok-desc@pm-sort.test").await;
    let building = seed_building(&pool, org, "ok-desc").await;
    seed_equipment(&repo, &pool, org, user, building, "Aaa").await;
    seed_equipment(&repo, &pool, org, user, building, "Zzz").await;

    let query = EquipmentQuery {
        sort_by: Some("name".to_string()),
        sort_order: Some("desc".to_string()),
        ..Default::default()
    };

    let equipment = repo.list_equipment(&pool, org, query).await.expect("list");
    assert_eq!(equipment.len(), 2);
    assert_eq!(equipment[0].name, "Zzz", "desc must put Zzz first");
    assert_eq!(equipment[1].name, "Aaa");
}

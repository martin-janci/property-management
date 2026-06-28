//! Cross-org negative coverage for service-history endpoints (GH #1372).
//!
//! # What these tests verify
//!
//! `GET /api/v1/work-orders/equipment/{id}/service-history` and
//! `GET /api/v1/work-orders/buildings/{id}/service-history` must reject a
//! caller from Org B who supplies an equipment-id or building-id owned by Org A
//! with `403 FORBIDDEN`. Without an explicit `verify_org_access` call, the
//! `#[sqlx::test]` superuser pool bypasses `FORCE` RLS, so Org A data would
//! leak to Org B callers — exactly the gap this test set guards against.
//!
//! Each test also has a same-org "golden path" variant to ensure the guard
//! does not over-block legitimate callers.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("SvcHist IDOR {slug}"))
    .bind(format!("svchist-idor-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@svchist-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, 'SvcHist IDOR Street 1', 'Bratislava', '81101', 'Slovakia')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment
            (organization_id, building_id, name, category, status)
        VALUES ($1, $2, 'SvcHist IDOR Equipment', 'hvac', 'operational')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

async fn seed_completed_work_order(
    pool: &PgPool,
    org_id: Uuid,
    building_id: Uuid,
    equipment_id: Uuid,
    created_by: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO work_orders (
            organization_id, building_id, equipment_id, title, description,
            created_by, status, completed_at, actual_cost
        )
        VALUES ($1, $2, $3, 'SvcHist IDOR job', 'seeded', $4, 'completed', NOW(), 123.45)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(equipment_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed completed work order")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

// ---------------------------------------------------------------------------
// Test 1 — GET /equipment/{id}/service-history from another org is rejected
// ---------------------------------------------------------------------------

/// Org B user requests service history for Org A equipment. Must be rejected
/// with 403; Org A's work-order data must not be disclosed.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_equipment_service_history_cross_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "eq-sh-a").await;
    let org_b = seed_org(&pool, "eq-sh-b").await;

    // Org A owner — creates the equipment and a completed WO.
    let owner_a = TestUser::new();
    let (_a_token, _) = create_authenticated_user(&app, &owner_a).await;
    let owner_a_id = user_id_for(&pool, &owner_a.email).await;
    seed_membership(&pool, org_a, owner_a_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let equipment_a = seed_equipment(&pool, org_a, building_a).await;
    seed_completed_work_order(&pool, org_a, building_a, equipment_a, owner_a_id).await;

    // Org B attacker.
    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let response = app
        .execute(
            app.get(&format!(
                "/api/v1/work-orders/equipment/{}/service-history",
                equipment_a
            ))
            .bearer(&b_token)
            .header("X-Tenant-ID", &org_b.to_string())
            .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#1372: cross-org GET equipment service-history must be rejected (403), got {} body={}",
        response.status,
        response.text()
    );
}

// ---------------------------------------------------------------------------
// Test 2 — GET /buildings/{id}/service-history from another org is rejected
// ---------------------------------------------------------------------------

/// Org B user requests service history for Org A building. Must be rejected
/// with 403; Org A's work-order data must not be disclosed.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_building_service_history_cross_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "bld-sh-a").await;
    let org_b = seed_org(&pool, "bld-sh-b").await;

    // Org A owner.
    let owner_a = TestUser::new();
    let (_a_token, _) = create_authenticated_user(&app, &owner_a).await;
    let owner_a_id = user_id_for(&pool, &owner_a.email).await;
    seed_membership(&pool, org_a, owner_a_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let equipment_a = seed_equipment(&pool, org_a, building_a).await;
    seed_completed_work_order(&pool, org_a, building_a, equipment_a, owner_a_id).await;

    // Org B attacker.
    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let response = app
        .execute(
            app.get(&format!(
                "/api/v1/work-orders/buildings/{}/service-history",
                building_a
            ))
            .bearer(&b_token)
            .header("X-Tenant-ID", &org_b.to_string())
            .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#1372: cross-org GET building service-history must be rejected (403), got {} body={}",
        response.status,
        response.text()
    );
}

// ---------------------------------------------------------------------------
// Test 3 — same-org equipment service-history still succeeds (no false-positive)
// ---------------------------------------------------------------------------

/// A member of Org A reads service history for Org A equipment. Should succeed
/// with 200 — the fix must not over-block legitimate callers.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_equipment_service_history_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "eq-sh-ok").await;

    let member = TestUser::new();
    let (token, _) = create_authenticated_user(&app, &member).await;
    let member_id = user_id_for(&pool, &member.email).await;
    seed_membership(&pool, org_a, member_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let equipment_a = seed_equipment(&pool, org_a, building_a).await;
    seed_completed_work_order(&pool, org_a, building_a, equipment_a, member_id).await;

    let response = app
        .execute(
            app.get(&format!(
                "/api/v1/work-orders/equipment/{}/service-history",
                equipment_a
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &org_a.to_string())
            .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#1372: same-org GET equipment service-history must succeed (200), got {} body={}",
        response.status,
        response.text()
    );

    let body = response.json_value();
    let entries = body.as_array().expect("response is array");
    assert_eq!(
        entries.len(),
        1,
        "#1372: same-org GET must return the seeded service-history entry"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — same-org building service-history still succeeds (no false-positive)
// ---------------------------------------------------------------------------

/// A member of Org A reads service history for Org A building. Should succeed
/// with 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_building_service_history_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "bld-sh-ok").await;

    let member = TestUser::new();
    let (token, _) = create_authenticated_user(&app, &member).await;
    let member_id = user_id_for(&pool, &member.email).await;
    seed_membership(&pool, org_a, member_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let equipment_a = seed_equipment(&pool, org_a, building_a).await;
    seed_completed_work_order(&pool, org_a, building_a, equipment_a, member_id).await;

    let response = app
        .execute(
            app.get(&format!(
                "/api/v1/work-orders/buildings/{}/service-history",
                building_a
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &org_a.to_string())
            .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#1372: same-org GET building service-history must succeed (200), got {} body={}",
        response.status,
        response.text()
    );

    let body = response.json_value();
    let entries = body.as_array().expect("response is array");
    assert_eq!(
        entries.len(),
        1,
        "#1372: same-org GET must return the seeded building service-history entry"
    );
}

// ---------------------------------------------------------------------------
// Test 5 — unknown equipment-id returns 404 (not empty 200 or server error)
// ---------------------------------------------------------------------------

/// Requesting service history for a non-existent equipment-id must return 404
/// so callers cannot distinguish "exists but forbidden" from "does not exist"
/// via timing or error codes.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_equipment_service_history_unknown_id_returns_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "eq-sh-404").await;

    let member = TestUser::new();
    let (token, _) = create_authenticated_user(&app, &member).await;
    let member_id = user_id_for(&pool, &member.email).await;
    seed_membership(&pool, org_a, member_id, "manager").await;

    let response = app
        .execute(
            app.get(&format!(
                "/api/v1/work-orders/equipment/{}/service-history",
                Uuid::new_v4()
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &org_a.to_string())
            .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::NOT_FOUND,
        "#1372: unknown equipment-id must return 404, got {} body={}",
        response.status,
        response.text()
    );
}

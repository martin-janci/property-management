//! Real-JWT regression tests for work-order / maintenance-schedule org-scope
//! (closes #821).
//!
//! # Background
//!
//! The Epic 20 work-order routes (`/api/v1/work-orders/*`) authenticated the
//! caller with `AuthUser` but never checked that the caller belonged to the
//! organization that owns the resource:
//!
//!   - by-id reads/mutations (`get`, `update`, `delete`, `assign`, `start`,
//!     `complete`, `hold`, comments, schedule by-id) called
//!     `work_order_repo.find_by_id(id)` (and friends) with no org predicate, so
//!     org A could read or mutate org B's work order by guessing its UUID;
//!   - org-supplied create/list endpoints trusted the client-supplied
//!     `organization_id` (request body / query string) without verifying the
//!     caller's membership, so org B could create or enumerate work orders in
//!     org A.
//!
//! # Fix
//!
//! Every handler now authorizes the caller against the resource's owning org
//! via `verify_org_access(&state, user_id, org_id)` (membership check against
//! `organization_members`). For by-id handlers the org is read from the
//! fetched row (`load_work_order_for_user` / `load_schedule_for_user`); a
//! missing row yields `404` and a foreign-org row yields `403`. For
//! org-supplied handlers the client `organization_id` is checked before any
//! repo call.
//!
//! # What these tests verify
//!
//! Using a *real* JWT minted for a user who is a member of Org B only:
//!
//!   - a cross-tenant by-id read/mutate of an Org A work order is rejected and
//!     the Org A row is left untouched;
//!   - a create that names Org A as the owner is rejected and no row is
//!     inserted;
//!   - the legitimate same-org path still succeeds (no false-positive 403).
//!
//! # TestApp wiring note
//!
//! `TestApp` mounts the full router but no `host_tenant_middleware`. Since
//! PAP-179 the work-order routes acquire an `RlsConnection`, which resolves the
//! caller's org via `ValidatedTenantExtractor` — that requires an `X-Tenant-ID`
//! header naming an org the caller is an active member of. Every request below
//! therefore sets `X-Tenant-ID` to the caller's own org (Org B for the
//! attacker, Org A for the legitimate member). The cross-tenant rejections then
//! come from `verify_org_access` against the *resource's* org (the membership
//! gate is bypassed at the DB layer here because the `#[sqlx::test]` pool
//! connects as a superuser, so `FORCE` RLS does not bind).

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
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
    .bind(format!("WO IDOR {slug}"))
    .bind(format!("wo-idor-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@wo-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, 'WO IDOR Street 1', 'Bratislava', '81101', 'Slovakia')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a `work_orders` row owned by `org_id`. `created_by` must be a real
/// user row (FK), so we pass one in.
async fn seed_work_order(pool: &PgPool, org_id: Uuid, building_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO work_orders
            (organization_id, building_id, title, description, created_by)
        VALUES ($1, $2, 'Original title', 'Original description', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed work order")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

async fn work_order_title(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar::<_, String>("SELECT title FROM work_orders WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("fetch work order title")
}

// ---------------------------------------------------------------------------
// Test 1 — GET /work-orders/{id} from another org is rejected (info disclosure)
// ---------------------------------------------------------------------------

/// An authenticated member of Org B fetches an Org A work order by id. The
/// handler reads the row, sees it belongs to Org A, and rejects the caller
/// (403) because they are not a member of Org A.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_work_order_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "get-a").await;
    let org_b = seed_org(&pool, "get-b").await;

    // Org A owner (created_by) — a real user row for the FK.
    let owner_a = TestUser::new();
    let (_a_token, _) = create_authenticated_user(&app, &owner_a).await;
    let owner_a_id = user_id_for(&pool, &owner_a.email).await;
    seed_membership(&pool, org_a, owner_a_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let wo_in_a = seed_work_order(&pool, org_a, building_a, owner_a_id).await;

    // Attacker — a real authenticated member of Org B (NOT Org A).
    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let response = app
        .execute(
            app.get(&format!("/api/v1/work-orders/{}", wo_in_a))
                .bearer(&b_token)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#821: cross-tenant GET work order must be rejected (403), got {} body={}",
        response.status,
        response.text()
    );
}

// ---------------------------------------------------------------------------
// Test 2 — PATCH /work-orders/{id} from another org is rejected + no mutation
// ---------------------------------------------------------------------------

/// A member of Org B attempts to update an Org A work order. The request is
/// rejected (403) and the Org A row's title is unchanged.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_work_order_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "upd-a").await;
    let org_b = seed_org(&pool, "upd-b").await;

    let owner_a = TestUser::new();
    let (_a_token, _) = create_authenticated_user(&app, &owner_a).await;
    let owner_a_id = user_id_for(&pool, &owner_a.email).await;
    seed_membership(&pool, org_a, owner_a_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let wo_in_a = seed_work_order(&pool, org_a, building_a, owner_a_id).await;

    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    // The work-orders router registers PATCH (not PUT) for `/{id}`, and the
    // common `RequestBuilder` has no PATCH helper, so build the request by hand.
    let patch_req = axum::http::Request::builder()
        .method(axum::http::Method::PATCH)
        .uri(format!("/api/v1/work-orders/{}", wo_in_a))
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", b_token),
        )
        .header("X-Tenant-ID", org_b.to_string())
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(
            json!({ "title": "hijacked-title" }).to_string(),
        ))
        .expect("build PATCH request");

    let response = app.execute(patch_req).await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#821: cross-tenant PATCH work order must be rejected (403), got {} body={}",
        response.status,
        response.text()
    );

    assert_eq!(
        work_order_title(&pool, wo_in_a).await,
        "Original title",
        "#821: Org A work order title must not be changed by cross-tenant PATCH"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — DELETE /work-orders/{id} from another org is rejected + row survives
// ---------------------------------------------------------------------------

/// A member of Org B attempts to delete an Org A work order. Rejected (403)
/// and the row still exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_work_order_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "del-a").await;
    let org_b = seed_org(&pool, "del-b").await;

    let owner_a = TestUser::new();
    let (_a_token, _) = create_authenticated_user(&app, &owner_a).await;
    let owner_a_id = user_id_for(&pool, &owner_a.email).await;
    seed_membership(&pool, org_a, owner_a_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let wo_in_a = seed_work_order(&pool, org_a, building_a, owner_a_id).await;

    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let response = app
        .execute(
            app.delete(&format!("/api/v1/work-orders/{}", wo_in_a))
                .bearer(&b_token)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#821: cross-tenant DELETE work order must be rejected (403), got {} body={}",
        response.status,
        response.text()
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM work_orders WHERE id = $1")
        .bind(wo_in_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 1,
        "#821: work order must survive cross-tenant DELETE"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — POST /work-orders naming a foreign org is rejected + nothing created
// ---------------------------------------------------------------------------

/// A member of Org B posts a create request whose body names Org A as the
/// owner. `verify_org_access` rejects it (403) and no work order is inserted
/// into Org A.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_work_order_for_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "crt-a").await;
    let org_b = seed_org(&pool, "crt-b").await;

    let owner_a = TestUser::new();
    let (_a_token, _) = create_authenticated_user(&app, &owner_a).await;
    let owner_a_id = user_id_for(&pool, &owner_a.email).await;
    seed_membership(&pool, org_a, owner_a_id, "manager").await;
    let building_a = seed_building(&pool, org_a).await;

    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    let response = app
        .execute(
            app.post("/api/v1/work-orders")
                .bearer(&b_token)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(json!({
                    "organization_id": org_a,
                    "building_id": building_a,
                    "title": "cross-tenant inject",
                    "description": "should be rejected",
                }))
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#821: cross-tenant POST work order must be rejected (403), got {} body={}",
        response.status,
        response.text()
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM work_orders WHERE organization_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 0,
        "#821: no work order must be created in Org A by a cross-tenant POST"
    );
}

// ---------------------------------------------------------------------------
// Test 5 — same-org access still succeeds (no false-positive 403)
// ---------------------------------------------------------------------------

/// A member of Org A reads an Org A work order by id. The membership check
/// passes and the handler returns 200 with the row — proving the fix does not
/// over-block legitimate same-org access.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_work_order_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "ok-a").await;

    let member = TestUser::new();
    let (token, _) = create_authenticated_user(&app, &member).await;
    let member_id = user_id_for(&pool, &member.email).await;
    seed_membership(&pool, org_a, member_id, "manager").await;

    let building_a = seed_building(&pool, org_a).await;
    let wo_in_a = seed_work_order(&pool, org_a, building_a, member_id).await;

    let response = app
        .execute(
            app.get(&format!("/api/v1/work-orders/{}", wo_in_a))
                .bearer(&token)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        response.status,
        StatusCode::OK,
        "#821: same-org GET work order must succeed (200), got {} body={}",
        response.status,
        response.text()
    );

    let body = response.json_value();
    assert_eq!(
        body.get("id").and_then(|v| v.as_str()),
        Some(wo_in_a.to_string().as_str()),
        "#821: same-org GET must return the requested work order"
    );
}

// ---------------------------------------------------------------------------
// Fixtures (service-history)
// ---------------------------------------------------------------------------

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment (organization_id, building_id, name, category)
        VALUES ($1, $2, 'Test HVAC Unit', 'hvac')
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
        INSERT INTO work_orders
            (organization_id, building_id, equipment_id, title, description, created_by, status, completed_at)
        VALUES ($1, $2, $3, 'Service visit', 'Completed maintenance', $4, 'completed', NOW())
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

// ---------------------------------------------------------------------------
// GH #1372 — service-history cross-org negative coverage
// ---------------------------------------------------------------------------

/// An attacker in Org B requests service history for equipment owned by Org A.
/// BIT-56 moved the org-gate lookup onto the caller's `RlsConnection`. In
/// PRODUCTION (app role) the tenant-isolation policy hides the foreign-org row,
/// so the id resolves to empty and the handler returns 404 (no cross-tenant
/// existence oracle). Under THIS test's `#[sqlx::test]` superuser pool, `FORCE`
/// RLS does not bind (see module note), so the row is still visible and the
/// retained `verify_org_access` defense-in-depth gate rejects with 403. The
/// test therefore asserts 403 — the floor guaranteed even if RLS were disabled.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn equipment_service_history_cross_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "svc-a").await;
    let org_b = seed_org(&pool, "svc-b").await;

    // Attacker is a member of Org B only.
    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "member").await;

    // Legitimate owner in Org A creates equipment.
    let owner = TestUser::new();
    let (_, _) = create_authenticated_user(&app, &owner).await;
    let owner_id = user_id_for(&pool, &owner.email).await;
    seed_membership(&pool, org_a, owner_id, "manager").await;
    let building_a = seed_building(&pool, org_a).await;
    let equipment_a = seed_equipment(&pool, org_a, building_a).await;

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
        "BIT-56: cross-org equipment service history rejected by verify_org_access (403) under the superuser test pool; prod returns 404 via RLS-scoped lookup. got {} body={}",
        response.status,
        response.text()
    );
}

/// An attacker in Org B requests service history for a building owned by Org A.
/// Same pattern as equipment (BIT-56): prod returns 404 via the RLS-scoped
/// lookup; this superuser test pool bypasses RLS, so `verify_org_access` is the
/// gate that fires and the assertion is 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_service_history_cross_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "bsvc-a").await;
    let org_b = seed_org(&pool, "bsvc-b").await;

    let attacker = TestUser::new();
    let (b_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "member").await;

    let owner = TestUser::new();
    let (_, _) = create_authenticated_user(&app, &owner).await;
    let owner_id = user_id_for(&pool, &owner.email).await;
    seed_membership(&pool, org_a, owner_id, "manager").await;
    let building_a = seed_building(&pool, org_a).await;

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
        "BIT-56: cross-org building service history rejected by verify_org_access (403) under the superuser test pool; prod returns 404 via RLS-scoped lookup. got {} body={}",
        response.status,
        response.text()
    );
}

/// A member of Org A reading Org A equipment service history succeeds — proves
/// the fix does not over-block legitimate same-org access.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn equipment_service_history_same_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "svc-same").await;

    let member = TestUser::new();
    let (token, _) = create_authenticated_user(&app, &member).await;
    let member_id = user_id_for(&pool, &member.email).await;
    seed_membership(&pool, org_a, member_id, "member").await;

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
        "#1372: same-org equipment service history must succeed (200), got {} body={}",
        response.status,
        response.text()
    );
}

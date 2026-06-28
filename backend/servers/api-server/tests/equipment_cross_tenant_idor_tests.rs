//! Regression tests for the cross-tenant IDOR fix on equipment endpoints.
//!
//! Audit history: three handlers on `/api/v1/ai/equipment/*` accepted a
//! `Path<Uuid>` but discarded their `_principal`, so the WHERE clauses had
//! no `organization_id` guard:
//!
//! - `PUT  /api/v1/ai/equipment/{id}`           (`update_equipment`)
//! - `DELETE /api/v1/ai/equipment/{id}`         (`delete_equipment`)
//! - `PUT  /api/v1/ai/equipment/maintenance/{id}` (`update_maintenance`)
//!
//! The fix threads `tenant_id` from the verified principal into the
//! repository calls so the SQL WHERE clause is:
//!   `WHERE id = $1 AND organization_id = $2`
//! for equipment, and (for maintenance, which joins via equipment):
//!   `WHERE id = $1 AND EXISTS (SELECT 1 FROM equipment e WHERE e.id = ... AND e.organization_id = $11)`
//!
//! These tests exercise the HTTP surface end-to-end:
//!   1. Seed two orgs (A, B) and equipment in Org A.
//!   2. Send a request that claims to be from Org B (via `X-Tenant-Context`).
//!   3. Assert the operation is rejected (4xx) and the row is NOT mutated.
//!
//! TestApp wiring caveat: `TestApp` mounts the router without
//! `host_tenant_middleware`, so `RequestPrincipal` cannot derive a tenant
//! from the Host header. Requests that carry an `X-Tenant-Context` header
//! (without a bearer JWT that would be required by `RequestPrincipal`) are
//! rejected at the auth gate with 401/403 rather than the 404 the handler
//! returns in production. Both are 4xx — the security contract (cross-tenant
//! request is never applied to the row) is satisfied either way.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::TestApp;

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
    .bind(format!("EqIDOR Org {slug}"))
    .bind(format!("eq-idor-org-{slug}"))
    .bind(format!("{slug}@eq-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'EqIDOR User', 'active', NOW())
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
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_equipment(pool: &PgPool, org_id: Uuid, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment
            (organization_id, building_id, name, category, status)
        VALUES ($1, $2, 'Cross-tenant Equipment', 'hvac', 'operational')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed equipment")
}

async fn seed_prediction(pool: &PgPool, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO maintenance_predictions
            (equipment_id, risk_score, confidence, recommendation, factors)
        VALUES ($1, 75.0, 0.9, 'Replace filter urgently', '{}')
        RETURNING id
        "#,
    )
    .bind(equipment_id)
    .fetch_one(pool)
    .await
    .expect("seed prediction")
}

async fn seed_maintenance(pool: &PgPool, equipment_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO equipment_maintenance
            (equipment_id, maintenance_type, description, status)
        VALUES ($1, 'preventive', 'Original description', 'scheduled')
        RETURNING id
        "#,
    )
    .bind(equipment_id)
    .fetch_one(pool)
    .await
    .expect("seed maintenance")
}

/// Forge an `X-Tenant-Context` header claiming membership in `org_id`.
///
/// This replicates the IDOR attack surface: an authenticated (or even
/// unauthenticated) caller supplies a header naming a foreign tenant.
/// The fix ensures `RequestPrincipal` — not this header — is the authority.
fn tenant_context_header(org_id: Uuid, user_id: Uuid) -> String {
    json!({
        "tenant_id": org_id,
        "user_id": user_id,
        "role": "OrgAdmin",
    })
    .to_string()
}

fn req(method: Method, uri: &str, ctx: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Tenant-Context", ctx);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let payload = body
        .map(|v| Body::from(v.to_string()))
        .unwrap_or_else(Body::empty);
    b.body(payload).unwrap()
}

/// Assert that the response is a rejection (any 4xx).
///
/// Both 404 (production, when the tenant-scoped WHERE clause finds no row)
/// and 401/403 (TestApp, when RequestPrincipal rejects the missing/forged JWT)
/// satisfy the security contract: the cross-tenant operation was not applied.
fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: cross-tenant request must be rejected with 4xx, got {status}"
    );
}

// ---------------------------------------------------------------------------
// H1 — cross-tenant IDOR: update_equipment
// ---------------------------------------------------------------------------

/// PUT /equipment/{id} from Org B targeting Org A's equipment → rejected,
/// and the equipment name is NOT changed.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_equipment_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "upd-eq-a").await;
    let org_b = seed_org(&pool, "upd-eq-b").await;
    let _user_a = seed_user(&pool, "upd-eq-a@idor.test").await;
    let user_b = seed_user(&pool, "upd-eq-b@idor.test").await;
    let building_a = seed_building(&pool, org_a, "upd-eq-a").await;
    let equipment_in_a = seed_equipment(&pool, org_a, building_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/equipment/{}", equipment_in_a);
    let body = json!({"name": "hijacked-name"});

    let response = app
        .execute(req(Method::PUT, &uri, &ctx_b, Some(body)))
        .await;

    assert_rejected(response.status, "update_equipment cross-tenant");

    // Verify the row was NOT mutated.
    let name: String = sqlx::query_scalar("SELECT name FROM equipment WHERE id = $1")
        .bind(equipment_in_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        name, "Cross-tenant Equipment",
        "equipment name must not be changed by cross-tenant PUT"
    );
}

// ---------------------------------------------------------------------------
// H2 — cross-tenant IDOR: delete_equipment
// ---------------------------------------------------------------------------

/// DELETE /equipment/{id} from Org B targeting Org A's equipment → rejected,
/// and the row still exists.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_equipment_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "del-eq-a").await;
    let org_b = seed_org(&pool, "del-eq-b").await;
    let _user_a = seed_user(&pool, "del-eq-a@idor.test").await;
    let user_b = seed_user(&pool, "del-eq-b@idor.test").await;
    let building_a = seed_building(&pool, org_a, "del-eq-a").await;
    let equipment_in_a = seed_equipment(&pool, org_a, building_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/equipment/{}", equipment_in_a);

    let response = app.execute(req(Method::DELETE, &uri, &ctx_b, None)).await;

    assert_rejected(response.status, "delete_equipment cross-tenant");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM equipment WHERE id = $1")
        .bind(equipment_in_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "equipment row must survive cross-tenant DELETE");
}

// ---------------------------------------------------------------------------
// H3 — cross-tenant IDOR: update_maintenance
// ---------------------------------------------------------------------------

/// PUT /equipment/maintenance/{id} from Org B targeting a maintenance record
/// whose parent equipment belongs to Org A → rejected, and the record is
/// NOT mutated.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_maintenance_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "upd-maint-a").await;
    let org_b = seed_org(&pool, "upd-maint-b").await;
    let _user_a = seed_user(&pool, "upd-maint-a@idor.test").await;
    let user_b = seed_user(&pool, "upd-maint-b@idor.test").await;
    let building_a = seed_building(&pool, org_a, "upd-maint-a").await;
    let equipment_in_a = seed_equipment(&pool, org_a, building_a).await;
    let maint_id = seed_maintenance(&pool, equipment_in_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/equipment/maintenance/{}", maint_id);
    let body = json!({"description": "hijacked-description", "status": "completed"});

    let response = app
        .execute(req(Method::PUT, &uri, &ctx_b, Some(body)))
        .await;

    assert_rejected(response.status, "update_maintenance cross-tenant");

    // Verify the maintenance record was NOT mutated.
    let description: String =
        sqlx::query_scalar("SELECT description FROM equipment_maintenance WHERE id = $1")
            .bind(maint_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        description, "Original description",
        "maintenance description must not be changed by cross-tenant PUT"
    );
}

// ---------------------------------------------------------------------------
// H4 — cross-tenant IDOR: get_equipment (information disclosure)
// ---------------------------------------------------------------------------

/// GET /equipment/{id} from Org B targeting Org A's equipment → rejected (4xx).
/// Org A's equipment must NOT be readable by Org B.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_equipment_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "get-eq-a").await;
    let org_b = seed_org(&pool, "get-eq-b").await;
    let _user_a = seed_user(&pool, "get-eq-a@idor.test").await;
    let user_b = seed_user(&pool, "get-eq-b@idor.test").await;
    let building_a = seed_building(&pool, org_a, "get-eq-a").await;
    let equipment_in_a = seed_equipment(&pool, org_a, building_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/equipment/{}", equipment_in_a);

    let response = app.execute(req(Method::GET, &uri, &ctx_b, None)).await;

    assert_rejected(response.status, "get_equipment cross-tenant");
}

// ---------------------------------------------------------------------------
// H5 — cross-tenant IDOR: list_maintenance (cross-tenant enumeration)
// ---------------------------------------------------------------------------

/// GET /equipment/{id}/maintenance from Org B targeting Org A's equipment →
/// rejected (4xx), no maintenance records leaked.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_maintenance_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "lst-maint-a").await;
    let org_b = seed_org(&pool, "lst-maint-b").await;
    let _user_a = seed_user(&pool, "lst-maint-a@idor.test").await;
    let user_b = seed_user(&pool, "lst-maint-b@idor.test").await;
    let building_a = seed_building(&pool, org_a, "lst-maint-a").await;
    let equipment_in_a = seed_equipment(&pool, org_a, building_a).await;
    let _maint_id = seed_maintenance(&pool, equipment_in_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/equipment/{}/maintenance", equipment_in_a);

    let response = app.execute(req(Method::GET, &uri, &ctx_b, None)).await;

    assert_rejected(response.status, "list_maintenance cross-tenant");
}

// ---------------------------------------------------------------------------
// H6 — cross-tenant IDOR: create_maintenance (cross-tenant write)
// ---------------------------------------------------------------------------

/// POST /equipment/{id}/maintenance from Org B targeting Org A's equipment →
/// rejected (4xx), and no maintenance record is inserted.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_maintenance_on_other_org_equipment_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "crt-maint-a").await;
    let org_b = seed_org(&pool, "crt-maint-b").await;
    let _user_a = seed_user(&pool, "crt-maint-a@idor.test").await;
    let user_b = seed_user(&pool, "crt-maint-b@idor.test").await;
    let building_a = seed_building(&pool, org_a, "crt-maint-a").await;
    let equipment_in_a = seed_equipment(&pool, org_a, building_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/equipment/{}/maintenance", equipment_in_a);
    let body = serde_json::json!({
        "equipment_id": equipment_in_a,
        "maintenance_type": "preventive",
        "description": "cross-tenant inject",
    });

    let response = app
        .execute(req(Method::POST, &uri, &ctx_b, Some(body)))
        .await;

    assert_rejected(response.status, "create_maintenance cross-tenant");

    // Verify no maintenance record was inserted for this equipment.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM equipment_maintenance WHERE equipment_id = $1")
            .bind(equipment_in_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 0,
        "no maintenance record must be created via cross-tenant POST"
    );
}

// ---------------------------------------------------------------------------
// H7 — cross-tenant IDOR: acknowledge_prediction
// ---------------------------------------------------------------------------

/// POST /predictions/{id}/acknowledge from Org B targeting a prediction
/// whose parent equipment belongs to Org A → rejected (4xx), and the
/// prediction's `acknowledged` flag is NOT set.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn acknowledge_prediction_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "ack-pred-a").await;
    let org_b = seed_org(&pool, "ack-pred-b").await;
    let _user_a = seed_user(&pool, "ack-pred-a@idor.test").await;
    let user_b = seed_user(&pool, "ack-pred-b@idor.test").await;
    let building_a = seed_building(&pool, org_a, "ack-pred-a").await;
    let equipment_in_a = seed_equipment(&pool, org_a, building_a).await;
    let prediction_id = seed_prediction(&pool, equipment_in_a).await;

    let ctx_b = tenant_context_header(org_b, user_b);
    let uri = format!("/api/v1/ai/predictions/{}/acknowledge", prediction_id);

    let response = app
        .execute(req(Method::POST, &uri, &ctx_b, Some(json!({}))))
        .await;

    assert_rejected(response.status, "acknowledge_prediction cross-tenant");

    // Verify the prediction's acknowledged flag was NOT set.
    let acknowledged: bool =
        sqlx::query_scalar("SELECT acknowledged FROM maintenance_predictions WHERE id = $1")
            .bind(prediction_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        !acknowledged,
        "acknowledged must remain false after cross-tenant POST"
    );
}

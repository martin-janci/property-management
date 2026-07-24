//! Faults endpoint happy-path integration tests — Wave 3 (BIT-267).
//!
//! Covers the remaining partial fault endpoints:
//!   get_fault, update_fault, update_status, resolve_fault,
//!   confirm_fault, reopen_fault, list_my_faults,
//!   list_comments, add_comment, add_work_note, get_statistics.

#![allow(dead_code)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Shared seed helpers
// ---------------------------------------------------------------------------

/// Inject the host-resolved [`ResolvedTenant`] extension the faults handlers
/// read via [`RequestPrincipal`]. The bare `TestApp` harness does not run
/// `host_tenant_middleware`, so this mirrors what that middleware would insert
/// after resolving the caller's host. Membership is still enforced downstream
/// against the `user_memberships` row seeded by `seed_membership`.
fn inject_tenant(mut req: Request<Body>, org_id: Uuid) -> Request<Body> {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    req.extensions_mut().insert(ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    });
    req
}

async fn seed_membership(pool: &sqlx::PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, 'OrgAdmin') ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

async fn seed_building(pool: &sqlx::PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'Fault Behaviour Building', 'Fault St 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a fault directly via SQL — mirrors the pattern from `faults_cross_org_idor_tests`.
/// Using raw SQL avoids the HTTP create path, which requires `host_tenant_middleware` to
/// be mounted (TestApp does not mount it). The `reporter_id` must belong to the test user
/// so that `GET /api/v1/faults/my` returns the row.
async fn seed_fault(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    building_id: Uuid,
    reporter_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO faults (
               organization_id, building_id, reporter_id,
               title, description, location_description, category, priority
           )
           VALUES (
               $1, $2, $3,
               'Test Fault', 'Integration test fault', 'Basement',
               'plumbing'::fault_category, 'medium'::fault_priority
           )
           RETURNING id"#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(reporter_id)
    .fetch_one(pool)
    .await
    .expect("seed fault")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_fault_returns_detail(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "getfault").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    let r = app
        .get(&format!("/api/v1/faults/{}", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "get fault: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(
        body["fault"]["id"].as_str(),
        Some(fault_id.to_string().as_str())
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_fault_title(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "updfult").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    let r = app
        .put(&format!("/api/v1/faults/{}", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "title": "Updated Fault Title" }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "update fault: {}", resp.text());
    assert_eq!(
        resp.json_value()["fault"]["title"].as_str(),
        Some("Updated Fault Title")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_fault_status_to_in_progress(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "stsfult").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    let r = app
        .put(&format!("/api/v1/faults/{}/status", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "status": "in_progress", "note": "Work started" }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.json_value()["fault"]["status"].as_str(),
        Some("in_progress")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_resolve_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "rslvflt").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    let r = app
        .post(&format!("/api/v1/faults/{}/resolve", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "resolution_notes": "Pipe was replaced." }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.json_value()["fault"]["status"].as_str(),
        Some("resolved")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_confirm_fault_resolution(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "cnfmflt").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    // Resolve first
    let r = app
        .post(&format!("/api/v1/faults/{}/resolve", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "resolution_notes": "Fixed." }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK);

    // Confirm
    let r = app
        .post(&format!("/api/v1/faults/{}/confirm", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "rating": 5, "feedback": "Great work!" }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "confirm fault: {}",
        resp.text()
    );
    let status = resp.json_value()["fault"]["status"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(
        status == "closed" || status == "confirmed",
        "expected closed/confirmed, got: {}",
        status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_reopen_resolved_fault(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "reopflt").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    // Resolve first
    let r = app
        .post(&format!("/api/v1/faults/{}/resolve", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "resolution_notes": "Fixed temporarily." }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK);

    // Reopen
    let r = app
        .post(&format!("/api/v1/faults/{}/reopen", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "reason": "Problem recurred." }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(resp.status, StatusCode::OK, "reopen fault: {}", resp.text());
    let status = resp.json_value()["fault"]["status"]
        .as_str()
        .unwrap_or("")
        .to_string();
    assert!(
        status == "open" || status == "reported" || status == "in_progress",
        "expected open-like status, got: {}",
        status
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_my_faults(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "myfults").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    seed_fault(&app.pool, org_id, building_id, user_id).await;

    let r = app
        .get("/api/v1/faults/my")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list my faults: {}",
        resp.text()
    );
    let body = resp.json_value();
    assert!(
        body["faults"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "expected at least one fault"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_add_and_list_fault_comments(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "cmtfult").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    // Add comment
    let r = app
        .post(&format!("/api/v1/faults/{}/comments", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "note": "Investigating the issue.", "is_internal": false }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add comment: {}",
        resp.text()
    );

    // List comments / timeline
    let r = app
        .get(&format!("/api/v1/faults/{}/comments", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list comments: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_add_work_note(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "wrkntflt").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let fault_id = seed_fault(&app.pool, org_id, building_id, user_id).await;

    let r = app
        .post(&format!("/api/v1/faults/{}/work-notes", fault_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "note": "Ordered replacement parts." }))
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add work note: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_fault_statistics(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "statflt").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;

    let r = app
        .get("/api/v1/faults/statistics")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(inject_tenant(r, org_id)).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get statistics: {}",
        resp.text()
    );
}

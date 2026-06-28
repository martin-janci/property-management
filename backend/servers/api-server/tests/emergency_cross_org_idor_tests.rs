//! Regression tests for the cross-tenant IDOR fix on the emergency endpoints
//! (`/api/v1/emergency/*`, Epic 23 — issue #827).
//!
//! Audit history: every emergency handler trusted a client-supplied
//! `organization_id` (query param or JSON body) and passed it straight to the
//! repository, performing no membership check. Any authenticated user could
//! read or mutate another org's protocols, contacts, incidents, broadcasts and
//! drills simply by supplying the victim org's UUID, and `create_broadcast` /
//! `create_incident` (mass-notification + incident lifecycle) were not gated to
//! managers.
//!
//! Since the PAP-80 RLS conversion, every emergency handler acquires an
//! `RlsConnection`: the tenant comes from the validated `X-Tenant-ID` header
//! (membership checked against `organization_members`, 403 for non-members),
//! the client-supplied `organization_id` query/body value is ignored for
//! scoping, and broadcast/incident lifecycle actions are gated on a manager
//! role (403 for ordinary members). Repository queries stay keyed on
//! `(id, organization_id)`, so a foreign UUID resolves to `None` → `404`.
//!
//! These tests exercise the HTTP surface end-to-end with real HS256 JWTs:
//!   1. Seed two orgs (A, B), a member user in each, and a protocol in Org A.
//!   2. Org B's member targets Org A via `X-Tenant-ID` → rejected (403);
//!      no leak/write.
//!   3. Org A's member reads/writes its own org → allowed (2xx) — proving the
//!      test establishes real org/tenant context (the membership row), so the
//!      cross-org rejection is NOT a trivial "no context" pass.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp, TestConfig};

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
    .bind(format!("EmergencyIDOR Org {slug}"))
    .bind(format!("emergency-idor-org-{slug}"))
    .bind(format!("{slug}@emergency-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'EmergencyIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed an emergency protocol in `org_id` and return its id.
async fn seed_protocol(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO emergency_protocols (organization_id, name, protocol_type)
        VALUES ($1, 'Fire Evacuation', 'fire')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed protocol")
}

/// Seed a building in `org_id` (emergency_broadcasts.building_id is NOT NULL).
async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, name)
        VALUES ($1, 'Main St 1', 'Bratislava', '81101', 'HQ')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Mint a real HS256 access token for `user_id`, signed with the same secret
/// the TestApp configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "EmergencyIDOR User", None, None)
        .expect("mint access token")
}

// ---------------------------------------------------------------------------
// T1 — cross-org list_protocols is rejected (information disclosure)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_protocols_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lst-a").await;
    let org_b = seed_org(&pool, "lst-b").await;
    let user_b = seed_user(&pool, "lst-b@emergency-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let _protocol_a = seed_protocol(&pool, org_a).await;

    let token_b = mint_token(user_b, "lst-b@emergency-idor.test");
    let uri = format!("/api/v1/emergency/protocols?organization_id={org_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not list Org A protocols: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// T2 — cross-org get_protocol by UUID is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_protocol_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "get-a").await;
    let org_b = seed_org(&pool, "get-b").await;
    let user_b = seed_user(&pool, "get-b@emergency-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let protocol_a = seed_protocol(&pool, org_a).await;

    let token_b = mint_token(user_b, "get-b@emergency-idor.test");
    // Attacker supplies the VICTIM org id together with the victim protocol id.
    let uri = format!("/api/v1/emergency/protocols/{protocol_a}?organization_id={org_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not read Org A protocol: {}",
        resp.text()
    );
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A protocol must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// T3 — cross-org create_protocol (attacker-supplied org_id) is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_protocol_for_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "crt-a").await;
    let org_b = seed_org(&pool, "crt-b").await;
    let user_b = seed_user(&pool, "crt-b@emergency-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    let token_b = mint_token(user_b, "crt-b@emergency-idor.test");
    let body = json!({
        "organization_id": org_a,
        "name": "Injected Protocol",
        "protocol_type": "fire",
        "steps": [],
    });
    let resp = app
        .execute(
            app.post("/api/v1/emergency/protocols")
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_a.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not create a protocol in Org A: {}",
        resp.text()
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM emergency_protocols WHERE organization_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "no protocol may be created via cross-tenant POST");
}

// ---------------------------------------------------------------------------
// T4 — cross-org delete_protocol by UUID is rejected (mutate IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_protocol_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "del-a").await;
    let org_b = seed_org(&pool, "del-b").await;
    let user_b = seed_user(&pool, "del-b@emergency-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let protocol_a = seed_protocol(&pool, org_a).await;

    let token_b = mint_token(user_b, "del-b@emergency-idor.test");
    let uri = format!("/api/v1/emergency/protocols/{protocol_a}?organization_id={org_a}");
    let resp = app
        .execute(
            app.delete(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not delete Org A protocol: {}",
        resp.text()
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emergency_protocols WHERE id = $1")
        .bind(protocol_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "Org A protocol must not be deleted cross-tenant");
}

// ---------------------------------------------------------------------------
// T5 — create_broadcast requires manager role (ordinary member rejected)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_broadcast_as_plain_member_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "bc-a").await;
    // A genuine member of Org A, but only a `resident` — not a manager.
    let resident = seed_user(&pool, "bc-resident@emergency-idor.test").await;
    seed_membership(&pool, org_a, resident, "resident").await;

    let building_id = seed_building(&pool, org_a).await;

    let token = mint_token(resident, "bc-resident@emergency-idor.test");
    let body = json!({
        "organization_id": org_a,
        "building_id": building_id,
        "title": "Evacuate now",
        "message": "Leave the building",
        "severity": "critical",
    });
    let resp = app
        .execute(
            app.post("/api/v1/emergency/broadcasts")
                .bearer(&token)
                .header("X-Tenant-ID", &org_a.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a non-manager member must not create an emergency broadcast: {}",
        resp.text()
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM emergency_broadcasts WHERE organization_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0, "no broadcast may be created by a non-manager");
}

// ---------------------------------------------------------------------------
// T6 — SAME-ORG SUCCESS: legitimate member reads its own org's protocol.
//
// This is the load-bearing positive case. It proves the harness establishes
// real org/tenant context (an active organization_members row), so the
// cross-org rejections above fail for the RIGHT reason (membership mismatch),
// not because the request lacked any context.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_protocol_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "own-a").await;
    let user_a = seed_user(&pool, "own-a@emergency-idor.test").await;
    seed_membership(&pool, org_a, user_a, "org_admin").await;
    let protocol_a = seed_protocol(&pool, org_a).await;

    let token_a = mint_token(user_a, "own-a@emergency-idor.test");
    let uri = format!("/api/v1/emergency/protocols/{protocol_a}?organization_id={org_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_a)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to read its own protocol: {}",
        resp.text()
    );
    let protocol = resp.json_value();
    assert_eq!(
        protocol.get("id").and_then(|v| v.as_str()),
        Some(protocol_a.to_string().as_str()),
        "expected the owning org's protocol, got {protocol}"
    );
}

// ---------------------------------------------------------------------------
// T7 — SAME-ORG SUCCESS: a manager member CAN create a broadcast in its org.
//
// Companion positive case for the manager gate (T5): confirms the gate admits
// the right role rather than rejecting everything.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_broadcast_as_manager_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "bcok-a").await;
    let manager = seed_user(&pool, "bcok-mgr@emergency-idor.test").await;
    seed_membership(&pool, org_a, manager, "manager").await;

    // emergency_broadcasts.building_id is NOT NULL — seed one for the org.
    let building_id = seed_building(&pool, org_a).await;

    let token = mint_token(manager, "bcok-mgr@emergency-idor.test");
    let body = json!({
        "organization_id": org_a,
        "building_id": building_id,
        "title": "Evacuate now",
        "message": "Leave the building",
        "severity": "critical",
    });
    let resp = app
        .execute(
            app.post("/api/v1/emergency/broadcasts")
                .bearer(&token)
                .header("X-Tenant-ID", &org_a.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "a manager must be able to create an emergency broadcast: {}",
        resp.text()
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM emergency_broadcasts WHERE organization_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1, "manager's broadcast must be persisted");
}

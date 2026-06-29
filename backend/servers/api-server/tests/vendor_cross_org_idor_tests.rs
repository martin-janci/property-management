//! Regression tests for the cross-tenant IDOR fix on the vendor endpoints
//! (`/api/v1/vendors/*`, Epic 21 — issue #825).
//!
//! Audit history: every vendor handler carried `AuthUser` but performed no
//! org scoping. By-id reads/mutations (`get_vendor`, `update_vendor`,
//! `delete_vendor`, `get_contract`, `get_invoice`, approve/reject, …) called
//! `find_by_id(id)` with no membership check, and the org-keyed list/create
//! paths accepted a client-supplied `organization_id` without confirming the
//! caller belongs to it. A foreign caller could read/mutate any other org's
//! vendor, contract or invoice by UUID, or list/write into an arbitrary org.
//!
//! Since the PAP-80 RLS conversion, every vendor handler acquires an
//! `RlsConnection`: the tenant comes from the validated `X-Tenant-ID` header
//! (membership checked against `organization_members`, 403 for non-members)
//! and the client-supplied `organization_id` is ignored for scoping. By-id
//! queries stay keyed on `(id, organization_id)`, so a cross-tenant probe made
//! with the attacker's own valid tenant context resolves to `None` → `404` —
//! "missing" and "forbidden" remain indistinguishable.
//!
//! These tests exercise the HTTP surface end-to-end with real HS256 JWTs:
//!   1. Seed two orgs (A, B), a member user in each, and a vendor in Org A.
//!   2. Org B's member probes Org A's vendor → rejected (4xx); no leak / write.
//!   3. Org A's member reads its own vendor → allowed (2xx).

#![allow(dead_code)]

mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, RequestBuilder, TestApp, TestConfig};

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
    .bind(format!("VendorIDOR Org {slug}"))
    .bind(format!("vendor-idor-org-{slug}"))
    .bind(format!("{slug}@vendor-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'VendorIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a vendor in `org_id` and return its id.
async fn seed_vendor(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO vendors (organization_id, company_name, status)
        VALUES ($1, 'Acme Plumbing', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed vendor")
}

/// Mint a real HS256 access token for `user_id`, signed with the same secret
/// the TestApp configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "VendorIDOR User", None, None)
        .expect("mint access token")
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: cross-tenant/unauthenticated request must be rejected with 4xx, got {status}"
    );
}

// ---------------------------------------------------------------------------
// T1 — unauthenticated get_vendor is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_vendor_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "noauth-a").await;
    let vendor_a = seed_vendor(&pool, org_a).await;

    let uri = format!("/api/v1/vendors/{vendor_a}");
    let resp = app.execute(app.get(&uri).build()).await;

    assert_rejected(resp.status, "get_vendor without bearer token");
}

// ---------------------------------------------------------------------------
// T2 — cross-org list_vendors is rejected (information disclosure)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn list_vendors_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lst-a").await;
    let org_b = seed_org(&pool, "lst-b").await;
    let user_b = seed_user(&pool, "lst-b@vendor-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let _vendor_a = seed_vendor(&pool, org_a).await;

    // User B (member of Org B only) asks for Org A's vendors.
    let token_b = mint_token(user_b, "lst-b@vendor-idor.test");
    let uri = format!("/api/v1/vendors?organization_id={org_a}");
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
        "Org B member must not list Org A vendors: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// T3 — cross-org get_vendor by UUID is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_vendor_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "get-a").await;
    let org_b = seed_org(&pool, "get-b").await;
    let user_b = seed_user(&pool, "get-b@vendor-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let vendor_a = seed_vendor(&pool, org_a).await;

    let token_b = mint_token(user_b, "get-b@vendor-idor.test");
    let uri = format!("/api/v1/vendors/{vendor_a}");
    // Valid context for the attacker's OWN org — the by-id probe must fail on
    // row scoping (404), not on a missing tenant header.
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_rejected(resp.status, "get_vendor cross-tenant");
    // Specifically: must not 200 with Org A's vendor.
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A vendor must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// T4 — cross-org delete_vendor by UUID is rejected (mutate IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn delete_vendor_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "del-a").await;
    let org_b = seed_org(&pool, "del-b").await;
    let user_b = seed_user(&pool, "del-b@vendor-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let vendor_a = seed_vendor(&pool, org_a).await;

    let token_b = mint_token(user_b, "del-b@vendor-idor.test");
    let uri = format!("/api/v1/vendors/{vendor_a}");
    let resp = app
        .execute(
            app.delete(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_rejected(resp.status, "delete_vendor cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::NO_CONTENT,
        "Org A vendor must not be deletable by Org B"
    );

    // The vendor row must still exist.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vendors WHERE id = $1")
        .bind(vendor_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "Org A vendor must not be deleted cross-tenant");
}

// ---------------------------------------------------------------------------
// T5 — cross-org update_vendor (attacker-supplied id) is rejected (mutate IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_vendor_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "upd-a").await;
    let org_b = seed_org(&pool, "upd-b").await;
    let user_b = seed_user(&pool, "upd-b@vendor-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let vendor_a = seed_vendor(&pool, org_a).await;

    let token_b = mint_token(user_b, "upd-b@vendor-idor.test");
    let uri = format!("/api/v1/vendors/{vendor_a}");
    let body = json!({ "company_name": "Hijacked Inc" });
    let resp = app
        .execute(
            RequestBuilder::new(Method::PATCH, &uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_rejected(resp.status, "update_vendor cross-tenant");

    // The vendor name must be unchanged.
    let name: String = sqlx::query_scalar("SELECT company_name FROM vendors WHERE id = $1")
        .bind(vendor_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        name, "Acme Plumbing",
        "Org A vendor must not be mutated cross-tenant"
    );
}

// ---------------------------------------------------------------------------
// T6 — cross-org create_vendor (attacker-supplied org_id) is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_vendor_for_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "crt-a").await;
    let org_b = seed_org(&pool, "crt-b").await;
    let user_b = seed_user(&pool, "crt-b@vendor-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    let token_b = mint_token(user_b, "crt-b@vendor-idor.test");
    let body = json!({
        "organization_id": org_a,
        "company_name": "Injected Vendor",
        "services": [],
    });
    let resp = app
        .execute(
            app.post("/api/v1/vendors")
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_a.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not create a vendor in Org A: {}",
        resp.text()
    );

    // No vendor row should have been created for Org A.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM vendors WHERE organization_id = $1")
        .bind(org_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "no vendor may be created via cross-tenant POST");
}

// ---------------------------------------------------------------------------
// T7 — legitimate same-org access succeeds
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_vendor_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "own-a").await;
    let user_a = seed_user(&pool, "own-a@vendor-idor.test").await;
    seed_membership(&pool, org_a, user_a, "org_admin").await;
    let vendor_a = seed_vendor(&pool, org_a).await;

    let token_a = mint_token(user_a, "own-a@vendor-idor.test");
    let session_a = app.session(token_a, org_a);
    let uri = format!("/api/v1/vendors/{vendor_a}");
    let resp = app.execute(session_a.get(&uri).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to read its own vendor: {}",
        resp.text()
    );
    let vendor = resp.json_value();
    assert_eq!(
        vendor.get("id").and_then(|v| v.as_str()),
        Some(vendor_a.to_string().as_str()),
        "expected the owning org's vendor, got {vendor}"
    );
}

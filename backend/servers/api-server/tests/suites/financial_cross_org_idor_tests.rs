//! Regression tests for the cross-tenant IDOR / missing-auth fix on the
//! financial endpoints (`/api/v1/financial/*`, Epic 11 — issue #802).
//!
//! Audit history: every financial handler either took no auth extractor at
//! all, or accepted a client-supplied `organization_id` (in a query param or
//! JSON body) without checking that the caller is actually a member of that
//! org. The combination let an unauthenticated or foreign caller:
//!   * read any account / invoice / payment by UUID,
//!   * list an org's financial data via `?organization_id=`,
//!   * record payments / create invoices with an attacker-chosen org_id.
//!
//! The fix requires `AuthUser` on every handler and threads the authenticated
//! `user_id` through a `verify_org_access` membership check (and, for by-ID
//! resources, re-derives the org from the fetched row). Cross-tenant requests
//! are rejected (403 for org-keyed reads/writes, 404 for by-ID probes); a
//! request with no bearer token is rejected (401).
//!
//! These tests exercise the HTTP surface end-to-end with real HS256 JWTs:
//!   1. Seed two orgs (A, B), a member user in each, and an account in Org A.
//!   2. Org B's member probes Org A's data → rejected (4xx); no leak / write.
//!   3. Org A's member reads its own data → allowed (2xx).

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, TestApp, TestConfig};

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
    .bind(format!("FinIDOR Org {slug}"))
    .bind(format!("fin-idor-org-{slug}"))
    .bind(format!("{slug}@fin-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'FinIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a financial account in `org_id` and return its id.
async fn seed_account(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO financial_accounts (
            organization_id, name, account_type, currency,
            balance, opening_balance, is_active
        )
        VALUES ($1, 'Org Operating Account', 'operating', 'EUR', 0, 0, true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed account")
}

/// Mint a real HS256 access token for `user_id`, signed with the same secret
/// the TestApp configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "FinIDOR User", None, None)
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
// T1 — unauthenticated list_accounts is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_accounts_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "noauth-a").await;
    let _account = seed_account(&pool, org_a).await;

    let uri = format!("/api/v1/financial/accounts?organization_id={org_a}");
    let resp = app.execute(app.get(&uri).build()).await;

    assert_rejected(resp.status, "list_accounts without bearer token");
}

// ---------------------------------------------------------------------------
// T2 — cross-org list_accounts is rejected (information disclosure)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_accounts_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lst-a").await;
    let org_b = seed_org(&pool, "lst-b").await;
    let user_b = seed_user(&pool, "lst-b@fin-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let _account_a = seed_account(&pool, org_a).await;

    // User B (member of Org B only) asks for Org A's accounts.
    let token_b = mint_token(user_b, "lst-b@fin-idor.test");
    let uri = format!("/api/v1/financial/accounts?organization_id={org_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not list Org A accounts: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// T3 — cross-org get_account by UUID is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_account_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "get-a").await;
    let org_b = seed_org(&pool, "get-b").await;
    let user_b = seed_user(&pool, "get-b@fin-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;
    let account_a = seed_account(&pool, org_a).await;

    let token_b = mint_token(user_b, "get-b@fin-idor.test");
    let uri = format!("/api/v1/financial/accounts/{account_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_b).build()).await;

    assert_rejected(resp.status, "get_account cross-tenant");
    // Specifically: must not 200 with Org A's account.
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A account must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// T4 — cross-org record_payment (attacker-supplied org_id) is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn record_payment_for_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "pay-a").await;
    let org_b = seed_org(&pool, "pay-b").await;
    let user_b = seed_user(&pool, "pay-b@fin-idor.test").await;
    seed_membership(&pool, org_b, user_b, "org_admin").await;

    let token_b = mint_token(user_b, "pay-b@fin-idor.test");
    let body = json!({
        "organization_id": org_a,
        "user_id": user_b,
        "unit_id": Uuid::new_v4(),
        "amount": "100.00",
        "payment_method": "bank_transfer",
        "payment_date": "2026-01-01",
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/payments")
                .bearer(&token_b)
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B member must not record a payment against Org A: {}",
        resp.text()
    );

    // No payment row should have been created for Org A.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payments WHERE organization_id = $1")
        .bind(org_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "no payment may be recorded via cross-tenant POST");
}

// ---------------------------------------------------------------------------
// T5 — legitimate same-org access succeeds
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_accounts_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "own-a").await;
    let user_a = seed_user(&pool, "own-a@fin-idor.test").await;
    seed_membership(&pool, org_a, user_a, "org_admin").await;
    let _account_a = seed_account(&pool, org_a).await;

    let token_a = mint_token(user_a, "own-a@fin-idor.test");
    let uri = format!("/api/v1/financial/accounts?organization_id={org_a}");
    let resp = app.execute(app.get(&uri).bearer(&token_a).build()).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to list Org A accounts: {}",
        resp.text()
    );
    let accounts = resp.json_value();
    assert!(
        accounts.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "expected at least one account for the owning org, got {accounts}"
    );
}

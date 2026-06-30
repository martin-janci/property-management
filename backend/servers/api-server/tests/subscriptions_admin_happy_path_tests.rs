//! Happy-path integration tests for the platform-admin subscription endpoints
//! (`GET /api/v1/admin/subscriptions`, `GET /api/v1/admin/invoices`) — BIT-420
//! (BIT-417 bucket B).
//!
//! Auth model: both handlers take a `RlsConnection` (resolved from the
//! `X-Tenant-ID` header + an active `organization_members` row) AND additionally
//! call `require_super_admin`, which decodes the bearer token and checks the
//! `roles` claim for a super-admin role. So a working caller needs BOTH a seeded
//! org membership (for the RLS extractor) AND a token carrying the `super_admin`
//! role (for the platform-admin gate). We mint that token directly via
//! `JwtService::generate_access_token` with `roles = ["super_admin"]`, mirroring
//! the recipe in `admin_platform_happy_path_tests.rs`.
//!
//! The negative (401/403) path is covered elsewhere (the platform-admin authz
//! suites); these assert the positive 200 contract.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use api_server::services::JwtService;
use common::{seed_membership, seed_org, TestApp, TestConfig};

/// Seed a regular active user and return its id.
async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'Subs Admin', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Mint a super-admin access token validated by `TestApp`'s JWT secret.
fn mint_super_admin(user_id: Uuid, email: &str) -> String {
    let secret = TestConfig::default().jwt_secret;
    let svc = JwtService::new(&secret).expect("jwt service");
    svc.generate_access_token(
        user_id,
        email,
        "Subs Admin",
        None,
        Some(vec!["super_admin".to_string()]),
    )
    .expect("mint super admin token")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_all_subscriptions_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_id = seed_org(&pool, "subsadmin").await;
    let email = format!("subs-{}@admin.test", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint_super_admin(user_id, &email);

    let resp = app
        .execute(
            app.get("/api/v1/admin/subscriptions")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    assert!(
        resp.json_value().is_array(),
        "list_all_subscriptions must return a JSON array, got: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_all_invoices_2xx(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_id = seed_org(&pool, "invadmin").await;
    let email = format!("inv-{}@admin.test", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint_super_admin(user_id, &email);

    let resp = app
        .execute(
            app.get("/api/v1/admin/invoices")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    assert!(
        resp.json_value().is_array(),
        "list_all_invoices must return a JSON array, got: {}",
        resp.text()
    );
}

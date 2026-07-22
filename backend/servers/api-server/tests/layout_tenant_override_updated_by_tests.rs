//! Regression test for GitHub issue #2435 — tenant layout-override writes must
//! record the acting user in `updated_by`.
//!
//! The bug: `put_tenant_override` bound the authenticated user as `_auth`
//! (deliberately unused) and passed `None` for `updated_by` into
//! `upsert_tenant_override`, so every tenant-override write persisted a NULL
//! actor — the audit trail silently dropped who made the change. Every other
//! write path in the Layout feature (`admin.rs`: upsert_draft / set_rails /
//! publish / kill) threads `Some(admin_id)`; this path was the odd one out.
//!
//! The fix binds `auth` and passes `Some(auth.user_id)`. This test drives the
//! full HTTP handler as an org admin and asserts the persisted row's
//! `updated_by` equals the caller's user id. It fails on `dev` (NULL) and
//! passes after the fix.
//!
//! Auth model mirrors `reserve_funds_happy_path_tests.rs`: under `TestApp`
//! (no `host_tenant_middleware`) the `X-Tenant-ID` header is the tenant source,
//! the JWT `role` claim drives the handler's `tenant.role.is_admin()` gate, and
//! an active `organization_members` row is required for `RlsConnection`.
//!
//! DB-backed via `#[sqlx::test]` (migrator = db::MIGRATOR) — the body runs in
//! CI where Postgres is available; the local dispatcher runner only
//! compile-gates.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp, TestConfig};

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

/// Mint an HS256 access token carrying the `org_admin` role claim (the
/// handler's `is_admin()` gate reads the JWT role) and the tenant id.
fn mint_org_admin_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("org_admin".to_string()),
        email: "layout-updated-by@test.test".to_string(),
        name: "Layout UpdatedBy Admin".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Layout Admin', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a published layout config for `screen` so the PUT handler passes its
/// "screen must be published" gate. The published config is an empty-section
/// `ScreenConfig` and rails default to `{}` (nothing editable); an empty
/// override then validates trivially and the write reaches the upsert.
async fn seed_published_screen(pool: &PgPool, screen: &str) {
    let published = json!({ "screen": screen, "version": 1, "sections": [] });
    sqlx::query(
        r#"INSERT INTO layout_configs (screen, draft, published, published_version, rails)
           VALUES ($1, $2, $2, 1, '{}'::jsonb)"#,
    )
    .bind(screen)
    .bind(&published)
    .execute(pool)
    .await
    .expect("seed published layout config");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn put_tenant_override_records_acting_user_in_updated_by(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_id = seed_org(&pool, "layout-updated-by").await;
    let user_id = seed_user(&pool, &format!("lub-{}@layout.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint_org_admin_token(user_id, org_id);

    let screen = "ppt/dashboard";
    seed_published_screen(&pool, screen).await;

    let resp = app
        .execute(
            app.put("/api/v1/layout/tenant-override")
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(json!({
                    "screen": screen,
                    // Empty override: no order, no section patches → passes
                    // rails validation regardless of the (default) rails.
                    "override_config": { "sections": {} },
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "org admin override write must return 200; body={}",
        resp.text()
    );

    // The persisted row must attribute the write to the acting user, not NULL.
    let persisted_updated_by = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT updated_by FROM layout_tenant_overrides
         WHERE organization_id = $1 AND screen = $2",
    )
    .bind(org_id)
    .bind(screen)
    .fetch_one(&pool)
    .await
    .expect("fetch persisted tenant override row");

    assert_eq!(
        persisted_updated_by,
        Some(user_id),
        "tenant-override write must record the acting user in updated_by (issue #2435)"
    );
}

//! Round-4 regression (audit4-operations): the operations API gates on the
//! platform-admin status stored in `users.principal_kind`, not the JWT `role`
//! claim (which login tokens never carry — previously every real platform
//! admin got 403). A staff user with a forged-looking `super_admin` claim is
//! still rejected, because the claim is no longer trusted.

#![allow(dead_code)]

use axum::http::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{TestApp, TestConfig};

async fn seed_user(pool: &PgPool, kind: &str) -> (Uuid, String) {
    let email = format!("ops-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, 'h', 'Ops R4', 'active', NOW(), $2) RETURNING id"#,
    )
    .bind(&email)
    .bind(kind)
    .fetch_one(pool)
    .await
    .unwrap();
    (id, email)
}

fn login_style_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(user_id, email, "Ops R4", None, None)
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_principal_reaches_operations_with_login_token(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (admin, email) = seed_user(&pool, "platform").await;
    let token = login_style_token(admin, &email);

    let resp = app
        .execute(
            app.get("/api/v1/operations/deployments")
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn staff_user_with_super_admin_claim_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (staff, email) = seed_user(&pool, "staff").await;
    let token = crate::common::mint_tenant_token(
        staff,
        &email,
        Uuid::new_v4(),
        common::tenant::TenantRole::SuperAdmin,
    );

    let resp = app
        .execute(
            app.get("/api/v1/operations/deployments")
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
}

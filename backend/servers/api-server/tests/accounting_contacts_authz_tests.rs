//! Role-gate tests for GET /api/v1/accounting/contacts (PAP-281).
//!
//! `list_contacts` is the only accounting handler that was missing the
//! manager-only gate. These tests assert:
//!   - a non-manager tenant member (resident) gets 403
//!   - a manager gets 200

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp};

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', 'PAP281 Test', 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn mint_jwt(user_id: Uuid, role: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;
    #[derive(Serialize)]
    struct Claims {
        sub: String,
        email: String,
        name: String,
        exp: i64,
        iat: i64,
        jti: String,
        token_type: String,
        role: String,
    }
    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            email: "pap281@test.example".into(),
            name: "PAP281".into(),
            exp: now + 900,
            iat: now,
            jti: Uuid::new_v4().to_string(),
            token_type: "access".into(),
            role: role.into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint_jwt")
}

fn get_contacts(org_id: Uuid, token: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri("/api/v1/accounting/contacts")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap()
}

/// Non-manager (resident) must be rejected with 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_manager_cannot_list_contacts(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "pap281-resident").await;
    let user_id = seed_user(&pool, "resident-pap281@test.example").await;
    seed_membership(&pool, org_id, user_id, "resident").await;

    let token = mint_jwt(user_id, "resident");
    let resp = app.execute(get_contacts(org_id, &token)).await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "resident must get 403 on GET /api/v1/accounting/contacts"
    );
}

/// Manager must receive 200 (even with an empty contacts list).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_can_list_contacts(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "pap281-manager").await;
    let user_id = seed_user(&pool, "manager-pap281@test.example").await;
    seed_membership(&pool, org_id, user_id, "manager").await;

    let token = mint_jwt(user_id, "manager");
    let resp = app.execute(get_contacts(org_id, &token)).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "manager must get 200 on GET /api/v1/accounting/contacts"
    );
}

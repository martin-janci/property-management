//! Phase 4 — RequestPrincipal × PlatformHost tests (B5 regression).
//!
//! The `PlatformHost` variant of `TenantSource` carries `organization_id ==
//! Uuid::nil()` as a sentinel meaning "no specific tenant — global read
//! context". The `RequestPrincipal` extractor must:
//!
//!   * Allow Platform principals through with `effective_org = None`.
//!   * Reject Staff / Public principals with 403 — the platform host is
//!     platform-only; treating the nil sentinel as a real org id used to
//!     produce a misleading "no membership in 00000000-..." 403.
//!
//! These tests stand up the real `host_tenant_middleware` with a
//! `platform_hosts` list containing `localhost`, then mint a JWT carrying only
//! `sub` (Phase 2 contract: tenant_id / role JWT claims are NEVER trusted) and
//! seed users with the requested `principal_kind` directly.

#![allow(dead_code)]

use std::sync::Arc;

use api_core::extractors::principal::RequestPrincipal;
use api_core::middleware::host_tenant::{
    host_tenant_middleware, HostTenantConfig, TenantResolutionCache,
};
use api_core::TenantMembershipProvider;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use chrono::{Duration, Utc};
use db::DbPool;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::{PgPool, Row};
use tower::ServiceExt;
use uuid::Uuid;

const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-phase4-tests";
const PLATFORM_HOST: &str = "platform.test";

#[derive(Clone)]
struct TestState {
    pool: DbPool,
}

impl TenantMembershipProvider for TestState {
    fn db_pool(&self) -> &DbPool {
        &self.pool
    }
}

#[derive(Serialize)]
struct PrincipalToken {
    sub: Uuid,
    iat: i64,
    exp: i64,
    kind: String,
}

fn mint_token(user_id: Uuid, kind_claim: &str) -> String {
    let now = Utc::now();
    let claims = PrincipalToken {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        // Informational only — server re-derives kind from the users table.
        kind: kind_claim.to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint token")
}

/// Echo handler — returns OK with whether `effective_org` is `None`. The
/// extractor itself returns 401/403 on failure.
async fn echo(p: RequestPrincipal) -> String {
    format!("ok effective_org_is_none={}", p.effective_org.is_none())
}

fn build_app(pool: PgPool) -> Router {
    let state = TestState { pool: pool.clone() };
    let cfg = HostTenantConfig::with_parts(
        pool,
        Arc::new(TenantResolutionCache::new(300, 30, 100)),
        false,
        vec![PLATFORM_HOST.to_string()],
    );
    Router::new()
        .route("/echo", get(echo))
        .layer(from_fn_with_state(cfg, host_tenant_middleware))
        .with_state(state)
}

async fn seed_user(pool: &PgPool, email: &str, kind: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'PH User', 'active', NOW(), $2)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("seed user");
    row.get("id")
}

fn req_with_host(host: &str, token: &str) -> Request<Body> {
    Request::builder()
        .uri("/echo")
        .header(header::HOST, host)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

fn ensure_jwt_secret() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| std::env::set_var("JWT_SECRET", JWT_SECRET));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_host_allows_platform_principal(pool: PgPool) {
    ensure_jwt_secret();

    let user = seed_user(&pool, "platform@phase4.test", "platform").await;
    let app = build_app(pool);
    let token = mint_token(user, "platform");

    let resp = app
        .oneshot(req_with_host(PLATFORM_HOST, &token))
        .await
        .expect("req");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "platform principal must be allowed through on platform host"
    );
    let body = axum::body::to_bytes(resp.into_body(), 1024)
        .await
        .expect("read body");
    let body_str = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(
        body_str.contains("effective_org_is_none=true"),
        "platform principal on platform host must have effective_org = None, got: {body_str}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_host_rejects_staff_principal(pool: PgPool) {
    ensure_jwt_secret();

    let user = seed_user(&pool, "staff@phase4.test", "staff").await;
    let app = build_app(pool);
    let token = mint_token(user, "staff");

    let resp = app
        .oneshot(req_with_host(PLATFORM_HOST, &token))
        .await
        .expect("req");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "staff principal must NOT be allowed through on platform host"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn platform_host_rejects_public_principal(pool: PgPool) {
    ensure_jwt_secret();

    let user = seed_user(&pool, "public@phase4.test", "public").await;
    let app = build_app(pool);
    let token = mint_token(user, "public");

    let resp = app
        .oneshot(req_with_host(PLATFORM_HOST, &token))
        .await
        .expect("req");
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "public principal must NOT be allowed through on platform host"
    );
}

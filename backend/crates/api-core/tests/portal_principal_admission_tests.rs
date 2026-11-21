//! Regression (#3669): `PortalPrincipal` ADMITS a reality-portal user that
//! `RequestPrincipal` 403s.
//!
//! # Why this exists
//!
//! #3669 flipped every reality-server route from `RequestPrincipal` to
//! `PortalPrincipal` because `RequestPrincipal` 403'd *every real portal user*:
//! reality-portal agencies/buyers are `principal_kind = 'public'`, never hold
//! an `organization_members` row (their membership lives in
//! `reality_agency_members`), and reach the portal on a `PlatformHost`. Under
//! `RequestPrincipal` that combination is a guaranteed 403 — so contact
//! inquiries, buyer threads, notification settings, and the agency dashboard
//! were all unreachable for the users they were built for.
//!
//! `portal_principal_token_type_tests.rs` already pins the *rejection* side of
//! the shared decode path (bad `token_type`, expired token). What was missing
//! is the *admission* side — a DB-backed proof that:
//!
//!   1. A membership-less `public` user on a **platform host** is 403'd by
//!      `RequestPrincipal` but **admitted** by `PortalPrincipal`.
//!   2. The same user on a **resolved tenant host** (no membership in that org)
//!      is 403'd by `RequestPrincipal` ("no active membership") but **admitted**
//!      by `PortalPrincipal`.
//!   3. `OptionalPortalPrincipal` yields `None` for anonymous callers, `Some`
//!      for a valid portal bearer, and propagates a 401 for a junk token
//!      (never silently demotes a bad token to anonymous).
//!   4. A deleted user's token is still rejected (the trusted-table kind
//!      re-derivation shared with `RequestPrincipal` is not bypassed).
//!
//! This is the broad reality surface's load-bearing gate. It is intentionally
//! placed in `api-core` (the shared extractor crate) so that any later refactor
//! of the push-fanout / notification wiring that touches the common
//! `decode_bearer_subject` / `resolve_principal_kind` path cannot silently
//! re-break admission for the whole reality portal without turning this red.
//!
//! The `#[sqlx::test(migrator = "db::MIGRATOR")]` harness applies production
//! migrations to a throwaway DB (same pattern as
//! `agency_domains_cache_invalidation_tests.rs`); if no Postgres is available
//! the framework skips gracefully.

use api_core::extractors::tenant::TenantMembershipProvider;
use api_core::extractors::{OptionalPortalPrincipal, PortalPrincipal, RequestPrincipal};
use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
use axum::extract::FromRequestParts;
use axum::http::{header::AUTHORIZATION, request::Parts, Request, StatusCode};
use db::models::PrincipalKind;
use db::DbPool;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

const TEST_SECRET: &str = "test-secret-key-at-least-32-chars-long!!";

// ---------------------------------------------------------------------------
// Test plumbing
// ---------------------------------------------------------------------------

/// State satisfying `TenantMembershipProvider`, wrapping the real `sqlx::test`
/// pool so the membership + principal-kind lookups actually run.
#[derive(Clone)]
struct DbState {
    pool: DbPool,
}

impl TenantMembershipProvider for DbState {
    fn db_pool(&self) -> &DbPool {
        &self.pool
    }
}

#[derive(Debug, Serialize)]
struct AccessClaims {
    sub: Uuid,
    iat: i64,
    exp: i64,
    token_type: &'static str,
}

/// Mint a valid, far-future access token for `sub`.
fn mint_access(sub: Uuid) -> String {
    // `JWT_SECRET` is read by `decode_bearer_subject` on every call; this test
    // binary owns the process env.
    std::env::set_var("JWT_SECRET", TEST_SECRET);
    let claims = AccessClaims {
        sub,
        iat: 1_700_000_000,
        exp: 4_102_444_800, // 2100-01-01
        token_type: "access",
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
    )
    .expect("encode access token")
}

/// Build request `Parts` carrying the given bearer token (if any) and the given
/// `ResolvedTenant` (if any, inserted into extensions exactly as
/// `host_tenant_middleware` would).
fn parts_with(token: Option<&str>, tenant: Option<ResolvedTenant>) -> Parts {
    let mut builder = Request::builder().uri("/portal/resource");
    if let Some(token) = token {
        builder = builder.header(AUTHORIZATION, format!("Bearer {token}"));
    }
    let (mut parts, _) = builder.body(()).expect("build request").into_parts();
    if let Some(tenant) = tenant {
        parts.extensions.insert(tenant);
    }
    parts
}

async fn seed_public_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'hash', 'Portal User', 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!(
        "{tag}-{}@portal-admit.test",
        Uuid::new_v4().simple()
    ))
    .fetch_one(pool)
    .await
    .expect("seed public user")
}

async fn seed_org(pool: &PgPool) -> Uuid {
    let slug = format!("admit-org-{}", Uuid::new_v4().simple());
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind("Admit Org")
    .bind(&slug)
    .bind(format!("{slug}@portal-admit.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

// ---------------------------------------------------------------------------
// (1) Platform host: RequestPrincipal 403s, PortalPrincipal admits.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn portal_admits_public_user_on_platform_host_where_request_principal_403s(pool: PgPool) {
    let user = seed_public_user(&pool, "platform").await;
    let token = mint_access(user);
    let state = DbState { pool };

    // The reality portal reaches the backend on the platform host: a nil-org
    // `PlatformHost` ResolvedTenant. This is the exact wiring #3669 had to
    // accommodate.
    let platform = ResolvedTenant {
        organization_id: Uuid::nil(),
        source: TenantSource::PlatformHost,
    };

    // RequestPrincipal: a non-platform principal on a platform host is 403.
    let mut parts = parts_with(Some(&token), Some(platform));
    let req_err = RequestPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect_err("RequestPrincipal must 403 a public user on the platform host");
    assert_eq!(
        req_err.0,
        StatusCode::FORBIDDEN,
        "RequestPrincipal must 403 (platform host requires platform principal)"
    );

    // PortalPrincipal: the same user, same request, is ADMITTED — this is the
    // #3669 fix. It ignores ResolvedTenant / membership entirely and keys
    // authz on `user_id` at the repo layer.
    let mut parts = parts_with(Some(&token), Some(platform));
    let principal = PortalPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect("PortalPrincipal must admit the portal user (#3669)");
    assert_eq!(
        principal.user_id, user,
        "admitted principal carries the user id"
    );
    assert_eq!(
        principal.kind,
        PrincipalKind::Public,
        "kind is re-derived from the trusted users table"
    );
}

// ---------------------------------------------------------------------------
// (2) Resolved tenant host, no membership: RequestPrincipal 403s, Portal admits.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn portal_admits_membershipless_public_user_on_resolved_tenant(pool: PgPool) {
    let user = seed_public_user(&pool, "tenant").await;
    let org = seed_org(&pool).await;
    let token = mint_access(user);
    let state = DbState { pool };

    // A host-resolved tenant org the user has NO membership in — reality
    // buyers never have `organization_members` rows.
    let tenant = ResolvedTenant {
        organization_id: org,
        source: TenantSource::Subdomain,
    };

    let mut parts = parts_with(Some(&token), Some(tenant));
    let req_err = RequestPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect_err("RequestPrincipal must 403 without an active membership");
    assert_eq!(
        req_err.0,
        StatusCode::FORBIDDEN,
        "RequestPrincipal must 403 (no active membership in this organization)"
    );

    let mut parts = parts_with(Some(&token), Some(tenant));
    let principal = PortalPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect("PortalPrincipal must admit the membership-less portal user (#3669)");
    assert_eq!(principal.user_id, user);
    assert_eq!(principal.kind, PrincipalKind::Public);
}

// ---------------------------------------------------------------------------
// (3) OptionalPortalPrincipal: None / Some / strict-401 branches (DB-backed).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn optional_portal_principal_none_some_and_strict_401(pool: PgPool) {
    let user = seed_public_user(&pool, "optional").await;
    let token = mint_access(user);
    let state = DbState { pool };

    // No Authorization header → anonymous `None` (never touches the DB).
    let mut parts = parts_with(None, None);
    let anon = OptionalPortalPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect("anonymous request must not error");
    assert!(anon.0.is_none(), "missing Authorization must yield None");

    // Valid portal bearer → Some(principal).
    let mut parts = parts_with(Some(&token), None);
    let signed = OptionalPortalPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect("valid portal token must resolve")
        .0
        .expect("a present+valid Authorization must yield Some");
    assert_eq!(signed.user_id, user);
    assert_eq!(signed.kind, PrincipalKind::Public);

    // Junk token with a header present → strict 401, NOT a silent demotion to
    // anonymous (mirrors OptionalRequestPrincipal's leak-#10/#11 contract).
    let mut parts = parts_with(Some("not-a-real-jwt"), None);
    let err = OptionalPortalPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect_err("a present-but-invalid token must propagate, not demote to None");
    assert_eq!(err.0, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// (4) A deleted user's token is rejected — the shared trusted-table kind
//     re-derivation is not bypassed by the portal path.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn portal_principal_rejects_deleted_user(pool: PgPool) {
    let user = seed_public_user(&pool, "deleted").await;
    sqlx::query("UPDATE users SET status = 'deleted' WHERE id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .expect("soft-delete user");
    let token = mint_access(user);
    let state = DbState { pool };

    let mut parts = parts_with(Some(&token), None);
    let err = PortalPrincipal::from_request_parts(&mut parts, &state)
        .await
        .expect_err("a deleted user's token must be rejected");
    assert_eq!(
        err.0,
        StatusCode::UNAUTHORIZED,
        "deleted principal → 401 (unknown principal)"
    );
}

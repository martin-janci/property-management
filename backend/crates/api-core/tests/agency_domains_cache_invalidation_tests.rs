//! B14 — `agency_domains` cache-invalidation integration tests (Phase 6, leak #3).
//!
//! Verifies the end-to-end contract:
//!
//!   1. A verified `agency_domains` row causes the host-resolution middleware to
//!      resolve Host: foo.test → org_a.
//!   2. After the row is deleted via `AgencyDomainRepository::release_rls`, the
//!      same host no longer resolves — NOT eventual consistency. The next request
//!      must see 404 immediately (cache invalidated on delete).
//!
//! The test wires the real `TenantResolutionCache` (from `api-core`) into the
//! repository as an `AgencyDomainCacheInvalidator` impl, then drives the full
//! `host_tenant_middleware` via Tower's `ServiceExt::oneshot`. This is the same
//! pattern used by `dev_mode_tenant_tests.rs` in `api-server`.
//!
//! `#[sqlx::test(migrator = "db::MIGRATOR")]` is used so the test DB has all
//! production migrations applied without needing a running Postgres from the
//! environment. If no Postgres is available the test framework skips gracefully.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use api_core::middleware::host_tenant::{
    host_tenant_middleware, HostTenantConfig, TenantResolutionCache,
};
use db::models::agency_domain::{AgencyDomainKind, CreateAgencyDomain};
use db::repositories::AgencyDomainRepository;

// ---------------------------------------------------------------------------
// Test plumbing
// ---------------------------------------------------------------------------

/// Minimal echo handler — returns 200 so we can distinguish "resolved" from
/// the middleware's 404 "unknown tenant host".
async fn echo_ok() -> axum::response::Response {
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap()
}

/// Build a router with `host_tenant_middleware` layered over `echo_ok`.
///
/// The cache is shared between the router middleware and the caller so the
/// caller can wire it into the repository invalidator.
fn test_router(pool: PgPool, cache: Arc<TenantResolutionCache>) -> Router {
    let cfg = HostTenantConfig::with_parts(
        pool,
        cache,
        false,            // dev_mode off — only host-based resolution
        Vec::new(),       // no platform_hosts
    );

    Router::new()
        .route("/probe", get(echo_ok))
        .fallback(echo_ok)
        .layer(from_fn_with_state(cfg, host_tenant_middleware))
}

/// Send a GET /probe with the given Host header and return the response status.
async fn probe(router: &Router, host: &str) -> StatusCode {
    let req = Request::builder()
        .uri("/probe")
        .header(header::HOST, host)
        .body(Body::empty())
        .expect("build request");

    router
        .clone()
        .oneshot(req)
        .await
        .expect("router oneshot")
        .status()
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

/// Insert an `active` organization and return its id.
async fn seed_org(pool: &PgPool) -> Uuid {
    let slug = format!("b14-org-{}", Uuid::new_v4().simple());
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("B14 Org {}", &slug[..12]))
    .bind(&slug)
    .bind(format!("{slug}@b14.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Insert a `verified` `agency_domains` row mapping `host` → `org_id`.
/// Returns the domain row's `id` so it can be deleted later.
async fn seed_verified_domain(pool: &PgPool, org_id: Uuid, host: &str) -> Uuid {
    // Insert via direct SQL (super-admin bypass) — we don't need cache
    // invalidation for the initial seed, only for the delete.
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO agency_domains
            (organization_id, host, kind, verification_state, is_primary)
        VALUES ($1, $2, 'subdomain', 'verified', true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(host)
    .fetch_one(pool)
    .await
    .expect("seed agency_domain")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Core invariant: after a domain row is deleted via the repository (which
/// calls `cache.invalidate`), the NEXT request sees 404 — not the old cached
/// resolution.
///
/// Steps:
///  1. Seed org + verified agency_domains row for host = `foo.test`.
///  2. Probe → expect 200 (resolves to org_a).
///  3. Delete via `release_rls` (which calls `cache.invalidate`).
///  4. Probe again → expect 404 (immediately, not after TTL).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_invalidates_cache_immediately(pool: PgPool) {
    let host = format!("b14-foo-{}.test", Uuid::new_v4().simple());
    let org_id = seed_org(&pool).await;
    let domain_id = seed_verified_domain(&pool, org_id, &host).await;

    // Build a cache and share it with both the middleware and the repository.
    let cache = Arc::new(TenantResolutionCache::new(300, 30, 1_000));
    let router = test_router(pool.clone(), cache.clone());

    // Step 2: first probe — must resolve (200).
    let status_before = probe(&router, &host).await;
    assert_eq!(
        status_before,
        StatusCode::OK,
        "verified domain must resolve before delete"
    );

    // Step 3: delete via repository with cache wired in.
    // The repository's `release_rls` calls `cache.invalidate(host)`.
    let repo = AgencyDomainRepository::new(pool.clone())
        .with_cache(cache.clone() as Arc<dyn db::repositories::agency_domain::AgencyDomainCacheInvalidator>);

    // release_rls needs a connection with super-admin RLS context.
    let mut conn = pool.acquire().await.expect("acquire conn");
    db::tenant_context::set_request_context(&mut *conn, None, None, true)
        .await
        .expect("set super-admin context for delete");

    let released = repo
        .release_rls(&mut *conn, domain_id)
        .await
        .expect("release_rls");
    assert_eq!(
        released.as_deref(),
        Some(host.as_str()),
        "release_rls must return the deleted host"
    );

    db::tenant_context::clear_request_context(&mut *conn)
        .await
        .ok();
    drop(conn);

    // Step 4: probe immediately after delete — must NOT resolve (404).
    let status_after = probe(&router, &host).await;
    assert_eq!(
        status_after,
        StatusCode::NOT_FOUND,
        "domain must NOT resolve immediately after delete (cache must be invalidated)"
    );
}

/// Regression guard: `invalidate` is a method on `TenantResolutionCache` and
/// is callable via `AgencyDomainCacheInvalidator`. This test confirms the
/// cache behaves correctly for the invalidation path in isolation (no DB
/// needed — pure in-memory).
#[tokio::test]
async fn tenant_resolution_cache_invalidate_removes_positive_entry() {
    use api_core::middleware::host_tenant::TenantResolutionCache;
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    use db::repositories::agency_domain::AgencyDomainCacheInvalidator;

    let cache = TenantResolutionCache::new(300, 30, 100);

    let host = "acme.test";
    let org_id = Uuid::new_v4();
    let resolved = ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    };

    // Populate the cache.
    cache.set(host, Some(resolved)).await;
    assert!(
        cache.get(host).await.is_some(),
        "cache must hold the entry before invalidate"
    );

    // Invalidate via the trait (the path the repository uses).
    cache.invalidate(host).await;

    assert!(
        cache.get(host).await.is_none(),
        "cache must be empty after invalidate"
    );
}

/// Confirm that `release_rls` on a domain that does NOT exist returns `None`
/// and does not call the invalidator spuriously. Uses the existing
/// `MockInvalidator` pattern inline.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn release_nonexistent_domain_returns_none(pool: PgPool) {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingInvalidator(AtomicUsize);

    #[async_trait::async_trait]
    impl db::repositories::agency_domain::AgencyDomainCacheInvalidator for CountingInvalidator {
        async fn invalidate(&self, _host: &str) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        async fn invalidate_all(&self) {}
    }

    let invalidator = Arc::new(CountingInvalidator(AtomicUsize::new(0)));
    let repo = AgencyDomainRepository::new(pool.clone())
        .with_cache(invalidator.clone() as Arc<dyn db::repositories::agency_domain::AgencyDomainCacheInvalidator>);

    let mut conn = pool.acquire().await.expect("acquire");
    db::tenant_context::set_request_context(&mut *conn, None, None, true)
        .await
        .expect("set ctx");

    let ghost_id = Uuid::new_v4(); // no matching row
    let result = repo.release_rls(&mut *conn, ghost_id).await.expect("release");

    db::tenant_context::clear_request_context(&mut *conn).await.ok();

    assert!(result.is_none(), "release of nonexistent domain must return None");
    assert_eq!(
        invalidator.0.load(Ordering::SeqCst),
        0,
        "invalidate must NOT be called when no row was deleted"
    );
}

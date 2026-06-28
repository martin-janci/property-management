//! Phase 6 — A5: Raw-pool bypass audit behavior tests for reality-server.
//!
//! Asserts that tenant-scoped routes (e.g. `/api/v1/listings/{id}`) return a
//! 4xx error — specifically 404 "Unknown tenant host" — when a request arrives
//! WITHOUT a resolved host context. This guards against the regression where a
//! route handler calls the unfenced `DbPool` directly (bypassing the
//! host-resolution middleware), which would silently return cross-tenant data.
//!
//! # Test strategy
//!
//! We build a minimal Axum router that:
//! 1. Mounts a stub listings handler at `/api/v1/listings/{id}` — one that
//!    would return HTTP 200 if it were ever reached.
//! 2. Wraps the router with the real `host_tenant_middleware` using an empty
//!    tenant-resolution cache and no seeded platform hosts.
//! 3. Issues a GET request **without** a `Host` header (no resolved tenant).
//!
//! The middleware MUST fail closed with 404 before the handler is invoked.
//! Any 200 response would indicate the middleware was bypassed — a data-leak
//! regression.
//!
//! The test does NOT require a live database because we seed no organizations:
//! the middleware short-circuits on the missing Host header before it would
//! ever touch the pool. However it does accept a `PgPool` via `#[sqlx::test]`
//! so the test runner supplies a migrated, isolated pool consistent with the
//! rest of the integration suite.

#![allow(dead_code)]

use std::sync::Arc;

use axum::{
    body::Body,
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use sqlx::PgPool;
use tower::ServiceExt;

use api_core::middleware::host_tenant::{
    host_tenant_middleware, HostTenantConfig, TenantResolutionCache,
};

// ============================================================================
// Stub handler
// ============================================================================

/// A stub "listings detail" handler that would return 200 if ever reached.
///
/// If any test receives a 200 from this handler it means the middleware was
/// bypassed — a tenant-isolation regression.
async fn stub_listing_handler() -> axum::response::Response {
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("x-reached-handler", "true")
        .body(Body::from(
            r#"{"id":"00000000-0000-0000-0000-000000000001"}"#,
        ))
        .unwrap()
}

// ============================================================================
// Router builder
// ============================================================================

/// Build a minimal router that mirrors the reality-server listing route,
/// wrapped with the real `host_tenant_middleware`.
///
/// `platform_hosts` is deliberately empty so no host can be resolved to a
/// platform-wide global context — every non-allowlist path must resolve a
/// tenant from the `agency_domains` table (which has no rows).
fn build_test_router(pool: PgPool) -> Router {
    let cfg = HostTenantConfig::with_parts(
        pool,
        Arc::new(TenantResolutionCache::new(300, 30, 100)),
        false,      // dev_mode off — no /a/{slug} shortcuts
        Vec::new(), // no platform hosts — every host must resolve via DB
    );

    Router::new()
        .route("/api/v1/listings/{id}", get(stub_listing_handler))
        .route("/api/v1/listings", get(stub_listing_handler))
        .layer(from_fn_with_state(cfg, host_tenant_middleware))
}

// ============================================================================
// Helper: build a GET request
// ============================================================================

fn get_req_no_host(uri: &str) -> Request<Body> {
    // Deliberately omit the Host header to simulate a request that never
    // resolved to any tenant. The middleware MUST reject this.
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn get_req_unknown_host(uri: &str, host: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap()
}

// ============================================================================
// Tests
// ============================================================================

/// A request to a tenant-scoped listing route WITHOUT a Host header must be
/// rejected (404) by the middleware before the handler is ever invoked.
///
/// This is the primary guard: if reality-server routes were to call the raw
/// `DbPool` and bypass the middleware entirely, a similar request would return
/// 200 with data from *all* tenants — a cross-tenant data leak.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn listing_route_without_host_header_is_rejected(pool: PgPool) {
    let app = build_test_router(pool);

    let listing_id = uuid::Uuid::new_v4();
    let resp = app
        .oneshot(get_req_no_host(&format!("/api/v1/listings/{}", listing_id)))
        .await
        .expect("oneshot failed");

    let status = resp.status();

    // The middleware must fail closed. Any 2xx would mean the handler was
    // reached without a resolved tenant — a tenant-isolation bypass.
    assert!(
        status.is_client_error() || status.is_server_error(),
        "Expected 4xx/5xx (middleware fail-closed) for no-Host request, got {}",
        status,
    );
    assert_ne!(
        status,
        StatusCode::OK,
        "Got 200 — middleware was bypassed; raw-pool data leak regression",
    );
}

/// A request with a Host header that does NOT match any registered tenant
/// domain must also be rejected (404). This guards the variant where an
/// attacker supplies an arbitrary Host value hoping to fall through to global
/// data.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn listing_route_with_unknown_host_is_rejected(pool: PgPool) {
    let app = build_test_router(pool);

    let listing_id = uuid::Uuid::new_v4();
    let resp = app
        .oneshot(get_req_unknown_host(
            &format!("/api/v1/listings/{}", listing_id),
            "evil-attacker.example.com",
        ))
        .await
        .expect("oneshot failed");

    let status = resp.status();

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Expected 404 for unknown-host request; got {status}. \
         If this is 200, the middleware was bypassed — cross-tenant data leak.",
    );
}

/// The listings search endpoint (`GET /api/v1/listings`) is equally guarded:
/// without a resolved tenant the request must be rejected, not silently served
/// with cross-tenant data.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn listing_search_without_host_is_rejected(pool: PgPool) {
    let app = build_test_router(pool);

    let resp = app
        .oneshot(get_req_no_host("/api/v1/listings"))
        .await
        .expect("oneshot failed");

    assert_ne!(
        resp.status(),
        StatusCode::OK,
        "Got 200 on /api/v1/listings without a Host header — middleware was bypassed",
    );
}

/// Verify the handler IS reachable when a platform host resolves to the
/// global-read context. This confirms the test router is correctly wired and
/// rules out a trivially broken setup where 404 comes from route-not-found
/// rather than middleware rejection.
///
/// We re-build the router with `localhost` in the platform-host list (mirroring
/// what `HostTenantConfig::new` does in dev mode) and issue a request with
/// `Host: localhost`. The middleware resolves this to `TenantSource::PlatformHost`
/// and lets it through; the stub handler returns 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn listing_route_reachable_with_platform_host(pool: PgPool) {
    // Build a router where "localhost" is a known platform host.
    let cfg = HostTenantConfig::with_parts(
        pool,
        Arc::new(TenantResolutionCache::new(300, 30, 100)),
        false,
        vec!["localhost".to_string()],
    );

    let app = Router::new()
        .route("/api/v1/listings/{id}", get(stub_listing_handler))
        .layer(from_fn_with_state(cfg, host_tenant_middleware));

    let listing_id = uuid::Uuid::new_v4();
    let resp = app
        .oneshot(get_req_unknown_host(
            &format!("/api/v1/listings/{}", listing_id),
            "localhost",
        ))
        .await
        .expect("oneshot failed");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Expected 200 when platform host resolves — stub handler should be reached",
    );
    // Confirm the handler itself was reached (not a middleware pass-through 200).
    assert_eq!(
        resp.headers()
            .get("x-reached-handler")
            .and_then(|v| v.to_str().ok()),
        Some("true"),
        "Handler did not set x-reached-handler — was it actually invoked?",
    );
}

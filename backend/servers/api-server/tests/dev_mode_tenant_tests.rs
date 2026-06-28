//! Dev-mode `/a/{slug}` tenant-resolution integration tests (Phase 1).
//!
//! The host-resolution middleware itself (`host_tenant_middleware`) and its
//! `/a/{slug}` dev-mode branch were built in Wave 2. Wave 2 also added 19
//! unit tests in `api-core` covering `parse_dev_slug_path`, `normalize_host`,
//! the public allowlist and the `TenantResolutionCache`.
//!
//! What those unit tests do NOT cover — and what this file adds — is the
//! middleware *wired into a real router with a real database*, exercising the
//! full request path:
//!
//! * `dev_mode = true`  → `/a/{slug}/...` resolves the slug to an org, strips
//!   the `/a/{slug}` prefix from the URI, and injects
//!   `ResolvedTenant { source: DevPath }` into request extensions.
//! * `dev_mode = false` → `/a/{slug}/...` is treated as a literal path: no
//!   resolution, no strip; it falls through to host-based resolution and, for
//!   an unknown host, fails closed with `404`.
//!
//! These tests need a database (the dev-slug branch calls
//! `AgencyDomainRepository::resolve_slug_system`, which queries `organizations`).
//! `#[sqlx::test]` provides a migrated, isolated pool, so they are NOT marked
//! `#[ignore]` — they run wherever the rest of the api-server integration
//! suite runs (CI sets `DATABASE_URL`). If you run them locally without a
//! database, `#[sqlx::test]` will skip/fail exactly like every other
//! integration test in this crate.

#![allow(dead_code)]

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

use api_core::middleware::host_tenant::{
    host_tenant_middleware, HostTenantConfig, ResolvedTenant, TenantResolutionCache, TenantSource,
};

/// Downstream handler. Reports, via response headers, exactly what the handler
/// observed *after* the middleware ran:
///
/// * `x-seen-path`     — the request path the handler received (post-rewrite).
/// * `x-resolved`      — `1` if a `ResolvedTenant` extension is present, else `0`.
/// * `x-tenant-source` — `devpath` | `subdomain` | `customdomain` when resolved.
async fn echo_handler(request: Request<Body>) -> axum::response::Response {
    let path = request.uri().path().to_string();
    let resolved = request.extensions().get::<ResolvedTenant>().copied();

    let mut builder = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("x-seen-path", path);

    match resolved {
        Some(rt) => {
            let source = match rt.source {
                TenantSource::DevPath => "devpath",
                TenantSource::Subdomain => "subdomain",
                TenantSource::CustomDomain => "customdomain",
                TenantSource::PlatformHost => "platformhost",
            };
            builder = builder
                .header("x-resolved", "1")
                .header("x-tenant-source", source)
                .header("x-org-id", rt.organization_id.to_string());
        }
        None => {
            builder = builder.header("x-resolved", "0");
        }
    }

    builder.body(Body::empty()).unwrap()
}

/// Build a minimal router with the real `host_tenant_middleware` layered over a
/// catch-all echo handler. `dev_mode` is set explicitly so the test does not
/// depend on the ambient `RUST_ENV`.
fn test_router(pool: PgPool, dev_mode: bool) -> Router {
    // Phase 4 added the `platform_hosts` field; the tests use an empty list so
    // `ignored-host.example.test` is never short-circuited to PlatformHost.
    let cfg = HostTenantConfig::with_parts(
        pool,
        Arc::new(TenantResolutionCache::new(300, 30, 100)),
        dev_mode,
        Vec::new(),
    );

    Router::new()
        .route("/", get(echo_handler))
        .route("/listings", get(echo_handler))
        .route("/listings/{id}", get(echo_handler))
        .fallback(echo_handler)
        .layer(from_fn_with_state(cfg, host_tenant_middleware))
}

/// Insert an `active` organization with the given slug and return its id.
async fn seed_org(pool: &PgPool, slug: &str) -> uuid::Uuid {
    sqlx::query_scalar::<_, uuid::Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Test Agency {slug}"))
    .bind(slug)
    .bind(format!("{slug}@example.test"))
    .fetch_one(pool)
    .await
    .expect("failed to seed organization")
}

fn get_req(uri: &str, host: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .header(header::HOST, host)
        .body(Body::empty())
        .unwrap()
}

// ============================================================================
// dev_mode = true
// ============================================================================

/// With `dev_mode = true`, `/a/{slug}/listings/123` resolves the slug to its
/// org, the handler sees the stripped path `/listings/123`, and the injected
/// `ResolvedTenant` has `source = DevPath`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dev_mode_resolves_slug_and_strips_prefix(pool: PgPool) {
    let slug = "acme-dev";
    let org_id = seed_org(&pool, slug).await;

    let app = test_router(pool, true);
    let resp = app
        .oneshot(get_req(
            &format!("/a/{slug}/listings/123"),
            "ignored-host.example.test",
        ))
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let headers = resp.headers();
    assert_eq!(
        headers.get("x-seen-path").unwrap(),
        "/listings/123",
        "handler should see the URI with the /a/{{slug}} prefix stripped"
    );
    assert_eq!(
        headers.get("x-resolved").unwrap(),
        "1",
        "a ResolvedTenant extension should be injected"
    );
    assert_eq!(
        headers.get("x-tenant-source").unwrap(),
        "devpath",
        "source must be DevPath for /a/{{slug}} resolution"
    );
    assert_eq!(
        headers.get("x-org-id").unwrap(),
        &org_id.to_string(),
        "resolved organization_id must match the seeded org"
    );
}

/// With `dev_mode = true`, `/a/{slug}` with no trailing path resolves and the
/// handler sees a bare `/`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dev_mode_resolves_slug_root_path(pool: PgPool) {
    let slug = "acme-root";
    seed_org(&pool, slug).await;

    let app = test_router(pool, true);
    let resp = app
        .oneshot(get_req(&format!("/a/{slug}"), "ignored-host.example.test"))
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("x-seen-path").unwrap(), "/");
    assert_eq!(resp.headers().get("x-resolved").unwrap(), "1");
    assert_eq!(resp.headers().get("x-tenant-source").unwrap(), "devpath");
}

/// With `dev_mode = true`, an `/a/{slug}` whose slug does NOT match any active
/// org fails closed with `404` (the middleware never reaches the handler).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dev_mode_unknown_slug_is_404(pool: PgPool) {
    // no org seeded for this slug
    let app = test_router(pool, true);
    let resp = app
        .oneshot(get_req(
            "/a/does-not-exist/listings",
            "ignored-host.example.test",
        ))
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "an unresolved dev slug must fail closed"
    );
}

// ============================================================================
// dev_mode = false
// ============================================================================

/// With `dev_mode = false`, `/a/{slug}/...` is NOT treated as a dev path. It is
/// a literal request path, so the middleware falls through to host-based
/// resolution. The `Host` header points at an unknown host, so the request
/// fails closed with `404` — and crucially the path is never rewritten.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn prod_mode_treats_a_slug_as_literal_path_and_404s_unknown_host(pool: PgPool) {
    // Seed an org with this slug to prove that even a *valid* slug is ignored
    // when dev_mode is off — resolution is strictly host-based.
    let slug = "acme-prod";
    seed_org(&pool, slug).await;

    let app = test_router(pool, false);
    let resp = app
        .oneshot(get_req(
            &format!("/a/{slug}/listings"),
            "unknown-host.example.test",
        ))
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "with dev_mode off, /a/{{slug}} is a literal path; unknown host must 404"
    );
}

/// With `dev_mode = false`, even a normal path on an unknown host fails closed
/// with `404` — confirming the dev-slug branch is not what produced the 404
/// in the test above.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn prod_mode_unknown_host_is_404(pool: PgPool) {
    let app = test_router(pool, false);
    let resp = app
        .oneshot(get_req("/listings", "another-unknown.example.test"))
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Public allowlist (sanity): dev-slug-looking paths under the allowlist bypass
// resolution entirely regardless of dev_mode.
// ============================================================================

/// `/health` is on the public allowlist, so it bypasses resolution and reaches
/// the handler unresolved even though the `Host` header is unknown. This guards
/// against a regression where the dev-slug branch or host resolution would
/// swallow allowlisted paths.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn allowlisted_path_bypasses_resolution(pool: PgPool) {
    let app = test_router(pool, true);
    let resp = app
        .oneshot(get_req("/health", "unknown-host.example.test"))
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("x-seen-path").unwrap(), "/health");
    assert_eq!(
        resp.headers().get("x-resolved").unwrap(),
        "0",
        "allowlisted paths are not tenant-resolved"
    );
}

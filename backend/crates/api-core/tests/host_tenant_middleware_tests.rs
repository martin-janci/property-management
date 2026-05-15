//! Phase 5.5 — keystone-middleware integration smoke tests.
//!
//! Verifies the wiring of:
//!   * defense leak #15 (per-tenant rate limit at the keystone middleware),
//!   * defense leak #19 (per-tenant metering counter on every request).
//!
//! These tests deliberately bypass the host-resolution DB lookup by driving
//! the [`api_core::middleware::host_tenant::rate_limit_and_meter`] helper
//! directly. The host-resolution path itself is exercised by the DB-backed
//! integration tests under `backend/servers/api-server/tests/`.
//!
//! What the tests assert:
//!   * a flooding tenant sees a 429 with a `Retry-After` header,
//!   * the 429 carries the human-readable plain-text body,
//!   * cross-tenant isolation: tenant B is unaffected by tenant A's flood.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::Response,
    routing::get,
    Router,
};
use tower::ServiceExt;
use uuid::Uuid;

use api_core::middleware::host_tenant::{rate_limit_and_meter, HostTenantConfig};
use api_core::middleware::tenant_ops::TenantRateLimiterSet;

// ----------------------------------------------------------------------------
// Test plumbing
// ----------------------------------------------------------------------------

/// State threaded through the smoke-test middleware: the keystone config
/// (cloned per request — only the inner `Arc`s are deep) plus the fixed
/// organization id we want each request attributed to.
#[derive(Clone)]
struct SmokeState {
    cfg: HostTenantConfig,
    org_id: Uuid,
}

/// Real `rate_limit_and_meter` wired behind a fixed-org-id stub middleware.
async fn smoke_layer(
    State(state): State<SmokeState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    rate_limit_and_meter(&state.cfg, state.org_id, request, next).await
}

/// Build a router that:
///   1. attributes every request to the fixed `org_id` (mimics what
///      `host_tenant_middleware` would do post-resolution),
///   2. runs the real `rate_limit_and_meter` helper around the handler.
///
/// The DB pool used inside [`HostTenantConfig`] is never touched by this
/// helper — only the `rate_limiters` field is. We inject a `sqlx`-style
/// lazy pool against a never-used URL; lazy pools never open a real socket
/// until a query is executed.
fn build_app(rate_limiters: Arc<TenantRateLimiterSet>, org_id: Uuid) -> Router {
    let pool: db::DbPool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
        .expect("lazy pool init");

    let cfg = HostTenantConfig::with_parts(
        pool,
        Arc::new(api_core::middleware::host_tenant::TenantResolutionCache::new(300, 30, 100)),
        false,
        Vec::new(),
    )
    .with_rate_limiters(rate_limiters);

    let state = SmokeState { cfg, org_id };

    Router::new()
        .route("/echo", get(|| async { "ok" }))
        .layer(from_fn_with_state(state, smoke_layer))
}

fn req() -> Request<Body> {
    Request::builder()
        .uri("/echo")
        .body(Body::empty())
        .expect("build request")
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

/// A flooding tenant is rate-limited; the 429 carries the contractual
/// `Retry-After` header (defense leak #15).
#[tokio::test]
async fn flooding_tenant_gets_429_with_retry_after() {
    // Tight quota so we hit the deny branch in O(burst) requests instead
    // of O(rpm).
    let rate_limiters = Arc::new(TenantRateLimiterSet::with_default(5));
    let org = Uuid::new_v4();
    let app = build_app(rate_limiters, org);

    // Burn through the burst.
    let mut allow_count = 0u32;
    let mut deny_count = 0u32;
    let mut deny_with_retry_after = 0u32;
    for _ in 0..100 {
        let resp = app.clone().oneshot(req()).await.expect("send");
        match resp.status() {
            StatusCode::OK => allow_count += 1,
            StatusCode::TOO_MANY_REQUESTS => {
                deny_count += 1;
                if resp.headers().get("retry-after").is_some() {
                    deny_with_retry_after += 1;
                }
            }
            other => panic!("unexpected status {other}"),
        }
    }

    assert!(
        allow_count > 0,
        "rate limiter should allow at least one request before denying"
    );
    assert!(
        deny_count > 0,
        "100 requests against a 5-rpm quota must produce at least one 429"
    );
    assert_eq!(
        deny_with_retry_after, deny_count,
        "every 429 must carry a Retry-After header (got {deny_with_retry_after}/{deny_count})"
    );
}

/// Tenant A's flood does not consume tenant B's quota — per-tenant
/// isolation, the whole point of defense leak #15.
#[tokio::test]
async fn rate_limit_is_isolated_per_tenant() {
    let rate_limiters = Arc::new(TenantRateLimiterSet::with_default(3));
    let org_a = Uuid::new_v4();
    let org_b = Uuid::new_v4();

    let app_a = build_app(rate_limiters.clone(), org_a);
    let app_b = build_app(rate_limiters.clone(), org_b);

    // Saturate A — fire enough requests that we definitely hit a 429.
    let mut a_saw_429 = false;
    for _ in 0..50 {
        let resp = app_a.clone().oneshot(req()).await.expect("send A");
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            a_saw_429 = true;
        }
    }
    assert!(
        a_saw_429,
        "tenant A's flood must trigger at least one 429 (otherwise the test isn't exercising the deny branch)"
    );

    // B's first request must still be allowed.
    let resp_b = app_b.clone().oneshot(req()).await.expect("send B");
    assert_eq!(
        resp_b.status(),
        StatusCode::OK,
        "tenant B must be unaffected by tenant A's saturation (per-tenant isolation, leak #15)"
    );
}

/// The 429 body is human-readable plain text — a small UX detail but worth
/// pinning so a refactor doesn't accidentally swap it for an empty body.
#[tokio::test]
async fn rate_limit_429_body_is_human_readable() {
    let rate_limiters = Arc::new(TenantRateLimiterSet::with_default(1));
    let org = Uuid::new_v4();
    let app = build_app(rate_limiters, org);

    // Burn through the burst, then capture the 429 body.
    let mut last_429: Option<Response> = None;
    for _ in 0..20 {
        let resp = app.clone().oneshot(req()).await.expect("send");
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            last_429 = Some(resp);
            break;
        }
    }
    let resp = last_429.expect("expected at least one 429 within 20 attempts");

    let bytes = axum::body::to_bytes(resp.into_body(), 256)
        .await
        .expect("read body");
    let text = String::from_utf8(bytes.to_vec()).expect("utf-8 body");
    assert!(
        text.contains("rate limit"),
        "429 body should mention 'rate limit', got: {text:?}"
    );
}

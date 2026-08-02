//! Story 2B.7 / BIT-179: generic Idempotency-Key middleware.
//!
//! Exercises the real DB-backed ledger:
//!   * duplicate replay returns the original response instead of re-running,
//!   * same key + different payload is rejected with 422,
//!   * expired rows are lazily discarded and the handler executes again.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::Body,
    extract::{Extension, State},
    http::{Request, StatusCode},
    middleware::{from_fn, Next},
    response::Response,
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use api_core::extractors::TenantMembershipProvider;
use api_core::middleware::{ResolvedTenant, TenantSource};

#[derive(Clone)]
struct TestState {
    pool: db::DbPool,
    hits: Arc<AtomicUsize>,
}

impl TenantMembershipProvider for TestState {
    fn db_pool(&self) -> &db::DbPool {
        &self.pool
    }
}

async fn demo_idempotency(
    Extension(state): Extension<TestState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    api_core::middleware::handle_idempotent_request(state.pool.clone(), request, next).await
}

async fn demo_handler(State(state): State<TestState>) -> (StatusCode, Json<Value>) {
    let call = state.hits.fetch_add(1, Ordering::SeqCst) + 1;
    (
        StatusCode::CREATED,
        Json(json!({
            "call": call,
            "status": "created"
        })),
    )
}

fn build_app(pool: db::DbPool, hits: Arc<AtomicUsize>) -> Router {
    let state = TestState { pool, hits };

    Router::new()
        .route(
            "/demo",
            post(demo_handler).route_layer(from_fn(demo_idempotency)),
        )
        .layer(Extension(state.clone()))
        .with_state(state)
}

fn post_demo(key: &str, tenant_id: Uuid, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/demo")
        .header("content-type", "application/json")
        .header("x-tenant-id", tenant_id.to_string())
        .header("idempotency-key", key)
        .body(Body::from(body.to_string()))
        .expect("build request")
}

/// Build a request that carries a **server-side** `ResolvedTenant` in its
/// extensions (as `host_tenant_middleware` would inject it), optionally
/// alongside a *spoofed* client-supplied `X-Tenant-ID` header. The idempotency
/// scope must derive from the extension, never from the header.
fn post_demo_resolved(
    key: &str,
    resolved_org: Uuid,
    spoofed_header: Option<Uuid>,
    body: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/demo")
        .header("content-type", "application/json")
        .header("idempotency-key", key);
    if let Some(spoof) = spoofed_header {
        builder = builder.header("x-tenant-id", spoof.to_string());
    }
    builder
        .extension(ResolvedTenant {
            organization_id: resolved_org,
            source: TenantSource::Subdomain,
        })
        .body(Body::from(body.to_string()))
        .expect("build request")
}

fn is_replayed(response: &Response) -> bool {
    response
        .headers()
        .get("x-idempotency-replayed")
        .and_then(|value| value.to_str().ok())
        == Some("true")
}

async fn read_json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    serde_json::from_slice(&bytes).expect("response json")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn duplicate_request_replays_cached_response(pool: db::DbPool) {
    let hits = Arc::new(AtomicUsize::new(0));
    let app = build_app(pool, hits.clone());
    let tenant_id = Uuid::new_v4();

    let first = app
        .clone()
        .oneshot(post_demo("idem-replay", tenant_id, json!({"value": 1})))
        .await
        .expect("first response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_body = read_json(first).await;
    assert_eq!(first_body["call"], 1);

    let second = app
        .clone()
        .oneshot(post_demo("idem-replay", tenant_id, json!({"value": 1})))
        .await
        .expect("second response");
    assert_eq!(second.status(), StatusCode::CREATED);
    assert_eq!(
        second
            .headers()
            .get("x-idempotency-replayed")
            .and_then(|value| value.to_str().ok()),
        Some("true"),
        "duplicate replay must be served from the cached response"
    );
    let second_body = read_json(second).await;
    assert_eq!(
        second_body, first_body,
        "cached replay must match the original"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the handler must execute only once for duplicate requests"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn payload_mismatch_with_reused_key_returns_422(pool: db::DbPool) {
    let hits = Arc::new(AtomicUsize::new(0));
    let app = build_app(pool, hits.clone());
    let tenant_id = Uuid::new_v4();

    let first = app
        .clone()
        .oneshot(post_demo("idem-mismatch", tenant_id, json!({"value": 1})))
        .await
        .expect("first response");
    assert_eq!(first.status(), StatusCode::CREATED);

    let second = app
        .clone()
        .oneshot(post_demo("idem-mismatch", tenant_id, json!({"value": 2})))
        .await
        .expect("second response");
    assert_eq!(second.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = read_json(second).await;
    assert_eq!(body["code"], "IDEMPOTENCY_KEY_REUSED");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "mismatched replay must not re-run the handler"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn expired_cached_row_is_recomputed(pool: db::DbPool) {
    let hits = Arc::new(AtomicUsize::new(0));
    let app = build_app(pool.clone(), hits.clone());
    let tenant_id = Uuid::new_v4();
    let path = "/demo";

    let first = app
        .clone()
        .oneshot(post_demo("idem-expired", tenant_id, json!({"value": 1})))
        .await
        .expect("first response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first_body = read_json(first).await;
    assert_eq!(first_body["call"], 1);

    sqlx::query(
        r#"
        UPDATE idempotency_keys
        SET expires_at = NOW() - INTERVAL '1 second'
        WHERE tenant_scope = $1
          AND request_method = 'POST'
          AND request_path = $2
          AND idempotency_key = 'idem-expired'
        "#,
    )
    // No `ResolvedTenant` is injected on these legacy requests, so the scope is
    // the server-side "global" sentinel — NOT the client-supplied header value.
    .bind("global")
    .bind(path)
    .execute(&pool)
    .await
    .expect("expire idempotency row");

    let second = app
        .clone()
        .oneshot(post_demo("idem-expired", tenant_id, json!({"value": 1})))
        .await
        .expect("second response");
    assert_eq!(second.status(), StatusCode::CREATED);
    assert!(
        second.headers().get("x-idempotency-replayed").is_none(),
        "expired entry must not be replayed from cache"
    );
    let second_body = read_json(second).await;
    assert_eq!(
        second_body["call"], 2,
        "expired entry must re-run the handler"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        2,
        "handler must execute again after TTL expiry"
    );
}

/// Two *different authenticated tenants* sending the SAME idempotency key must
/// NOT collide — each tenant gets its own scope, so the second tenant's request
/// executes the handler fresh rather than replaying tenant A's cached response.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn different_authenticated_tenants_do_not_collide(pool: db::DbPool) {
    let hits = Arc::new(AtomicUsize::new(0));
    let app = build_app(pool, hits.clone());
    let org_a = Uuid::new_v4();
    let org_b = Uuid::new_v4();

    // Tenant A, key "shared-key" -> handler runs (call 1).
    let first = app
        .clone()
        .oneshot(post_demo_resolved(
            "shared-key",
            org_a,
            None,
            json!({"value": 1}),
        ))
        .await
        .expect("tenant A response");
    assert_eq!(first.status(), StatusCode::CREATED);
    assert!(!is_replayed(&first));
    let first_body = read_json(first).await;
    assert_eq!(first_body["call"], 1);

    // Tenant B, SAME key, SAME payload -> must NOT be served from A's cache.
    let second = app
        .clone()
        .oneshot(post_demo_resolved(
            "shared-key",
            org_b,
            None,
            json!({"value": 1}),
        ))
        .await
        .expect("tenant B response");
    assert_eq!(second.status(), StatusCode::CREATED);
    assert!(
        !is_replayed(&second),
        "tenant B must not replay tenant A's cached response for the same key"
    );
    let second_body = read_json(second).await;
    assert_eq!(
        second_body["call"], 2,
        "a different authenticated tenant must execute the handler fresh"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        2,
        "each tenant's identical key must run the handler once — no cross-tenant collision"
    );
}

/// A spoofed client-supplied `X-Tenant-ID` header must NOT influence the
/// idempotency cache scope — only the server-side `ResolvedTenant` counts.
///
/// Proven two ways:
///   1. Same resolved tenant + *different* spoofed headers still replays
///      (the header is ignored, so the scope is stable).
///   2. Different resolved tenants + the *same* spoofed header do NOT collide
///      (the header cannot force two tenants into one scope — the exact bypass
///      the old header fallback allowed).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn spoofed_tenant_header_cannot_change_scope(pool: db::DbPool) {
    let hits = Arc::new(AtomicUsize::new(0));
    let app = build_app(pool, hits.clone());
    let org_a = Uuid::new_v4();
    let org_b = Uuid::new_v4();
    let spoof_x = Uuid::new_v4();
    let spoof_y = Uuid::new_v4();

    // (1) Tenant A, spoofed header X, key "spoof-key" -> handler runs (call 1).
    let first = app
        .clone()
        .oneshot(post_demo_resolved(
            "spoof-key",
            org_a,
            Some(spoof_x),
            json!({"value": 1}),
        ))
        .await
        .expect("first response");
    assert_eq!(first.status(), StatusCode::CREATED);
    assert!(!is_replayed(&first));
    let first_body = read_json(first).await;
    assert_eq!(first_body["call"], 1);

    // (1) Same resolved tenant A, but a DIFFERENT spoofed header Y, same key +
    //     payload -> must REPLAY. The header does not participate in the scope,
    //     so changing it must not change the outcome.
    let second = app
        .clone()
        .oneshot(post_demo_resolved(
            "spoof-key",
            org_a,
            Some(spoof_y),
            json!({"value": 1}),
        ))
        .await
        .expect("second response");
    assert_eq!(second.status(), StatusCode::CREATED);
    assert!(
        is_replayed(&second),
        "changing the spoofed X-Tenant-ID header must not change the cache scope"
    );
    let second_body = read_json(second).await;
    assert_eq!(
        second_body, first_body,
        "must replay tenant A's cached body"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "same resolved tenant replays regardless of the spoofed header"
    );

    // (2) DIFFERENT resolved tenant B, but the SAME spoofed header X as the
    //     first request. Under the old header-trusting scope this would collide
    //     with tenant A and wrongly replay A's response. With the fix it must
    //     run the handler fresh under tenant B's own scope.
    let third = app
        .clone()
        .oneshot(post_demo_resolved(
            "spoof-key",
            org_b,
            Some(spoof_x),
            json!({"value": 1}),
        ))
        .await
        .expect("third response");
    assert_eq!(third.status(), StatusCode::CREATED);
    assert!(
        !is_replayed(&third),
        "a spoofed header matching another tenant must not grant that tenant's cache scope"
    );
    let third_body = read_json(third).await;
    assert_eq!(
        third_body["call"], 2,
        "tenant B must execute the handler under its own scope"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        2,
        "the spoofed X-Tenant-ID header must never merge two tenants into one scope"
    );
}

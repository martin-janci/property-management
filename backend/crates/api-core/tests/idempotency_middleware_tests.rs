//! Story 2B.7 / BIT-179: generic Idempotency-Key middleware.
//!
//! Exercises the real DB-backed ledger:
//!   * duplicate replay returns the original response instead of re-running,
//!   * same key + different payload is rejected with 422,
//!   * expired rows are lazily discarded and the handler executes again.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
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
use sha2::{Digest, Sha256};
use sqlx::Connection;
use tokio::sync::Notify;
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

/// State for the "hanging handler" fixture used by the cancellation-safety
/// regression test below: the handler signals `started` the moment it begins
/// running (proving the idempotency middleware has acquired its lock and is
/// now parked inside `next.run(request).await`), then hangs forever so the
/// test can abort the request task mid-flight, deterministically.
#[derive(Clone)]
struct HangingTestState {
    pool: db::DbPool,
    started: Arc<Notify>,
}

impl TenantMembershipProvider for HangingTestState {
    fn db_pool(&self) -> &db::DbPool {
        &self.pool
    }
}

async fn hanging_idempotency(
    Extension(state): Extension<HangingTestState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    api_core::middleware::handle_idempotent_request(state.pool.clone(), request, next).await
}

async fn hanging_handler(State(state): State<HangingTestState>) -> (StatusCode, Json<Value>) {
    state.started.notify_one();
    // Never resolves — the test aborts the enclosing task while this is
    // pending, simulating a client disconnect mid-request.
    std::future::pending::<()>().await;
    unreachable!("hanging_handler must be cancelled before resolving")
}

fn build_hanging_app(pool: db::DbPool, started: Arc<Notify>) -> Router {
    let state = HangingTestState { pool, started };

    Router::new()
        .route(
            "/demo",
            post(hanging_handler).route_layer(from_fn(hanging_idempotency)),
        )
        .layer(Extension(state.clone()))
        .with_state(state)
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

/// Reimplements `advisory_lock_id` (private to `idempotency.rs`) so the test
/// can independently compute the exact lock id a given request will hash to,
/// without needing the middleware to expose its internals.
fn expected_lock_id(tenant_scope: &str, method: &str, path: &str, key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(tenant_scope.as_bytes());
    hasher.update(b"\n");
    hasher.update(method.as_bytes());
    hasher.update(b"\n");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(key.as_bytes());

    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

/// Regression test for the advisory-lock cancellation leak: if the request
/// future carrying `handle_idempotent_request` is dropped mid-flight (e.g. a
/// client disconnect while the wrapped handler is still running), the
/// idempotency lock it holds must still be released — not leaked for the
/// life of the pooled connection.
///
/// Pre-fix, the middleware acquired a *session-level* `pg_advisory_lock` on a
/// bare pooled connection and released it only via an explicit
/// `pg_advisory_unlock` after `next.run(request).await` returned. Dropping
/// the future between those two points (as happens here) skipped the
/// release entirely, wedging every subsequent request — from ANY session —
/// that shares the same idempotency key behind a lock nobody would ever
/// release.
///
/// The check below deliberately opens an *independent* Postgres connection
/// (its own backend session/PID) rather than reusing `pool` to test for the
/// lock: Postgres advisory locks are re-entrant *within the same session*,
/// so probing from a connection the pool happens to hand back from its own
/// recycled/leaked connection would trivially "succeed" even under the bug
/// and prove nothing. Only a genuinely different session can distinguish
/// "released" from "leaked but incidentally re-entered".
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancelled_request_does_not_leak_advisory_lock(pool: db::DbPool) {
    // `#[sqlx::test]`'s fixture pool sets `idle_timeout(Some(1s))` on the
    // per-test database (see sqlx-postgres's `testing::test_context`), so an
    // idle connection — including one that leaked a session-level advisory
    // lock — gets physically closed (and the lock incidentally released
    // along with it) within about a second regardless of whether this
    // middleware releases it correctly. That would mask the exact bug this
    // test exists to catch. Build a dedicated pool against the same
    // database with idle reaping disabled so only the middleware's own
    // release behavior can free the lock.
    let app_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .expect("open dedicated no-idle-reap pool for the app");

    let started = Arc::new(Notify::new());
    let hanging_app = build_hanging_app(app_pool.clone(), started.clone());
    let tenant_id = Uuid::new_v4();
    let idempotency_key = "idem-cancel-leak";

    let first_request = post_demo(idempotency_key, tenant_id, json!({"value": 1}));
    let first_task = tokio::spawn(async move {
        let _ = hanging_app.oneshot(first_request).await;
    });

    // Block until the handler has actually started — i.e. the idempotency
    // middleware has acquired its advisory lock and is now parked inside
    // `next.run(request).await`, exactly the window the bug leaked in.
    started.notified().await;

    // Simulate a client disconnect mid-flight by aborting the task while it
    // is still inside the handler. This drops `handle_idempotent_request`'s
    // future — and everything it was holding — without ever reaching the
    // unlock/commit at the end of the function. Awaiting the aborted handle
    // guarantees the future's Drop glue (and thus the transaction's queued
    // rollback) has run before we proceed.
    first_task.abort();
    let _ = first_task.await;

    // `post_demo` requests carry no `ResolvedTenant` extension, so
    // `tenant_scope()` resolves to the fixed "global" sentinel regardless of
    // the `x-tenant-id` header value (see `spoofed_tenant_header_cannot_change_scope`).
    let lock_id = expected_lock_id("global", "POST", "/demo", idempotency_key);

    // Open a brand-new connection to the same database, independent of
    // `pool`'s recycling, so we're guaranteed to probe from a different
    // backend session.
    let mut checker = sqlx::PgConnection::connect_with(pool.connect_options().as_ref())
        .await
        .expect("open independent checker connection");

    // The background flush that turns the queued `ROLLBACK` into bytes on
    // the wire (sqlx's `Floating::return_to_pool` pinging the connection
    // before releasing it) races with this check, so poll briefly instead of
    // asserting on the first attempt. Pre-fix, this never becomes true and
    // the outer timeout fails the test instead of hanging the suite.
    let poll_result = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
                .bind(lock_id)
                .fetch_one(&mut checker)
                .await
                .expect("pg_try_advisory_lock query");
            if acquired {
                let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
                    .bind(lock_id)
                    .execute(&mut checker)
                    .await;
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;

    assert!(
        poll_result.is_ok(),
        "advisory lock id={lock_id} is still held by another session after the \
         first request was cancelled — the idempotency lock leaked past task \
         cancellation instead of being released"
    );
}

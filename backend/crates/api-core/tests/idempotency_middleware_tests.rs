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
    extract::State,
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
    State(state): State<TestState>,
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
        .route("/demo", post(demo_handler).route_layer(from_fn(demo_idempotency)))
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
    assert_eq!(second_body, first_body, "cached replay must match the original");
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
    .bind(tenant_id.to_string())
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
    assert_eq!(second_body["call"], 2, "expired entry must re-run the handler");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        2,
        "handler must execute again after TTL expiry"
    );
}

//! HTTP-level integration test for the **outbound** layout-change webhook's
//! replay/timestamp handling (issue #2485).
//!
//! ## What the existing coverage already pins — and what it doesn't
//!
//! The signing primitives (`sign_payload`, `sign_timestamped_payload`,
//! `build_webhook_body`) are unit-tested inline in
//! `routes/layout/webhook.rs`, and the *receiver* side (reality-web
//! `/api/layout-revalidate`) has Vitest coverage for the ±300s freshness
//! window, stale-timestamp replay, and the swapped-fresh-timestamp attack.
//!
//! Neither exercises the **dispatcher**, `notify_layout_change`, over the
//! wire. A regression there — signing the body ALONE (the pre-#2485 bug),
//! dropping the `X-Webhook-Timestamp` header, or shipping a stale/hardcoded
//! timestamp — would leave every unit test green while silently
//! re-introducing the replayable delivery #2485 closed. This test closes
//! that gap the same way `portal_webhook_signature_tests` closes its own:
//! by driving the real, wired code path (here: `notify_layout_change`)
//! against a live receiver and asserting the bytes/headers that actually go
//! out.
//!
//! ## Method
//!
//! Spin an ephemeral in-process axum receiver, point `LAYOUT_WEBHOOK_URL` at
//! it, invoke `notify_layout_change` (fire-and-forget), then capture the
//! delivered request and assert:
//!
//! 1. `X-Webhook-Timestamp` is present and is the **current** wall clock
//!    (bracketed by before/after) — a stale/hardcoded timestamp would defeat
//!    the receiver's freshness window.
//! 2. `X-Webhook-Signature` reconstructs over `"{timestamp}.{body}"` — the
//!    exact string the reality-web receiver verifies.
//! 3. The delivered signature does NOT equal the legacy body-only signature —
//!    the timestamp must be *bound* into the HMAC (replay defense).
//! 4. The signed body is the correlatable JSON (screen / event /
//!    published_version / delivery_id).
//!
//! Needs a `PgPool` because `notify_layout_change` also records a
//! `layout_webhook_dispatched` analytics event; under `SQLX_OFFLINE=true`
//! with no DB the harness skips it, and CI runs it against an ephemeral
//! migrated database.

#![allow(dead_code)]

use std::net::SocketAddr;
use std::time::Duration;

use api_server::routes::layout::webhook::notify_layout_change;
use axum::{body::Bytes, extract::State, http::HeaderMap, http::StatusCode, routing::post, Router};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const SECRET: &str = "layout-outbound-itest-secret";

/// `std::env::{set_var,remove_var}` are process-global and not safe to
/// interleave with `getenv` on glibc; integration tests in this binary run
/// concurrently. Every test mutating the `LAYOUT_WEBHOOK_*` env serializes
/// behind this async mutex, held across the `.await`s so the vars stay stable
/// while `notify_layout_change` reads them and the delivery lands.
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Everything the mock receiver captured off the wire for one delivery.
#[derive(Debug)]
struct Captured {
    timestamp_header: Option<String>,
    signature_header: Option<String>,
    body: Vec<u8>,
}

/// HMAC-SHA256 over `body` alone, `sha256=<base64>` — the *legacy* (pre-#2485)
/// signature shape. Used only as the negative in the replay-binding assertion.
fn sign_body_only(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac accepts any key");
    mac.update(body);
    format!("sha256={}", BASE64.encode(mac.finalize().into_bytes()))
}

/// Reconstruct the receiver-side signature over `"{timestamp}.{body}"`.
/// Independent of the api-server implementation so a bug there can't mask
/// itself in the expectation.
fn sign_timestamped(secret: &str, timestamp: i64, body: &[u8]) -> String {
    let mut composed = Vec::with_capacity(body.len() + 20);
    composed.extend_from_slice(timestamp.to_string().as_bytes());
    composed.push(b'.');
    composed.extend_from_slice(body);
    sign_body_only(secret, &composed)
}

/// Mock receiver handler: forward the headers + raw body to the test and 200.
async fn capture(
    State(tx): State<mpsc::Sender<Captured>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let header_str = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let _ = tx
        .send(Captured {
            timestamp_header: header_str("X-Webhook-Timestamp"),
            signature_header: header_str("X-Webhook-Signature"),
            body: body.to_vec(),
        })
        .await;
    StatusCode::OK
}

/// Bind an ephemeral loopback receiver; returns its URL and the capture channel.
async fn spawn_receiver() -> (String, mpsc::Receiver<Captured>) {
    let (tx, rx) = mpsc::channel::<Captured>(1);
    let app = Router::new().route("/", post(capture)).with_state(tx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral loopback port");
    let addr: SocketAddr = listener.local_addr().expect("resolve local addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}/"), rx)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn outbound_delivery_carries_fresh_timestamp_and_timestamped_signature(pool: PgPool) {
    let _guard = ENV_LOCK.lock().await;
    let (url, mut rx) = spawn_receiver().await;

    std::env::set_var("LAYOUT_WEBHOOK_URL", &url);
    std::env::set_var("LAYOUT_WEBHOOK_SECRET", SECRET);

    let delivery_id = Uuid::from_u128(0x0bad_c0de_0bad_c0de_0bad_c0de_0bad_c0de);

    // Bracket the dispatch so we can assert the shipped timestamp is "now".
    let before = chrono::Utc::now().timestamp();
    notify_layout_change(
        pool.clone(),
        "reality/listing-detail",
        "published",
        Some(7),
        delivery_id,
    );

    let captured = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("mock receiver must receive the outbound delivery within 5s")
        .expect("capture channel closed before a delivery arrived");
    let after = chrono::Utc::now().timestamp();

    // Env already consumed by `notify_layout_change`; clear it while still under
    // the lock so a later assert-panic can't leave it set for a sibling test.
    std::env::remove_var("LAYOUT_WEBHOOK_URL");
    std::env::remove_var("LAYOUT_WEBHOOK_SECRET");

    // 1. Fresh signed timestamp (issue #2485).
    let ts_raw = captured
        .timestamp_header
        .expect("X-Webhook-Timestamp header must be present on the delivery");
    let ts: i64 = ts_raw
        .parse()
        .unwrap_or_else(|_| panic!("X-Webhook-Timestamp must be unix-seconds, got {ts_raw:?}"));
    assert!(
        (before..=after).contains(&ts),
        "delivered timestamp {ts} must be the current wall clock (in [{before}, {after}]) — a \
         stale or hardcoded timestamp would fall outside the receiver's freshness window"
    );

    // 2. Signature covers exactly "{timestamp}.{body}" — the receiver reconstruction.
    let sig = captured
        .signature_header
        .expect("X-Webhook-Signature header must be present on the delivery");
    assert_eq!(
        sig,
        sign_timestamped(SECRET, ts, &captured.body),
        "outbound signature must be HMAC over \"{{timestamp}}.{{body}}\" so reality-web's \
         replay-defense verification matches"
    );

    // 3. Replay-binding regression guard for #2485: the shipped signature must
    //    NOT equal the legacy body-only signature, or a captured POST could be
    //    replayed with a swapped-in fresh timestamp.
    assert_ne!(
        sig,
        sign_body_only(SECRET, &captured.body),
        "outbound signature must bind the timestamp into the HMAC, not sign the body alone"
    );

    // 4. The signed body is the correlatable publish payload.
    let v: serde_json::Value =
        serde_json::from_slice(&captured.body).expect("delivered body must be valid JSON");
    assert_eq!(v["screen"], "reality/listing-detail");
    assert_eq!(v["event"], "published");
    assert_eq!(v["published_version"], 7);
    assert_eq!(v["delivery_id"], delivery_id.to_string());
}

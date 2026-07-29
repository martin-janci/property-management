//! Outbound layout-change webhook notifier.
//!
//! When `LAYOUT_WEBHOOK_URL` and `LAYOUT_WEBHOOK_SECRET` are both set (and
//! non-empty), every successful `publish`, `rollback`, `kill`, or `unkill`
//! operation fires a signed POST to the configured URL so downstream consumers
//! (CDN invalidators, caches, notification services) can react immediately.
//!
//! ## Signature format
//!
//! Each delivery carries an `X-Webhook-Timestamp` header (unix seconds) and an
//! `X-Webhook-Signature` header computed over the **timestamped** payload:
//!
//! ```text
//! X-Webhook-Timestamp: <unix-seconds>
//! X-Webhook-Signature: sha256=<base64(HMAC-SHA256(LAYOUT_WEBHOOK_SECRET, "{timestamp}.{body}"))>
//! ```
//!
//! This mirrors the timestamped portal-webhook convention
//! (`routes/portal_webhooks.rs::verify_timestamped_portal_webhook` and the
//! Stripe receiver): folding the timestamp into the signed payload binds it to
//! the signature so it cannot be swapped independently, and the receiver
//! (`reality-web /api/layout-revalidate`) enforces a ±300s freshness window.
//!
//! ## Replay protection (issue #2485)
//!
//! Previously the signature was computed over the raw body ALONE with no
//! timestamp, so a captured POST could be replayed against the reality-web
//! revalidation endpoint indefinitely. Signing `"{timestamp}.{body}"` and
//! shipping the timestamp lets the receiver reject stale/replayed deliveries.
//!
//! ## Fire-and-forget
//!
//! The notification is spawned via `tokio::spawn` so it never blocks or fails
//! the originating request. A non-2xx response or network error is logged as
//! `warn!` only — the admin operation already succeeded.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use db::repositories::{LayoutChangeEventKind, LayoutRepository};
use db::DbPool;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// The header name carrying the outbound HMAC-SHA256 signature.
///
/// Intentionally identical to the inbound portal-webhook convention so the
/// receiving end can reuse the same verification logic.
const SIGNATURE_HEADER: &str = "X-Webhook-Signature";

/// The header name carrying the signed unix-seconds timestamp used for replay
/// protection (issue #2485). Matches the portal-webhook convention
/// (`portal_webhooks.rs::TIMESTAMP_HEADER`).
const TIMESTAMP_HEADER: &str = "X-Webhook-Timestamp";

/// Sign `bytes` with `secret` using HMAC-SHA256 and return the header value.
///
/// The returned string has the form `sha256=<base64-digest>`. This is the
/// low-level primitive; outbound layout deliveries sign the *timestamped*
/// payload via [`sign_timestamped_payload`].
///
/// This is a pure function with no I/O; it is unit-tested with a hardcoded
/// known vector below.
pub fn sign_payload(secret: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(body);
    format!("sha256={}", BASE64.encode(mac.finalize().into_bytes()))
}

/// Sign the timestamped payload `"{timestamp}.{body}"` with `secret`.
///
/// This is the format the reality-web receiver reconstructs and verifies: the
/// unix-seconds `timestamp` (also shipped in the `X-Webhook-Timestamp` header)
/// is folded into the signed bytes so it cannot be swapped independently of the
/// signature, defeating replay of a captured delivery outside the freshness
/// window (issue #2485). Mirrors
/// `portal_webhooks.rs::verify_timestamped_portal_webhook`'s
/// `"{timestamp}.{raw_body}"` construction.
pub fn sign_timestamped_payload(secret: &str, timestamp: i64, body: &[u8]) -> String {
    let mut signed = Vec::with_capacity(body.len() + 20);
    signed.extend_from_slice(timestamp.to_string().as_bytes());
    signed.push(b'.');
    signed.extend_from_slice(body);
    sign_payload(secret, &signed)
}

/// Build the outbound webhook body bytes.
///
/// The body carries the correlation `delivery_id` (gap D) and the post-mutation
/// `published_version` (gap A) in addition to `screen`/`event`, so the receiver
/// can echo the id back on `layout_revalidate_received` and dashboards can join
/// the three events of one mutation. The reality-web receiver validates only
/// `screen` and tolerates extra fields, and the HMAC covers the whole body, so
/// widening the shape is signature-safe.
///
/// `published_version` is `None` for `killed`/`unkilled` (which create no
/// version row); it serialises to JSON `null`.
pub fn build_webhook_body(
    screen: &str,
    event: &str,
    published_version: Option<i32>,
    delivery_id: Uuid,
) -> Vec<u8> {
    serde_json::json!({
        "screen": screen,
        "event": event,
        "published_version": published_version,
        "delivery_id": delivery_id,
    })
    .to_string()
    .into_bytes()
}

/// Persist a `layout_webhook_dispatched` analytics event to the append-only
/// sink (migration 00225). Fire-and-forget: a persistence failure is logged at
/// `warn!` and never affects the admin mutation or the delivery.
async fn record_dispatched(
    pool: DbPool,
    screen: String,
    event: &'static str,
    published_version: Option<i32>,
    delivery_id: Uuid,
    dispatch_outcome: &'static str,
    http_status: Option<u16>,
) {
    let props = serde_json::json!({
        "event": LayoutChangeEventKind::WebhookDispatched.as_str(),
        "change_kind": event,
        "dispatch_outcome": dispatch_outcome,
        "http_status": http_status,
        "published_version": published_version,
        // target_tenant: layout config is global/platform-owned (events doc §4.3).
        "target_tenant": "*",
    });
    if let Err(e) = LayoutRepository::new()
        .record_change_event(
            &pool,
            LayoutChangeEventKind::WebhookDispatched,
            Some(&screen),
            Some(delivery_id),
            None,
            &props,
        )
        .await
    {
        tracing::warn!(
            error = %e,
            screen = %screen,
            event = %event,
            delivery_id = %delivery_id,
            "failed to persist layout_webhook_dispatched event"
        );
    }
}

/// Notify an external webhook endpoint that a layout change occurred, and record
/// a `layout_webhook_dispatched` analytics event for the attempt.
///
/// Reads `LAYOUT_WEBHOOK_URL` and `LAYOUT_WEBHOOK_SECRET` from the environment
/// at call time (no startup plumbing needed). When either variable is absent or
/// empty the outbound POST is skipped, but a `skipped_unconfigured` analytics
/// event is still recorded so "webhook silently off" is observable.
///
/// The notification and the analytics write are fire-and-forget (`tokio::spawn`):
/// a non-2xx response, a network error, or a failed analytics insert is logged
/// as `warn!` but does not propagate to the caller.
///
/// # Parameters
/// - `pool`              — DB handle for the append-only analytics sink.
/// - `screen`            — the layout screen identifier (e.g. `"home"`).
/// - `event`             — one of `"published"`, `"rolled_back"`, `"killed"`,
///   `"unkilled"`.
/// - `published_version` — the post-mutation version (gap A); `None` for
///   `killed`/`unkilled`.
/// - `delivery_id`       — correlation id (gap D) shared with the
///   `layout_change_published` event and echoed to the receiver.
pub fn notify_layout_change(
    pool: DbPool,
    screen: &str,
    event: &'static str,
    published_version: Option<i32>,
    delivery_id: Uuid,
) {
    let screen = screen.to_owned();

    let url = std::env::var("LAYOUT_WEBHOOK_URL")
        .ok()
        .filter(|s| !s.is_empty());
    let secret = std::env::var("LAYOUT_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.is_empty());

    let (url, secret) = match (url, secret) {
        (Some(u), Some(s)) => (u, s),
        _ => {
            // Unconfigured: skip the POST but still record the event so the
            // "webhook off" state is observable in analytics.
            tracing::debug!(
                screen = %screen,
                event = %event,
                "LAYOUT_WEBHOOK_URL/SECRET not set — skipping layout-change notification"
            );
            tokio::spawn(record_dispatched(
                pool,
                screen,
                event,
                published_version,
                delivery_id,
                "skipped_unconfigured",
                None,
            ));
            return;
        }
    };

    let body = build_webhook_body(&screen, event, published_version, delivery_id);

    // Replay protection (issue #2485): ship a signed unix-seconds timestamp and
    // sign over "{timestamp}.{body}" so the reality-web receiver can reject any
    // captured delivery replayed outside its ±300s freshness window.
    let timestamp = chrono::Utc::now().timestamp();
    let signature = sign_timestamped_payload(&secret, timestamp, &body);

    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build();

        let client = match client {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "layout-change webhook: failed to build HTTP client"
                );
                record_dispatched(
                    pool,
                    screen,
                    event,
                    published_version,
                    delivery_id,
                    "transport_error",
                    None,
                )
                .await;
                return;
            }
        };

        let (outcome, http_status): (&'static str, Option<u16>) = match client
            .post(&url)
            .header("Content-Type", "application/json")
            .header(TIMESTAMP_HEADER, timestamp.to_string())
            .header(SIGNATURE_HEADER, &signature)
            .body(body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let status = resp.status();
                tracing::debug!(
                    screen = %screen,
                    event = %event,
                    status = %status,
                    "layout-change webhook delivered"
                );
                ("delivered", Some(status.as_u16()))
            }
            Ok(resp) => {
                let status = resp.status();
                tracing::warn!(
                    screen = %screen,
                    event = %event,
                    status = %status,
                    "layout-change webhook returned non-2xx"
                );
                ("non_2xx", Some(status.as_u16()))
            }
            Err(e) => {
                tracing::warn!(
                    screen = %screen,
                    event = %event,
                    error = %e,
                    "layout-change webhook delivery failed"
                );
                ("transport_error", None)
            }
        };

        record_dispatched(
            pool,
            screen,
            event,
            published_version,
            delivery_id,
            outcome,
            http_status,
        )
        .await;
    });
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-vector: pin the exact output of `sign_payload` so regressions in
    /// the algorithm or encoding are caught immediately.
    ///
    /// Vector computed independently:
    ///   secret = "test-secret"
    ///   body   = b"{\"screen\":\"home\",\"event\":\"published\"}"
    ///   digest = HMAC-SHA256(secret, body) as standard base64
    ///
    /// Verified with:
    ///   echo -n '{"screen":"home","event":"published"}' \
    ///     | openssl dgst -sha256 -hmac "test-secret" -binary | base64
    const KNOWN_SECRET: &str = "test-secret";
    const KNOWN_BODY: &[u8] = b"{\"screen\":\"home\",\"event\":\"published\"}";
    /// Pre-computed expected value for the known vector above.
    /// Verified with: echo -n '{"screen":"home","event":"published"}' \
    ///   | openssl dgst -sha256 -hmac "test-secret" -binary | base64
    const KNOWN_EXPECTED: &str = "sha256=TXF7Uzi0HS1OSbiy8iK/1JuQ3mJUYzTt+tSYZy9tdfk=";

    #[test]
    fn sign_payload_known_vector() {
        let sig = sign_payload(KNOWN_SECRET, KNOWN_BODY);
        assert_eq!(
            sig, KNOWN_EXPECTED,
            "sign_payload must produce the pinned known-vector output byte-for-byte"
        );
    }

    #[test]
    fn sign_payload_different_secret_produces_different_output() {
        let sig1 = sign_payload("secret-a", KNOWN_BODY);
        let sig2 = sign_payload("secret-b", KNOWN_BODY);
        assert_ne!(
            sig1, sig2,
            "different secrets must produce different signatures"
        );
    }

    #[test]
    fn sign_payload_different_body_produces_different_output() {
        let sig1 = sign_payload(KNOWN_SECRET, b"body-one");
        let sig2 = sign_payload(KNOWN_SECRET, b"body-two");
        assert_ne!(
            sig1, sig2,
            "different bodies must produce different signatures"
        );
    }

    #[test]
    fn sign_payload_format_has_sha256_prefix() {
        let sig = sign_payload(KNOWN_SECRET, KNOWN_BODY);
        assert!(
            sig.starts_with("sha256="),
            "signature must start with 'sha256=' prefix, got: {sig}"
        );
    }

    #[test]
    fn sign_payload_digest_is_valid_base64() {
        let sig = sign_payload(KNOWN_SECRET, KNOWN_BODY);
        let b64_part = sig.trim_start_matches("sha256=");
        assert!(
            BASE64.decode(b64_part).is_ok(),
            "the portion after 'sha256=' must be valid standard base64, got: {b64_part}"
        );
    }

    // ------------------------------------------------------------------
    // Timestamped signing (issue #2485): the outbound delivery must sign
    // "{timestamp}.{body}" so the reality-web receiver can enforce a
    // freshness window and reject replayed captures.
    // ------------------------------------------------------------------

    const TS: i64 = 1_700_000_000;

    #[test]
    fn sign_timestamped_payload_matches_prefixed_body() {
        // The timestamped signer must equal signing the composed
        // "{ts}.{body}" bytes through the low-level primitive — this is the
        // exact string the receiver reconstructs.
        let mut composed = Vec::new();
        composed.extend_from_slice(format!("{TS}.").as_bytes());
        composed.extend_from_slice(KNOWN_BODY);

        assert_eq!(
            sign_timestamped_payload(KNOWN_SECRET, TS, KNOWN_BODY),
            sign_payload(KNOWN_SECRET, &composed),
            "timestamped signature must cover exactly \"{{timestamp}}.{{body}}\""
        );
    }

    #[test]
    fn sign_timestamped_payload_known_vector() {
        // Independently verified:
        //   printf '1700000000.{"screen":"home","event":"published"}' \
        //     | openssl dgst -sha256 -hmac "test-secret" -binary | base64
        let sig = sign_timestamped_payload(KNOWN_SECRET, TS, KNOWN_BODY);
        assert_eq!(
            sig, "sha256=JUbVDRhJ5cUMQCjGQZ3EM3xJbMOGT/D7UqU/TXmAvEg=",
            "timestamped signer must produce the pinned known-vector output"
        );
    }

    #[test]
    fn sign_timestamped_payload_differs_per_timestamp() {
        // A different timestamp must yield a different signature — this is what
        // makes a captured (timestamp, signature) pair non-replayable with a
        // swapped-in fresh timestamp.
        assert_ne!(
            sign_timestamped_payload(KNOWN_SECRET, TS, KNOWN_BODY),
            sign_timestamped_payload(KNOWN_SECRET, TS + 1, KNOWN_BODY),
            "the timestamp must be bound into the signature"
        );
    }

    #[test]
    fn sign_timestamped_payload_differs_from_body_only() {
        // Regression guard for #2485: the timestamped signature must NOT equal
        // the old body-only signature, or a legacy replayable delivery would
        // still validate.
        assert_ne!(
            sign_timestamped_payload(KNOWN_SECRET, TS, KNOWN_BODY),
            sign_payload(KNOWN_SECRET, KNOWN_BODY),
            "timestamped and body-only signatures must diverge"
        );
    }

    // ------------------------------------------------------------------
    // Webhook body shape (issue #2532): the body must carry delivery_id
    // (gap D) and published_version (gap A) so the receiver can correlate
    // events and dashboards can join a publish → revalidate trace. These
    // tests fail on `dev` (the body was only {screen, event}).
    // ------------------------------------------------------------------

    #[test]
    fn build_webhook_body_carries_delivery_id_and_version() {
        let delivery_id = Uuid::from_u128(0x1234_5678_9abc_def0_1122_3344_5566_7788);
        let bytes = build_webhook_body("reality/listing-detail", "published", Some(7), delivery_id);
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("body is valid JSON");

        assert_eq!(v["screen"], "reality/listing-detail");
        assert_eq!(v["event"], "published");
        assert_eq!(v["published_version"], 7);
        assert_eq!(v["delivery_id"], delivery_id.to_string());
    }

    #[test]
    fn build_webhook_body_null_version_for_kills() {
        let delivery_id = Uuid::nil();
        let bytes = build_webhook_body("home", "killed", None, delivery_id);
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("body is valid JSON");

        assert!(
            v["published_version"].is_null(),
            "kill/unkill carry no version — must serialise as null"
        );
        assert_eq!(v["delivery_id"], delivery_id.to_string());
    }
}

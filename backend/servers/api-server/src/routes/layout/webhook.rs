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
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

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

/// Notify an external webhook endpoint that a layout change occurred.
///
/// Reads `LAYOUT_WEBHOOK_URL` and `LAYOUT_WEBHOOK_SECRET` from the environment
/// at call time (no startup plumbing needed). When either variable is absent or
/// empty the call is a no-op (logged at `debug!` only).
///
/// On success the notification is fire-and-forget (`tokio::spawn`): a non-2xx
/// response or a network error is logged as `warn!` but does not propagate to
/// the caller.
///
/// # Parameters
/// - `screen` — the layout screen identifier (e.g. `"home"`)
/// - `event`  — one of `"published"`, `"rolled_back"`, `"killed"`, `"unkilled"`
pub fn notify_layout_change(screen: &str, event: &'static str) {
    let url = match std::env::var("LAYOUT_WEBHOOK_URL")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(u) => u,
        None => {
            tracing::debug!(
                screen = %screen,
                event = %event,
                "LAYOUT_WEBHOOK_URL not set — skipping layout-change notification"
            );
            return;
        }
    };

    let secret = match std::env::var("LAYOUT_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(s) => s,
        None => {
            tracing::debug!(
                screen = %screen,
                event = %event,
                "LAYOUT_WEBHOOK_SECRET not set — skipping layout-change notification"
            );
            return;
        }
    };

    // Owned copies so the spawned task is `'static`.
    let screen = screen.to_owned();

    let body = serde_json::json!({ "screen": screen, "event": event })
        .to_string()
        .into_bytes();

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
                return;
            }
        };

        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .header(TIMESTAMP_HEADER, timestamp.to_string())
            .header(SIGNATURE_HEADER, &signature)
            .body(body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!(
                    screen = %screen,
                    event = %event,
                    status = %resp.status(),
                    "layout-change webhook delivered"
                );
            }
            Ok(resp) => {
                tracing::warn!(
                    screen = %screen,
                    event = %event,
                    status = %resp.status(),
                    "layout-change webhook returned non-2xx"
                );
            }
            Err(e) => {
                tracing::warn!(
                    screen = %screen,
                    event = %event,
                    error = %e,
                    "layout-change webhook delivery failed"
                );
            }
        }
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
}

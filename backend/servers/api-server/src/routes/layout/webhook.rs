//! Outbound layout-change webhook notifier.
//!
//! When `LAYOUT_WEBHOOK_URL` and `LAYOUT_WEBHOOK_SECRET` are both set (and
//! non-empty), every successful `publish`, `rollback`, `kill`, or `unkill`
//! operation fires a signed POST to the configured URL so downstream consumers
//! (CDN invalidators, caches, notification services) can react immediately.
//!
//! ## Signature format
//!
//! The outbound signature is placed in the `X-Webhook-Signature` header as:
//!
//! ```text
//! sha256=<base64(HMAC-SHA256(LAYOUT_WEBHOOK_SECRET, body))>
//! ```
//!
//! This matches the exact format used by the inbound portal-webhook receivers
//! (`routes/portal_webhooks.rs`), enabling the same `verify_webhook_signature`
//! helper to verify outbound deliveries on the receiving end.
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

/// Sign `body` with `secret` using HMAC-SHA256 and return the header value.
///
/// The returned string has the form `sha256=<base64-digest>`, matching the
/// format the inbound `verify_webhook_signature` helper in
/// `routes/portal_webhooks.rs` expects:
///
/// ```text
/// sha256=<base64(HMAC-SHA256(secret, body))>
/// ```
///
/// This is a pure function with no I/O; it is unit-tested with a hardcoded
/// known vector below.
pub fn sign_payload(secret: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(body);
    format!("sha256={}", BASE64.encode(mac.finalize().into_bytes()))
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

    let signature = sign_payload(&secret, &body);

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
}

//! Inbound portal/listing-site webhook receiver (public, signature-verified).
//!
//! Split out of the former monolithic `webhook.rs` (behaviour preserved).

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use common::errors::ErrorResponse;

use crate::routes::integrations::install::ConnectionIdPath;
use crate::state::AppState;

// ==================== Inbound: Portal Webhook ====================

/// Maximum accepted skew (seconds) between an inbound portal webhook's signed
/// timestamp and the receiver's clock. Mirrors the Stripe receiver's 5-minute
/// replay-protection window ([`crate::services::stripe::DEFAULT_SIGNATURE_TOLERANCE_SECS`]).
const PORTAL_WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Why an inbound portal webhook was rejected. Every variant fails closed; the
/// handler maps all of them to `401 INVALID_SIGNATURE` (a single opaque status
/// so the reason is not leaked to the caller) while logging the discriminant.
#[derive(Debug, PartialEq, Eq)]
enum PortalWebhookError {
    /// `X-Webhook-Timestamp` was absent or not an integer unix-seconds value.
    MalformedTimestamp,
    /// The signed timestamp is outside the tolerance window — a stale capture
    /// or an absurd future-dated delivery (replay defense, gap 83-3).
    StaleTimestamp,
    /// `X-Webhook-Signature` was absent or did not match the HMAC over the
    /// `"{timestamp}.{body}"` signed payload.
    BadSignature,
}

/// Verify an inbound portal webhook's authenticity **and** freshness.
///
/// *Authenticity*: `X-Webhook-Signature` is the hex-encoded HMAC-SHA256
/// (optionally `sha256=`-prefixed) over the signed payload `"{timestamp}.{body}"`
/// keyed by the shared `PORTAL_WEBHOOK_SECRET`.
///
/// NOTE: this **inbound** contract signs `"{timestamp}.{body}"`. It is
/// deliberately *not* the same as the receiver's own **outbound** test-delivery
/// signer ([`test_webhook`]), which signs the bare body with no timestamp — do
/// not use the outbound signer as a reference for this inbound format (the two
/// diverged when gap 83-3 folded the timestamp into the inbound signature).
/// The timestamp is bound *into* the signed material rather than trusted as a
/// bare header — this is what makes the freshness check meaningful: an attacker
/// replaying a captured delivery cannot swap in a fresh timestamp without
/// invalidating the signature.
///
/// *Freshness* (gap 83-3): the `X-Webhook-Timestamp` header (unix seconds) must
/// be within `tolerance_secs` of `now_unix` in either direction, rejecting both
/// stale replays and future-dated deliveries. HMAC alone proves authenticity
/// but not freshness.
///
/// `now_unix`/`tolerance_secs` are injected so the window is unit-testable
/// without wall-clock dependence, matching
/// [`crate::services::stripe::verify_signature`]. The freshness + constant-time
/// HMAC check is delegated to the shared
/// [`integrations::verify_timestamped_signature`] helper (the 2026-07 audit R4
/// consolidation) so every `"{ts}.{body}"` hex receiver runs one reviewed
/// implementation; this wrapper only adapts header parsing and the opaque
/// error taxonomy.
fn verify_portal_webhook(
    secret: &str,
    body: &str,
    timestamp_header: Option<&str>,
    signature_header: Option<&str>,
    now_unix: i64,
    tolerance_secs: i64,
) -> Result<(), PortalWebhookError> {
    // Defense-in-depth: the shared helper computes a valid HMAC even for an empty
    // key, so an unset secret must be rejected here too and not only in the
    // handler's up-front `500 CONFIG_ERROR` guard — otherwise a caller signing
    // with the empty key would verify. (The helper also rejects it, but keeping
    // the guard preserves the original error ordering pinned by the unit tests.)
    if secret.is_empty() {
        return Err(PortalWebhookError::BadSignature);
    }

    let timestamp: i64 = timestamp_header
        .and_then(|t| t.trim().parse::<i64>().ok())
        .ok_or(PortalWebhookError::MalformedTimestamp)?;

    let signature = signature_header.ok_or(PortalWebhookError::BadSignature)?;

    // Freshness + authenticity over "{timestamp}.{raw_body}" — same construction
    // as the Stripe receiver — via the shared helper (overflow-proof window,
    // constant-time compare). Map its opaque error variants onto this receiver's.
    match integrations::verify_timestamped_signature(
        secret,
        timestamp,
        body,
        signature,
        now_unix,
        tolerance_secs,
    ) {
        Ok(()) => Ok(()),
        Err(integrations::TimestampedSignatureError::StaleTimestamp) => {
            Err(PortalWebhookError::StaleTimestamp)
        }
        Err(integrations::TimestampedSignatureError::BadSignature) => {
            Err(PortalWebhookError::BadSignature)
        }
    }
}

/// Handle incoming portal webhook (public endpoint, no session auth).
///
/// The endpoint has no request principal — external portals POST here using the
/// URL handed to them at connection time. Authenticity is instead established by
/// verifying the `X-Webhook-Signature` HMAC-SHA256 over the signed payload
/// `"{X-Webhook-Timestamp}.{raw_body}"` against the shared `PORTAL_WEBHOOK_SECRET`
/// **before** the payload is parsed or acted on, exactly like the sibling Airbnb
/// ([`handle_airbnb_webhook`]) and Stripe ([`handle_payment_webhook`]) receivers.
///
/// Replay/freshness (gap 83-3): HMAC alone proves authenticity but not freshness
/// — a captured valid delivery could otherwise be replayed indefinitely. The
/// signed `X-Webhook-Timestamp` is checked against a
/// [`PORTAL_WEBHOOK_TOLERANCE_SECS`] window (rejecting stale/future deliveries),
/// and because the timestamp is folded into the HMAC input it cannot be forged
/// independently of the signature.
///
/// The receiver fails closed: an unset secret is a `500 CONFIG_ERROR`, and a
/// missing/malformed timestamp, a stale/future timestamp, or an invalid/absent
/// signature are all `401 INVALID_SIGNATURE` — an unverified or replayed payload
/// is never processed.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/webhooks/portal/{connection_id}",
    params(
        ConnectionIdPath,
        ("X-Webhook-Timestamp" = String, Header,
         description = "Unix-seconds timestamp of the delivery. Signed as part of \"{timestamp}.{body}\"; deliveries whose timestamp is outside ±300s of server time are rejected 401 (replay defense, gap 83-3)."),
        ("X-Webhook-Signature" = String, Header,
         description = "Hex HMAC-SHA256 (optionally `sha256=`-prefixed) over \"{X-Webhook-Timestamp}.{raw_body}\" keyed by PORTAL_WEBHOOK_SECRET."),
    ),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 400, description = "Invalid webhook"),
        (status = 401, description = "Invalid signature"),
        (status = 404, description = "Connection not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations - Portals"
)]
pub async fn handle_portal_webhook(
    State(state): State<AppState>,
    Path(path): Path<ConnectionIdPath>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!(
        connection_id = %path.connection_id,
        "Received portal webhook"
    );

    // Fail closed when the signing secret is not configured — never accept an
    // unverified webhook.
    let secret = state.portal_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("PORTAL_WEBHOOK_SECRET is not configured — refusing portal webhook");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "CONFIG_ERROR",
                "Portal webhook signature verification is not configured",
            )),
        ));
    }

    // Verify the HMAC signature AND timestamp freshness over the raw body BEFORE
    // parsing or acting. HMAC proves authenticity; the signed `X-Webhook-Timestamp`
    // closes the replay window (gap 83-3) — a captured delivery can no longer be
    // replayed once its timestamp falls outside the tolerance window, and the
    // timestamp cannot be refreshed without invalidating the signature.
    let timestamp = headers
        .get("X-Webhook-Timestamp")
        .and_then(|v| v.to_str().ok());
    let signature = headers
        .get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok());

    let now_unix = chrono::Utc::now().timestamp();
    if let Err(reason) = verify_portal_webhook(
        secret,
        &body,
        timestamp,
        signature,
        now_unix,
        PORTAL_WEBHOOK_TOLERANCE_SECS,
    ) {
        tracing::warn!(
            connection_id = %path.connection_id,
            ?reason,
            "Portal webhook rejected (signature/freshness)"
        );
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature or freshness verification failed",
            )),
        ));
    }

    let _: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse webhook body");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid JSON body")),
        )
    })?;

    // FOLLOW-UP (issue #2330, originally #2196): the signed-timestamp window
    // bounds the replay *duration* (±PORTAL_WEBHOOK_TOLERANCE_SECS) but NOT the
    // *count* — within the tolerance a captured valid delivery can still be
    // replayed N times. Harmless today because this receiver only parses the
    // body and has no side effects, but BEFORE it is ever wired to mutate state
    // (persist views/inquiries like the per-portal receiver), add an
    // idempotency/nonce dedup keyed on a delivery id (e.g. an `X-Webhook-ID`
    // header folded into the signed payload, or a payload `event_id`), reusing
    // the Airbnb dedup-ledger pattern (`airbnb_dedup_key` +
    // `record_airbnb_webhook_event`) with retention ≥ the tolerance window.
    Ok(StatusCode::OK)
}

// ==================== Portal webhook auth + freshness tests ====================
//
// Regression cover for two properties of the connection-scoped portal webhook
// receiver:
//   1. authenticity — the `X-Webhook-Signature` HMAC must be verified (the
//      latent auth-bypass where any unsigned caller was accepted with 200);
//   2. freshness (gap 83-3) — a signed `X-Webhook-Timestamp` must fall inside a
//      tolerance window so a captured valid delivery cannot be replayed, and the
//      timestamp is folded into the HMAC so it cannot be refreshed independently.
//
// These pin the contract of `verify_portal_webhook`, the guard the handler runs
// before parsing/acting on the payload.
#[cfg(test)]
mod portal_webhook_signature_tests {
    use super::{verify_portal_webhook, PortalWebhookError, PORTAL_WEBHOOK_TOLERANCE_SECS};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    const SECRET: &str = "portal_webhook_secret";
    const BODY: &str = r#"{"event":"listing.viewed","external_id":"abc123"}"#;
    // Fixed reference "now" so the window is deterministic and wall-clock-free.
    const NOW: i64 = 1_700_000_000;

    /// Sign `"{timestamp}.{body}"` — the same signed-payload construction the
    /// handler verifies (mirrors the Stripe receiver).
    fn sign_at(secret: &str, ts: i64, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(format!("{ts}.{body}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    // ---- authenticity ----

    #[test]
    fn accepts_valid_hex_signature() {
        let sig = sign_at(SECRET, NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn accepts_valid_signature_with_sha256_prefix() {
        let sig = format!("sha256={}", sign_at(SECRET, NOW, BODY));
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_missing_signature_header() {
        // The original auth-bypass bug: no signature header was still accepted.
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                None,
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_wrong_signature() {
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some("deadbeef"),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_signature_for_tampered_body() {
        let sig = sign_at(SECRET, NOW, BODY);
        let tampered = r#"{"event":"listing.viewed","external_id":"evil999"}"#;
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                tampered,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_signature_under_wrong_secret() {
        let sig = sign_at("attacker_secret", NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    #[test]
    fn rejects_when_secret_empty() {
        // Defense-in-depth: even a syntactically valid signature cannot pass
        // when the server secret is unset (the handler also fails closed).
        let sig = sign_at("", NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                "",
                BODY,
                Some(&NOW.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }

    // ---- freshness / replay protection (gap 83-3) ----

    #[test]
    fn rejects_stale_timestamp() {
        // A correctly-signed delivery captured and replayed once its signed
        // timestamp is older than the tolerance window must be rejected — HMAC
        // is still valid but the request is no longer fresh.
        let stale = NOW - PORTAL_WEBHOOK_TOLERANCE_SECS - 1;
        let sig = sign_at(SECRET, stale, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&stale.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::StaleTimestamp)
        );
    }

    #[test]
    fn accepts_fresh_timestamp_within_window() {
        // A delivery whose signed timestamp is inside the window is accepted.
        let fresh = NOW - (PORTAL_WEBHOOK_TOLERANCE_SECS - 1);
        let sig = sign_at(SECRET, fresh, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&fresh.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn accepts_timestamp_at_exact_tolerance_boundary() {
        // The freshness gate is inclusive (`|now - ts| == tolerance` passes).
        // Pin both edges so an off-by-one refactor (`<=` -> `<`) is caught.
        for boundary in [
            NOW - PORTAL_WEBHOOK_TOLERANCE_SECS,
            NOW + PORTAL_WEBHOOK_TOLERANCE_SECS,
        ] {
            let sig = sign_at(SECRET, boundary, BODY);
            assert_eq!(
                verify_portal_webhook(
                    SECRET,
                    BODY,
                    Some(&boundary.to_string()),
                    Some(&sig),
                    NOW,
                    PORTAL_WEBHOOK_TOLERANCE_SECS,
                ),
                Ok(()),
                "a delivery exactly {PORTAL_WEBHOOK_TOLERANCE_SECS}s away must still be fresh"
            );
        }
    }

    #[test]
    fn rejects_adversarial_overflow_timestamp_without_panicking() {
        // Regression for issue #2330: a parseable but adversarial timestamp near
        // i64::MIN/MAX must be rejected as stale WITHOUT panicking under
        // overflow-checks builds (the old `(now - ts).abs()` overflowed).
        for ts in [i64::MIN, i64::MIN + 1, i64::MAX, i64::MAX - 1] {
            let sig = sign_at(SECRET, ts, BODY);
            assert_eq!(
                verify_portal_webhook(
                    SECRET,
                    BODY,
                    Some(&ts.to_string()),
                    Some(&sig),
                    NOW,
                    PORTAL_WEBHOOK_TOLERANCE_SECS,
                ),
                Err(PortalWebhookError::StaleTimestamp),
                "adversarial timestamp {ts} must be rejected as stale, not panic"
            );
        }
    }

    #[test]
    fn rejects_future_timestamp_beyond_tolerance() {
        // An absurd future-dated delivery is rejected in the same way.
        let future = NOW + PORTAL_WEBHOOK_TOLERANCE_SECS + 1;
        let sig = sign_at(SECRET, future, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&future.to_string()),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::StaleTimestamp)
        );
    }

    #[test]
    fn rejects_missing_timestamp_header() {
        // Without a timestamp there is no freshness to check — fail closed so an
        // attacker cannot strip the header to defeat replay protection.
        let sig = sign_at(SECRET, NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                None,
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::MalformedTimestamp)
        );
    }

    #[test]
    fn rejects_non_numeric_timestamp() {
        let sig = sign_at(SECRET, NOW, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some("not-a-number"),
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::MalformedTimestamp)
        );
    }

    #[test]
    fn rejects_timestamp_swap_on_captured_signature() {
        // The core replay defense: an attacker captures a valid (timestamp, sig)
        // pair, then presents a *fresh* timestamp header to beat the window while
        // reusing the old signature. Because the timestamp is folded into the
        // HMAC, the signature no longer matches and the swap is rejected.
        let old_ts = NOW - 10_000; // well outside the window
        let sig = sign_at(SECRET, old_ts, BODY);
        assert_eq!(
            verify_portal_webhook(
                SECRET,
                BODY,
                Some(&NOW.to_string()), // fresh header, but not what was signed
                Some(&sig),
                NOW,
                PORTAL_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(PortalWebhookError::BadSignature)
        );
    }
}

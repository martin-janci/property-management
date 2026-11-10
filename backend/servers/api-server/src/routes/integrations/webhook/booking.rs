//! Inbound Booking.com OTA push-notification receiver.
//!
//! Split out of the former monolithic `webhook.rs` (behaviour preserved).

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use sha2::Sha256;

use common::errors::ErrorResponse;
use integrations::booking::ota_xml;

use crate::state::AppState;

// ==================== Inbound: Booking.com Push ====================

/// Maximum accepted skew (seconds) between a Booking.com push's signed
/// timestamp and the receiver's clock. Mirrors the portal/Stripe receivers'
/// 5-minute replay-protection window.
const BOOKING_WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// In-process, TTL-bounded replay ledger for the Booking.com push receiver
/// (audit R1 dedup leg).
///
/// The signed-timestamp window bounds the replay *duration* (±tolerance) but not
/// the *count*: within the window a captured valid delivery could still be
/// replayed N times. Booking.com guarantees at-least-once delivery, so a genuine
/// retry is byte-identical and yields the same dedup key. This guard suppresses
/// both — a duplicate key seen inside the retention window is treated as
/// already-processed and acked idempotently.
///
/// It is deliberately in-process (no migration): the receiver has no state
/// mutation today (audit F1 — it only parses OTA XML and acks), so a persistent,
/// cross-replica ledger (the `airbnb_webhook_events` pattern) is only *required*
/// before this receiver is wired to persist reservations (audit R1 blocking
/// condition). Retention is pinned to the freshness window so an entry never
/// outlives the timestamp tolerance that would let the same delivery back in
/// anyway, which also bounds the map size.
///
/// The decision logic ([`BookingReplayGuard::check_and_record`]) takes an
/// injected `now`, so it is unit-testable without wall-clock dependence.
#[derive(Default)]
struct BookingReplayGuard {
    seen: std::sync::Mutex<std::collections::HashMap<String, i64>>,
}

impl BookingReplayGuard {
    /// Record `key` as seen at `now_unix`. Returns `true` when the key is
    /// first-seen (caller should process), `false` when it is a duplicate still
    /// inside the `retention_secs` window (caller should ack idempotently).
    /// Expired entries are opportunistically evicted on each call so the map
    /// cannot grow without bound.
    fn check_and_record(&self, key: &str, now_unix: i64, retention_secs: i64) -> bool {
        let mut seen = match self.seen.lock() {
            Ok(g) => g,
            // A poisoned lock must not wedge the receiver. The dedup layer is
            // defense-in-depth (signature + freshness already gate authenticity),
            // so recover the guard and continue rather than fail the request.
            Err(p) => p.into_inner(),
        };
        // Evict entries older than the retention window (overflow-proof).
        seen.retain(|_, &mut ts| {
            now_unix
                .checked_sub(ts)
                .map(|skew| skew <= retention_secs)
                .unwrap_or(false)
        });
        if seen.contains_key(key) {
            return false;
        }
        seen.insert(key.to_string(), now_unix);
        true
    }
}

/// Process-global Booking.com replay ledger (see [`BookingReplayGuard`]).
static BOOKING_REPLAY_GUARD: std::sync::OnceLock<BookingReplayGuard> = std::sync::OnceLock::new();

fn booking_replay_guard() -> &'static BookingReplayGuard {
    BOOKING_REPLAY_GUARD.get_or_init(BookingReplayGuard::default)
}

/// Compute the idempotency/dedup key for an inbound Booking.com push delivery.
///
/// Booking.com redelivers the byte-identical signed body on an at-least-once
/// retry, so a SHA-256 over the raw body is a stable key across redeliveries
/// while differing for any distinct notification. Reservation ids parsed from
/// the OTA payload are folded in first (for observability and stability against
/// insignificant reordering), with the body hash as the collision-proof tail.
/// Mirrors the Airbnb synthetic-key derivation ([`airbnb_dedup_key`]).
fn booking_dedup_key(body: &str, res_ids: &[String]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    for id in res_ids {
        hasher.update(id.as_bytes());
        hasher.update(b"|");
    }
    hasher.update(body.as_bytes());
    format!("booking:{}", hex::encode(hasher.finalize()))
}

/// Handle Booking.com push notification.
///
/// Fail-closed inbound receiver (audit F1/R1). The endpoint has no request
/// principal — Booking.com POSTs OTA `OTA_HotelResNotifRQ` XML to the URL handed
/// to them at connection time — so authenticity is established by verifying an
/// HMAC-SHA256 `X-Booking-Signature` over `"{X-Booking-Timestamp}.{raw_body}"`
/// against the shared `BOOKING_WEBHOOK_SECRET`, exactly like the sibling portal
/// ([`handle_portal_webhook`]) / Airbnb ([`handle_airbnb_webhook`]) / Stripe
/// ([`handle_payment_webhook`]) receivers, **before** the payload is parsed.
///
/// The receiver fails closed: an unset secret is `503 NOT_CONFIGURED`; a
/// missing/malformed timestamp, a stale/future timestamp, or an invalid/absent
/// signature are all `401 INVALID_SIGNATURE`. Timestamp freshness closes the
/// replay window and an in-process dedup ledger ([`BookingReplayGuard`])
/// suppresses at-least-once redeliveries within that window; both are idempotent
/// (a duplicate is acked with the same OTA `<Success/>`).
///
/// Raw `Bytes` are taken so the HMAC is computed over the exact bytes signed —
/// UTF-8 decoding happens after signature verification.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/booking/push",
    request_body(content = String, content_type = "application/xml", description = "Raw signed OTA_HotelResNotifRQ webhook payload (HMAC-SHA256 verified over \"{X-Booking-Timestamp}.{body}\" before parsing)"),
    params(
        ("X-Booking-Timestamp" = String, Header,
         description = "Unix-seconds timestamp of the delivery. Signed as part of \"{timestamp}.{body}\"; deliveries outside ±300s of server time are rejected 401 (replay defense)."),
        ("X-Booking-Signature" = String, Header,
         description = "Hex HMAC-SHA256 (optionally `sha256=`-prefixed) over \"{X-Booking-Timestamp}.{raw_body}\" keyed by BOOKING_WEBHOOK_SECRET."),
    ),
    responses(
        (status = 200, description = "Push notification processed"),
        (status = 400, description = "Invalid notification"),
        (status = 401, description = "Invalid signature or stale timestamp"),
        (status = 500, description = "Internal server error"),
        (status = 503, description = "Webhook signature verification not configured")
    ),
    tag = "Integrations - Booking.com"
)]
pub async fn booking_push_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, (StatusCode, Json<ErrorResponse>)> {
    tracing::info!("Received Booking.com push notification");

    // Fail closed when the signing secret is not configured — never accept an
    // unverified OTA push (audit F1/R1).
    let secret = state.booking_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("BOOKING_WEBHOOK_SECRET is not configured — refusing Booking.com push");
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Booking.com push signature verification is not configured",
            )),
        ));
    }

    // Decode for signing over the exact received bytes (HMAC is over the raw
    // body; a non-UTF-8 body can never be a valid OTA XML push anyway).
    let body_str = std::str::from_utf8(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ENCODING",
                "Request body is not valid UTF-8",
            )),
        )
    })?;

    // Verify HMAC signature AND timestamp freshness over the raw body BEFORE
    // parsing or acting. A missing/malformed timestamp fails closed (an attacker
    // cannot strip the header to defeat replay protection).
    let timestamp = headers
        .get("X-Booking-Timestamp")
        .and_then(|v| v.to_str().ok())
        .and_then(|t| t.trim().parse::<i64>().ok());
    let signature = headers
        .get("X-Booking-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let now_unix = chrono::Utc::now().timestamp();
    let verified = match timestamp {
        Some(ts) => integrations::verify_timestamped_signature(
            secret,
            ts,
            body_str,
            signature,
            now_unix,
            BOOKING_WEBHOOK_TOLERANCE_SECS,
        ),
        None => Err(integrations::TimestampedSignatureError::BadSignature),
    };
    if let Err(reason) = verified {
        tracing::warn!(?reason, "Booking.com push rejected (signature/freshness)");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature or freshness verification failed",
            )),
        ));
    }

    // Parse only AFTER authenticity + freshness are established.
    let notifications = ota_xml::parse_res_notif_rq_raw(body_str).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse Booking.com notification");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PARSE_ERROR",
                "Invalid notification format",
            )),
        )
    })?;

    // Dedup / replay-count defense: suppress an at-least-once redelivery within
    // the freshness window. Harmless-but-first-class today (no side effects);
    // becomes load-bearing the moment this receiver mutates state (audit R1).
    let res_ids: Vec<String> = notifications.iter().map(|(id, _, _)| id.clone()).collect();
    let dedup_key = booking_dedup_key(body_str, &res_ids);
    let first_seen = booking_replay_guard().check_and_record(
        &dedup_key,
        now_unix,
        BOOKING_WEBHOOK_TOLERANCE_SECS,
    );
    if first_seen {
        tracing::debug!(
            dedup_key = %dedup_key,
            reservation_count = notifications.len(),
            "Booking.com push: first-seen delivery recorded in replay guard"
        );
    } else {
        // Idempotent ack — no reprocessing, same OTA <Success/> response.
        tracing::info!(
            dedup_key = %dedup_key,
            "Booking.com push: duplicate delivery suppressed by replay guard, acking idempotently"
        );
    }

    let response_xml = ota_xml::build_res_notif_rs(true, None).map_err(|e| {
        tracing::error!(error = %e, "Failed to build Booking.com response");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "RESPONSE_ERROR",
                "Failed to build response",
            )),
        )
    })?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/xml")
        .body(Body::from(response_xml))
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to build response");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "RESPONSE_ERROR",
                    "Failed to build response",
                )),
            )
        })
}

// ==================== Booking.com push: auth + freshness + dedup tests ====================
//
// Regression cover for the audit F1/R1 fix: the Booking.com push receiver used
// to perform NO signature, timestamp, or dedup check — any caller could POST OTA
// XML and receive a `<Success/>` ack. These pin the three properties it now
// enforces before parsing/acting:
//   1. authenticity + freshness over "{X-Booking-Timestamp}.{body}" (shared
//      `verify_timestamped_signature` at BOOKING_WEBHOOK_TOLERANCE_SECS);
//   2. a stable dedup key across at-least-once redeliveries;
//   3. an in-process replay guard that suppresses a duplicate within the window.
#[cfg(test)]
mod booking_webhook_tests {
    use super::{booking_dedup_key, BookingReplayGuard, BOOKING_WEBHOOK_TOLERANCE_SECS};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    const SECRET: &str = "booking_webhook_secret";
    const BODY: &str = r#"<OTA_HotelResNotifRQ><HotelReservation ResStatus="Commit"><HotelReservationID ResID_Value="R1"/></HotelReservation></OTA_HotelResNotifRQ>"#;
    const NOW: i64 = 1_700_000_000;

    fn sign_ts(secret: &str, ts: i64, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(format!("{ts}.{body}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    // ---- authenticity + freshness (the F1 auth-bypass fix) ----

    #[test]
    fn accepts_valid_signed_and_fresh_delivery() {
        let sig = sign_ts(SECRET, NOW, BODY);
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                NOW,
                BODY,
                &sig,
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_forged_or_unsigned_delivery() {
        // The pre-fix bypass: an arbitrary/forged signature must not verify.
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                NOW,
                BODY,
                "deadbeef",
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(integrations::TimestampedSignatureError::BadSignature)
        );
    }

    #[test]
    fn rejects_stale_replay_and_timestamp_swap() {
        // Stale capture outside the window.
        let stale = NOW - BOOKING_WEBHOOK_TOLERANCE_SECS - 1;
        let stale_sig = sign_ts(SECRET, stale, BODY);
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                stale,
                BODY,
                &stale_sig,
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(integrations::TimestampedSignatureError::StaleTimestamp)
        );
        // Captured old signature re-presented with a fresh timestamp: the
        // folded timestamp invalidates the HMAC.
        let old_sig = sign_ts(SECRET, NOW - 10_000, BODY);
        assert_eq!(
            integrations::verify_timestamped_signature(
                SECRET,
                NOW,
                BODY,
                &old_sig,
                NOW,
                BOOKING_WEBHOOK_TOLERANCE_SECS,
            ),
            Err(integrations::TimestampedSignatureError::BadSignature)
        );
    }

    // ---- dedup key ----

    #[test]
    fn dedup_key_is_stable_across_byte_identical_redelivery() {
        let a = booking_dedup_key(BODY, &["R1".to_string()]);
        let b = booking_dedup_key(BODY, &["R1".to_string()]);
        assert_eq!(a, b);
        assert!(a.starts_with("booking:"), "unexpected key: {a}");
    }

    #[test]
    fn dedup_key_differs_for_distinct_deliveries() {
        let a = booking_dedup_key(BODY, &["R1".to_string()]);
        let other_body = BODY.replace("R1", "R2");
        let b = booking_dedup_key(&other_body, &["R2".to_string()]);
        assert_ne!(a, b);
    }

    // ---- in-process replay guard ----

    #[test]
    fn replay_guard_first_seen_then_duplicate_suppressed() {
        let guard = BookingReplayGuard::default();
        let key = "booking:abc";
        // First delivery is processed; an immediate redelivery is suppressed.
        assert!(guard.check_and_record(key, NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
        assert!(!guard.check_and_record(key, NOW + 1, BOOKING_WEBHOOK_TOLERANCE_SECS));
    }

    #[test]
    fn replay_guard_reaccepts_after_retention_window() {
        let guard = BookingReplayGuard::default();
        let key = "booking:abc";
        assert!(guard.check_and_record(key, NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
        // Once the entry ages past the retention window it is evicted; a delivery
        // that far in the future would itself be stale at the signature gate, so
        // re-accepting here does not widen the replay window.
        assert!(guard.check_and_record(
            key,
            NOW + BOOKING_WEBHOOK_TOLERANCE_SECS + 1,
            BOOKING_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn replay_guard_distinct_keys_are_independent() {
        let guard = BookingReplayGuard::default();
        assert!(guard.check_and_record("booking:a", NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
        assert!(guard.check_and_record("booking:b", NOW, BOOKING_WEBHOOK_TOLERANCE_SECS));
    }
}

//! Inbound Airbnb webhook receiver (gap 83-1).
//!
//! Split out of the former monolithic `webhook.rs` (behaviour preserved).

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use sha2::Sha256;

use common::errors::ErrorResponse;
use db::models::infrastructure::{job_type, queue, CreateBackgroundJob};
use integrations::{AirbnbClient, AirbnbWebhookEventType};

use crate::state::AppState;

// ==================== Gap 83-1: Airbnb Inbound Webhook ====================

/// Compute the idempotency key for an inbound Airbnb webhook delivery.
///
/// Prefers the Airbnb-assigned `event_id`. When absent (older webhook schema
/// versions omit it), derives a deterministic synthetic key from the stable,
/// HMAC-signed fields of the delivery so that an at-least-once redelivery of
/// the same payload maps to the same key and is suppressed by the dedup
/// ledger. The `synthetic:` prefix keeps these keys from ever colliding with
/// a real Airbnb `event_id`.
fn airbnb_dedup_key(event: &integrations::AirbnbWebhookEvent) -> String {
    if let Some(event_id) = event.event_id.as_deref() {
        return event_id.to_string();
    }

    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", event.event_type).as_bytes());
    hasher.update(b"|");
    hasher.update(event.listing_id.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(event.confirmation_code.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(event.timestamp.to_rfc3339().as_bytes());
    format!("synthetic:{}", hex::encode(hasher.finalize()))
}

/// Maximum accepted skew (seconds) between an inbound Airbnb webhook's signed
/// timestamp and the receiver's clock, when the delivery carries an
/// `X-Airbnb-Timestamp` header. Mirrors the portal/Stripe receivers' 5-minute
/// replay-protection window (audit R2).
const AIRBNB_WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Establish the authenticity (and, when timestamped, freshness) of an inbound
/// Airbnb delivery — the staged accept-both decision (audit R2), extracted so it
/// is unit-testable without wall-clock dependence (`now_unix` is injected).
///
/// * `X-Airbnb-Timestamp` **present** ⇒ verify the signature over
///   `"{timestamp}.{body}"` and require the timestamp within `tolerance_secs`
///   (shared [`integrations::verify_timestamped_signature`]). A present-but-
///   malformed timestamp fails closed, so an attacker cannot send a garbage
///   header to fall back to the no-freshness path.
/// * **absent** ⇒ accept a legacy body-only signature (migration window); replay
///   for those deliveries stays neutralized by the persistent dedup ledger.
///
/// Returns `true` when the delivery is authentic (and fresh, if timestamped).
fn verify_airbnb_delivery(
    secret: &str,
    timestamp_header: Option<&str>,
    signature: &str,
    body: &str,
    now_unix: i64,
    tolerance_secs: i64,
) -> bool {
    match timestamp_header {
        Some(ts_raw) => match ts_raw.trim().parse::<i64>() {
            Ok(ts) => integrations::verify_timestamped_signature(
                secret,
                ts,
                body,
                signature,
                now_unix,
                tolerance_secs,
            )
            .is_ok(),
            Err(_) => false,
        },
        None => AirbnbClient::verify_webhook_signature(signature, body, secret),
    }
}

/// Receive and dispatch an inbound Airbnb webhook event.
///
/// Airbnb signs every delivery with HMAC-SHA256 keyed by the shared secret from
/// `AIRBNB_WEBHOOK_SECRET`; the signature (`X-Airbnb-Signature`) is verified
/// before the payload is parsed to reject unauthenticated callers.
///
/// Replay freshness (audit R2, staged accept-both): when the delivery carries an
/// `X-Airbnb-Timestamp` header the signature is verified over the timestamped
/// payload `"{timestamp}.{body}"` (via the shared
/// [`integrations::verify_timestamped_signature`]) and the timestamp must fall
/// within [`AIRBNB_WEBHOOK_TOLERANCE_SECS`] of server time, giving the same
/// replay posture as the portal/Stripe receivers. Until Airbnb senders are
/// migrated to send that header, a legacy **body-only** signature is still
/// accepted (with a deprecation warning); replay for those deliveries remains
/// neutralized by the persistent dedup ledger in step 4. Once every sender sends
/// the header the legacy branch should be removed so freshness is mandatory.
///
/// Raw `Bytes` are used so the HMAC is computed over the exact bytes Airbnb
/// signed — UTF-8 decoding happens after signature verification.
#[utoipa::path(
    post,
    path = "/api/v1/integrations/airbnb/webhook",
    request_body(content = String, content_type = "application/json", description = "Airbnb webhook event payload"),
    params(
        ("X-Airbnb-Timestamp" = Option<String>, Header,
         description = "Optional unix-seconds timestamp. When present the signature covers \"{timestamp}.{body}\" and deliveries outside ±300s of server time are rejected 401 (replay defense, audit R2). Absent ⇒ legacy body-only signature (accepted during the accept-both rollout)."),
    ),
    responses(
        (status = 200, description = "Webhook processed"),
        (status = 401, description = "Invalid signature or stale timestamp"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Integrations - Airbnb"
)]
pub async fn handle_airbnb_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // 1. Load the shared secret from the cached AppState config (issue #711).
    //    The env var is read once at server startup; per-request env reads
    //    were a minor perf concern and made misconfiguration only visible
    //    once a real delivery arrived.
    let secret = state.airbnb_config.webhook_secret.as_str();
    if secret.is_empty() {
        tracing::error!("AIRBNB_WEBHOOK_SECRET is not configured");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Airbnb webhook secret is not configured",
            )),
        ));
    }

    // 2. Verify HMAC-SHA256 signature over raw bytes.
    let signature = headers
        .get("X-Airbnb-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let body_str = std::str::from_utf8(&body).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_ENCODING",
                "Request body is not valid UTF-8",
            )),
        )
    })?;

    // Staged accept-both (audit R2): when the delivery carries an
    // `X-Airbnb-Timestamp` header, enforce the same `"{timestamp}.{body}"`
    // freshness window as the portal/Stripe receivers (folding the timestamp
    // into the signed material so it cannot be swapped without invalidating the
    // signature). Absent header ⇒ legacy body-only signature is still accepted
    // during the migration; replay for those is already neutralized by the
    // dedup ledger in step 4.
    let ts_header = headers
        .get("X-Airbnb-Timestamp")
        .and_then(|v| v.to_str().ok());
    let now_unix = chrono::Utc::now().timestamp();
    let authentic = verify_airbnb_delivery(
        secret,
        ts_header,
        signature,
        body_str,
        now_unix,
        AIRBNB_WEBHOOK_TOLERANCE_SECS,
    );
    if authentic && ts_header.is_none() {
        tracing::warn!(
            "Airbnb webhook accepted with a legacy body-only signature and no \
             X-Airbnb-Timestamp — replay freshness is NOT in effect for this delivery \
             (dedup ledger still applies). Migrate this sender to sign \
             \"{{timestamp}}.{{body}}\" and send X-Airbnb-Timestamp (audit R2)."
        );
    }

    if !authentic {
        tracing::warn!("Airbnb webhook signature verification failed");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "INVALID_SIGNATURE",
                "Webhook signature verification failed",
            )),
        ));
    }

    // 3. Parse the event.
    let event = AirbnbClient::parse_webhook_event(body_str).map_err(|e| {
        tracing::warn!(error = %e, "Failed to parse Airbnb webhook event");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("PARSE_ERROR", "Invalid webhook payload")),
        )
    })?;

    // 4. Persistent deduplication (issue #711, bug-webhook-airbnb-dup-sync-jobs).
    //
    // Airbnb guarantees at-least-once delivery. Without persistent dedup,
    // duplicate ReservationCreated/Updated deliveries enqueue racing
    // SYNC_EXTERNAL jobs, and ReservationCancelled deliveries hammer the
    // booking-status guard. Migration 00169 created
    // `airbnb_webhook_events(event_id PRIMARY KEY, event_type, received_at)`;
    // we attempt to insert the delivery's dedup key and bail out with 200 on
    // conflict (idempotent acknowledgement, no side effects).
    //
    // The dedup key is the Airbnb-assigned `event_id` when present. Older
    // webhook schema versions omit it; in that case we derive a *synthetic*
    // key from the stable, signed fields of the delivery (event type +
    // listing id + confirmation code + timestamp). Airbnb redelivers the
    // byte-identical signed payload, so a redelivery yields the same
    // synthetic key and is suppressed — closing the previously-unguarded
    // event_id-absent path that could still double-enqueue SYNC_EXTERNAL.
    let dedup_key = airbnb_dedup_key(&event);
    let event_type_label = format!("{:?}", event.event_type);
    // PAP-170 (PAP-150): the dedup-ledger insert now lives in the repository
    // layer (`airbnb_webhook_events` is a global, tenant-less table) instead of
    // a raw `state.db` pool access here. `Ok(false)` ⇒ already seen ⇒ suppress.
    let recorded = state
        .rental_repo
        .record_airbnb_webhook_event(&dedup_key, &event_type_label)
        .await;

    match recorded {
        Ok(false) => {
            tracing::info!(
                dedup_key = %dedup_key,
                event_type = %event_type_label,
                "Airbnb webhook: duplicate delivery suppressed by dedup ledger"
            );
            return Ok(StatusCode::OK);
        }
        Ok(true) => {
            tracing::debug!(
                dedup_key = %dedup_key,
                event_type = %event_type_label,
                "Airbnb webhook: delivery recorded in dedup ledger"
            );
        }
        Err(e) => {
            // Best-effort: if the ledger insert fails (DB blip, table
            // not yet migrated on a stale env, etc.) we degrade to the
            // pre-#711 behaviour and process the event. Failing closed
            // here would let a transient DB issue silently drop real
            // reservation updates, which is worse than a rare double-
            // processing (downstream handlers are idempotent upserts).
            tracing::warn!(
                error = %e,
                dedup_key = %dedup_key,
                "Airbnb webhook: dedup ledger insert failed, processing anyway"
            );
        }
    }

    // 5. Dispatch by event type.
    match event.event_type {
        AirbnbWebhookEventType::ReservationCreated | AirbnbWebhookEventType::ReservationUpdated => {
            if let Some(listing_id) = &event.listing_id {
                match state
                    .rental_repo
                    .find_airbnb_connection_by_listing_id(listing_id)
                    .await
                {
                    Ok(Some(conn)) => {
                        let payload = serde_json::json!({
                            "org_id": conn.organization_id,
                            "connection_id": conn.id,
                            "sync_type": "reservations",
                            "trigger": "webhook",
                            "event_id": event.event_id,
                        });
                        // Idempotency note: at-least-once redeliveries are
                        // suppressed up-front by the dedup ledger in step 4
                        // (Airbnb event_id, or a synthetic key for legacy
                        // deliveries that omit it), so reaching this point
                        // means a first-seen delivery and the SYNC_EXTERNAL
                        // job is enqueued exactly once per delivery.
                        let job_data = CreateBackgroundJob {
                            job_type: job_type::SYNC_EXTERNAL.to_string(),
                            priority: Some(1),
                            payload,
                            scheduled_at: None,
                            queue: Some(queue::LOW_PRIORITY.to_string()),
                            max_attempts: Some(3),
                            org_id: Some(conn.organization_id),
                        };
                        if let Err(e) = state.background_job_repo.create(job_data, None).await {
                            tracing::error!(
                                error = %e,
                                listing_id = %listing_id,
                                event_type = ?event.event_type,
                                "Failed to enqueue Airbnb reservation sync job from webhook"
                            );
                        } else {
                            tracing::info!(
                                listing_id = %listing_id,
                                org_id = %conn.organization_id,
                                event_type = ?event.event_type,
                                "Airbnb webhook: enqueued reservation sync job"
                            );
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            listing_id = %listing_id,
                            "Airbnb webhook: no active connection found for listing, ignoring"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            listing_id = %listing_id,
                            "Airbnb webhook: DB error looking up connection"
                        );
                    }
                }
            }
        }
        AirbnbWebhookEventType::ReservationCancelled => {
            if let Some(code) = &event.confirmation_code {
                match state
                    .rental_repo
                    .find_booking_by_external_id("airbnb", code)
                    .await
                {
                    Ok(Some(booking)) => {
                        if booking.status == db::models::rental::booking_status::CANCELLED {
                            tracing::warn!(
                                booking_id = %booking.id,
                                confirmation_code = %code,
                                event_id = ?event.event_id,
                                "Airbnb webhook: ReservationCancelled for already-cancelled booking, likely duplicate delivery — ignoring"
                            );
                        } else {
                            let cancel_data = db::models::rental::UpdateBookingStatus {
                                status: db::models::rental::booking_status::CANCELLED.to_string(),
                                cancellation_reason: Some(
                                    "Cancelled via Airbnb webhook".to_string(),
                                ),
                            };
                            // PAP-141: key the status mutation to the booking's
                            // own organization (`update_booking_status_for_org`)
                            // rather than the bare id, so a forged/replayed
                            // webhook can never flip a booking the lookup did
                            // not legitimately resolve. `booking` was just found
                            // by (`platform`, `external_id`), so its
                            // `organization_id` is the authoritative owner.
                            match state
                                .rental_repo
                                .update_booking_status_for_org(
                                    booking.organization_id,
                                    booking.id,
                                    cancel_data,
                                )
                                .await
                            {
                                Ok(Some(_)) => {
                                    tracing::info!(
                                        booking_id = %booking.id,
                                        confirmation_code = %code,
                                        "Airbnb webhook: booking cancelled"
                                    );
                                }
                                Ok(None) => {
                                    tracing::warn!(
                                        booking_id = %booking.id,
                                        org_id = %booking.organization_id,
                                        confirmation_code = %code,
                                        "Airbnb webhook: booking not cancelled (org mismatch on status update)"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        booking_id = %booking.id,
                                        confirmation_code = %code,
                                        "Airbnb webhook: failed to cancel booking"
                                    );
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!(
                            confirmation_code = %code,
                            "Airbnb webhook: ReservationCancelled for unknown booking, ignoring"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            confirmation_code = %code,
                            "Airbnb webhook: DB error looking up booking for cancellation"
                        );
                    }
                }
            }
        }
        AirbnbWebhookEventType::ListingUpdated => {
            tracing::info!(listing_id = ?event.listing_id, "Airbnb webhook: ListingUpdated (not yet handled)");
        }
        AirbnbWebhookEventType::MessageReceived => {
            tracing::info!(listing_id = ?event.listing_id, "Airbnb webhook: MessageReceived (not yet handled)");
        }
        AirbnbWebhookEventType::ReviewReceived => {
            tracing::info!(listing_id = ?event.listing_id, "Airbnb webhook: ReviewReceived (not yet handled)");
        }
    }

    Ok(StatusCode::OK)
}

// ==================== Gap 83-1 Unit Tests ====================

#[cfg(test)]
mod airbnb_webhook_tests {
    use integrations::AirbnbClient;

    #[test]
    fn test_signature_valid() {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let secret = "test_secret_key";
        let body = r#"{"event_type":"reservation_created","listing_id":"123","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;

        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(AirbnbClient::verify_webhook_signature(
            &signature, body, secret
        ));
    }

    #[test]
    fn test_signature_invalid() {
        assert!(!AirbnbClient::verify_webhook_signature(
            "deadbeef",
            r#"{"event_type":"listing_updated","listing_id":"42","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#,
            "some_secret",
        ));
    }

    #[test]
    fn test_event_parse_valid() {
        let body = r#"{"event_type":"listing_updated","listing_id":"abc123","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
        let event = AirbnbClient::parse_webhook_event(body);
        assert!(event.is_ok(), "parse failed: {:?}", event.err());
        let ev = event.unwrap();
        assert_eq!(ev.listing_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_event_parse_invalid() {
        let result = AirbnbClient::parse_webhook_event("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_event_parse_with_event_id() {
        let body = r#"{
            "event_type": "reservation_created",
            "event_id": "evt_abc123",
            "listing_id": "listing_42",
            "timestamp": "2026-01-01T00:00:00Z",
            "payload": {}
        }"#;
        let event = AirbnbClient::parse_webhook_event(body);
        assert!(event.is_ok(), "parse failed: {:?}", event.err());
        let ev = event.unwrap();
        assert_eq!(ev.event_id.as_deref(), Some("evt_abc123"));
        assert_eq!(ev.listing_id.as_deref(), Some("listing_42"));
    }

    #[test]
    fn test_event_parse_without_event_id() {
        let body = r#"{
            "event_type": "listing_updated",
            "listing_id": "listing_99",
            "timestamp": "2026-01-01T00:00:00Z",
            "payload": {}
        }"#;
        let event = AirbnbClient::parse_webhook_event(body);
        assert!(event.is_ok(), "parse failed: {:?}", event.err());
        let ev = event.unwrap();
        assert!(ev.event_id.is_none());
    }

    // ---- Dedup-key regression tests (bug-webhook-airbnb-dup-sync-jobs) ----
    //
    // The dedup key is what makes the `airbnb_webhook_events` ledger
    // (INSERT ... ON CONFLICT DO NOTHING) suppress at-least-once
    // redeliveries before any SYNC_EXTERNAL job is enqueued. A redelivery
    // MUST map to the same key as the original delivery, otherwise the
    // ledger lets it through and a duplicate sync job is created.

    fn parse(body: &str) -> integrations::AirbnbWebhookEvent {
        AirbnbClient::parse_webhook_event(body).expect("valid event")
    }

    const RESERVATION_WITH_ID: &str = r#"{
        "event_type": "reservation_created",
        "event_id": "evt_abc123",
        "listing_id": "listing_42",
        "confirmation_code": "HMABC123",
        "timestamp": "2026-01-01T00:00:00Z",
        "payload": {}
    }"#;

    const RESERVATION_NO_ID: &str = r#"{
        "event_type": "reservation_created",
        "listing_id": "listing_42",
        "confirmation_code": "HMABC123",
        "timestamp": "2026-01-01T00:00:00Z",
        "payload": {}
    }"#;

    #[test]
    fn dedup_key_uses_event_id_verbatim_when_present() {
        let key = super::airbnb_dedup_key(&parse(RESERVATION_WITH_ID));
        assert_eq!(key, "evt_abc123");
    }

    #[test]
    fn dedup_key_redelivery_with_event_id_is_stable() {
        // Same event_id redelivered => identical key => ledger conflict =>
        // no second SYNC_EXTERNAL enqueue.
        let first = super::airbnb_dedup_key(&parse(RESERVATION_WITH_ID));
        let redelivery = super::airbnb_dedup_key(&parse(RESERVATION_WITH_ID));
        assert_eq!(first, redelivery);
    }

    #[test]
    fn dedup_key_synthetic_when_event_id_absent() {
        let key = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        assert!(
            key.starts_with("synthetic:"),
            "expected synthetic key, got {key}"
        );
        // Synthetic keys can never collide with a real Airbnb event_id.
        assert_ne!(key, "evt_abc123");
    }

    #[test]
    fn dedup_key_synthetic_redelivery_is_stable() {
        // The previously-unguarded path: an event WITHOUT event_id, redelivered
        // byte-for-byte, must still produce the same key so the duplicate
        // SYNC_EXTERNAL enqueue is suppressed.
        let first = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        let redelivery = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        assert_eq!(first, redelivery);
    }

    #[test]
    fn dedup_key_synthetic_differs_for_distinct_deliveries() {
        let a = super::airbnb_dedup_key(&parse(RESERVATION_NO_ID));
        // Different listing => different delivery => different key.
        let other = RESERVATION_NO_ID.replace("listing_42", "listing_99");
        let b = super::airbnb_dedup_key(&parse(&other));
        assert_ne!(a, b);
        // Different timestamp => different delivery => different key.
        let later = RESERVATION_NO_ID.replace("00:00:00Z", "00:05:00Z");
        let c = super::airbnb_dedup_key(&parse(&later));
        assert_ne!(a, c);
    }
}

// ==================== Airbnb timestamp-parity (staged accept-both) tests ====================
//
// Regression cover for the audit R2 fix: the Airbnb receiver verified a body-only
// HMAC with no freshness window. It now enforces "{timestamp}.{body}" freshness
// when the delivery carries `X-Airbnb-Timestamp`, while still accepting a legacy
// body-only signature during the migration. These pin `verify_airbnb_delivery`.
#[cfg(test)]
mod airbnb_timestamp_parity_tests {
    use super::{verify_airbnb_delivery, AIRBNB_WEBHOOK_TOLERANCE_SECS};
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    const SECRET: &str = "airbnb_webhook_secret";
    const BODY: &str = r#"{"event_type":"reservation_created","listing_id":"L1","timestamp":"2026-01-01T00:00:00Z","payload":{}}"#;
    const NOW: i64 = 1_700_000_000;

    fn sign_ts(secret: &str, ts: i64, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(format!("{ts}.{body}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    fn sign_body(secret: &str, body: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(body.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn timestamped_delivery_within_window_is_accepted() {
        let sig = sign_ts(SECRET, NOW, BODY);
        assert!(verify_airbnb_delivery(
            SECRET,
            Some(&NOW.to_string()),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn timestamped_delivery_outside_window_is_rejected() {
        let stale = NOW - AIRBNB_WEBHOOK_TOLERANCE_SECS - 1;
        let sig = sign_ts(SECRET, stale, BODY);
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some(&stale.to_string()),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn timestamp_swap_on_captured_signature_is_rejected() {
        let sig = sign_ts(SECRET, NOW - 10_000, BODY);
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some(&NOW.to_string()),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn present_but_malformed_timestamp_fails_closed() {
        // A body-only signature paired with a garbage timestamp header must not
        // fall back to the no-freshness path.
        let sig = sign_body(SECRET, BODY);
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some("not-a-number"),
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn legacy_body_only_signature_accepted_when_no_timestamp() {
        // Accept-both migration window: no header ⇒ legacy body-only signature.
        let sig = sign_body(SECRET, BODY);
        assert!(verify_airbnb_delivery(
            SECRET,
            None,
            &sig,
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }

    #[test]
    fn forged_signature_rejected_on_both_paths() {
        assert!(!verify_airbnb_delivery(
            SECRET,
            None,
            "deadbeef",
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
        assert!(!verify_airbnb_delivery(
            SECRET,
            Some(&NOW.to_string()),
            "deadbeef",
            BODY,
            NOW,
            AIRBNB_WEBHOOK_TOLERANCE_SECS,
        ));
    }
}

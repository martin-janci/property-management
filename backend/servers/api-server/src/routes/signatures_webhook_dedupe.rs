//! Replay/idempotency dedupe for the inbound e-signature provider webhook
//! (`POST /api/v1/signature-requests/webhook/{provider}`).
//!
//! ## Why
//!
//! The webhook receiver authenticates deliveries with a *static* shared secret
//! (`ESIGN_WEBHOOK_SECRET`, constant-time compared) carried in the
//! `X-Webhook-Secret` header. A static bearer secret proves authenticity but
//! **not** freshness: a captured valid delivery — the exact bytes plus the
//! secret header — can be replayed indefinitely. The existing terminal-state
//! idempotency guard (`SignatureRequestStatus::is_terminal()`) absorbs the most
//! dangerous replays (re-storing a signed document, re-firing completion /
//! decline emails on an already-finalized request), but it only fires *once the
//! request is terminal*. A delivery replayed while the request is still
//! `pending` / `in_progress` slips past it and re-runs its side effects on every
//! duplicate.
//!
//! This module closes that residual window with event-fingerprint dedupe: the
//! first delivery of a given event claims a short-TTL key and is processed; any
//! replay carrying the same fingerprint within the window is recognised as a
//! duplicate and acknowledged (200) without re-running side effects.
//!
//! ## How
//!
//! Production dedupe runs against Redis via an atomic `SET key NX EX ttl`
//! ([`RedisClient::set_nx`]): exactly one caller claims the fingerprint, every
//! racing/later replay within the TTL observes "already claimed". When Redis is
//! unavailable the guard degrades to [`DedupeOutcome::StoreUnavailable`] and the
//! caller falls back to the terminal-state guard (defence in depth) rather than
//! refusing legitimate traffic. A test-only in-memory [`WebhookDedupeStore`]
//! (installed via `AppState::with_webhook_dedupe_store`) mirrors the serial
//! single-claim semantics so the handler-level replay-rejection branch is
//! exercisable in CI, which wires no Redis — the same pattern the OAuth `state`
//! store ([`super::integrations::oauth_state`]) uses.

use sha2::{Digest, Sha256};

use crate::state::AppState;
use db::models::SignatureWebhookEvent;

/// TTL for a processed-event fingerprint, in seconds (24h).
///
/// E-signature providers retry undelivered webhooks for hours; the window must
/// comfortably outlast a provider's retry schedule so a retry storm cannot
/// outlive the dedupe key and re-trigger side effects, while staying short
/// enough to bound Redis key growth.
const WEBHOOK_DEDUPE_TTL_SECS: u64 = 24 * 60 * 60;

/// Redis key namespace for processed e-signature webhook fingerprints.
const KEY_PREFIX: &str = "esign:webhook:dedupe:";

fn redis_key(fingerprint: &str) -> String {
    format!("{KEY_PREFIX}{fingerprint}")
}

/// Derive a stable, collision-resistant fingerprint for a webhook delivery.
///
/// A byte-for-byte replay of a delivery produces an identical fingerprint (and
/// is therefore deduped), while two legitimately-distinct events differ in at
/// least one identifying field and so get distinct fingerprints. The fields are
/// length-prefixed before hashing so no field value can be shifted across a
/// boundary to collide with a different event (e.g. a `provider_request_id`
/// ending in what looks like the next field).
///
/// Pure (no I/O) so the fingerprint contract is unit-testable without a store.
pub fn event_fingerprint(provider: &str, event: &SignatureWebhookEvent) -> String {
    let mut hasher = Sha256::new();
    // `SignerStatus` is a `Copy` enum rendered via `Display`; materialise it to
    // an owned string so it can sit alongside the borrowed `&str` fields.
    let signer_status = event
        .signer_status
        .map(|s| s.to_string())
        .unwrap_or_default();
    // Length-prefix every component: the leading byte-length makes the
    // concatenation unambiguous and injection-proof regardless of field contents.
    for field in [
        provider,
        event.event_type.as_str(),
        event.provider_request_id.as_str(),
        event.signer_email.as_deref().unwrap_or(""),
        signer_status.as_str(),
        event.decline_reason.as_deref().unwrap_or(""),
        event.signed_document_url.as_deref().unwrap_or(""),
    ] {
        hasher.update(field.len().to_le_bytes());
        hasher.update(field.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Outcome of the first-seen check for a webhook delivery.
pub enum DedupeOutcome {
    /// This fingerprint was not seen within the window — the caller claimed it
    /// and MUST process the delivery.
    FirstDelivery,
    /// This fingerprint was already claimed within the window — a replay /
    /// at-least-once duplicate. The caller MUST acknowledge (200) and skip side
    /// effects.
    Duplicate,
    /// No dedupe store is available (no Redis, or a Redis error). The caller
    /// proceeds and relies on the terminal-state idempotency guard as backstop.
    StoreUnavailable,
}

/// Record a delivery's fingerprint and report whether it is the first time this
/// event has been seen within the dedupe window.
///
/// Prefers a test-installed in-memory store (so CI without Redis can exercise
/// the replay branch), then the Redis `SET NX EX` path, then
/// [`DedupeOutcome::StoreUnavailable`].
pub async fn check_and_record(
    state: &AppState,
    provider: &str,
    event: &SignatureWebhookEvent,
) -> DedupeOutcome {
    let fingerprint = event_fingerprint(provider, event);

    // Test-only in-memory store takes precedence; `None` in production so this
    // branch is inert and the Redis path is authoritative.
    if let Some(store) = &state.webhook_dedupe_store {
        return store.record(&fingerprint);
    }

    let Some(redis) = &state.redis_client else {
        return DedupeOutcome::StoreUnavailable;
    };

    match redis
        .set_nx(&redis_key(&fingerprint), WEBHOOK_DEDUPE_TTL_SECS)
        .await
    {
        Ok(true) => DedupeOutcome::FirstDelivery,
        Ok(false) => DedupeOutcome::Duplicate,
        Err(e) => {
            // A Redis error must not drop the delivery: degrade to the
            // terminal-state backstop rather than refusing or silently deduping.
            tracing::warn!(error = %e, "e-signature webhook dedupe store unavailable; relying on terminal-state guard");
            DedupeOutcome::StoreUnavailable
        }
    }
}

/// Test-only in-memory webhook dedupe store.
///
/// Production dedupe lives in Redis (`AppState::redis_client`). Because CI wires
/// no Redis, [`check_and_record`] would always return
/// [`DedupeOutcome::StoreUnavailable`] under the integration-test harness, so
/// the handler's replay-skip branch would have zero handler-level coverage.
///
/// This store mirrors the Redis `SET NX` semantics for the *serial* case: the
/// first [`record`](Self::record) of a fingerprint inserts it and returns
/// [`DedupeOutcome::FirstDelivery`]; a second `record` of the same fingerprint
/// finds it present and returns [`DedupeOutcome::Duplicate`]. Installed into
/// `AppState` only by tests via `AppState::with_webhook_dedupe_store`; it is
/// `None` in production and never touched by real traffic.
///
/// Limitation: this models only *serial* first-claim. The production Redis path
/// enforces first-claim atomically (`SET NX`) so it is also race-safe under
/// concurrent deliveries; this store's per-call `Mutex` makes each `record`
/// atomic in isolation but the harness never issues concurrent records, so it
/// does not exercise the concurrent-replay race that `SET NX` closes.
#[derive(Clone, Default)]
pub struct WebhookDedupeStore {
    inner: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl WebhookDedupeStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a fingerprint, returning [`DedupeOutcome::FirstDelivery`] the first
    /// time it is seen and [`DedupeOutcome::Duplicate`] on every subsequent call
    /// for the same fingerprint.
    fn record(&self, fingerprint: &str) -> DedupeOutcome {
        let newly_inserted = self
            .inner
            .lock()
            .expect("webhook dedupe store mutex poisoned")
            .insert(fingerprint.to_string());
        if newly_inserted {
            DedupeOutcome::FirstDelivery
        } else {
            DedupeOutcome::Duplicate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::models::SignerStatus;

    fn event() -> SignatureWebhookEvent {
        SignatureWebhookEvent {
            event_type: "signer_signed".into(),
            provider_request_id: "prov-req-1".into(),
            signer_email: Some("alice@example.com".into()),
            signer_status: Some(SignerStatus::Signed),
            decline_reason: None,
            signed_document_url: None,
            raw_payload: None,
        }
    }

    #[test]
    fn redis_key_is_namespaced() {
        let key = redis_key("abc123");
        assert_eq!(key, "esign:webhook:dedupe:abc123");
        assert!(key.starts_with(KEY_PREFIX));
    }

    #[test]
    fn fingerprint_is_stable_for_identical_deliveries() {
        // A byte-for-byte replay hashes to the same fingerprint => deduped.
        assert_eq!(
            event_fingerprint("lightweight", &event()),
            event_fingerprint("lightweight", &event())
        );
    }

    #[test]
    fn fingerprint_differs_on_any_identifying_field() {
        let base = event_fingerprint("lightweight", &event());

        // Different provider.
        assert_ne!(base, event_fingerprint("docusign", &event()));

        // Different event type.
        let mut e = event();
        e.event_type = "signer_declined".into();
        assert_ne!(base, event_fingerprint("lightweight", &e));

        // Different provider_request_id.
        let mut e = event();
        e.provider_request_id = "prov-req-2".into();
        assert_ne!(base, event_fingerprint("lightweight", &e));

        // Different signer.
        let mut e = event();
        e.signer_email = Some("bob@example.com".into());
        assert_ne!(base, event_fingerprint("lightweight", &e));

        // Different signer status.
        let mut e = event();
        e.signer_status = Some(SignerStatus::Declined);
        assert_ne!(base, event_fingerprint("lightweight", &e));
    }

    #[test]
    fn length_prefix_prevents_field_boundary_collisions() {
        // Without length-prefixing, moving a character across the
        // provider/event_type boundary would collide. With it, they differ.
        let mut a = event();
        a.event_type = "ab".into();
        a.provider_request_id = "cd".into();
        let mut b = event();
        b.event_type = "a".into();
        b.provider_request_id = "bcd".into();
        assert_ne!(
            event_fingerprint("p", &a),
            event_fingerprint("p", &b),
            "concatenation must be unambiguous across field boundaries"
        );
    }

    #[test]
    fn store_records_first_then_flags_replay() {
        let store = WebhookDedupeStore::new();
        let fp = event_fingerprint("lightweight", &event());

        // First delivery claims the fingerprint.
        assert!(matches!(store.record(&fp), DedupeOutcome::FirstDelivery));
        // Replay of the SAME fingerprint is flagged as a duplicate (the branch
        // the handler turns into a no-op 200 ack).
        assert!(matches!(store.record(&fp), DedupeOutcome::Duplicate));
        assert!(matches!(store.record(&fp), DedupeOutcome::Duplicate));
    }

    #[test]
    fn store_treats_distinct_events_independently() {
        let store = WebhookDedupeStore::new();
        let mut other = event();
        other.provider_request_id = "prov-req-2".into();

        assert!(matches!(
            store.record(&event_fingerprint("lightweight", &event())),
            DedupeOutcome::FirstDelivery
        ));
        // A different event is not deduped against the first.
        assert!(matches!(
            store.record(&event_fingerprint("lightweight", &other)),
            DedupeOutcome::FirstDelivery
        ));
    }
}

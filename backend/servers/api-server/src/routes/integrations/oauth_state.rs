//! Server-bound OAuth `state` parameter store (Issue #765).
//!
//! The OAuth `state` parameter must be a real CSRF token: single-use and tied
//! to the session/user that initiated the authorization flow. Previously the
//! Airbnb flow used a stateless `{org_id}:{uuid}` string that the server never
//! persisted, so it could not detect forgery or replay — any value with the
//! right org prefix was accepted.
//!
//! This module persists a generated state in Redis (short TTL) bound to the
//! initiating `org_id` + `user_id`, and verifies + consumes it on callback so a
//! state can be used at most once. When Redis is unavailable the caller falls
//! back to the stateless org-binding + `verify_org_access` checks (defence in
//! depth), and we log that single-use enforcement is degraded.

use uuid::Uuid;

use crate::state::AppState;

/// TTL for a pending OAuth state, in seconds. An authorization round-trip is
/// interactive and short; 10 minutes is generous while keeping the replay
/// window small.
const OAUTH_STATE_TTL_SECS: u64 = 600;

/// Redis key namespace for pending OAuth states.
const KEY_PREFIX: &str = "integrations:oauth:state:";

/// The server-side record bound to a generated OAuth state.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct OAuthStateRecord {
    pub org_id: Uuid,
    pub user_id: Uuid,
}

fn redis_key(state: &str) -> String {
    format!("{KEY_PREFIX}{state}")
}

/// Generate a new OAuth state and persist it (if Redis is available) bound to
/// the initiating org + user.
///
/// The returned string keeps the `{org_id}:{uuid}` shape so the existing
/// stateless org-prefix check continues to work as a second layer. When Redis
/// is configured the random component is the single-use token; when it is not,
/// the value is still returned but cannot be enforced as single-use.
pub async fn issue(state: &AppState, org_id: Uuid, user_id: Uuid) -> String {
    let nonce = Uuid::new_v4();
    let oauth_state = format!("{org_id}:{nonce}");

    if let Some(redis) = &state.redis_client {
        let record = OAuthStateRecord { org_id, user_id };
        match redis
            .set(
                &redis_key(&oauth_state),
                &record,
                Some(OAUTH_STATE_TTL_SECS),
            )
            .await
        {
            Ok(()) => {
                tracing::debug!(org_id = %org_id, "Persisted single-use OAuth state");
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to persist OAuth state in Redis; single-use enforcement degraded"
                );
            }
        }
    } else {
        tracing::warn!("Redis not configured; OAuth state single-use enforcement is unavailable");
    }

    oauth_state
}

/// Outcome of verifying + consuming an OAuth state on callback.
pub enum ConsumeOutcome {
    /// State was found, matched the org, and has now been consumed (deleted).
    Consumed,
    /// State did not match (forged/replayed/expired) — reject the callback.
    Rejected,
    /// Redis is unavailable, so single-use enforcement could not run. Callers
    /// fall back to stateless org-binding + `verify_org_access`.
    StoreUnavailable,
}

/// Verify a state on callback and consume it so it cannot be reused.
///
/// Returns [`ConsumeOutcome::Consumed`] only if the stored record exists and
/// its `org_id` matches the callback's path org. The Redis path uses an atomic
/// `GETDEL`, so the read and the delete happen in a single command: exactly one
/// concurrent caller receives the record and all others see `None`. This makes
/// single-use race-safe — two concurrent callbacks carrying the same valid
/// state cannot both be `Consumed` (#2241). A non-matching `org_id` still
/// deletes the key (via the same `GETDEL`) and rejects (prevents replay across
/// orgs).
pub async fn verify_and_consume(
    state: &AppState,
    oauth_state: &str,
    org_id: Uuid,
) -> ConsumeOutcome {
    // Issue #2203: a test-installed in-memory store takes precedence so the
    // real consume path (`Consumed` on a freshly-issued state, `Rejected` on a
    // replayed / already-consumed one) is exercisable at the handler level in
    // CI, which wires no Redis. `oauth_state_store` is `None` in production, so
    // this branch is inert there and the Redis path below is authoritative.
    if let Some(store) = &state.oauth_state_store {
        return store.consume(oauth_state, org_id);
    }

    let Some(redis) = &state.redis_client else {
        return ConsumeOutcome::StoreUnavailable;
    };

    let key = redis_key(oauth_state);
    // Atomic get-and-delete (`GETDEL`): under concurrency exactly one caller
    // receives the stored record and every other concurrent callback sees
    // `None`. A prior non-atomic `get()` + `delete()` left a TOCTOU window where
    // two concurrent callbacks carrying the *same* valid state could both read
    // `Some(record)` before either deleted it, so both would be `Consumed` —
    // defeating single-use and reopening the CSRF replay window (#2241). Doing
    // the read+delete in one command closes that race: the loser sees `None`
    // and is rejected as a replay.
    let record: Option<OAuthStateRecord> = match redis.get_del(&key).await {
        Ok(r) => r,
        Err(e) => {
            // A Redis error must not silently pass the check; treat as
            // unavailable so the stateless fallback decides.
            tracing::warn!(error = %e, "Failed to read+consume OAuth state from Redis");
            return ConsumeOutcome::StoreUnavailable;
        }
    };

    decide_consume(record, org_id)
}

/// Pure decision for [`verify_and_consume`]: given the record looked up from the
/// store (if any) and the `org_id` from the callback path, decide whether the
/// state is accepted.
///
/// Kept side-effect free (no Redis) so the CSRF single-use + org-binding rules
/// can be unit-tested deterministically (#1374): a state is [`Consumed`] only
/// when it was found AND bound to the same org; a missing record (expired,
/// forged, or already-consumed → replay) and an org mismatch both [`Rejected`].
///
/// [`Consumed`]: ConsumeOutcome::Consumed
/// [`Rejected`]: ConsumeOutcome::Rejected
fn decide_consume(record: Option<OAuthStateRecord>, path_org_id: Uuid) -> ConsumeOutcome {
    match record {
        Some(rec) if rec.org_id == path_org_id => ConsumeOutcome::Consumed,
        Some(rec) => {
            tracing::warn!(
                stored_org = %rec.org_id,
                path_org = %path_org_id,
                "OAuth state org mismatch on callback"
            );
            ConsumeOutcome::Rejected
        }
        None => {
            tracing::warn!("OAuth state not found or already consumed (possible replay)");
            ConsumeOutcome::Rejected
        }
    }
}

/// Test-only in-memory OAuth state store (issue #2203).
///
/// Production single-use enforcement lives in Redis (`AppState::redis_client`).
/// Because CI wires no Redis, [`verify_and_consume`] always returned
/// [`ConsumeOutcome::StoreUnavailable`] under the integration-test harness, so
/// the handler's replay-rejection branch (`Rejected → 400 INVALID_STATE`) — the
/// actual heart of the CSRF single-use check — had zero handler-level coverage.
///
/// This store mirrors the Redis consume semantics for the *serial* single-use
/// case: a state is bound to an `org_id` when issued ([`seed`](Self::seed)) and
/// removed on first lookup ([`consume`](Self::consume)), so a second callback
/// with the same state hits the `None` (replay) arm of [`decide_consume`].
/// Installed into `AppState` only by tests via
/// `AppState::with_oauth_state_store`; it is `None` in production and never
/// touched by real traffic.
///
/// Limitation: this in-memory double models only *serial* single-use. The
/// production Redis path enforces single-use atomically (`GETDEL`) so it is also
/// race-safe under concurrent callbacks; this store's per-call `Mutex` makes
/// each `consume` atomic in isolation but the harness never issues concurrent
/// consumes, so it does not exercise the concurrent-replay race that the Redis
/// `GETDEL` closes. Treat handler tests driven through this store as covering
/// the serial replay contract only.
#[derive(Clone, Default)]
pub struct OAuthStateStore {
    inner: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, OAuthStateRecord>>>,
}

impl OAuthStateStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed a freshly-issued, single-use `oauth_state` bound to `org_id` /
    /// `user_id`, exactly as [`issue`] would have persisted it in Redis.
    pub fn seed(&self, oauth_state: &str, org_id: Uuid, user_id: Uuid) {
        self.inner
            .lock()
            .expect("oauth state store mutex poisoned")
            .insert(
                oauth_state.to_string(),
                OAuthStateRecord { org_id, user_id },
            );
    }

    /// Look up + consume (remove) the record for `oauth_state`, then apply the
    /// same accept/reject decision as the Redis path. Single-use: a second call
    /// for the same state finds no record and is [`ConsumeOutcome::Rejected`].
    fn consume(&self, oauth_state: &str, org_id: Uuid) -> ConsumeOutcome {
        let record = self
            .inner
            .lock()
            .expect("oauth state store mutex poisoned")
            .remove(oauth_state);
        decide_consume(record, org_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_key_is_namespaced() {
        let key = redis_key("org:nonce");
        assert_eq!(key, "integrations:oauth:state:org:nonce");
        assert!(key.starts_with(KEY_PREFIX));
    }

    #[test]
    fn state_record_roundtrips_through_json() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let rec = OAuthStateRecord { org_id, user_id };

        let json = serde_json::to_string(&rec).unwrap();
        let back: OAuthStateRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.org_id, org_id);
        assert_eq!(back.user_id, user_id);
    }

    // ---- CSRF single-use + org-binding decision (#1374) -------------------
    //
    // `verify_and_consume` reads the record from Redis and then defers to the
    // pure `decide_consume`. These pin the security-relevant outcomes — valid,
    // invalid (org mismatch), and missing/replayed state — without a live Redis.

    #[test]
    fn decide_consume_accepts_state_bound_to_the_callback_org() {
        let org_id = Uuid::new_v4();
        let rec = OAuthStateRecord {
            org_id,
            user_id: Uuid::new_v4(),
        };
        // A state that was issued for this org is the only accepted ("valid") case.
        assert!(matches!(
            decide_consume(Some(rec), org_id),
            ConsumeOutcome::Consumed
        ));
    }

    #[test]
    fn decide_consume_rejects_state_bound_to_a_different_org() {
        // Forged/cross-org replay: the stored record exists but is bound to a
        // different org than the callback path — must be rejected (no IDOR).
        let rec = OAuthStateRecord {
            org_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
        };
        assert!(matches!(
            decide_consume(Some(rec), Uuid::new_v4()),
            ConsumeOutcome::Rejected
        ));
    }

    #[test]
    fn decide_consume_rejects_missing_state() {
        // No record => the state never existed, expired, or was already consumed
        // (single-use replay). Either way the callback must be rejected.
        assert!(matches!(
            decide_consume(None, Uuid::new_v4()),
            ConsumeOutcome::Rejected
        ));
    }

    // ---- In-memory OAuthStateStore single-use semantics (#2203) -----------

    #[test]
    fn store_consumes_seeded_state_once_then_rejects_replay() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let oauth_state = format!("{org_id}:{}", Uuid::new_v4());

        let store = OAuthStateStore::new();
        store.seed(&oauth_state, org_id, user_id);

        // First use: found + org-bound => Consumed.
        assert!(matches!(
            store.consume(&oauth_state, org_id),
            ConsumeOutcome::Consumed
        ));
        // Replay of the SAME state: already removed => Rejected (the branch the
        // handler turns into 400 INVALID_STATE).
        assert!(matches!(
            store.consume(&oauth_state, org_id),
            ConsumeOutcome::Rejected
        ));
    }

    #[test]
    fn store_rejects_state_bound_to_a_different_org() {
        let issued_org = Uuid::new_v4();
        let oauth_state = format!("{issued_org}:{}", Uuid::new_v4());

        let store = OAuthStateStore::new();
        store.seed(&oauth_state, issued_org, Uuid::new_v4());

        // Consumed against a DIFFERENT path org => cross-org replay => Rejected.
        assert!(matches!(
            store.consume(&oauth_state, Uuid::new_v4()),
            ConsumeOutcome::Rejected
        ));
    }
}

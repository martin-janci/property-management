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
/// its `org_id` matches the callback's path org. The key is deleted on a match
/// to guarantee single use. A non-matching `org_id` deletes the key and
/// rejects (prevents replay across orgs).
pub async fn verify_and_consume(
    state: &AppState,
    oauth_state: &str,
    org_id: Uuid,
) -> ConsumeOutcome {
    let Some(redis) = &state.redis_client else {
        return ConsumeOutcome::StoreUnavailable;
    };

    let key = redis_key(oauth_state);
    let record: Option<OAuthStateRecord> = match redis.get(&key).await {
        Ok(r) => r,
        Err(e) => {
            // A Redis read error must not silently pass the check; treat as
            // unavailable so the stateless fallback decides.
            tracing::warn!(error = %e, "Failed to read OAuth state from Redis");
            return ConsumeOutcome::StoreUnavailable;
        }
    };

    // Always consume the key once seen (match or mismatch) to prevent reuse /
    // replay, then decide the outcome from the (now consumed) record.
    if record.is_some() {
        let _ = redis.delete(&key).await;
    }

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
}

//! Generic in-process sliding-window rate limiter shared by auth-surface
//! routes.
//!
//! This is the single implementation of the per-key brute-force / abuse
//! throttle that previously lived (Uuid-keyed) inside `routes::mfa` and
//! (IP-keyed) inside `routes::caddy_ask`. Extracting it here lets the
//! auth email-keyed surfaces (`/forgot-password`, `/resend-verification`)
//! reuse the exact same, already-reviewed algorithm instead of growing a
//! third copy.
//!
//! Semantics (unchanged from the original mfa limiter):
//! - A rolling `window` per key; once more than `max_attempts` attempts land
//!   in the current window the key is blocked until the window rolls over.
//! - Expired entries are evicted, but only once the map exceeds
//!   `sweep_threshold`, so the `retain` walk is amortized rather than run on
//!   every request. This bounds memory on auth paths where the key space is
//!   the whole user/email population (a slow memory-exhaustion DoS otherwise).
//! - The counter is per-process; a Redis-backed counter (the sessions Redis is
//!   already in the stack) would hold the limit across instances — tracked as a
//!   follow-up.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// One key's attempt counter within the current rolling window.
pub struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

/// A process-global limiter keyed on `K` (e.g. `uuid::Uuid` for authenticated
/// user surfaces, `String` for email-keyed unauthenticated surfaces).
pub type InProcessRateLimiter<K> = Mutex<HashMap<K, RateLimitEntry>>;

/// Record an attempt for `key` in `limiter` and report whether it is within
/// the limit. Returns `false` once more than `max_attempts` attempts occur in a
/// rolling `window`.
///
/// See the module docs for the eviction / memory-bound rationale behind
/// `sweep_threshold`.
pub fn rate_limit_allowed<K: Eq + Hash>(
    limiter: &InProcessRateLimiter<K>,
    key: K,
    max_attempts: u32,
    window: Duration,
    sweep_threshold: usize,
) -> bool {
    let mut map = match limiter.lock() {
        Ok(m) => m,
        Err(poisoned) => poisoned.into_inner(),
    };
    let now = Instant::now();

    if map.len() > sweep_threshold {
        map.retain(|_, e| now.duration_since(e.window_start) < window);
    }

    let entry = map.entry(key).or_insert(RateLimitEntry {
        count: 0,
        window_start: now,
    });
    if now.duration_since(entry.window_start) >= window {
        entry.count = 0;
        entry.window_start = now;
    }
    entry.count = entry.count.saturating_add(1);
    entry.count <= max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    static TEST_LIMITER: LazyLock<InProcessRateLimiter<String>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    #[test]
    fn allows_up_to_max_then_blocks() {
        let key = "allows_up_to_max_then_blocks".to_string();
        let max = 5;
        let window = Duration::from_secs(900);
        for i in 0..max {
            assert!(
                rate_limit_allowed(&TEST_LIMITER, key.clone(), max, window, 1024),
                "attempt {i} within the limit must be allowed"
            );
        }
        assert!(
            !rate_limit_allowed(&TEST_LIMITER, key, max, window, 1024),
            "the (max + 1)th attempt in the window must be blocked"
        );
    }

    #[test]
    fn separate_keys_do_not_interfere() {
        let window = Duration::from_secs(900);
        // Exhaust key A.
        for _ in 0..3 {
            assert!(rate_limit_allowed(
                &TEST_LIMITER,
                "iface_key_a".to_string(),
                3,
                window,
                1024
            ));
        }
        assert!(!rate_limit_allowed(
            &TEST_LIMITER,
            "iface_key_a".to_string(),
            3,
            window,
            1024
        ));
        // A different key is unaffected.
        assert!(rate_limit_allowed(
            &TEST_LIMITER,
            "iface_key_b".to_string(),
            3,
            window,
            1024
        ));
    }
}

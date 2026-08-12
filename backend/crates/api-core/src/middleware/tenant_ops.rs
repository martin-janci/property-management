//! Per-tenant rate limit + metering primitives (Phase 5.5).
//!
//! Wired into [`super::host_tenant::host_tenant_middleware`] at the
//! `SEAM(leak#15)` and `SEAM(leak#19)` markers. The keystone middleware
//! already runs per-request and already knows the resolved tenant —
//! piggybacking the rate limiter and the meter on it is the design payoff
//! highlighted in the brainstorming session.
//!
//! Defenses:
//! * **#15** — noisy-neighbor protection. A flooding tenant gets 429s
//!   without affecting the latency of any other tenant.
//! * **#19** — per-tenant metering. Every request emits a `requests_total`
//!   counter increment tagged with `org_id`, plus a
//!   `request_bytes_total` counter for the response body.

use governor::{
    clock::{Clock, DefaultClock},
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Per-tenant rate limiter (defense #15)
// ============================================================================

/// Default rate limit: 600 req/min per tenant.
///
/// Configurable per tenant via Phase 5's `tenant_settings.rate_limit_rpm`
/// (we read it through [`TenantRateLimiterConfig::override_for`]).
pub const DEFAULT_RATE_LIMIT_RPM: u32 = 600;

/// Result of a per-tenant rate-limit check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Request is allowed.
    Allow,
    /// Request must be rejected with HTTP 429.
    DenyTooManyRequests,
}

/// In-process per-tenant limiter set.
///
/// One [`DefaultDirectRateLimiter`] per organization id, lazily created
/// on first sight. Old idle entries are evicted by a TTL sweep so a long-
/// lived process doesn't grow without bound.
///
/// # Key cardinality & unbounded-growth protection
///
/// When the key space is bounded by the platform (the per-*tenant* use — one
/// entry per real organization), the map is naturally small and the optional
/// [`max_entries`](Self::max_entries) cap can stay `None`. When the key space
/// is **attacker-influenced** — e.g. a per-client-IP throttle for anonymous
/// endpoints, where an adversary rotating source IPs would otherwise insert an
/// unbounded number of distinct keys — construct the set with
/// [`Self::with_default_bounded`] so the cold-path insert enforces a hard cap
/// (TTL-expired entries dropped first, then oldest-created entries evicted
/// LRU-style). That makes the map's memory footprint bounded **inline**,
/// without relying on a background [`Self::sweep_idle`] task ever being run.
pub struct TenantRateLimiterSet {
    /// Per-tenant limiters keyed by org id, with their last-touched time.
    limiters: Arc<RwLock<HashMap<Uuid, LimiterEntry>>>,
    default_rpm: u32,
    overrides: Arc<RwLock<HashMap<Uuid, u32>>>,
    /// How long an unused limiter entry survives before being swept.
    idle_ttl: Duration,
    /// Optional hard cap on the number of tracked keys. `None` = unbounded
    /// (safe only when the key space is platform-bounded, e.g. per-tenant).
    /// `Some(cap)` bounds memory even under an adversarial key stream (e.g.
    /// per-IP), enforced on every cold-path insert.
    max_entries: Option<usize>,
}

struct LimiterEntry {
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    /// The configured rpm for this tenant. Captured at build-time for
    /// future diagnostics (e.g. an admin endpoint reporting per-tenant
    /// effective rates) — not currently read.
    #[allow(dead_code)]
    rpm: u32,
    last_touched: Instant,
}

impl TenantRateLimiterSet {
    /// Build with the default 600 rpm baseline and a 1-hour idle eviction.
    pub fn new() -> Self {
        Self::with_default(DEFAULT_RATE_LIMIT_RPM)
    }

    /// Build with a custom baseline. Unbounded key space (`max_entries: None`)
    /// — use only when keys are platform-bounded (per-tenant).
    pub fn with_default(default_rpm: u32) -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            default_rpm: default_rpm.max(1),
            overrides: Arc::new(RwLock::new(HashMap::new())),
            idle_ttl: Duration::from_secs(3600),
            max_entries: None,
        }
    }

    /// Build with a custom baseline, idle TTL, and a **hard cap** on the number
    /// of tracked keys.
    ///
    /// Use this for adversary-influenced key spaces (per-client-IP throttles on
    /// anonymous endpoints): an attacker rotating source IPs cannot grow the
    /// map past `max_entries`, because each cold-path insert first sweeps
    /// TTL-expired entries and then, if still at the cap, evicts the
    /// oldest-created entries to make room. Memory is bounded inline — no
    /// background [`Self::sweep_idle`] task is required. `max_entries` is
    /// clamped to at least 1.
    pub fn with_default_bounded(default_rpm: u32, idle_ttl: Duration, max_entries: usize) -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            default_rpm: default_rpm.max(1),
            overrides: Arc::new(RwLock::new(HashMap::new())),
            idle_ttl,
            max_entries: Some(max_entries.max(1)),
        }
    }

    /// Install a per-tenant rpm override (typically loaded from
    /// `tenant_settings.rate_limit_rpm`).
    pub async fn set_override(&self, org_id: Uuid, rpm: u32) {
        self.overrides.write().await.insert(org_id, rpm.max(1));
        // Drop the cached limiter so the next request rebuilds it with the
        // new rpm.
        self.limiters.write().await.remove(&org_id);
    }

    /// Drop a per-tenant override (revert to default).
    pub async fn clear_override(&self, org_id: Uuid) {
        self.overrides.write().await.remove(&org_id);
        self.limiters.write().await.remove(&org_id);
    }

    /// Make one decision for one tenant.
    pub async fn check(&self, org_id: Uuid) -> RateLimitDecision {
        match self.check_with_retry(org_id).await {
            Ok(()) => RateLimitDecision::Allow,
            Err(_) => RateLimitDecision::DenyTooManyRequests,
        }
    }

    /// Same decision as [`Self::check`], but on deny returns the
    /// minimum [`Duration`] the caller must wait before retrying. The
    /// keystone middleware uses this to populate the `Retry-After` header
    /// on 429 responses (defense leak #15).
    ///
    /// `Ok(())`  — request is allowed.
    /// `Err(d)`  — request must be rejected with HTTP 429; advise the
    /// client to retry no sooner than `d` from now.
    pub async fn check_with_retry(&self, org_id: Uuid) -> Result<(), Duration> {
        // Fast path — read lock, hit existing limiter.
        {
            let limiters = self.limiters.read().await;
            if let Some(entry) = limiters.get(&org_id) {
                return match entry.limiter.check() {
                    Ok(_) => Ok(()),
                    Err(not_until) => Err(not_until.wait_time_from(DefaultClock::default().now())),
                };
            }
        }
        // Cold path — write lock, create.
        let rpm = self
            .overrides
            .read()
            .await
            .get(&org_id)
            .copied()
            .unwrap_or(self.default_rpm);
        let limiter = build_limiter(rpm);
        let outcome = limiter.check();

        let mut w = self.limiters.write().await;
        // Unbounded-growth protection: before inserting a *new* key, make sure
        // the hard cap (if any) is honoured so an adversarial key stream (e.g.
        // rotating source IPs on an anonymous throttle) cannot balloon the map.
        // Only runs on the cold path (new key), so the common fast path stays
        // lock-light. Re-check `contains_key` because another writer may have
        // inserted this key while we were on the cold path.
        if !w.contains_key(&org_id) {
            self.evict_for_insert(&mut w);
        }
        w.insert(
            org_id,
            LimiterEntry {
                limiter: Arc::new(limiter),
                rpm,
                last_touched: Instant::now(),
            },
        );

        match outcome {
            Ok(_) => Ok(()),
            Err(not_until) => Err(not_until.wait_time_from(DefaultClock::default().now())),
        }
    }

    /// Sweep entries that haven't been touched for [`Self::idle_ttl`].
    /// Call periodically from a background task.
    pub async fn sweep_idle(&self) -> usize {
        let now = Instant::now();
        let mut w = self.limiters.write().await;
        let before = w.len();
        w.retain(|_, e| now.duration_since(e.last_touched) < self.idle_ttl);
        before - w.len()
    }

    /// Make room for one new key so the map never exceeds `max_entries`.
    ///
    /// Called under the write lock on the cold-path insert only. Strategy:
    /// 1. If uncapped or below the cap, do nothing.
    /// 2. Drop TTL-expired entries first (cheapest, and they were going to be
    ///    swept anyway) — this alone usually frees room.
    /// 3. If still at the cap, evict the oldest-created entries (LRU-style)
    ///    until there is room for exactly one more insert.
    ///
    /// Eviction merely resets a key's bucket to a fresh (more lenient) state;
    /// it never grants access, so evicting under pressure is safe. The cap is
    /// the DoS-relevant invariant: memory stays bounded regardless of how many
    /// distinct keys an attacker presents.
    fn evict_for_insert(&self, w: &mut HashMap<Uuid, LimiterEntry>) {
        let Some(cap) = self.max_entries else {
            return;
        };
        if w.len() < cap {
            return;
        }
        let now = Instant::now();
        w.retain(|_, e| now.duration_since(e.last_touched) < self.idle_ttl);
        if w.len() < cap {
            return;
        }
        // Still full of live entries — evict oldest-created first. Need room
        // for one more, so bring the count down to `cap - 1`.
        let overflow = w.len() + 1 - cap;
        let mut by_age: Vec<(Uuid, Instant)> =
            w.iter().map(|(k, e)| (*k, e.last_touched)).collect();
        by_age.sort_by_key(|(_, touched)| *touched);
        for (k, _) in by_age.into_iter().take(overflow) {
            w.remove(&k);
        }
    }

    /// Number of tracked tenants (for tests / metrics).
    pub async fn len(&self) -> usize {
        self.limiters.read().await.len()
    }

    /// Whether no tenants are currently tracked. Pairs with [`Self::len`] to
    /// satisfy clippy's `len_without_is_empty` lint.
    pub async fn is_empty(&self) -> bool {
        self.limiters.read().await.is_empty()
    }
}

impl Default for TenantRateLimiterSet {
    fn default() -> Self {
        Self::new()
    }
}

fn build_limiter(rpm: u32) -> RateLimiter<NotKeyed, InMemoryState, DefaultClock> {
    let nz = NonZeroU32::new(rpm.max(1)).unwrap();
    let quota = Quota::per_minute(nz);
    RateLimiter::direct(quota)
}

// ============================================================================
// Per-tenant metering (defense #19)
// ============================================================================

/// Record one request for one tenant. Emits Prometheus counters tagged with
/// `org_id` (high-cardinality but acceptable: at most one label per tenant,
/// and the tenant set is bounded by the platform).
pub fn meter_request(org_id: Uuid, response_bytes: u64) {
    let org_label = org_id.to_string();
    metrics::counter!("requests_total", "org_id" => org_label.clone()).increment(1);
    if response_bytes > 0 {
        metrics::counter!("request_bytes_total", "org_id" => org_label).increment(response_bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limiter_allows_below_quota() {
        let set = TenantRateLimiterSet::with_default(100);
        let org = Uuid::new_v4();
        for _ in 0..50 {
            assert_eq!(set.check(org).await, RateLimitDecision::Allow);
        }
    }

    #[tokio::test]
    async fn rate_limiter_denies_above_quota() {
        let set = TenantRateLimiterSet::with_default(5);
        let org = Uuid::new_v4();
        // Burn through the burst; eventually the limiter says deny.
        let mut allow = 0;
        let mut deny = 0;
        for _ in 0..50 {
            match set.check(org).await {
                RateLimitDecision::Allow => allow += 1,
                RateLimitDecision::DenyTooManyRequests => deny += 1,
            }
        }
        assert!(allow > 0, "limiter should allow at least one");
        assert!(deny > 0, "limiter must eventually deny under burst");
    }

    #[tokio::test]
    async fn limiter_isolated_per_tenant() {
        // Defense for leak #15: tenant A bursting must not affect tenant B's
        // quota.
        let set = TenantRateLimiterSet::with_default(5);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        // Saturate A.
        for _ in 0..20 {
            let _ = set.check(a).await;
        }
        // B's first call must still be Allow.
        assert_eq!(set.check(b).await, RateLimitDecision::Allow);
    }

    #[tokio::test]
    async fn override_takes_effect() {
        let set = TenantRateLimiterSet::with_default(1);
        let org = Uuid::new_v4();
        set.set_override(org, 1000).await;
        // With a generous override, we expect more allows than the default.
        let mut allow = 0;
        for _ in 0..100 {
            if set.check(org).await == RateLimitDecision::Allow {
                allow += 1;
            }
        }
        assert!(
            allow > 10,
            "override of 1000 rpm should allow many more than default 1: got {allow}"
        );
    }

    #[tokio::test]
    async fn bounded_set_never_exceeds_cap_under_key_flood() {
        // Defense: an attacker rotating source IPs (each a distinct key)
        // must NOT grow the limiter map without bound. With a hard cap the
        // map stays <= cap no matter how many unique keys arrive, WITHOUT any
        // background sweep task running.
        const CAP: usize = 64;
        let set = TenantRateLimiterSet::with_default_bounded(
            600,
            Duration::from_secs(3600), // long TTL: nothing expires during the test
            CAP,
        );
        for _ in 0..10_000 {
            // Every call is a brand-new key => always the cold-path insert.
            let _ = set.check(Uuid::new_v4()).await;
            assert!(
                set.len().await <= CAP,
                "bounded limiter map must never exceed its cap"
            );
        }
        assert_eq!(
            set.len().await,
            CAP,
            "after a large key flood the map should sit exactly at the cap"
        );
    }

    #[tokio::test]
    async fn bounded_insert_prefers_dropping_expired_over_live() {
        // With a tiny TTL, expired entries are reclaimed on insert so live
        // traffic is not collaterally evicted while stale keys linger.
        let set = TenantRateLimiterSet::with_default_bounded(600, Duration::from_millis(10), 8);
        // Fill with keys, let them go idle/expired.
        for _ in 0..8 {
            let _ = set.check(Uuid::new_v4()).await;
        }
        assert_eq!(set.len().await, 8);
        tokio::time::sleep(Duration::from_millis(30)).await;
        // Next insert triggers the TTL sweep first, reclaiming all 8 expired
        // entries, leaving just the new one.
        let _ = set.check(Uuid::new_v4()).await;
        assert_eq!(
            set.len().await,
            1,
            "TTL-expired entries must be reclaimed on the bounded insert path"
        );
    }

    #[tokio::test]
    async fn unbounded_set_keeps_growing() {
        // Sanity: the per-tenant (uncapped) construction is unchanged — it does
        // NOT evict, because its key space is platform-bounded.
        let set = TenantRateLimiterSet::with_default(600);
        for _ in 0..200 {
            let _ = set.check(Uuid::new_v4()).await;
        }
        assert_eq!(set.len().await, 200);
    }

    #[tokio::test]
    async fn sweep_evicts_idle_entries() {
        let mut set = TenantRateLimiterSet::with_default(100);
        // Force a tiny idle TTL for the test.
        set.idle_ttl = Duration::from_millis(10);
        let org = Uuid::new_v4();
        let _ = set.check(org).await;
        assert_eq!(set.len().await, 1);
        tokio::time::sleep(Duration::from_millis(50)).await;
        let dropped = set.sweep_idle().await;
        assert_eq!(dropped, 1);
        assert_eq!(set.len().await, 0);
    }
}

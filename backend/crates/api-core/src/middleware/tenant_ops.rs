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
pub struct TenantRateLimiterSet {
    /// Per-tenant limiters keyed by org id, with their last-touched time.
    limiters: Arc<RwLock<HashMap<Uuid, LimiterEntry>>>,
    default_rpm: u32,
    overrides: Arc<RwLock<HashMap<Uuid, u32>>>,
    /// How long an unused limiter entry survives before being swept.
    idle_ttl: Duration,
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

    /// Build with a custom baseline.
    pub fn with_default(default_rpm: u32) -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            default_rpm: default_rpm.max(1),
            overrides: Arc::new(RwLock::new(HashMap::new())),
            idle_ttl: Duration::from_secs(3600),
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

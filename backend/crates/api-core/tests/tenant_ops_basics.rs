//! Phase 5.5 — basic invariant tests for tenant-ops middleware primitives.
//!
//! These tests exercise the type-level + per-tenant isolation guarantees
//! without standing up a database or a real Redis. The deeper integration
//! coverage lives inside each module's `#[cfg(test)]` block.

use api_core::middleware::tenant_ops::{
    meter_request, RateLimitDecision, TenantRateLimiterSet, DEFAULT_RATE_LIMIT_RPM,
};
use uuid::Uuid;

#[tokio::test]
async fn rate_limiter_set_default_constructs() {
    let set = TenantRateLimiterSet::new();
    assert_eq!(set.len().await, 0);
}

#[tokio::test]
async fn rate_limiter_per_tenant_isolation() {
    // Defense for leak #15: tenant A bursting must not affect tenant B.
    let set = TenantRateLimiterSet::with_default(3);
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    // Saturate A.
    for _ in 0..30 {
        let _ = set.check(a).await;
    }
    // B's first call must still be Allow.
    assert_eq!(set.check(b).await, RateLimitDecision::Allow);
}

#[tokio::test]
async fn rate_limit_default_constant_is_sane() {
    // 600 rpm default: ~10 rps. Sanity-check the constant didn't get
    // accidentally dropped to a value that would brick prod.
    // (Constant comparisons trip clippy::assertions_on_constants; this is a
    // compile-time canary so the lint isn't useful here.)
    #[allow(clippy::assertions_on_constants)]
    {
        assert!(DEFAULT_RATE_LIMIT_RPM >= 60);
        assert!(DEFAULT_RATE_LIMIT_RPM <= 100_000);
    }
}

#[test]
fn meter_request_does_not_panic_without_recorder() {
    // The Prometheus recorder is wired in main.rs at boot; this test
    // confirms the meter helper is no-op-safe outside that wiring.
    let org = Uuid::new_v4();
    meter_request(org, 0);
    meter_request(org, 1024);
}

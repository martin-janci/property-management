//! Reality-server utility modules.
//!
//! Crate-internal helpers shared across route handlers. Keep this module
//! lean — anything reusable across servers belongs in `api-core` or `common`.

pub mod errors;
pub mod slug;
pub mod url;

/// Maximum row `offset` accepted by any list endpoint.
///
/// Deep pagination past this point is a performance foot-gun — Postgres must
/// walk and discard every skipped row — and, for `page`-derived offsets, an
/// integer-overflow risk. Requests asking for a larger offset are clamped down
/// to this ceiling rather than rejected, mirroring how `limit` is clamped on
/// these same handlers (see e.g. `routes::portal_listings::MyListingsQuery`).
/// Callers that legitimately need to page deeper should move to keyset
/// pagination. The value fits comfortably in both `i32` and `i64`.
pub const MAX_LIST_OFFSET: i64 = 100_000;

/// Clamp a raw `offset` query param to `0..=MAX_LIST_OFFSET` for the `i64`
/// list endpoints. Missing values default to `0`.
pub fn clamp_offset(raw: Option<i64>) -> i64 {
    raw.unwrap_or(0).clamp(0, MAX_LIST_OFFSET)
}

/// Clamp a raw `offset` query param to `0..=MAX_LIST_OFFSET` for the `i32`
/// list endpoints. Missing values default to `0`. `MAX_LIST_OFFSET` fits in
/// `i32`, so the cast is lossless.
pub fn clamp_offset_i32(raw: Option<i32>) -> i32 {
    raw.unwrap_or(0).clamp(0, MAX_LIST_OFFSET as i32)
}

/// Maximum page size (`limit`) accepted by any list endpoint.
///
/// A fixed, documented ceiling bounds the size of any single response (a
/// resource-exhaustion guard) and gives clients a predictable cap to page
/// against via `offset`. Requests asking for a larger page are clamped down
/// to this value rather than rejected, mirroring how `offset` is clamped to
/// [`MAX_LIST_OFFSET`]. This is the `1..=100` convention already used inline
/// by the sibling portal list handlers (see `routes::imports::list_import_jobs`
/// and `routes::portal_listings`); [`clamp_list_limit`] centralises it so the
/// cap lives in exactly one place.
pub const MAX_LIST_LIMIT: i32 = 100;

/// Clamp a raw `limit` query param to `1..=MAX_LIST_LIMIT`. Missing values
/// default to `default`.
///
/// A negative or zero `limit` binds into `LIMIT $n` as `LIMIT -1`/`LIMIT 0`,
/// which Postgres rejects or treats as "no rows"; an arbitrarily large value
/// is a resource-exhaustion vector. Clamping to a documented ceiling keeps
/// every list endpoint page-able (the full result set is reachable via
/// `offset`) instead of silently truncating at an undocumented hard cap.
pub fn clamp_list_limit(raw: Option<i32>, default: i32) -> i32 {
    raw.unwrap_or(default).clamp(1, MAX_LIST_LIMIT)
}

/// Process-wide mutex for tests that mutate global env vars (e.g. `RUST_ENV`).
///
/// Rust's test harness runs tests in parallel by default, so any test that
/// touches a shared env var has to serialize against every other such test
/// in the same process. Use [`env_lock`] in any test that reads or writes
/// `RUST_ENV` (and friends) to avoid flaky cross-test interference.
#[cfg(test)]
pub(crate) fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    // Tests may panic while holding the guard; recovering from a poisoned
    // mutex is fine because the protected data is `()`.
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod offset_tests {
    use super::*;

    #[test]
    fn clamp_offset_defaults_to_zero() {
        assert_eq!(clamp_offset(None), 0);
        assert_eq!(clamp_offset_i32(None), 0);
    }

    #[test]
    fn clamp_offset_floors_negative_at_zero() {
        assert_eq!(clamp_offset(Some(-1)), 0);
        assert_eq!(clamp_offset(Some(i64::MIN)), 0);
        assert_eq!(clamp_offset_i32(Some(-1)), 0);
        assert_eq!(clamp_offset_i32(Some(i32::MIN)), 0);
    }

    #[test]
    fn clamp_offset_passes_values_within_bounds() {
        assert_eq!(clamp_offset(Some(0)), 0);
        assert_eq!(clamp_offset(Some(500)), 500);
        assert_eq!(clamp_offset(Some(MAX_LIST_OFFSET)), MAX_LIST_OFFSET);
        assert_eq!(clamp_offset_i32(Some(500)), 500);
    }

    #[test]
    fn clamp_offset_caps_pathological_values() {
        // Arbitrarily large offsets must be capped, never handed to SQL raw.
        assert_eq!(clamp_offset(Some(i64::MAX)), MAX_LIST_OFFSET);
        assert_eq!(clamp_offset(Some(MAX_LIST_OFFSET + 1)), MAX_LIST_OFFSET);
        assert_eq!(clamp_offset_i32(Some(i32::MAX)), MAX_LIST_OFFSET as i32);
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;

    #[test]
    fn clamp_list_limit_defaults_when_absent() {
        assert_eq!(clamp_list_limit(None, 20), 20);
        assert_eq!(clamp_list_limit(None, 50), 50);
    }

    #[test]
    fn clamp_list_limit_floors_non_positive_at_one() {
        // A zero or negative limit must never reach SQL (LIMIT 0 / LIMIT -1).
        assert_eq!(clamp_list_limit(Some(0), 20), 1);
        assert_eq!(clamp_list_limit(Some(-1), 20), 1);
        assert_eq!(clamp_list_limit(Some(i32::MIN), 20), 1);
    }

    #[test]
    fn clamp_list_limit_passes_values_within_bounds() {
        assert_eq!(clamp_list_limit(Some(1), 20), 1);
        assert_eq!(clamp_list_limit(Some(37), 20), 37);
        assert_eq!(clamp_list_limit(Some(MAX_LIST_LIMIT), 20), MAX_LIST_LIMIT);
    }

    #[test]
    fn clamp_list_limit_caps_at_documented_ceiling() {
        // The whole point of the fix: a caller asking for more than the cap is
        // clamped to it (and pages past it with `offset`), never silently
        // truncated at an undocumented hard limit.
        assert_eq!(
            clamp_list_limit(Some(MAX_LIST_LIMIT + 1), 20),
            MAX_LIST_LIMIT
        );
        assert_eq!(clamp_list_limit(Some(10_000), 20), MAX_LIST_LIMIT);
        assert_eq!(clamp_list_limit(Some(i32::MAX), 20), MAX_LIST_LIMIT);
    }
}

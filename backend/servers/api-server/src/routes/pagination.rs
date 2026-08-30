//! Shared pagination helpers for list endpoints.
//!
//! List handlers accept a client-supplied `?limit=N`. Passing that value
//! straight into a SQL `LIMIT` lets a caller request an unbounded number of
//! rows — a cheap denial-of-service. These helpers clamp the value at the
//! handler boundary into a sane range before it ever reaches the repository
//! layer (see the same `.min(100)` pattern already used by
//! `admin/users.rs`, `admin/audit.rs`, and `market_pricing.rs`).

/// Default upper bound for a page size when a handler does not override it.
/// Matches the pervasive `.min(100)` clamp used across the data layer.
pub const MAX_PAGE_LIMIT: i64 = 100;

/// Clamp a client-supplied `limit` into `[1, MAX_PAGE_LIMIT]`, falling back to
/// `default` when the caller omits it.
///
/// The lower bound of `1` also neutralises hostile non-positive values
/// (`?limit=0`, `?limit=-1`) that would otherwise be cast to a huge `usize`
/// or produce a `LIMIT` Postgres rejects.
pub fn clamp_limit(limit: Option<i64>, default: i64) -> i64 {
    clamp_limit_capped(limit, default, MAX_PAGE_LIMIT)
}

/// Like [`clamp_limit`] but with a caller-specified upper bound, for endpoints
/// whose documented contract allows a larger page (e.g. audit-log exports).
pub fn clamp_limit_capped(limit: Option<i64>, default: i64, max: i64) -> i64 {
    limit.unwrap_or(default).clamp(1, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_absent() {
        assert_eq!(clamp_limit(None, 50), 50);
    }

    #[test]
    fn passes_through_in_range() {
        assert_eq!(clamp_limit(Some(25), 50), 25);
    }

    #[test]
    fn caps_at_max() {
        // IG3: a hostile `?limit=1000000` must not exceed the cap.
        assert_eq!(clamp_limit(Some(1_000_000), 50), MAX_PAGE_LIMIT);
        assert_eq!(clamp_limit(Some(i64::MAX), 50), MAX_PAGE_LIMIT);
    }

    #[test]
    fn floors_non_positive() {
        assert_eq!(clamp_limit(Some(0), 50), 1);
        assert_eq!(clamp_limit(Some(-1), 50), 1);
        assert_eq!(clamp_limit(Some(i64::MIN), 50), 1);
    }

    #[test]
    fn capped_variant_respects_custom_max() {
        assert_eq!(clamp_limit_capped(Some(5_000), 100, 1_000), 1_000);
        assert_eq!(clamp_limit_capped(Some(500), 100, 1_000), 500);
        assert_eq!(clamp_limit_capped(None, 100, 1_000), 100);
        assert_eq!(clamp_limit_capped(Some(-5), 100, 1_000), 1);
    }
}

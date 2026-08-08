//! Phase 5 — Admin route tree.
//!
//! Mounts under `/api/v1/admin`. Three trees coexist here:
//!
//!   * `users_lifecycle` — pre-Phase-5 user CRUD (Epic 1, Story 1.6). Kept
//!     under its existing paths via `lifecycle_router()`.
//!   * `memberships` — Phase 2 invite/accept/revoke sub-router for the
//!     unified identity model.
//!   * Phase 5 sub-routes (`agencies`, `users`, `audit`, `capabilities`,
//!     `impersonation`) — every endpoint goes through `RequireCapability`.
//!
//! The Phase 5 routers all assume `AdminDeps` and `AppState` have been wired
//! into request extensions by `lib.rs::create_router`. Without those, the
//! extractor returns `AdminError::Internal`.

pub mod agencies;
pub mod audit;
pub mod capabilities;
pub mod impersonation;
pub mod memberships;
pub mod metrics;
pub mod mfa;
pub mod mobile_config;
pub mod notifications;
pub mod users;
pub mod users_lifecycle;

// `main.rs` and the OpenAPI registry reference the pre-Phase-5 admin
// handlers via `routes::admin::users_lifecycle::*` directly — utoipa's
// `__path_*` items don't follow `pub use` re-exports, so the registry
// must use the real module path.
pub use users_lifecycle::{
    AdminActionResponse, AdminUserInfo, ListUsersQuery, ListUsersResponse, UserActionRequest,
};

use axum::http::HeaderMap;
use axum::Router;

use crate::state::AppState;

/// Maximum width of `audit_logs.ip_address` (VARCHAR(45)) — covers IPv6.
/// Defined here so the audit-row construction never silently fails its
/// INSERT because an upstream proxy passed back a comma-separated chain.
pub(super) const AUDIT_IP_MAX_LEN: usize = 45;

/// Extract the client IP for audit logging from the `x-forwarded-for`
/// header.
///
/// `x-forwarded-for` is canonically a comma-separated list (`client, proxy1,
/// proxy2`). The leftmost entry is the original client; downstream entries
/// are intermediaries. We take the first entry, trim whitespace, and
/// truncate at [`AUDIT_IP_MAX_LEN`] (45) so the value always fits the
/// `audit_logs.ip_address` column. Returning the raw header value risks
/// silently dropping the audit row when the column rejects an overlong
/// chain — see PR #300 review.
///
/// Returns `None` when the header is absent, non-ASCII, or empty after
/// trimming, matching the previous `.and_then(|h| h.to_str().ok())`
/// behaviour at the call sites.
pub(super) fn audit_ip_from_headers(headers: &HeaderMap) -> Option<&str> {
    let raw = headers.get("x-forwarded-for")?.to_str().ok()?;
    let first = raw.split(',').next()?.trim();
    if first.is_empty() {
        return None;
    }
    // Truncate on a char boundary; ASCII IPs are 1-byte chars so this is a
    // no-op for valid IPs but keeps us safe if a proxy injects garbage.
    let end = first
        .char_indices()
        .take_while(|(i, _)| *i < AUDIT_IP_MAX_LEN)
        .last()
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    Some(&first[..end])
}

/// Combined `/api/v1/admin` router. Layered with admin extensions in
/// `lib.rs::create_router`.
pub fn router() -> Router<AppState> {
    Router::new()
        // Pre-Phase-5 user-lifecycle endpoints (Epic 1, Story 1.6) own
        // `/users`, `/users/{id}`, `/users/{id}/{suspend,reactivate,delete}`.
        .merge(users_lifecycle::lifecycle_router())
        // Phase 2 membership invite/accept/revoke.
        .merge(memberships::router())
        // Phase 5 admin tree (capability-gated). The Phase 5 admin/users
        // module owns global user search + principal_kind transitions —
        // mounted under /principals to avoid colliding with the legacy
        // /users routes from users_lifecycle above.
        .nest("/agencies", agencies::router())
        .nest("/principals", users::router())
        .nest("/audit", audit::router())
        .nest("/capabilities", capabilities::router())
        .nest("/impersonation", impersonation::router())
        .nest("/mfa", mfa::router())
        .nest("/metrics", metrics::router())
        .nest("/mobile-config", mobile_config::router())
        .nest("/notifications", notifications::router())
}

#[cfg(test)]
mod tests {
    use super::audit_ip_from_headers;
    use axum::http::HeaderMap;

    fn hdr(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", value.parse().unwrap());
        h
    }

    #[test]
    fn returns_none_when_header_absent() {
        assert_eq!(audit_ip_from_headers(&HeaderMap::new()), None);
    }

    #[test]
    fn returns_first_ip_from_chain() {
        let h = hdr("203.0.113.7, 198.51.100.1, 10.0.0.1");
        assert_eq!(audit_ip_from_headers(&h), Some("203.0.113.7"));
    }

    #[test]
    fn trims_whitespace() {
        let h = hdr("  203.0.113.7  ");
        assert_eq!(audit_ip_from_headers(&h), Some("203.0.113.7"));
    }

    #[test]
    fn returns_none_for_empty_first_entry() {
        let h = hdr("   , 198.51.100.1");
        assert_eq!(audit_ip_from_headers(&h), None);
    }

    #[test]
    fn truncates_to_45_chars() {
        // A pathological 60-char garbage value — still fits VARCHAR(45).
        let garbage = "x".repeat(60);
        let h = hdr(&garbage);
        assert_eq!(audit_ip_from_headers(&h).unwrap().len(), 45);
    }

    #[test]
    fn ipv6_fits_unchanged() {
        let v6 = "2001:0db8:85a3:0000:0000:8a2e:0370:7334"; // 39 chars
        let h = hdr(v6);
        assert_eq!(audit_ip_from_headers(&h), Some(v6));
    }
}

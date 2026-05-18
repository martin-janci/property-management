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
pub mod users;
pub mod users_lifecycle;

// `main.rs` and the OpenAPI registry reference the pre-Phase-5 admin
// handlers via `routes::admin::users_lifecycle::*` directly — utoipa's
// `__path_*` items don't follow `pub use` re-exports, so the registry
// must use the real module path.
pub use users_lifecycle::{
    AdminActionResponse, AdminUserInfo, ListUsersQuery, ListUsersResponse, UserActionRequest,
};

use axum::Router;

use crate::state::AppState;

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
}

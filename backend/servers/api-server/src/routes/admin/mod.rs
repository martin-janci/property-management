//! Phase 5 — Admin route tree.
//!
//! Mounts under `/api/v1/admin`. Two trees coexist here:
//!
//!   * `users_lifecycle` — pre-Phase-5 user CRUD (Epic 1, Story 1.6). Kept
//!     under its existing paths.
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
pub mod users;
pub mod users_lifecycle;

// Backwards compatibility re-exports — `main.rs` and the OpenAPI registry
// reference the pre-Phase-5 admin handlers under `routes::admin::*`. We keep
// those paths working by re-exporting from the moved file.
pub use users_lifecycle::{
    delete_user, get_user, list_users, reactivate_user, suspend_user, AdminActionResponse,
    AdminUserInfo, ListUsersQuery, ListUsersResponse, UserActionRequest,
};

use axum::Router;

use crate::state::AppState;

/// Combined `/api/v1/admin` router. Layered with admin extensions in
/// `lib.rs::create_router`.
pub fn router() -> Router<AppState> {
    Router::new()
        // Pre-Phase-5 user-lifecycle endpoints (Epic 1, Story 1.6).
        .merge(users_lifecycle::lifecycle_router())
        // Phase 5 admin tree.
        .nest("/agencies", agencies::router())
        .nest("/users", users::router())
        .nest("/audit", audit::router())
        .nest("/capabilities", capabilities::router())
        .nest("/impersonation", impersonation::router())
}

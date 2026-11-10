//! Reality Portal Professional repository (Epics 31-34).
//!
//! Repository for agencies, realtors, inquiries, and property import.
//!
//! # Runtime `sqlx::query` convention (#1852 finding-1)
//!
//! Every query in this module uses the runtime, *unchecked* `sqlx::query` /
//! `query_as` forms rather than the compile-time-checked `query!` macros. This
//! is a deliberate, **repo-wide** convention, not an oversight: the checked
//! macros require a live DB connection (or a committed `.sqlx/` offline cache
//! via `cargo sqlx prepare`) at build time, but this workspace compiles DB-free
//! — CI runs `SQLX_OFFLINE=true` and there is **no `.sqlx/` cache** in the tree
//! (same rationale documented at `rental.rs::find_airbnb_connection_by_listing_id`
//! and `platform_admin.rs`). A `query!` added here in isolation would fail the
//! `check` / `fmt-clippy` gates. A column rename therefore won't be caught at
//! compile time; the DB-backed `#[sqlx::test]` suites are the guard until a
//! workspace-wide offline-cache workflow exists.
//!
//! # Module layout
//!
//! The repository surface is large (saved searches & favorites, agencies,
//! realtor profiles, inquiries, portal listings and property import). To keep
//! each area readable and to reduce the churn surface of any single file, the
//! `impl RealityPortalRepository` block is split across cohesive sub-modules.
//! Every sub-module adds methods to the *same* [`RealityPortalRepository`]
//! struct defined here:
//!
//! - [`searches`]  — saved-search alert engine, favorites & saved searches (Stories 16.3, 31.1-31.4)
//! - [`agencies`]  — agency CRUD, branding, members & invitations (Story 32.x)
//! - [`realtors`]  — realtor profiles (Story 33.1)
//! - [`inquiries`] — listing inquiries & responses (Story 33.3)
//! - [`listings`]  — portal listing CRUD, views & analytics (Epic 15, Story 33.4)
//! - [`imports`]   — import jobs & feed subscriptions (Epic 34)

use crate::DbPool;

mod agencies;
mod imports;
mod inquiries;
mod listings;
mod realtors;
mod searches;

#[cfg(test)]
mod agencies_test;
#[cfg(test)]
mod tests;

pub use searches::{FavoriteAlertReadOutcome, SavedSearchError, MAX_SAVED_SEARCHES_PER_USER};

/// Repository for Reality Portal Professional operations.
#[derive(Clone)]
pub struct RealityPortalRepository {
    pub(super) pool: DbPool,
}

impl RealityPortalRepository {
    /// Create a new RealityPortalRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

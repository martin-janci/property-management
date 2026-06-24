//! Reality Portal Professional repository (Epics 31-34).
//!
//! Repository for agencies, realtors, inquiries, and property import.
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
mod tests;

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

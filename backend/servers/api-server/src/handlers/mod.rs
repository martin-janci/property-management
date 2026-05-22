//! Business logic handlers.
//!
//! Each handler module contains the actual implementation logic,
//! while routes handle HTTP concerns (request/response, validation).
//!
//! Note: rentals, listings, organizations, and integrations functionality
//! is implemented directly in the routes/ modules.

pub mod auth;
pub mod buildings;

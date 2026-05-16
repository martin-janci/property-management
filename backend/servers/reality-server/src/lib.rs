//! Reality Server library — exposes crate internals for integration tests.
//!
//! The production entry point is `main.rs`. This module re-exports the
//! sub-modules so integration tests in `tests/` can reference them without
//! duplicating the `mod` declarations.

// Allow dead code for stub implementations during development
#![allow(clippy::doc_overindented_list_items)]
#![allow(dead_code)]

pub mod extractors;
pub mod handlers;
pub mod observability;
pub mod routes;
pub mod state;

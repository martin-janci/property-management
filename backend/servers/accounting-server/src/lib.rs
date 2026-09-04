//! Accounting Server library — exposes crate internals for integration tests.
//!
//! The production entry point is `main.rs`. This module re-exports the
//! sub-modules so integration tests in `tests/` can reference them without
//! duplicating the `mod` declarations. Mirrors reality-server's lib/bin split.

// The crate's production entry is `main.rs`, which redeclares the sub-modules
// itself. This lib target exists ONLY so integration tests in `tests/` can
// reach internals — helpers reachable from main.rs are unused from the lib's
// perspective and would otherwise surface as `dead_code` warnings here even
// though they're live in the binary. Real dead code is still caught by clippy
// on the `bin` target. The allow is crate-wide because rustc has no built-in
// way to scope "dead from this target only" warnings.
#![allow(dead_code)]
#![allow(unused)]

pub mod observability;
pub mod pdf;
pub mod routes;
pub mod state;

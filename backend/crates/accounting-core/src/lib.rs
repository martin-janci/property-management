//! accounting-core — pure domain logic for the ACC Invoicing/Accounting MVP.
//!
//! This crate holds platform-independent domain logic for the accounting
//! server: shared types, money/VAT/rounding math, invoice numbering, and the
//! cross-cutting authorization + audit helpers. It depends only on
//! `common` + `db` (for shared models), never on axum/tokio/web concerns — the
//! same separation reality-server's domain logic follows.
//!
//! Architecture: see `.research/accounting/architecture.md` (option C).
//!
//! ## Module map & ownership (Phase 1 coders)
//! - `errors` — domain error enum (Foundation; stable).
//! - `types`  — shared value types used across modules (Foundation; stable).
//! - `money`  — line/total money math (BE-Invoicing).
//! - `vat`    — VAT grouping/computation (BE-Invoicing).
//! - `rounding` — rounding per mode (BE-Invoicing).
//! - `numbering` — series format + sequence helpers (BE-Config).
//! - `authz`  — CROSS-CUTTING role checks; called by EVERY route file (BE-Platform).
//! - `audit`  — CROSS-CUTTING audit-entry builder; called by mutating handlers (BE-Platform).

// Phase 0b ships SIGNATURES with `todo!()` bodies. Allow during active dev,
// mirroring reality-server's crate-wide allow.
#![allow(dead_code)]
#![allow(unused)]

pub mod audit;
pub mod bysquare;
pub mod crypto;
pub mod errors;
pub mod money;
pub mod numbering;
pub mod rounding;
pub mod types;
pub mod vat;

pub mod authz;

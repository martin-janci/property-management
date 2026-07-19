//! Layout & Content Manager contract: config types, merge resolver, validation.
//!
//! Spec: docs/superpowers/specs/2026-07-19-layout-content-manager-design.md

pub mod types;
pub mod resolve;
pub mod validate;

pub use types::*;
pub use resolve::resolve;
pub use validate::{validate_publish, validate_tenant_override, ValidationError};

//! Reality-server utility modules.
//!
//! Crate-internal helpers shared across route handlers. Keep this module
//! lean — anything reusable across servers belongs in `api-core` or `common`.

pub mod errors;
pub mod url_validator;

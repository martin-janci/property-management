//! Domain error type for accounting-core.
//!
//! A single `thiserror`-derived enum the domain modules (money/VAT,
//! numbering, authz, audit) return. Web/transport mapping (e.g. to HTTP
//! StatusCode) lives in accounting-server, not here — this crate stays
//! transport-agnostic.

use thiserror::Error;

/// Errors surfaced by accounting-core domain logic.
#[derive(Debug, Error)]
pub enum AccountingError {
    /// A monetary/VAT/rounding computation received invalid input
    /// (e.g. negative quantity, unsupported currency).
    #[error("invalid amount or computation input: {0}")]
    InvalidAmount(String),

    /// A supplied value failed domain validation (numbering format,
    /// field constraints, etc.).
    #[error("validation failed: {0}")]
    Validation(String),

    /// An invoice/document numbering operation failed (e.g. exhausted
    /// sequence, malformed template).
    #[error("numbering error: {0}")]
    Numbering(String),

    /// The current principal is not authorized for the requested
    /// accounting operation.
    #[error("not authorized: {0}")]
    Unauthorized(String),

    /// A requested entity could not be found within the tenant scope.
    #[error("not found: {0}")]
    NotFound(String),

    /// A persistence-layer error bubbled up from the database.
    #[error("database error: {0}")]
    Database(String),

    /// A catch-all for internal invariants that should not normally occur.
    #[error("internal accounting error: {0}")]
    Internal(String),
}

/// Convenience result alias for accounting-core domain operations.
pub type Result<T> = std::result::Result<T, AccountingError>;

//! Public-surface error helpers.
//!
//! Reality-server is internet-facing. Raw `sqlx::Error` / `anyhow::Error`
//! strings can leak column names, constraint names, sometimes pool / TLS
//! state — none of which a portal client should ever see. Use [`db_error`]
//! at the `map_err` boundary of every DB call: it writes the full error to
//! the tracing log (where ops can still find it) and returns a generic
//! `500 Internal server error` to the caller.
//!
//! Pattern mirrors the inline `db_error` helper in
//! `routes::listings::get_listing` — this module hoists it so every route
//! file gets the same treatment without copy-pasting.

use axum::http::StatusCode;

/// Log a database / infrastructure error server-side and return a generic
/// `(500, "Internal server error")` tuple suitable for `?` on Axum handlers
/// that return `Result<_, (StatusCode, String)>`.
///
/// `ctx` is a short human description of the operation that failed
/// (e.g. `"list inquiries"`, `"acquire db connection"`). It is included in
/// the server-side log line but NOT returned to the client.
///
/// The `Display` bound (rather than `sqlx::Error`) keeps the helper usable
/// for anyhow, serde_json, repository-wrapped errors, etc.
pub fn db_error(ctx: &str, e: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::error!(context = ctx, error = %e, "[{}] {}", ctx, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The client-facing body must be a fixed generic string and a 500 — never
    /// the underlying error. This is the security contract reality-server relies
    /// on: raw `sqlx::Error` / `serde` / connection strings can expose column
    /// names, constraint names, and pool/TLS state to internet clients.
    #[test]
    fn db_error_returns_generic_500_body() {
        let (status, body) = db_error("acquire db connection", "boom");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "Internal server error");
    }

    /// The returned body must not carry any fragment of the source error, even
    /// when that error string contains schema/constraint/connection detail that
    /// a raw `format!("...: {e}")` boundary would have leaked.
    #[test]
    fn db_error_body_leaks_no_source_detail() {
        // Mimics the kind of text a raw sqlx::Error can surface.
        let sensitive = "error returned from database: duplicate key value \
             violates unique constraint \"realtor_profiles_user_id_key\" \
             DETAIL: Key (user_id)=(...) already exists; \
             host=db.internal port=5432 dbname=reality sslmode=require";
        let (status, body) = db_error("create realtor profile", sensitive);

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "Internal server error");
        // None of the sensitive tokens may appear in the client-facing body.
        for needle in [
            "constraint",
            "realtor_profiles_user_id_key",
            "DETAIL",
            "user_id",
            "host=",
            "dbname=",
            "sslmode",
            "duplicate key",
        ] {
            assert!(
                !body.contains(needle),
                "client body leaked DB detail: found `{needle}` in `{body}`"
            );
        }
    }

    /// The `Display` bound (not `sqlx::Error`) keeps the helper reusable for
    /// anyhow / serde / repository-wrapped errors — all of which must be
    /// scrubbed identically.
    #[test]
    fn db_error_accepts_any_display_error() {
        let serde_err = serde_json::from_str::<serde_json::Value>("{ not json")
            .expect_err("invalid json should fail to parse");
        let (status, body) = db_error("parse stored layout config", serde_err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "Internal server error");
    }
}

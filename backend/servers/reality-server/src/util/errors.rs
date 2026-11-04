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

/// Semantic class of a database error, derived from **structured** signals —
/// the [`sqlx::Error`] variant and (for driver errors) the typed
/// [`sqlx::error::ErrorKind`] that sqlx maps from the Postgres SQLSTATE code —
/// never from the human-readable `Display`/message text.
///
/// Message text is a moving target across sqlx and Postgres releases, so keying
/// HTTP status off `err.to_string().contains("…")` is brittle and breaks
/// silently on upgrade. It was also outright *wrong* for some callers:
/// `sqlx::Error::RowNotFound` renders as
/// `"no rows returned by a query that expected to return at least one row"`,
/// which does NOT contain the phrase `"not found"` — so the old
/// `contains("not found")` checks in the realtor/agency handlers never matched
/// and silently fell through to a 500. This enum is the structured replacement
/// (reality-server code-review finding "string-typed status codes").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbErrorClass {
    /// A query that expected exactly one row returned none
    /// (`sqlx::Error::RowNotFound`).
    NotFound,
    /// Unique-constraint violation — Postgres SQLSTATE `23505`
    /// (`sqlx::error::ErrorKind::UniqueViolation`).
    UniqueViolation,
    /// Foreign-key violation — Postgres SQLSTATE `23503`
    /// (`sqlx::error::ErrorKind::ForeignKeyViolation`).
    ForeignKeyViolation,
    /// Any other database/driver error — treated as an opaque internal failure.
    Other,
}

/// Classify a [`sqlx::Error`] into a [`DbErrorClass`] using only structured
/// signals: the error variant and, for driver errors, the typed
/// [`sqlx::error::ErrorKind`] (mapped by sqlx from the SQLSTATE code). The
/// free-text message is never inspected.
pub fn classify_db_error(e: &sqlx::Error) -> DbErrorClass {
    if matches!(e, sqlx::Error::RowNotFound) {
        return DbErrorClass::NotFound;
    }
    match e.as_database_error().map(|db| db.kind()) {
        Some(sqlx::error::ErrorKind::UniqueViolation) => DbErrorClass::UniqueViolation,
        Some(sqlx::error::ErrorKind::ForeignKeyViolation) => DbErrorClass::ForeignKeyViolation,
        _ => DbErrorClass::Other,
    }
}

/// Name of the constraint a driver error violated, when the error carries one
/// (`sqlx::error::DatabaseError::constraint`).
///
/// A constraint name (e.g. `listing_inquiries_realtor_id_fkey`) is a stable
/// schema identifier fixed by the migrations, not a human-readable message, so
/// matching on it to disambiguate *which* constraint tripped survives
/// driver/Postgres message rewording.
pub fn db_constraint(e: &sqlx::Error) -> Option<&str> {
    e.as_database_error().and_then(|db| db.constraint())
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

    /// A minimal [`sqlx::error::DatabaseError`] so the unique/FK branches of
    /// [`classify_db_error`] can be exercised without a live Postgres. Mirrors
    /// the `MockUniqueViolation` pattern used in api-server's portal_webhooks
    /// tests, generalised to carry an arbitrary `kind`/`code`/`constraint`.
    /// `sqlx::error::ErrorKind` is neither `Copy` nor `Clone`, so the mock
    /// stores this small marker and reconstructs the real `ErrorKind` on
    /// demand in `kind()`.
    #[derive(Debug, Clone, Copy)]
    enum MockKind {
        Unique,
        ForeignKey,
        Check,
    }

    #[derive(Debug)]
    struct MockDbError {
        kind: MockKind,
        code: &'static str,
        constraint: Option<&'static str>,
    }

    impl std::fmt::Display for MockDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Deliberately a message that does NOT contain the human words the
            // old substring checks looked for, to prove classification does not
            // depend on message text.
            f.write_str("mock database error")
        }
    }

    impl std::error::Error for MockDbError {}

    impl sqlx::error::DatabaseError for MockDbError {
        fn message(&self) -> &str {
            "mock database error"
        }
        fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
            Some(std::borrow::Cow::Borrowed(self.code))
        }
        fn constraint(&self) -> Option<&str> {
            self.constraint
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            match self.kind {
                MockKind::Unique => sqlx::error::ErrorKind::UniqueViolation,
                MockKind::ForeignKey => sqlx::error::ErrorKind::ForeignKeyViolation,
                MockKind::Check => sqlx::error::ErrorKind::CheckViolation,
            }
        }
    }

    fn db_err(kind: MockKind, code: &'static str, constraint: Option<&'static str>) -> sqlx::Error {
        sqlx::Error::Database(Box::new(MockDbError {
            kind,
            code,
            constraint,
        }))
    }

    /// `RowNotFound` must classify as [`DbErrorClass::NotFound`] via the enum
    /// variant, independently of its `Display` text. This is the regression
    /// lock for the whole finding: status derivation no longer reads the
    /// message. It also pins the concrete brittleness that motivated the
    /// change — `RowNotFound` renders as "no rows returned by a query that
    /// expected to return at least one row", which does NOT contain the phrase
    /// `"not found"`, so the old `contains("not found")` checks in the
    /// realtor/agency handlers silently fell through to a 500.
    #[test]
    fn row_not_found_classifies_without_reading_message_text() {
        let e = sqlx::Error::RowNotFound;
        let msg = e.to_string();
        assert!(
            !msg.contains("not found"),
            "RowNotFound Display text is `{msg}` — the old `contains(\"not found\")` \
             checks were matching text that isn't there"
        );
        assert_eq!(classify_db_error(&e), DbErrorClass::NotFound);
    }

    #[test]
    fn unique_violation_classifies_by_kind() {
        let e = db_err(MockKind::Unique, "23505", None);
        assert_eq!(classify_db_error(&e), DbErrorClass::UniqueViolation);
    }

    #[test]
    fn foreign_key_violation_classifies_by_kind() {
        let e = db_err(
            MockKind::ForeignKey,
            "23503",
            Some("listing_inquiries_realtor_id_fkey"),
        );
        assert_eq!(classify_db_error(&e), DbErrorClass::ForeignKeyViolation);
    }

    #[test]
    fn non_matching_errors_classify_as_other() {
        assert_eq!(
            classify_db_error(&sqlx::Error::PoolClosed),
            DbErrorClass::Other
        );
        let check = db_err(MockKind::Check, "23514", None);
        assert_eq!(classify_db_error(&check), DbErrorClass::Other);
    }

    /// `db_constraint` surfaces the typed constraint name for driver errors and
    /// `None` for non-driver errors, so handlers can disambiguate *which*
    /// foreign key tripped (listing vs realtor) off the schema identifier
    /// rather than scanning the message.
    #[test]
    fn db_constraint_returns_name_for_driver_errors_only() {
        let fk = db_err(
            MockKind::ForeignKey,
            "23503",
            Some("listing_inquiries_realtor_id_fkey"),
        );
        assert_eq!(
            db_constraint(&fk),
            Some("listing_inquiries_realtor_id_fkey")
        );
        assert_eq!(db_constraint(&sqlx::Error::RowNotFound), None);
    }
}

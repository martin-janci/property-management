//! Shared query DTOs, the crate-local `ApiResult` alias, and error-mapping
//! helpers for the reserve-fund route surface.
//!
//! These were lifted verbatim out of the former single-file `reserve_funds.rs`
//! during the move-only split; behaviour is unchanged. Sub-modules import them
//! via `use super::shared::*`.

use api_core::extractors::RlsConnection;
use axum::{http::StatusCode, Json};
use common::errors::ErrorResponse;
use common::TenantRole;
use db::models::reserve_funds::FundType;
use serde::Deserialize;
use uuid::Uuid;

/// Result alias used by every reserve-fund handler.
pub(super) type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct FundListQuery {
    pub fund_type: Option<FundType>,
    pub building_id: Option<Uuid>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ActiveOnlyQuery {
    pub active_only: Option<bool>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Require a manager-class role (`manager`, `org_admin`, or a platform admin)
/// for a mutation.
///
/// The role comes from the `RlsConnection`, which `ValidatedTenantExtractor`
/// resolves from the caller's `organization_members` row (the database), never
/// from the JWT. On rejection the connection is released before returning
/// `403`, so the RLS context is cleared on this path too.
pub(super) async fn require_manager(
    rls: &mut RlsConnection,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if rls.has_role(TenantRole::Manager) {
        return Ok(());
    }
    rls.release().await;
    Err((
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::new(
            "FORBIDDEN",
            "Manager role required for this operation",
        )),
    ))
}

pub(super) fn internal_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::internal_error(msg)),
    )
}

pub(super) fn not_found_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::NOT_FOUND, Json(ErrorResponse::not_found(msg)))
}

/// Map a repository error to an HTTP response.
///
/// Org-scoped repo methods surface a foreign-tenant or missing row as
/// [`sqlx::Error::RowNotFound`]; we render that as a 404 (never 500, and
/// never leaking that the row exists in another org) — this is the
/// caller-facing half of the #810 IDOR fix. Any other error is a genuine
/// 500.
pub(super) fn repo_error(
    action: &str,
    not_found_msg: &str,
    e: sqlx::Error,
) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        sqlx::Error::RowNotFound => not_found_error(not_found_msg),
        other => internal_error(&format!("Failed to {}: {}", action, other)),
    }
}

/// Map an INSERT error on a child table to 404 when the parent fund is not
/// visible in the caller's RLS context (`42501` — RLS WITH CHECK rejected)
/// or does not exist (`23503` — FK violation); anything else is 500.
pub(super) fn insert_child_error(
    action: &str,
    not_found_msg: &str,
    e: sqlx::Error,
) -> (StatusCode, Json<ErrorResponse>) {
    if let sqlx::Error::Database(ref db_err) = e {
        if matches!(db_err.code().as_deref(), Some("42501") | Some("23503")) {
            return not_found_error(not_found_msg);
        }
    }
    match e {
        sqlx::Error::RowNotFound => not_found_error(not_found_msg),
        other => internal_error(&format!("Failed to {}: {}", action, other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "reserve_fund_ledger.secret_column: internal constraint detail";

    fn leaky_db_error() -> sqlx::Error {
        // A non-`RowNotFound`, non-`Database` error whose `Display`/`Debug`
        // carries internals (table/column names, SQL fragments, connection
        // detail) that a client must never see.
        sqlx::Error::Protocol(SECRET.to_string())
    }

    /// `repo_error` funnels every non-404 failure to the client. The body must
    /// never echo the raw `sqlx::Error` — regression guard mirroring
    /// `routes::budgets::db_error`.
    #[test]
    fn repo_error_never_leaks_raw_sqlx_detail() {
        let (status, body) = repo_error("list funds", "Fund not found", leaky_db_error());
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        let json = serde_json::to_string(&body.0).expect("serialize error body");
        assert!(
            !json.contains(SECRET),
            "repo_error leaked raw sqlx detail to the client: {json}"
        );
    }

    /// `insert_child_error`'s fallthrough (an error that is neither a 404 nor an
    /// RLS/FK child-insert rejection) must likewise stay generic.
    #[test]
    fn insert_child_error_never_leaks_raw_sqlx_detail() {
        let (status, body) =
            insert_child_error("record transaction", "Fund not found", leaky_db_error());
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        let json = serde_json::to_string(&body.0).expect("serialize error body");
        assert!(
            !json.contains(SECRET),
            "insert_child_error leaked raw sqlx detail to the client: {json}"
        );
    }
}

//! Shared query DTOs, the crate-local `ApiResult` alias, and error-mapping
//! helpers for the reserve-fund route surface.
//!
//! These were lifted verbatim out of the former single-file `reserve_funds.rs`
//! during the move-only split; behaviour is unchanged. Sub-modules import them
//! via `use super::shared::*`.

use axum::{http::StatusCode, Json};
use common::errors::ErrorResponse;
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

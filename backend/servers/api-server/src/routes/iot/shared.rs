//! Shared error helpers and DTOs for the IoT route surfaces.
//!
//! These items are split out of the per-surface handler modules (`sensors`,
//! `readings`, `thresholds`, `alerts`, `correlations`, `dashboard`,
//! `realtime`) so each surface can `use super::shared::*` without duplicating
//! the error-mapping helpers or the shared response/query DTOs.
//!
//! # RLS (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on every sensor table
//! (`sensors`, `sensor_readings`, `sensor_alerts`, `sensor_thresholds`,
//! `sensor_threshold_templates`, `sensor_fault_correlations`), so every query
//! MUST run on a connection that has `app.current_org_id` set or it collapses to
//! deny-all. Each handler therefore acquires an
//! [`RlsConnection`](api_core::extractors::RlsConnection) (which validates tenant
//! membership and sets the org/user GUCs on a dedicated connection) and passes
//! `&mut **rls.conn()` to the repository. The authoritative organization is
//! `rls.tenant_id()` — the tenant the caller was validated against — not a
//! client-supplied `organization_id`, so the SQL org filter and the RLS context
//! can never disagree. `rls.release()` clears the context before the connection
//! returns to the pool.

use axum::{http::StatusCode, Json};
use common::errors::ErrorResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Error Helpers
// ============================================================================

/// Map a repository error to a `500` with a stable code, logging the cause.
pub(super) fn db_error(msg: &'static str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("{}: {:?}", msg, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("INTERNAL_ERROR", msg)),
    )
}

/// Build a `404` response.
pub(super) fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

/// Map a `fetch_one` error from a by-id write to either `404` (the row is not
/// visible under the caller's RLS context — cross-tenant or genuinely missing)
/// or `500` for any other database failure.
pub(super) fn write_error(
    msg: &'static str,
    not_found_msg: &'static str,
    e: sqlx::Error,
) -> (StatusCode, Json<ErrorResponse>) {
    match e {
        sqlx::Error::RowNotFound => not_found(not_found_msg),
        other => db_error(msg, other),
    }
}

/// Map an INSERT error on a child table to `404` when the parent is not
/// visible in the caller's RLS context (`42501` — RLS WITH CHECK rejected)
/// or does not exist (`23503` — FK violation); anything else is `500`.
pub(super) fn insert_child_error(
    msg: &'static str,
    not_found_msg: &'static str,
    e: sqlx::Error,
) -> (StatusCode, Json<ErrorResponse>) {
    if let sqlx::Error::Database(ref db_err) = e {
        if matches!(db_err.code().as_deref(), Some("42501") | Some("23503")) {
            return not_found(not_found_msg);
        }
    }
    db_error(msg, e)
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

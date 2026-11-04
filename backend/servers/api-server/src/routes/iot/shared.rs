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

use api_core::extractors::{RlsConnection, TenantMembershipProvider, ValidatedTenantExtractor};
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Json,
};
use common::errors::ErrorResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Authorization
// ============================================================================

/// Extractor that admits only manager-tier members of the request tenant.
///
/// The role comes from `ValidatedTenantExtractor`, which resolves it from the
/// caller's DB membership (never the JWT). Add it as a parameter to every IoT
/// mutation handler; reads stay open to any member. Carries the validated
/// tenant so handlers can scope SQL without re-borrowing the `RlsConnection`.
pub(super) struct IotManager {
    pub tenant_id: Uuid,
}

impl<S> FromRequestParts<S> for IotManager
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let tenant = ValidatedTenantExtractor::from_request_parts(parts, state).await?;
        if !tenant.role.is_manager() {
            return Err((StatusCode::FORBIDDEN, "Manager role required"));
        }
        Ok(IotManager {
            tenant_id: tenant.tenant_id,
        })
    }
}

/// Which by-id IoT resource a scope check targets.
pub(super) enum IotScope {
    Sensor(Uuid),
    Threshold(Uuid),
    Alert(Uuid),
    Correlation(Uuid),
    Template(Uuid),
    Fault(Uuid),
}

/// Ensure a by-id IoT resource belongs to the caller's org (404 otherwise).
///
/// Ownership is resolved through `sensors.organization_id` so it holds even
/// when RLS is bypassed by the connection's role. Releases the connection on
/// failure.
pub(super) async fn ensure_in_org(
    state: &crate::state::AppState,
    rls: &mut RlsConnection,
    scope: IotScope,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let repo = &state.sensor_repo;
    let conn = &mut **rls.conn();
    let (found, msg) = match scope {
        IotScope::Sensor(id) => (
            repo.sensor_in_org(conn, id, org_id).await,
            "Sensor not found",
        ),
        IotScope::Threshold(id) => (
            repo.threshold_in_org(conn, id, org_id).await,
            "Threshold not found",
        ),
        IotScope::Alert(id) => (repo.alert_in_org(conn, id, org_id).await, "Alert not found"),
        IotScope::Correlation(id) => (
            repo.correlation_in_org(conn, id, org_id).await,
            "Correlation not found",
        ),
        IotScope::Template(id) => (
            repo.template_visible_to_org(conn, id, org_id).await,
            "Template not found",
        ),
        IotScope::Fault(id) => (repo.fault_in_org(conn, id, org_id).await, "Fault not found"),
    };
    match found {
        Ok(true) => Ok(()),
        Ok(false) => {
            rls.release().await;
            Err(not_found(msg))
        }
        Err(e) => {
            rls.release().await;
            Err(db_error("Failed to verify resource scope", e))
        }
    }
}

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

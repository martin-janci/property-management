//! Shared helpers and request/response types for the vendor routes.
//!
//! See [`super`] for the RLS contract that every handler upholds.

use api_core::extractors::RlsConnection;
use axum::{http::StatusCode, Json};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::{
    ContractQuery, CreateVendor, CreateVendorContract, CreateVendorInvoice, InvoiceQuery, Vendor,
    VendorContract, VendorInvoice, VendorQuery,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

// ==================== Error Helpers ====================

/// Map a repository error to a `500` with a stable code, logging the cause.
pub(super) fn db_error(msg: &'static str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("{}: {:?}", msg, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

/// Build a `404` response.
pub(super) fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

/// Load a vendor by id under the caller's RLS context.
///
/// RLS scopes the connection to the caller's org, and the repo query is
/// additionally org-keyed (PAP-133 defense-in-depth — the org predicate holds
/// even on a connection whose role bypasses RLS). A vendor owned by another
/// organization surfaces as `404 NOT_FOUND`.
pub(super) async fn load_vendor_for_user(
    state: &AppState,
    rls: &mut RlsConnection,
    vendor_id: Uuid,
) -> Result<Vendor, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    state
        .vendor_repo
        .find_by_id(&mut **rls.conn(), vendor_id, org_id)
        .await
        .map_err(|e| db_error("Failed to load vendor", e))?
        .ok_or_else(|| not_found("Vendor not found"))
}

/// Load a contract by id under the caller's RLS context. Same contract as
/// [`load_vendor_for_user`].
pub(super) async fn load_contract_for_user(
    state: &AppState,
    rls: &mut RlsConnection,
    contract_id: Uuid,
) -> Result<VendorContract, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    state
        .vendor_repo
        .find_contract_by_id(&mut **rls.conn(), contract_id, org_id)
        .await
        .map_err(|e| db_error("Failed to load contract", e))?
        .ok_or_else(|| not_found("Contract not found"))
}

/// Load an invoice by id under the caller's RLS context. Same contract as
/// [`load_vendor_for_user`].
pub(super) async fn load_invoice_for_user(
    state: &AppState,
    rls: &mut RlsConnection,
    invoice_id: Uuid,
) -> Result<VendorInvoice, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    state
        .vendor_repo
        .find_invoice_by_id(&mut **rls.conn(), invoice_id, org_id)
        .await
        .map_err(|e| db_error("Failed to load invoice", e))?
        .ok_or_else(|| not_found("Invoice not found"))
}

// ==================== Request/Response Types ====================

/// Organization query parameter.
///
/// Retained for wire compatibility; the authoritative org is `rls.tenant_id()`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Option<Uuid>,
}

/// Create vendor request wrapper.
///
/// `organization_id` is retained for wire compatibility but ignored — the
/// authoritative org is `rls.tenant_id()`.
#[derive(Debug, Deserialize)]
pub struct CreateVendorRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateVendor,
}

/// List vendors query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListVendorsQuery {
    pub organization_id: Option<Uuid>,
    pub status: Option<String>,
    pub service: Option<String>,
    pub is_preferred: Option<bool>,
    pub contract_expiring_days: Option<i32>,
    pub search: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListVendorsQuery> for VendorQuery {
    fn from(q: &ListVendorsQuery) -> Self {
        VendorQuery {
            status: q.status.clone(),
            service: q.service.clone(),
            is_preferred: q.is_preferred,
            contract_expiring_days: q.contract_expiring_days,
            search: q.search.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Preferred request.
#[derive(Debug, Deserialize)]
pub struct PreferredRequest {
    pub is_preferred: bool,
}

/// Create contract request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateContractRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateVendorContract,
}

/// List contracts query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListContractsQuery {
    pub organization_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub status: Option<String>,
    pub contract_type: Option<String>,
    pub expiring_days: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListContractsQuery> for ContractQuery {
    fn from(q: &ListContractsQuery) -> Self {
        ContractQuery {
            vendor_id: q.vendor_id,
            status: q.status.clone(),
            contract_type: q.contract_type.clone(),
            expiring_days: q.expiring_days,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Expiring contracts query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ExpiringQuery {
    pub organization_id: Option<Uuid>,
    pub days: Option<i32>,
}

/// Create invoice request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub organization_id: Option<Uuid>,
    #[serde(flatten)]
    pub data: CreateVendorInvoice,
}

/// List invoices query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListInvoicesQuery {
    pub organization_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    pub status: Option<String>,
    pub due_before: Option<NaiveDate>,
    pub due_after: Option<NaiveDate>,
    pub work_order_id: Option<Uuid>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListInvoicesQuery> for InvoiceQuery {
    fn from(q: &ListInvoicesQuery) -> Self {
        InvoiceQuery {
            vendor_id: q.vendor_id,
            status: q.status.clone(),
            due_before: q.due_before,
            due_after: q.due_after,
            work_order_id: q.work_order_id,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Invoice summary query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct InvoiceSummaryQuery {
    pub organization_id: Option<Uuid>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

/// Reject invoice request.
#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

/// Record payment request.
#[derive(Debug, Deserialize)]
pub struct RecordPaymentRequest {
    pub amount: Decimal,
    pub method: Option<String>,
    pub reference: Option<String>,
}

/// Pagination query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

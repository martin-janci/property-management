//! Vendor management routes (Epic 21).
//!
//! # RLS (PAP-67 / PAP-70)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the vendor tables
//! (`vendors`, `vendor_contacts`, `vendor_contracts`, `vendor_invoices`,
//! `vendor_ratings`), so every query MUST run on a connection that has
//! `app.current_org_id` set or it collapses to deny-all. Each handler therefore
//! acquires an [`RlsConnection`] (which validates tenant membership and sets the
//! org/user GUCs on a dedicated connection) and passes `&mut **rls.conn()` to the
//! repository. The authoritative organization is `rls.tenant_id()` — the tenant
//! the caller was validated against — not a client-supplied `organization_id`,
//! so the SQL org filter and the RLS context can never disagree. Cross-tenant
//! access is blocked by RLS: a by-id read of another org's row returns no row
//! (`404`), and a write targeting another org fails the policy's `WITH CHECK`.
//! `rls.release()` clears the context before the connection returns to the pool.

use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::{
    ContractQuery, CreateVendor, CreateVendorContact, CreateVendorContract, CreateVendorInvoice,
    CreateVendorRating, ExpiringContract, InvoiceQuery, InvoiceSummary, UpdateVendor,
    UpdateVendorContract, UpdateVendorInvoice, Vendor, VendorContact, VendorContract,
    VendorInvoice, VendorQuery, VendorRating, VendorStatistics, VendorWithDetails,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::state::AppState;

// ==================== Error Helpers ====================

/// Map a repository error to a `500` with a stable code, logging the cause.
fn db_error(msg: &'static str, e: sqlx::Error) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("{}: {:?}", msg, e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

/// Build a `404` response.
fn not_found(msg: &'static str) -> (StatusCode, Json<ErrorResponse>) {
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
async fn load_vendor_for_user(
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
async fn load_contract_for_user(
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
async fn load_invoice_for_user(
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

/// Create vendors router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Vendors
        .route("/", post(create_vendor))
        .route("/", get(list_vendors))
        .route("/with-details", get(list_vendors_with_details))
        .route("/statistics", get(get_statistics))
        .route("/{id}", get(get_vendor))
        .route("/{id}", patch(update_vendor))
        .route("/{id}", delete(delete_vendor))
        .route("/{id}/preferred", post(set_preferred))
        // Vendor Contacts
        .route("/{id}/contacts", post(add_contact))
        .route("/{id}/contacts", get(list_contacts))
        .route("/contacts/{contact_id}", delete(delete_contact))
        // Vendor Ratings
        .route("/{id}/ratings", post(add_rating))
        .route("/{id}/ratings", get(list_ratings))
        // Contracts
        .route("/contracts", post(create_contract))
        .route("/contracts", get(list_contracts))
        .route("/contracts/expiring", get(get_expiring_contracts))
        .route("/contracts/{id}", get(get_contract))
        .route("/contracts/{id}", patch(update_contract))
        .route("/contracts/{id}", delete(delete_contract))
        // Invoices
        .route("/invoices", post(create_invoice))
        .route("/invoices", get(list_invoices))
        .route("/invoices/overdue", get(get_overdue_invoices))
        .route("/invoices/summary", get(get_invoice_summary))
        .route("/invoices/{id}", get(get_invoice))
        .route("/invoices/{id}", patch(update_invoice))
        .route("/invoices/{id}", delete(delete_invoice))
        .route("/invoices/{id}/approve", post(approve_invoice))
        .route("/invoices/{id}/reject", post(reject_invoice))
        .route("/invoices/{id}/payment", post(record_payment))
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

// ==================== Vendor Endpoints (Story 21.1) ====================

async fn create_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateVendorRequest>,
) -> Result<(StatusCode, Json<Vendor>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .create(&mut **rls.conn(), org_id, payload.data)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| db_error("Failed to create vendor", e));
    rls.release().await;
    out
}

async fn list_vendors(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListVendorsQuery>,
) -> Result<Json<Vec<Vendor>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list vendors", e));
    rls.release().await;
    out
}

async fn list_vendors_with_details(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListVendorsQuery>,
) -> Result<Json<Vec<VendorWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list_with_details(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list vendors", e));
    rls.release().await;
    out
}

async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_query): Query<OrgQuery>,
) -> Result<Json<VendorStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_statistics(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get statistics", e));
    rls.release().await;
    out
}

async fn get_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_vendor_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

async fn update_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendor>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        let org_id = rls.tenant_id();
        state
            .vendor_repo
            .update(&mut **rls.conn(), id, org_id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update vendor", e))
    }
    .await;
    rls.release().await;
    out
}

async fn delete_vendor(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        let org_id = rls.tenant_id();
        let deleted = state
            .vendor_repo
            .delete(&mut **rls.conn(), id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete vendor", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Vendor not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

async fn set_preferred(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<PreferredRequest>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .set_preferred(&mut **rls.conn(), id, org_id, data.is_preferred)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to set preferred", e))
    }
    .await;
    rls.release().await;
    out
}

// ==================== Vendor Contacts ====================

async fn add_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateVendorContact>,
) -> Result<(StatusCode, Json<VendorContact>), (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .add_contact(&mut **rls.conn(), id, data)
            .await
            .map(|c| (StatusCode::CREATED, Json(c)))
            .map_err(|e| db_error("Failed to add contact", e))
    }
    .await;
    rls.release().await;
    out
}

async fn list_contacts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VendorContact>>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .list_contacts(&mut **rls.conn(), id)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to list contacts", e))
    }
    .await;
    rls.release().await;
    out
}

async fn delete_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(contact_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        // Resolve the owning vendor, keyed to the caller's org (PAP-129: the
        // org join holds even where RLS does not bind). A contact whose vendor
        // is owned by another org resolves to `None` → 404. We then load the
        // vendor to confirm tenant visibility before deleting.
        let vendor_id = state
            .vendor_repo
            .find_contact_vendor_id(&mut **rls.conn(), contact_id, org_id)
            .await
            .map_err(|e| db_error("Failed to resolve contact", e))?
            .ok_or_else(|| not_found("Contact not found"))?;
        load_vendor_for_user(&state, &mut rls, vendor_id).await?;

        let deleted = state
            .vendor_repo
            .delete_contact(&mut **rls.conn(), contact_id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete contact", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Contact not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

// ==================== Vendor Ratings ====================

async fn add_rating(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateVendorRating>,
) -> Result<(StatusCode, Json<VendorRating>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .add_rating(&mut **rls.conn(), id, user_id, data)
            .await
            .map(|r| (StatusCode::CREATED, Json(r)))
            .map_err(|e| db_error("Failed to add rating", e))
    }
    .await;
    rls.release().await;
    out
}

async fn list_ratings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<VendorRating>>, (StatusCode, Json<ErrorResponse>)> {
    let out = async {
        load_vendor_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .list_ratings(
                &mut **rls.conn(),
                id,
                query.limit.unwrap_or(50),
                query.offset.unwrap_or(0),
            )
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to list ratings", e))
    }
    .await;
    rls.release().await;
    out
}

// ==================== Contract Endpoints (Story 21.3) ====================

async fn create_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateContractRequest>,
) -> Result<(StatusCode, Json<VendorContract>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .create_contract(&mut **rls.conn(), org_id, payload.data)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(|e| db_error("Failed to create contract", e));
    rls.release().await;
    out
}

async fn list_contracts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListContractsQuery>,
) -> Result<Json<Vec<VendorContract>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list_contracts(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list contracts", e));
    rls.release().await;
    out
}

async fn get_expiring_contracts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<ExpiringContract>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_expiring_contracts(&mut **rls.conn(), org_id, query.days.unwrap_or(30))
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get expiring contracts", e));
    rls.release().await;
    out
}

async fn get_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorContract>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_contract_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

async fn update_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendorContract>,
) -> Result<Json<VendorContract>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_contract_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .update_contract(&mut **rls.conn(), id, org_id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update contract", e))
    }
    .await;
    rls.release().await;
    out
}

async fn delete_contract(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_contract_for_user(&state, &mut rls, id).await?;
        let deleted = state
            .vendor_repo
            .delete_contract(&mut **rls.conn(), id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete contract", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Contract not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

// ==================== Invoice Endpoints (Story 21.4) ====================

async fn create_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<(StatusCode, Json<VendorInvoice>), (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .vendor_repo
        .create_invoice(&mut **rls.conn(), org_id, user_id, payload.data)
        .await
        .map(|i| (StatusCode::CREATED, Json(i)))
        .map_err(|e| db_error("Failed to create invoice", e));
    rls.release().await;
    out
}

async fn list_invoices(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<Vec<VendorInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .list_invoices(&mut **rls.conn(), org_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to list invoices", e));
    rls.release().await;
    out
}

async fn get_overdue_invoices(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(_query): Query<OrgQuery>,
) -> Result<Json<Vec<VendorInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_overdue_invoices(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get overdue invoices", e));
    rls.release().await;
    out
}

async fn get_invoice_summary(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<InvoiceSummaryQuery>,
) -> Result<Json<Vec<InvoiceSummary>>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .vendor_repo
        .get_invoice_summary(&mut **rls.conn(), org_id, query.start_date, query.end_date)
        .await
        .map(Json)
        .map_err(|e| db_error("Failed to get invoice summary", e));
    rls.release().await;
    out
}

async fn get_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let out = load_invoice_for_user(&state, &mut rls, id).await.map(Json);
    rls.release().await;
    out
}

async fn update_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendorInvoice>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .update_invoice(&mut **rls.conn(), id, org_id, data)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to update invoice", e))
    }
    .await;
    rls.release().await;
    out
}

async fn delete_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        let deleted = state
            .vendor_repo
            .delete_invoice(&mut **rls.conn(), id, org_id)
            .await
            .map_err(|e| db_error("Failed to delete invoice", e))?;
        if deleted {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(not_found("Invoice not found"))
        }
    }
    .await;
    rls.release().await;
    out
}

async fn approve_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .approve_invoice(&mut **rls.conn(), id, org_id, user_id)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to approve invoice", e))
    }
    .await;
    rls.release().await;
    out
}

async fn reject_invoice(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<RejectRequest>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .reject_invoice(&mut **rls.conn(), id, org_id, user_id, &data.reason)
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to reject invoice", e))
    }
    .await;
    rls.release().await;
    out
}

async fn record_payment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(data): Json<RecordPaymentRequest>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = async {
        load_invoice_for_user(&state, &mut rls, id).await?;
        state
            .vendor_repo
            .record_payment(
                &mut **rls.conn(),
                id,
                org_id,
                data.amount,
                data.method.as_deref(),
                data.reference.as_deref(),
            )
            .await
            .map(Json)
            .map_err(|e| db_error("Failed to record payment", e))
    }
    .await;
    rls.release().await;
    out
}

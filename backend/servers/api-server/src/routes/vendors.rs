//! Vendor management routes (Epic 21).

use api_core::extractors::AuthUser;
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
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Create vendor request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateVendorRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateVendor,
}

/// List vendors query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListVendorsQuery {
    pub organization_id: Uuid,
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
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateVendorContract,
}

/// List contracts query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListContractsQuery {
    pub organization_id: Uuid,
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
    pub organization_id: Uuid,
    pub days: Option<i32>,
}

/// Create invoice request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateVendorInvoice,
}

/// List invoices query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListInvoicesQuery {
    pub organization_id: Uuid,
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
    pub organization_id: Uuid,
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
    _user: AuthUser,
    Json(payload): Json<CreateVendorRequest>,
) -> Result<(StatusCode, Json<Vendor>), (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .create(payload.organization_id, payload.data)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| {
            tracing::error!("Failed to create vendor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create vendor")),
            )
        })
}

async fn list_vendors(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListVendorsQuery>,
) -> Result<Json<Vec<Vendor>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .list(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list vendors: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list vendors")),
            )
        })
}

async fn list_vendors_with_details(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListVendorsQuery>,
) -> Result<Json<Vec<VendorWithDetails>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .list_with_details(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list vendors: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list vendors")),
            )
        })
}

async fn get_statistics(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<VendorStatistics>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .get_statistics(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get statistics")),
            )
        })
}

async fn get_vendor(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .find_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get vendor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get vendor")),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vendor not found")),
            )
        })
}

async fn update_vendor(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendor>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .update(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update vendor: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update vendor")),
            )
        })
}

async fn delete_vendor(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.vendor_repo.delete(id).await.map_err(|e| {
        tracing::error!("Failed to delete vendor: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to delete vendor")),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Vendor not found")),
        ))
    }
}

async fn set_preferred(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<PreferredRequest>,
) -> Result<Json<Vendor>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .set_preferred(id, data.is_preferred)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to set preferred: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to set preferred")),
            )
        })
}

// ==================== Vendor Contacts ====================

async fn add_contact(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateVendorContact>,
) -> Result<(StatusCode, Json<VendorContact>), (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .add_contact(id, data)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(|e| {
            tracing::error!("Failed to add contact: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to add contact")),
            )
        })
}

async fn list_contacts(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<VendorContact>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .list_contacts(id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list contacts: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list contacts")),
            )
        })
}

async fn delete_contact(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(contact_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .vendor_repo
        .delete_contact(contact_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete contact: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to delete contact")),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Contact not found")),
        ))
    }
}

// ==================== Vendor Ratings ====================

async fn add_rating(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateVendorRating>,
) -> Result<(StatusCode, Json<VendorRating>), (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .add_rating(id, user.user_id, data)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| {
            tracing::error!("Failed to add rating: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to add rating")),
            )
        })
}

async fn list_ratings(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<VendorRating>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .list_ratings(id, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list ratings: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list ratings")),
            )
        })
}

// ==================== Contract Endpoints (Story 21.3) ====================

async fn create_contract(
    State(state): State<AppState>,
    _user: AuthUser,
    Json(payload): Json<CreateContractRequest>,
) -> Result<(StatusCode, Json<VendorContract>), (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .create_contract(payload.organization_id, payload.data)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(|e| {
            tracing::error!("Failed to create contract: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create contract")),
            )
        })
}

async fn list_contracts(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListContractsQuery>,
) -> Result<Json<Vec<VendorContract>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .list_contracts(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list contracts: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list contracts")),
            )
        })
}

async fn get_expiring_contracts(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<ExpiringContract>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .get_expiring_contracts(query.organization_id, query.days.unwrap_or(30))
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get expiring contracts: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get expiring contracts",
                )),
            )
        })
}

async fn get_contract(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorContract>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .find_contract_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get contract: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get contract")),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Contract not found")),
            )
        })
}

async fn update_contract(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendorContract>,
) -> Result<Json<VendorContract>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .update_contract(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update contract: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update contract")),
            )
        })
}

async fn delete_contract(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.vendor_repo.delete_contract(id).await.map_err(|e| {
        tracing::error!("Failed to delete contract: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to delete contract")),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Contract not found")),
        ))
    }
}

// ==================== Invoice Endpoints (Story 21.4) ====================

async fn create_invoice(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<(StatusCode, Json<VendorInvoice>), (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .create_invoice(payload.organization_id, user.user_id, payload.data)
        .await
        .map(|i| (StatusCode::CREATED, Json(i)))
        .map_err(|e| {
            tracing::error!("Failed to create invoice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create invoice")),
            )
        })
}

async fn list_invoices(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<Vec<VendorInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .list_invoices(query.organization_id, (&query).into())
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list invoices: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list invoices")),
            )
        })
}

async fn get_overdue_invoices(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<VendorInvoice>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .get_overdue_invoices(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get overdue invoices: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get overdue invoices",
                )),
            )
        })
}

async fn get_invoice_summary(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<InvoiceSummaryQuery>,
) -> Result<Json<Vec<InvoiceSummary>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .get_invoice_summary(query.organization_id, query.start_date, query.end_date)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get invoice summary: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get invoice summary",
                )),
            )
        })
}

async fn get_invoice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .find_invoice_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get invoice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get invoice")),
            )
        })?
        .map(Json)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
            )
        })
}

async fn update_invoice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateVendorInvoice>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .update_invoice(id, data)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to update invoice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update invoice")),
            )
        })
}

async fn delete_invoice(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state.vendor_repo.delete_invoice(id).await.map_err(|e| {
        tracing::error!("Failed to delete invoice: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to delete invoice")),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
        ))
    }
}

async fn approve_invoice(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .approve_invoice(id, user.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to approve invoice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to approve invoice")),
            )
        })
}

async fn reject_invoice(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<RejectRequest>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .reject_invoice(id, user.user_id, &data.reason)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to reject invoice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to reject invoice")),
            )
        })
}

async fn record_payment(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<RecordPaymentRequest>,
) -> Result<Json<VendorInvoice>, (StatusCode, Json<ErrorResponse>)> {
    state
        .vendor_repo
        .record_payment(
            id,
            data.amount,
            data.method.as_deref(),
            data.reference.as_deref(),
        )
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to record payment: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to record payment")),
            )
        })
}

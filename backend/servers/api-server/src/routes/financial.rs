//! Financial routes (Epic 11).
//!
//! Implements financial management including accounts, transactions,
//! invoices, payments, and reporting.

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::{
    CreateFeeSchedule, CreateFinancialAccount, CreateInvoice, CreateTransaction, InvoiceStatus,
    RecordPayment,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

/// Default page size for listings
const DEFAULT_LIST_LIMIT: i64 = 50;

/// Maximum page size
const MAX_LIST_LIMIT: i64 = 100;

/// Create financial router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Financial accounts (Story 11.1)
        .route("/accounts", post(create_account))
        .route("/accounts", get(list_accounts))
        .route("/accounts/{id}", get(get_account))
        .route("/accounts/{id}/transactions", get(list_transactions))
        .route("/accounts/{id}/transactions", post(create_transaction))
        .route("/units/{unit_id}/ledger", get(get_unit_ledger))
        // Fee schedules (Story 11.2)
        .route("/fee-schedules", post(create_fee_schedule))
        .route("/fee-schedules", get(list_fee_schedules))
        .route("/fee-schedules/{id}", get(get_fee_schedule))
        .route("/units/{unit_id}/fees", get(get_unit_fees))
        .route("/units/{unit_id}/fees", post(assign_unit_fee))
        // Invoices (Story 11.3)
        .route("/invoices", post(create_invoice))
        .route("/invoices", get(list_invoices))
        .route("/invoices/{id}", get(get_invoice))
        .route("/invoices/{id}/send", post(send_invoice))
        .route("/units/{unit_id}/invoices", get(list_unit_invoices))
        // Payments (Story 11.4)
        .route("/payments", post(record_payment))
        .route("/payments/{id}", get(get_payment))
        .route("/units/{unit_id}/payments", get(list_unit_payments))
        // Payment reminders (Story 11.6)
        .route("/reminder-schedules", get(get_reminder_schedules))
        .route("/late-fee-config", get(get_late_fee_config))
        .route("/overdue-invoices", get(get_overdue_invoices))
        // Reports (Story 11.7)
        .route("/reports/ar-aging", get(get_ar_aging_report))
}

// ==================== Request/Response Types ====================

/// List accounts query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListAccountsQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Filter by building ID
    pub building_id: Option<Uuid>,
}

/// List transactions query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListTransactionsQuery {
    /// Start date filter
    pub from: Option<NaiveDate>,
    /// End date filter
    pub to: Option<NaiveDate>,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// List invoices query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListInvoicesQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Filter by status
    pub status: Option<InvoiceStatus>,
    /// Filter by unit
    pub unit_id: Option<Uuid>,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// List fee schedules query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListFeeSchedulesQuery {
    /// Building to list for
    pub building_id: Uuid,
    /// Only active schedules
    #[serde(default = "default_true")]
    pub active_only: bool,
}

/// Assign unit fee request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignUnitFeeRequest {
    /// Fee schedule to assign
    pub fee_schedule_id: Uuid,
    /// Override the standard amount
    pub override_amount: Option<Decimal>,
    /// Start date
    pub effective_from: NaiveDate,
    /// End date
    pub effective_to: Option<NaiveDate>,
}

/// Unit fees query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct UnitFeesQuery {
    /// As of date for active fees
    pub as_of: Option<NaiveDate>,
}

/// AR report query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ARReportQuery {
    /// Organization ID
    pub organization_id: Uuid,
    /// Filter by building
    pub building_id: Option<Uuid>,
}

/// Create account with org.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// Account details
    #[serde(flatten)]
    pub data: CreateFinancialAccount,
}

/// Create fee schedule with org.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFeeScheduleRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Schedule details
    #[serde(flatten)]
    pub data: CreateFeeSchedule,
}

/// Create invoice with org.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateInvoiceRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Invoice details
    #[serde(flatten)]
    pub data: CreateInvoice,
}

/// Record payment with org.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordPaymentRequest {
    /// Organization ID
    pub organization_id: Uuid,
    /// User ID
    pub user_id: Uuid,
    /// Payment details
    #[serde(flatten)]
    pub data: RecordPayment,
}

/// Org query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

fn default_limit() -> i64 {
    DEFAULT_LIST_LIMIT
}

fn default_true() -> bool {
    true
}

// ==================== Account Handlers ====================

async fn create_account(
    State(state): State<AppState>,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<db::models::FinancialAccount>), (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .create_account(payload.organization_id, payload.data)
        .await
        .map(|account| (StatusCode::CREATED, Json(account)))
        .map_err(|e| {
            tracing::error!("Failed to create account: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create account")),
            )
        })
}

async fn list_accounts(
    State(state): State<AppState>,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<Vec<db::models::FinancialAccount>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .list_accounts(query.organization_id, query.building_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list accounts: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list accounts")),
            )
        })
}

async fn get_account(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::FinancialAccountResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .financial_repo
        .get_account_with_transactions(id, 20)
        .await
    {
        Ok(Some(account)) => Ok(Json(account)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Account not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get account: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get account")),
            ))
        }
    }
}

async fn list_transactions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<ListTransactionsQuery>,
) -> Result<Json<Vec<db::models::AccountTransaction>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = query.limit.min(MAX_LIST_LIMIT);
    state
        .financial_repo
        .list_transactions(id, query.from, query.to, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list transactions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list transactions",
                )),
            )
        })
}

async fn create_transaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(mut payload): Json<CreateTransaction>,
) -> Result<(StatusCode, Json<db::models::AccountTransaction>), (StatusCode, Json<ErrorResponse>)> {
    payload.account_id = id;

    state
        .financial_repo
        .create_transaction(auth.user_id, payload)
        .await
        .map(|tx| (StatusCode::CREATED, Json(tx)))
        .map_err(|e| {
            tracing::error!("Failed to create transaction: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to create transaction",
                )),
            )
        })
}

async fn get_unit_ledger(
    State(state): State<AppState>,
    Path(unit_id): Path<Uuid>,
) -> Result<Json<db::models::FinancialAccountResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_unit_ledger(unit_id).await {
        Ok(Some(account)) => {
            match state
                .financial_repo
                .get_account_with_transactions(account.id, 20)
                .await
            {
                Ok(Some(response)) => Ok(Json(response)),
                _ => Ok(Json(db::models::FinancialAccountResponse {
                    account,
                    recent_transactions: vec![],
                })),
            }
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Unit ledger not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get unit ledger: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get unit ledger")),
            ))
        }
    }
}

// ==================== Fee Schedule Handlers ====================

async fn create_fee_schedule(
    State(state): State<AppState>,
    Json(payload): Json<CreateFeeScheduleRequest>,
) -> Result<(StatusCode, Json<db::models::FeeSchedule>), (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .create_fee_schedule(payload.organization_id, payload.user_id, payload.data)
        .await
        .map(|schedule| (StatusCode::CREATED, Json(schedule)))
        .map_err(|e| {
            tracing::error!("Failed to create fee schedule: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to create fee schedule",
                )),
            )
        })
}

async fn list_fee_schedules(
    State(state): State<AppState>,
    Query(query): Query<ListFeeSchedulesQuery>,
) -> Result<Json<Vec<db::models::FeeSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .list_fee_schedules(query.building_id, query.active_only)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list fee schedules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list fee schedules",
                )),
            )
        })
}

async fn get_fee_schedule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::FeeSchedule>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_fee_schedule(id).await {
        Ok(Some(schedule)) => Ok(Json(schedule)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Fee schedule not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get fee schedule: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get fee schedule")),
            ))
        }
    }
}

async fn get_unit_fees(
    State(state): State<AppState>,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<UnitFeesQuery>,
) -> Result<Json<Vec<db::models::UnitFee>>, (StatusCode, Json<ErrorResponse>)> {
    let as_of = query
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    state
        .financial_repo
        .get_unit_fees(unit_id, as_of)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get unit fees: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get unit fees")),
            )
        })
}

async fn assign_unit_fee(
    State(state): State<AppState>,
    Path(unit_id): Path<Uuid>,
    Json(payload): Json<AssignUnitFeeRequest>,
) -> Result<(StatusCode, Json<db::models::UnitFee>), (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .assign_unit_fee(
            unit_id,
            payload.fee_schedule_id,
            payload.override_amount,
            payload.effective_from,
            payload.effective_to,
        )
        .await
        .map(|fee| (StatusCode::CREATED, Json(fee)))
        .map_err(|e| {
            tracing::error!("Failed to assign unit fee: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to assign unit fee")),
            )
        })
}

// ==================== Invoice Handlers ====================

async fn create_invoice(
    State(state): State<AppState>,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<(StatusCode, Json<db::models::InvoiceResponse>), (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .create_invoice(payload.organization_id, payload.user_id, payload.data)
        .await
        .map(|invoice| (StatusCode::CREATED, Json(invoice)))
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
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<db::models::ListInvoicesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = query.limit.min(MAX_LIST_LIMIT);
    let result = if let Some(unit_id) = query.unit_id {
        state
            .financial_repo
            .list_invoices_for_unit(unit_id, query.status, limit, query.offset)
            .await
    } else {
        state
            .financial_repo
            .list_invoices_for_org(query.organization_id, query.status, limit, query.offset)
            .await
    };

    result.map(Json).map_err(|e| {
        tracing::error!("Failed to list invoices: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to list invoices")),
        )
    })
}

async fn get_invoice(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::InvoiceResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_invoice_with_details(id).await {
        Ok(Some(invoice)) => Ok(Json(invoice)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get invoice: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get invoice")),
            ))
        }
    }
}

async fn send_invoice(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::Invoice>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.mark_invoice_sent(id).await {
        Ok(Some(invoice)) => Ok(Json(invoice)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to send invoice: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to send invoice")),
            ))
        }
    }
}

async fn list_unit_invoices(
    State(state): State<AppState>,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<db::models::ListInvoicesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let limit = query.limit.min(MAX_LIST_LIMIT);
    state
        .financial_repo
        .list_invoices_for_unit(unit_id, query.status, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list unit invoices: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list invoices")),
            )
        })
}

// ==================== Payment Handlers ====================

async fn record_payment(
    State(state): State<AppState>,
    Json(payload): Json<RecordPaymentRequest>,
) -> Result<(StatusCode, Json<db::models::PaymentResponse>), (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .record_payment(payload.organization_id, payload.user_id, payload.data)
        .await
        .map(|payment| (StatusCode::CREATED, Json(payment)))
        .map_err(|e| {
            tracing::error!("Failed to record payment: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to record payment")),
            )
        })
}

async fn get_payment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::PaymentResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_payment_with_allocations(id).await {
        Ok(Some(payment)) => Ok(Json(payment)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Payment not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get payment: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get payment")),
            ))
        }
    }
}

async fn list_unit_payments(
    State(state): State<AppState>,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<ListTransactionsQuery>,
) -> Result<Json<Vec<db::models::Payment>>, (StatusCode, Json<ErrorResponse>)> {
    let limit = query.limit.min(MAX_LIST_LIMIT);
    state
        .financial_repo
        .list_payments_for_unit(unit_id, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list payments: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list payments")),
            )
        })
}

// ==================== Reminder/Late Fee Handlers ====================

async fn get_reminder_schedules(
    State(state): State<AppState>,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<db::models::ReminderSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .get_reminder_schedules(query.organization_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get reminder schedules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get reminder schedules",
                )),
            )
        })
}

async fn get_late_fee_config(
    State(state): State<AppState>,
    Query(query): Query<OrgQuery>,
) -> Result<Json<db::models::LateFeeConfig>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .financial_repo
        .get_late_fee_config(query.organization_id)
        .await
    {
        Ok(Some(config)) => Ok(Json(config)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Late fee not configured")),
        )),
        Err(e) => {
            tracing::error!("Failed to get late fee config: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get late fee config",
                )),
            ))
        }
    }
}

async fn get_overdue_invoices(
    State(state): State<AppState>,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<db::models::Invoice>>, (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
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

// ==================== Report Handlers ====================

async fn get_ar_aging_report(
    State(state): State<AppState>,
    Query(query): Query<ARReportQuery>,
) -> Result<Json<db::models::AccountsReceivableReport>, (StatusCode, Json<ErrorResponse>)> {
    state
        .financial_repo
        .get_ar_aging_report(query.organization_id, query.building_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to generate AR report: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to generate report")),
            )
        })
}

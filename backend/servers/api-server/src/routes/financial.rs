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
    CreateFeeSchedule, CreateFinancialAccount, CreateInvoice, CreateTransaction,
    InitiatePaymentResponse, InvoiceStatus, Payment, PaymentAllocation, RecordPayment,
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

/// Verify the authenticated user is a member of `org_id`.
///
/// This is the tenant-isolation gate for every financial route (issue #802).
/// The client-supplied `organization_id` (in a query param, request body, or
/// derived from a fetched resource) is never trusted on its own — it is only
/// accepted once it is confirmed to belong to the authenticated principal.
/// Cross-tenant callers receive `403 FORBIDDEN`.
///
/// Mirrors the `verify_org_access` idiom used by the integrations routes
/// (`routes/integrations/sync.rs`).
async fn verify_org_access(
    state: &AppState,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_member = state
        .org_member_repo
        .is_member(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You are not a member of this organization",
            )),
        ));
    }

    Ok(())
}

/// Resolve the organization that owns `account_id` and verify the
/// authenticated user belongs to it. Returns `404` when the account does not
/// exist or the user is not a member (so cross-tenant probing cannot
/// distinguish "missing" from "forbidden").
async fn verify_account_access(
    state: &AppState,
    user_id: Uuid,
    account_id: Uuid,
) -> Result<db::models::FinancialAccount, (StatusCode, Json<ErrorResponse>)> {
    let account = state
        .financial_repo
        .get_account(account_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load account: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to load account")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Account not found")),
            )
        })?;

    if !is_member_or_not_found(state, user_id, account.organization_id).await? {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Account not found")),
        ));
    }

    Ok(account)
}

/// Membership check that maps DB errors to a 500 but returns a plain bool for
/// the membership outcome, letting the caller choose 403 vs 404 semantics.
async fn is_member_or_not_found(
    state: &AppState,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<bool, (StatusCode, Json<ErrorResponse>)> {
    state
        .org_member_repo
        .is_member(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })
}

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
        .route("/invoices/{id}/pdf", get(get_invoice_pdf))
        // Online payment / gateway checkout (Story 11.5, BIT-181)
        .route("/invoices/{id}/checkout", post(initiate_invoice_checkout))
        .route("/units/{unit_id}/invoices", get(list_unit_invoices))
        // Payments (Story 11.4)
        .route("/payments", post(record_payment))
        .route("/payments", get(list_payments))
        // Static segments before `/{id}` so they aren't captured as an id.
        .route("/payments/unallocated", get(list_unallocated_payments))
        .route("/payments/auto-match", post(auto_match_payments))
        .route("/payments/{id}", get(get_payment))
        .route("/payments/{id}/allocate", post(allocate_payment))
        .route("/units/{unit_id}/payments", get(list_unit_payments))
        // Payment reminders (Story 11.6)
        .route("/reminder-schedules", get(get_reminder_schedules))
        .route("/late-fee-config", get(get_late_fee_config))
        .route("/overdue-invoices", get(get_overdue_invoices))
        // Reports (Story 11.7)
        .route("/reports/ar-aging", get(get_ar_aging_report))
        .route("/reports/income-statement", get(get_income_statement))
        .route("/reports/balance-sheet", get(get_balance_sheet))
        .route("/reports/cash-flow", get(get_cash_flow))
        .route("/reports/{report}/export", get(export_report))
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

/// Unit payments query parameters (org membership is verified).
#[derive(Debug, Deserialize, IntoParams)]
pub struct UnitPaymentsQuery {
    /// Organization that owns the unit (membership is verified)
    pub organization_id: Uuid,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// Org-wide payments list query (Story 11.4 Payment Management).
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListPaymentsQuery {
    /// Organization whose payments to list (membership is verified)
    pub organization_id: Uuid,
    /// Optional payment-status filter (pending|completed|failed|refunded|cancelled)
    #[serde(default)]
    pub status: Option<String>,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// Unallocated-payments query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct UnallocatedPaymentsQuery {
    /// Organization whose unallocated payments to list (membership is verified)
    pub organization_id: Uuid,
    /// Page limit
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Page offset
    #[serde(default)]
    pub offset: i64,
}

/// Paginated payments list response.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct PaymentListResponse {
    pub payments: Vec<Payment>,
    pub total: i64,
}

/// Request to allocate an existing payment to an invoice (the manual match).
#[derive(Debug, Deserialize, ToSchema)]
pub struct AllocatePaymentRequest {
    /// Organization that owns both the payment and the invoice (verified)
    pub organization_id: Uuid,
    /// Invoice to allocate the payment against
    pub invoice_id: Uuid,
    /// Amount to allocate (defaults to the lesser of the payment's remaining
    /// balance and the invoice's outstanding balance)
    #[serde(default)]
    pub amount: Option<Decimal>,
}

/// Request to bulk auto-match unallocated payments in an organization.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AutoMatchRequest {
    /// Organization whose unallocated payments to auto-match (verified)
    pub organization_id: Uuid,
}

/// Auto-match result.
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AutoMatchResponse {
    /// Number of allocations created
    pub matched: i64,
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
    /// Organization that owns the building (membership is verified)
    pub organization_id: Uuid,
    /// Building to list for
    pub building_id: Uuid,
    /// Only active schedules
    #[serde(default = "default_true")]
    pub active_only: bool,
}

/// Assign unit fee request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignUnitFeeRequest {
    /// Organization that owns the unit (membership is verified)
    pub organization_id: Uuid,
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
    /// Organization that owns the unit (membership is verified)
    pub organization_id: Uuid,
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

/// Income statement / cash flow date-range query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct DateRangeReportQuery {
    pub organization_id: Uuid,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// Balance sheet as-of query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct BalanceSheetQuery {
    pub organization_id: Uuid,
    pub as_of: Option<NaiveDate>,
}

/// Export query — wraps either a date range or as_of, plus format.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ExportQuery {
    pub organization_id: Uuid,
    /// "pdf" or "xlsx"
    pub format: String,
    /// Required for income-statement and cash-flow exports
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// Required for balance-sheet export
    pub as_of: Option<NaiveDate>,
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
    auth: AuthUser,
    Json(payload): Json<CreateAccountRequest>,
) -> Result<(StatusCode, Json<db::models::FinancialAccount>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, payload.organization_id).await?;
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
    auth: AuthUser,
    Query(query): Query<ListAccountsQuery>,
) -> Result<Json<Vec<db::models::FinancialAccount>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
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
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::FinancialAccountResponse>, (StatusCode, Json<ErrorResponse>)> {
    verify_account_access(&state, auth.user_id, id).await?;
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
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<ListTransactionsQuery>,
) -> Result<Json<Vec<db::models::AccountTransaction>>, (StatusCode, Json<ErrorResponse>)> {
    verify_account_access(&state, auth.user_id, id).await?;
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
    verify_account_access(&state, auth.user_id, id).await?;
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
    auth: AuthUser,
    Path(unit_id): Path<Uuid>,
) -> Result<Json<db::models::FinancialAccountResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_unit_ledger(unit_id).await {
        Ok(Some(account)) => {
            // The ledger account carries the owning org; verify membership
            // before returning any data (treat non-members as 404).
            if !is_member_or_not_found(&state, auth.user_id, account.organization_id).await? {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Unit ledger not found")),
                ));
            }
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
    auth: AuthUser,
    Json(payload): Json<CreateFeeScheduleRequest>,
) -> Result<(StatusCode, Json<db::models::FeeSchedule>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, payload.organization_id).await?;
    state
        .financial_repo
        .create_fee_schedule(payload.organization_id, auth.user_id, payload.data)
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
    auth: AuthUser,
    Query(query): Query<ListFeeSchedulesQuery>,
) -> Result<Json<Vec<db::models::FeeSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    let schedules = state
        .financial_repo
        .list_fee_schedules(query.building_id, query.active_only)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list fee schedules: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list fee schedules",
                )),
            )
        })?;
    // Defense-in-depth: only return schedules belonging to the verified org.
    let scoped = schedules
        .into_iter()
        .filter(|s| s.organization_id == query.organization_id)
        .collect::<Vec<_>>();
    Ok(Json(scoped))
}

async fn get_fee_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::FeeSchedule>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_fee_schedule(id).await {
        Ok(Some(schedule)) => {
            if !is_member_or_not_found(&state, auth.user_id, schedule.organization_id).await? {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Fee schedule not found")),
                ));
            }
            Ok(Json(schedule))
        }
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
    auth: AuthUser,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<UnitFeesQuery>,
) -> Result<Json<Vec<db::models::UnitFee>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
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
    auth: AuthUser,
    Path(unit_id): Path<Uuid>,
    Json(payload): Json<AssignUnitFeeRequest>,
) -> Result<(StatusCode, Json<db::models::UnitFee>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, payload.organization_id).await?;
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
    auth: AuthUser,
    Json(payload): Json<CreateInvoiceRequest>,
) -> Result<(StatusCode, Json<db::models::InvoiceResponse>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, payload.organization_id).await?;
    state
        .financial_repo
        .create_invoice(payload.organization_id, auth.user_id, payload.data)
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
    auth: AuthUser,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<db::models::ListInvoicesResponse>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
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

    let mut response = result.map_err(|e| {
        tracing::error!("Failed to list invoices: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to list invoices")),
        )
    })?;
    // The unit-scoped query is not org-bound at the DB layer; filter the
    // result to the verified org so a foreign unit_id cannot leak invoices.
    response
        .invoices
        .retain(|inv| inv.organization_id == query.organization_id);
    response.total = response.invoices.len() as i64;
    Ok(Json(response))
}

async fn get_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::InvoiceResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_invoice_with_details(id).await {
        Ok(Some(invoice)) => {
            if !is_member_or_not_found(&state, auth.user_id, invoice.invoice.organization_id)
                .await?
            {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
                ));
            }
            Ok(Json(invoice))
        }
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

/// Render a financial invoice (header + line items + totals) to a PDF byte
/// buffer with `genpdf` (#975.6). Generated on demand; the invoice's
/// `pdf_file_path` caching column is left for a future S3-archival follow-up.
fn render_invoice_pdf(resp: &db::models::InvoiceResponse) -> Result<Vec<u8>, String> {
    use genpdf::{elements, style::Style, Element};

    let inv = &resp.invoice;
    let font_family = genpdf::fonts::from_files(
        "/usr/share/fonts/truetype/liberation",
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("Failed to load font LiberationSans: {}", e))?;

    let mut doc = genpdf::Document::new(font_family);
    doc.set_title(format!("Invoice {}", inv.invoice_number));
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new(format!("Invoice {}", inv.invoice_number))
            .styled(Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Break::new(1));

    // Header metadata.
    let mut meta = elements::LinearLayout::vertical();
    meta.push(elements::Paragraph::new(format!(
        "Status: {:?}",
        inv.status
    )));
    meta.push(elements::Paragraph::new(format!(
        "Issue date: {}",
        inv.issue_date
    )));
    meta.push(elements::Paragraph::new(format!(
        "Due date: {}",
        inv.due_date
    )));
    if let (Some(start), Some(end)) = (inv.billing_period_start, inv.billing_period_end) {
        meta.push(elements::Paragraph::new(format!(
            "Billing period: {} - {}",
            start, end
        )));
    }
    meta.push(elements::Paragraph::new(format!("Unit: {}", inv.unit_id)));
    doc.push(meta);
    doc.push(elements::Break::new(1));

    // Line items table.
    let mut table = elements::TableLayout::new(vec![6, 2, 3, 3]);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));
    table
        .row()
        .element(elements::Paragraph::new("Description").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Qty").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Unit price").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Amount").styled(Style::new().bold()))
        .push()
        .map_err(|e| format!("invoice PDF table header: {}", e))?;
    for item in &resp.items {
        table
            .row()
            .element(elements::Paragraph::new(item.description.clone()))
            .element(elements::Paragraph::new(item.quantity.to_string()))
            .element(elements::Paragraph::new(format!(
                "{} {}",
                item.unit_price, inv.currency
            )))
            .element(elements::Paragraph::new(format!(
                "{} {}",
                item.amount, inv.currency
            )))
            .push()
            .map_err(|e| format!("invoice PDF table row: {}", e))?;
    }
    doc.push(table);
    doc.push(elements::Break::new(1));

    // Totals.
    let mut totals = elements::LinearLayout::vertical();
    totals.push(elements::Paragraph::new(format!(
        "Subtotal: {} {}",
        inv.subtotal, inv.currency
    )));
    totals.push(elements::Paragraph::new(format!(
        "Tax: {} {}",
        inv.tax_amount, inv.currency
    )));
    totals.push(
        elements::Paragraph::new(format!("Total: {} {}", inv.total, inv.currency))
            .styled(Style::new().bold()),
    );
    totals.push(elements::Paragraph::new(format!(
        "Paid: {} {}",
        inv.amount_paid, inv.currency
    )));
    totals.push(
        elements::Paragraph::new(format!("Balance due: {} {}", inv.balance_due, inv.currency))
            .styled(Style::new().bold()),
    );
    doc.push(totals);

    if let Some(notes) = &inv.notes {
        doc.push(elements::Break::new(1));
        doc.push(elements::Paragraph::new(format!("Notes: {}", notes)));
    }

    let mut bytes = Vec::new();
    doc.render(&mut bytes)
        .map_err(|e| format!("Failed to render invoice PDF: {}", e))?;
    Ok(bytes)
}

/// Generate and stream an invoice as a PDF (#975.6). Org-scoped via membership
/// (same guard as `get_invoice`): a non-member gets 404.
async fn get_invoice_pdf(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use axum::response::IntoResponse;

    let response = state
        .financial_repo
        .get_invoice_with_details(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice for PDF: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to load invoice")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
            )
        })?;

    if !is_member_or_not_found(&state, auth.user_id, response.invoice.organization_id).await? {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
        ));
    }

    let pdf_bytes = render_invoice_pdf(&response).map_err(|e| {
        tracing::error!("Failed to render invoice PDF: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "PDF_ERROR",
                "Failed to render invoice PDF",
            )),
        )
    })?;

    let headers = [
        (
            axum::http::header::CONTENT_TYPE,
            "application/pdf".to_string(),
        ),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"invoice-{}.pdf\"",
                response.invoice.invoice_number
            ),
        ),
    ];
    Ok((StatusCode::OK, headers, pdf_bytes).into_response())
}

async fn send_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::Invoice>, (StatusCode, Json<ErrorResponse>)> {
    // Verify ownership before mutating: load the invoice, check membership.
    match state.financial_repo.get_invoice(id).await {
        Ok(Some(existing)) => {
            if !is_member_or_not_found(&state, auth.user_id, existing.organization_id).await? {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
                ));
            }
        }
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to load invoice: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to send invoice")),
            ));
        }
    }
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
    auth: AuthUser,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<ListInvoicesQuery>,
) -> Result<Json<db::models::ListInvoicesResponse>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    let limit = query.limit.min(MAX_LIST_LIMIT);
    let mut response = state
        .financial_repo
        .list_invoices_for_unit(unit_id, query.status, limit, query.offset)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list unit invoices: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list invoices")),
            )
        })?;
    // Filter to the verified org: a foreign unit_id must not leak invoices.
    response
        .invoices
        .retain(|inv| inv.organization_id == query.organization_id);
    response.total = response.invoices.len() as i64;
    Ok(Json(response))
}

// ==================== Payment Handlers ====================

async fn record_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<RecordPaymentRequest>,
) -> Result<(StatusCode, Json<db::models::PaymentResponse>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, payload.organization_id).await?;
    state
        .financial_repo
        .record_payment(payload.organization_id, auth.user_id, payload.data)
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

/// Initiate a hosted Stripe Checkout for an invoice (Story 11.5, BIT-181).
///
/// Tenant isolation: the invoice is loaded by id, then the caller's membership
/// of the invoice's organization is verified — a foreign invoice id returns
/// `404` (so cross-tenant probing cannot distinguish missing from forbidden).
/// Returns the hosted `checkout_url` the client redirects the payer to; the
/// invoice is only marked paid once the signed `checkout.session.completed`
/// webhook is received.
async fn initiate_invoice_checkout(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(invoice_id): Path<Uuid>,
) -> Result<Json<InitiatePaymentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let cfg = &state.stripe_config;
    // Fail closed: never attempt a checkout without server-side credentials.
    if cfg.secret_key.is_empty() || cfg.success_url.is_empty() || cfg.cancel_url.is_empty() {
        tracing::error!(
            "Stripe Checkout is not fully configured (secret_key/success_url/cancel_url)"
        );
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "NOT_CONFIGURED",
                "Online payments are not configured",
            )),
        ));
    }

    let invoice = state
        .financial_repo
        .get_invoice(invoice_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load invoice: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to load invoice")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
            )
        })?;

    if !is_member_or_not_found(&state, auth.user_id, invoice.organization_id).await? {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Invoice not found")),
        ));
    }

    // Nothing to collect — don't open a gateway session for a settled invoice.
    if invoice.balance_due <= Decimal::ZERO {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "ALREADY_SETTLED",
                "Invoice has no outstanding balance",
            )),
        ));
    }

    let created = crate::services::stripe::create_checkout_session(
        &cfg.secret_key,
        &cfg.success_url,
        &cfg.cancel_url,
        &invoice.id.to_string(),
        &invoice.organization_id.to_string(),
        &invoice.invoice_number,
        invoice.balance_due,
        &invoice.currency,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Stripe checkout session creation failed");
        (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                "GATEWAY_ERROR",
                "Failed to create payment session",
            )),
        )
    })?;

    let session = state
        .financial_repo
        .create_payment_session(
            invoice.organization_id,
            invoice.id,
            "stripe",
            &created.id,
            &created.url,
            invoice.balance_due,
            &invoice.currency,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to persist payment session: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to record payment session",
                )),
            )
        })?;

    Ok(Json(InitiatePaymentResponse {
        session_id: session.id,
        checkout_url: created.url,
        expires_at: session.expires_at.unwrap_or(session.created_at),
    }))
}

async fn get_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<db::models::PaymentResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.financial_repo.get_payment_with_allocations(id).await {
        Ok(Some(payment)) => {
            if !is_member_or_not_found(&state, auth.user_id, payment.payment.organization_id)
                .await?
            {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Payment not found")),
                ));
            }
            Ok(Json(payment))
        }
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
    auth: AuthUser,
    Path(unit_id): Path<Uuid>,
    Query(query): Query<UnitPaymentsQuery>,
) -> Result<Json<Vec<db::models::Payment>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    let limit = query.limit.min(MAX_LIST_LIMIT);
    let payments = state
        .financial_repo
        .list_payments_for_unit(unit_id, limit, query.offset)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list payments: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list payments")),
            )
        })?;
    // Filter to the verified org: a foreign unit_id must not leak payments.
    let scoped = payments
        .into_iter()
        .filter(|p| p.organization_id == query.organization_id)
        .collect::<Vec<_>>();
    Ok(Json(scoped))
}

/// List all payments for an organization (paginated, optional status filter).
async fn list_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListPaymentsQuery>,
) -> Result<Json<PaymentListResponse>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    let limit = query.limit.min(MAX_LIST_LIMIT);
    let (payments, total) = tokio::try_join!(
        state.financial_repo.list_payments(
            query.organization_id,
            query.status.clone(),
            limit,
            query.offset,
        ),
        state
            .financial_repo
            .count_payments(query.organization_id, query.status.clone()),
    )
    .map_err(|e| {
        tracing::error!("Failed to list payments: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to list payments")),
        )
    })?;
    Ok(Json(PaymentListResponse { payments, total }))
}

/// List payments with an unallocated balance (the reconciliation queue).
async fn list_unallocated_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<UnallocatedPaymentsQuery>,
) -> Result<Json<Vec<Payment>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    let limit = query.limit.min(MAX_LIST_LIMIT);
    let payments = state
        .financial_repo
        .list_unallocated_payments(query.organization_id, limit, query.offset)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list unallocated payments: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list unallocated payments",
                )),
            )
        })?;
    Ok(Json(payments))
}

/// Allocate an existing payment to an invoice (manual match).
async fn allocate_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<AllocatePaymentRequest>,
) -> Result<(StatusCode, Json<PaymentAllocation>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, payload.organization_id).await?;
    let allocation = state
        .financial_repo
        .allocate_existing_payment(
            payload.organization_id,
            id,
            payload.invoice_id,
            payload.amount,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to allocate payment: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to allocate payment")),
            )
        })?;
    match allocation {
        Some(a) => Ok((StatusCode::CREATED, Json(a))),
        // payment/invoice not in org, or nothing left to allocate.
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                "Payment or invoice not found, or nothing to allocate",
            )),
        )),
    }
}

/// Bulk auto-match unallocated payments in an organization.
async fn auto_match_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<AutoMatchRequest>,
) -> Result<Json<AutoMatchResponse>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, payload.organization_id).await?;
    let matched = state
        .financial_repo
        .auto_match_payments(payload.organization_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to auto-match payments: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to auto-match payments",
                )),
            )
        })?;
    Ok(Json(AutoMatchResponse {
        matched: matched as i64,
    }))
}

// ==================== Reminder/Late Fee Handlers ====================

async fn get_reminder_schedules(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<db::models::ReminderSchedule>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
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
    auth: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<db::models::LateFeeConfig>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
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
    auth: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<Vec<db::models::Invoice>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
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
    auth: AuthUser,
    Query(query): Query<ARReportQuery>,
) -> Result<Json<db::models::AccountsReceivableReport>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
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

// ==================== Financial Statement Handlers (Story 11.7) ====================

async fn get_income_statement(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<DateRangeReportQuery>,
) -> Result<Json<db::models::IncomeStatement>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    state
        .financial_repo
        .get_income_statement(query.organization_id, query.from, query.to)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to generate income statement: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to generate income statement",
                )),
            )
        })
}

async fn get_balance_sheet(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<BalanceSheetQuery>,
) -> Result<Json<db::models::BalanceSheetReport>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    let as_of = query
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    state
        .financial_repo
        .get_balance_sheet(query.organization_id, as_of)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to generate balance sheet: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to generate balance sheet",
                )),
            )
        })
}

async fn get_cash_flow(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<DateRangeReportQuery>,
) -> Result<Json<db::models::CashFlowReport>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, auth.user_id, query.organization_id).await?;
    state
        .financial_repo
        .get_cash_flow(query.organization_id, query.from, query.to)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to generate cash flow report: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to generate cash flow report",
                )),
            )
        })
}

/// Render an income statement to PDF bytes using genpdf.
fn render_income_statement_pdf(report: &db::models::IncomeStatement) -> Result<Vec<u8>, String> {
    use genpdf::{elements, style::Style, Element};

    let font_family = genpdf::fonts::from_files(
        "/usr/share/fonts/truetype/liberation",
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("font load error: {}", e))?;
    let mut doc = genpdf::Document::new(font_family);
    doc.set_title("Income Statement");
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new("Income Statement").styled(Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Paragraph::new(format!(
        "Period: {} – {}",
        report.from_date, report.to_date
    )));
    doc.push(elements::Break::new(1));

    doc.push(elements::Paragraph::new("Revenue").styled(Style::new().bold().with_font_size(13)));
    for line in &report.revenue {
        doc.push(elements::Paragraph::new(format!(
            "  {}: {}",
            line.category, line.amount
        )));
    }
    doc.push(
        elements::Paragraph::new(format!("Total Revenue: {}", report.total_revenue))
            .styled(Style::new().bold()),
    );
    doc.push(elements::Break::new(1));

    doc.push(elements::Paragraph::new("Expenses").styled(Style::new().bold().with_font_size(13)));
    for line in &report.expenses {
        doc.push(elements::Paragraph::new(format!(
            "  {}: {}",
            line.category, line.amount
        )));
    }
    doc.push(
        elements::Paragraph::new(format!("Total Expenses: {}", report.total_expenses))
            .styled(Style::new().bold()),
    );
    doc.push(elements::Break::new(1));

    doc.push(
        elements::Paragraph::new(format!("Net Income: {}", report.net_income))
            .styled(Style::new().bold().with_font_size(14)),
    );

    let mut bytes = Vec::new();
    doc.render(&mut bytes)
        .map_err(|e| format!("PDF render error: {}", e))?;
    Ok(bytes)
}

/// Render a balance sheet to PDF bytes.
fn render_balance_sheet_pdf(report: &db::models::BalanceSheetReport) -> Result<Vec<u8>, String> {
    use genpdf::{elements, style::Style, Element};

    let font_family = genpdf::fonts::from_files(
        "/usr/share/fonts/truetype/liberation",
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("font load error: {}", e))?;
    let mut doc = genpdf::Document::new(font_family);
    doc.set_title("Balance Sheet");
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new("Balance Sheet").styled(Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Paragraph::new(format!(
        "As of: {}",
        report.as_of_date
    )));
    doc.push(elements::Break::new(1));

    let mut table = elements::TableLayout::new(vec![6, 3, 3]);
    table.set_cell_decorator(elements::FrameCellDecorator::new(true, true, false));
    table
        .row()
        .element(elements::Paragraph::new("Account").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Type").styled(Style::new().bold()))
        .element(elements::Paragraph::new("Balance").styled(Style::new().bold()))
        .push()
        .map_err(|e| format!("table header: {}", e))?;
    for line in &report.accounts {
        table
            .row()
            .element(elements::Paragraph::new(line.account_name.clone()))
            .element(elements::Paragraph::new(line.account_type.clone()))
            .element(elements::Paragraph::new(line.balance.to_string()))
            .push()
            .map_err(|e| format!("table row: {}", e))?;
    }
    doc.push(table);
    doc.push(elements::Break::new(1));

    doc.push(
        elements::Paragraph::new(format!("Total Assets: {}", report.total_assets))
            .styled(Style::new().bold()),
    );
    doc.push(
        elements::Paragraph::new(format!("Total Liabilities: {}", report.total_liabilities))
            .styled(Style::new().bold()),
    );
    doc.push(
        elements::Paragraph::new(format!("Net Equity: {}", report.net_equity))
            .styled(Style::new().bold()),
    );

    let mut bytes = Vec::new();
    doc.render(&mut bytes)
        .map_err(|e| format!("PDF render error: {}", e))?;
    Ok(bytes)
}

/// Render a cash flow report to PDF bytes.
fn render_cash_flow_pdf(report: &db::models::CashFlowReport) -> Result<Vec<u8>, String> {
    use genpdf::{elements, style::Style, Element};

    let font_family = genpdf::fonts::from_files(
        "/usr/share/fonts/truetype/liberation",
        "LiberationSans",
        None,
    )
    .map_err(|e| format!("font load error: {}", e))?;
    let mut doc = genpdf::Document::new(font_family);
    doc.set_title("Cash Flow Statement");
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(15);
    doc.set_page_decorator(decorator);

    doc.push(
        elements::Paragraph::new("Cash Flow Statement")
            .styled(Style::new().bold().with_font_size(18)),
    );
    doc.push(elements::Paragraph::new(format!(
        "Period: {} – {}",
        report.from_date, report.to_date
    )));
    doc.push(elements::Break::new(1));

    doc.push(elements::Paragraph::new("Inflows").styled(Style::new().bold().with_font_size(13)));
    for line in &report.inflows {
        doc.push(elements::Paragraph::new(format!(
            "  {}: {}",
            line.category, line.amount
        )));
    }
    doc.push(
        elements::Paragraph::new(format!("Total Inflows: {}", report.total_inflows))
            .styled(Style::new().bold()),
    );
    doc.push(elements::Break::new(1));

    doc.push(elements::Paragraph::new("Outflows").styled(Style::new().bold().with_font_size(13)));
    for line in &report.outflows {
        doc.push(elements::Paragraph::new(format!(
            "  {}: {}",
            line.category, line.amount
        )));
    }
    doc.push(
        elements::Paragraph::new(format!("Total Outflows: {}", report.total_outflows))
            .styled(Style::new().bold()),
    );
    doc.push(elements::Break::new(1));

    doc.push(
        elements::Paragraph::new(format!("Net Cash Flow: {}", report.net_cash_flow))
            .styled(Style::new().bold().with_font_size(14)),
    );

    let mut bytes = Vec::new();
    doc.render(&mut bytes)
        .map_err(|e| format!("PDF render error: {}", e))?;
    Ok(bytes)
}

/// Render an income statement to xlsx bytes.
fn render_income_statement_xlsx(report: &db::models::IncomeStatement) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Income Statement")
        .map_err(|e| e.to_string())?;

    sheet
        .write(0, 0, "Income Statement")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            1,
            0,
            format!("Period: {} to {}", report.from_date, report.to_date),
        )
        .map_err(|e| e.to_string())?;

    let mut row: u32 = 3;
    sheet.write(row, 0, "Revenue").map_err(|e| e.to_string())?;
    sheet.write(row, 1, "Amount").map_err(|e| e.to_string())?;
    row += 1;
    for line in &report.revenue {
        sheet
            .write(row, 0, line.category.as_str())
            .map_err(|e| e.to_string())?;
        sheet
            .write(
                row,
                1,
                line.amount.to_string().parse::<f64>().unwrap_or(0.0),
            )
            .map_err(|e| e.to_string())?;
        row += 1;
    }
    sheet
        .write(row, 0, "Total Revenue")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            1,
            report
                .total_revenue
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;

    row += 2;
    sheet.write(row, 0, "Expenses").map_err(|e| e.to_string())?;
    sheet.write(row, 1, "Amount").map_err(|e| e.to_string())?;
    row += 1;
    for line in &report.expenses {
        sheet
            .write(row, 0, line.category.as_str())
            .map_err(|e| e.to_string())?;
        sheet
            .write(
                row,
                1,
                line.amount.to_string().parse::<f64>().unwrap_or(0.0),
            )
            .map_err(|e| e.to_string())?;
        row += 1;
    }
    sheet
        .write(row, 0, "Total Expenses")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            1,
            report
                .total_expenses
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;

    row += 2;
    sheet
        .write(row, 0, "Net Income")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            1,
            report.net_income.to_string().parse::<f64>().unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;

    workbook.save_to_buffer().map_err(|e| e.to_string())
}

/// Render a balance sheet to xlsx bytes.
fn render_balance_sheet_xlsx(report: &db::models::BalanceSheetReport) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Balance Sheet").map_err(|e| e.to_string())?;

    sheet
        .write(0, 0, "Balance Sheet")
        .map_err(|e| e.to_string())?;
    sheet
        .write(1, 0, format!("As of: {}", report.as_of_date))
        .map_err(|e| e.to_string())?;

    let mut row: u32 = 3;
    sheet.write(row, 0, "Account").map_err(|e| e.to_string())?;
    sheet.write(row, 1, "Type").map_err(|e| e.to_string())?;
    sheet.write(row, 2, "Balance").map_err(|e| e.to_string())?;
    row += 1;
    for line in &report.accounts {
        sheet
            .write(row, 0, line.account_name.as_str())
            .map_err(|e| e.to_string())?;
        sheet
            .write(row, 1, line.account_type.as_str())
            .map_err(|e| e.to_string())?;
        sheet
            .write(
                row,
                2,
                line.balance.to_string().parse::<f64>().unwrap_or(0.0),
            )
            .map_err(|e| e.to_string())?;
        row += 1;
    }
    row += 1;
    sheet
        .write(row, 0, "Total Assets")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            2,
            report
                .total_assets
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;
    row += 1;
    sheet
        .write(row, 0, "Total Liabilities")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            2,
            report
                .total_liabilities
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;
    row += 1;
    sheet
        .write(row, 0, "Net Equity")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            2,
            report.net_equity.to_string().parse::<f64>().unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;

    workbook.save_to_buffer().map_err(|e| e.to_string())
}

/// Render a cash flow report to xlsx bytes.
fn render_cash_flow_xlsx(report: &db::models::CashFlowReport) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Cash Flow").map_err(|e| e.to_string())?;

    sheet
        .write(0, 0, "Cash Flow Statement")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            1,
            0,
            format!("Period: {} to {}", report.from_date, report.to_date),
        )
        .map_err(|e| e.to_string())?;

    let mut row: u32 = 3;
    sheet.write(row, 0, "Inflows").map_err(|e| e.to_string())?;
    sheet.write(row, 1, "Amount").map_err(|e| e.to_string())?;
    row += 1;
    for line in &report.inflows {
        sheet
            .write(row, 0, line.category.as_str())
            .map_err(|e| e.to_string())?;
        sheet
            .write(
                row,
                1,
                line.amount.to_string().parse::<f64>().unwrap_or(0.0),
            )
            .map_err(|e| e.to_string())?;
        row += 1;
    }
    sheet
        .write(row, 0, "Total Inflows")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            1,
            report
                .total_inflows
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;

    row += 2;
    sheet.write(row, 0, "Outflows").map_err(|e| e.to_string())?;
    sheet.write(row, 1, "Amount").map_err(|e| e.to_string())?;
    row += 1;
    for line in &report.outflows {
        sheet
            .write(row, 0, line.category.as_str())
            .map_err(|e| e.to_string())?;
        sheet
            .write(
                row,
                1,
                line.amount.to_string().parse::<f64>().unwrap_or(0.0),
            )
            .map_err(|e| e.to_string())?;
        row += 1;
    }
    sheet
        .write(row, 0, "Total Outflows")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            1,
            report
                .total_outflows
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;

    row += 2;
    sheet
        .write(row, 0, "Net Cash Flow")
        .map_err(|e| e.to_string())?;
    sheet
        .write(
            row,
            1,
            report
                .net_cash_flow
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0),
        )
        .map_err(|e| e.to_string())?;

    workbook.save_to_buffer().map_err(|e| e.to_string())
}

/// Export a financial report as PDF or xlsx.
/// Path: GET /financial/reports/{report}/export
/// `report` is one of: income-statement, balance-sheet, cash-flow
async fn export_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(report): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<axum::response::Response, (StatusCode, Json<ErrorResponse>)> {
    use axum::response::IntoResponse;

    verify_org_access(&state, auth.user_id, query.organization_id).await?;

    let format = query.format.to_lowercase();
    if format != "pdf" && format != "xlsx" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "format must be pdf or xlsx",
            )),
        ));
    }

    let today = chrono::Utc::now().date_naive();

    match report.as_str() {
        "income-statement" => {
            let from = query.from.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "BAD_REQUEST",
                        "from date required for income-statement",
                    )),
                )
            })?;
            let to = query.to.unwrap_or(today);
            let data = state
                .financial_repo
                .get_income_statement(query.organization_id, from, to)
                .await
                .map_err(|e| {
                    tracing::error!("income statement export: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "DB_ERROR",
                            "Failed to load income statement",
                        )),
                    )
                })?;
            if format == "pdf" {
                let bytes = render_income_statement_pdf(&data).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("PDF_ERROR", &e)),
                    )
                })?;
                let headers = [
                    (axum::http::header::CONTENT_TYPE, "application/pdf"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"income-statement.pdf\"",
                    ),
                ];
                Ok((StatusCode::OK, headers, bytes).into_response())
            } else {
                let bytes = render_income_statement_xlsx(&data).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("XLSX_ERROR", &e)),
                    )
                })?;
                let headers = [
                    (
                        axum::http::header::CONTENT_TYPE,
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    ),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"income-statement.xlsx\"",
                    ),
                ];
                Ok((StatusCode::OK, headers, bytes).into_response())
            }
        }
        "balance-sheet" => {
            let as_of = query.as_of.unwrap_or(today);
            let data = state
                .financial_repo
                .get_balance_sheet(query.organization_id, as_of)
                .await
                .map_err(|e| {
                    tracing::error!("balance sheet export: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "DB_ERROR",
                            "Failed to load balance sheet",
                        )),
                    )
                })?;
            if format == "pdf" {
                let bytes = render_balance_sheet_pdf(&data).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("PDF_ERROR", &e)),
                    )
                })?;
                let headers = [
                    (axum::http::header::CONTENT_TYPE, "application/pdf"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"balance-sheet.pdf\"",
                    ),
                ];
                Ok((StatusCode::OK, headers, bytes).into_response())
            } else {
                let bytes = render_balance_sheet_xlsx(&data).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("XLSX_ERROR", &e)),
                    )
                })?;
                let headers = [
                    (
                        axum::http::header::CONTENT_TYPE,
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    ),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"balance-sheet.xlsx\"",
                    ),
                ];
                Ok((StatusCode::OK, headers, bytes).into_response())
            }
        }
        "cash-flow" => {
            let from = query.from.ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "BAD_REQUEST",
                        "from date required for cash-flow",
                    )),
                )
            })?;
            let to = query.to.unwrap_or(today);
            let data = state
                .financial_repo
                .get_cash_flow(query.organization_id, from, to)
                .await
                .map_err(|e| {
                    tracing::error!("cash flow export: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("DB_ERROR", "Failed to load cash flow")),
                    )
                })?;
            if format == "pdf" {
                let bytes = render_cash_flow_pdf(&data).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("PDF_ERROR", &e)),
                    )
                })?;
                let headers = [
                    (axum::http::header::CONTENT_TYPE, "application/pdf"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"cash-flow.pdf\"",
                    ),
                ];
                Ok((StatusCode::OK, headers, bytes).into_response())
            } else {
                let bytes = render_cash_flow_xlsx(&data).map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("XLSX_ERROR", &e)),
                    )
                })?;
                let headers = [
                    (
                        axum::http::header::CONTENT_TYPE,
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    ),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        "attachment; filename=\"cash-flow.xlsx\"",
                    ),
                ];
                Ok((StatusCode::OK, headers, bytes).into_response())
            }
        }
        _ => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Unknown report type")),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use db::models::{Invoice, InvoiceItem, InvoiceResponse, InvoiceStatus};

    fn sample_invoice_response() -> InvoiceResponse {
        let now = Utc::now();
        InvoiceResponse {
            invoice: Invoice {
                id: Uuid::new_v4(),
                organization_id: Uuid::new_v4(),
                unit_id: Uuid::new_v4(),
                invoice_number: "INV-2026-001".to_string(),
                billing_period_start: Some(now.date_naive()),
                billing_period_end: Some(now.date_naive()),
                status: InvoiceStatus::Sent,
                issue_date: now.date_naive(),
                due_date: now.date_naive(),
                paid_date: None,
                subtotal: Decimal::new(10000, 2),
                tax_amount: Decimal::new(2000, 2),
                total: Decimal::new(12000, 2),
                amount_paid: Decimal::ZERO,
                balance_due: Decimal::new(12000, 2),
                currency: "EUR".to_string(),
                notes: Some("Thank you for your business".to_string()),
                internal_notes: None,
                pdf_file_path: None,
                pdf_generated_at: None,
                created_by: None,
                sent_at: None,
                created_at: now,
                updated_at: now,
            },
            items: vec![InvoiceItem {
                id: Uuid::new_v4(),
                invoice_id: Uuid::new_v4(),
                description: "Monthly service fee".to_string(),
                quantity: Decimal::new(100, 2),
                unit_price: Decimal::new(10000, 2),
                amount: Decimal::new(10000, 2),
                tax_rate: Some(Decimal::new(2000, 2)),
                tax_amount: Some(Decimal::new(2000, 2)),
                fee_schedule_id: None,
                meter_reading_id: None,
                sort_order: 0,
                created_at: now,
            }],
            payments: vec![],
        }
    }

    #[test]
    fn render_income_statement_pdf_produces_valid_pdf() {
        use db::models::{IncomeStatement, IncomeStatementLine};
        use rust_decimal::Decimal;
        let report = IncomeStatement {
            from_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            to_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            revenue: vec![IncomeStatementLine {
                category: "payment_received".into(),
                amount: Decimal::new(50000, 2),
            }],
            expenses: vec![IncomeStatementLine {
                category: "maintenance_fee".into(),
                amount: Decimal::new(10000, 2),
            }],
            total_revenue: Decimal::new(50000, 2),
            total_expenses: Decimal::new(10000, 2),
            net_income: Decimal::new(40000, 2),
        };
        let bytes = render_income_statement_pdf(&report).expect("render income statement PDF");
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[..5], b"%PDF-");
    }

    #[test]
    fn render_balance_sheet_pdf_produces_valid_pdf() {
        use db::models::{BalanceSheetLine, BalanceSheetReport};
        use rust_decimal::Decimal;
        let report = BalanceSheetReport {
            as_of_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            accounts: vec![BalanceSheetLine {
                account_id: uuid::Uuid::new_v4(),
                account_name: "Operating Account".into(),
                account_type: "operating".into(),
                balance: Decimal::new(100000, 2),
            }],
            total_assets: Decimal::new(100000, 2),
            total_liabilities: Decimal::ZERO,
            net_equity: Decimal::new(100000, 2),
        };
        let bytes = render_balance_sheet_pdf(&report).expect("render balance sheet PDF");
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[..5], b"%PDF-");
    }

    #[test]
    fn render_cash_flow_pdf_produces_valid_pdf() {
        use db::models::{CashFlowLine, CashFlowReport};
        use rust_decimal::Decimal;
        let report = CashFlowReport {
            from_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            to_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            inflows: vec![CashFlowLine {
                category: "payment_received".into(),
                amount: Decimal::new(50000, 2),
            }],
            outflows: vec![CashFlowLine {
                category: "maintenance_fee".into(),
                amount: Decimal::new(10000, 2),
            }],
            total_inflows: Decimal::new(50000, 2),
            total_outflows: Decimal::new(10000, 2),
            net_cash_flow: Decimal::new(40000, 2),
        };
        let bytes = render_cash_flow_pdf(&report).expect("render cash flow PDF");
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[..5], b"%PDF-");
    }

    #[test]
    fn render_income_statement_xlsx_produces_valid_xlsx() {
        use db::models::{IncomeStatement, IncomeStatementLine};
        use rust_decimal::Decimal;
        let report = IncomeStatement {
            from_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            to_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            revenue: vec![IncomeStatementLine {
                category: "payment_received".into(),
                amount: Decimal::new(50000, 2),
            }],
            expenses: vec![],
            total_revenue: Decimal::new(50000, 2),
            total_expenses: Decimal::ZERO,
            net_income: Decimal::new(50000, 2),
        };
        let bytes = render_income_statement_xlsx(&report).expect("render xlsx");
        // xlsx files start with PK (zip)
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[..2], b"PK");
    }

    #[test]
    fn render_invoice_pdf_produces_a_valid_pdf() {
        let resp = sample_invoice_response();
        let bytes = render_invoice_pdf(&resp).expect("render invoice PDF");
        assert!(
            bytes.len() > 100,
            "PDF should be non-trivial, got {} bytes",
            bytes.len()
        );
        assert_eq!(&bytes[..5], b"%PDF-", "output must be a PDF");
    }

    #[test]
    fn render_invoice_pdf_handles_no_line_items() {
        let mut resp = sample_invoice_response();
        resp.items.clear();
        let bytes = render_invoice_pdf(&resp).expect("render empty-items invoice PDF");
        assert_eq!(&bytes[..5], b"%PDF-");
    }
}

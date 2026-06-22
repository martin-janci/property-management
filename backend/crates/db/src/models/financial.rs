//! Financial management models (Epic 11).
//!
//! Provides types for financial accounts, transactions, fee schedules,
//! invoices, payments, and related financial operations.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// ENUMS
// ============================================================================

/// Type of financial account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "financial_account_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FinancialAccountType {
    Operating,
    Reserve,
    Utilities,
    UnitLedger,
    Custom,
}

/// Type of transaction (debit or credit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "transaction_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Debit,
    Credit,
}

/// Category of transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "transaction_category", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionCategory {
    MaintenanceFee,
    UtilityCharge,
    SpecialAssessment,
    Penalty,
    PaymentReceived,
    Refund,
    Transfer,
    Adjustment,
    OpeningBalance,
    Other,
}

/// Frequency of recurring fees.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "fee_frequency", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FeeFrequency {
    #[default]
    Monthly,
    Quarterly,
    SemiAnnual,
    Annual,
    OneTime,
}

/// Invoice status.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "invoice_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    #[default]
    Draft,
    Sent,
    Paid,
    Partial,
    Overdue,
    Cancelled,
    Void,
}

/// Payment method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    BankTransfer,
    Card,
    Cash,
    Check,
    Online,
    DirectDebit,
    Other,
}

/// Payment status.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[sqlx(type_name = "payment_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    #[default]
    Pending,
    Completed,
    Failed,
    Refunded,
    Cancelled,
}

// ============================================================================
// FINANCIAL ACCOUNTS (Story 11.1)
// ============================================================================

/// Financial account entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FinancialAccount {
    pub id: Uuid,
    pub organization_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Uuid>,
    pub name: String,
    pub account_type: FinancialAccountType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub currency: String,
    pub balance: Decimal,
    pub opening_balance: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a financial account.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFinancialAccount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub building_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_id: Option<Uuid>,
    pub name: String,
    pub account_type: FinancialAccountType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub opening_balance: Decimal,
}

fn default_currency() -> String {
    "EUR".to_string()
}

/// Account transaction entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountTransaction {
    pub id: Uuid,
    pub account_id: Uuid,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    pub category: TransactionCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterpart_account_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<Uuid>,
    pub balance_after: Decimal,
    pub transaction_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a transaction.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateTransaction {
    pub account_id: Uuid,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    pub category: TransactionCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterpart_account_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

// ============================================================================
// FEE SCHEDULES (Story 11.2)
// ============================================================================

/// Fee schedule entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FeeSchedule {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub frequency: FeeFrequency,
    pub unit_filter: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_day: Option<i32>,
    pub is_active: bool,
    pub effective_from: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_to: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a fee schedule.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateFeeSchedule {
    pub building_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub amount: Decimal,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default)]
    pub frequency: FeeFrequency,
    #[serde(default)]
    pub unit_filter: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_day: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_from: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_to: Option<NaiveDate>,
}

/// Unit fee assignment.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitFee {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub fee_schedule_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_amount: Option<Decimal>,
    pub effective_from: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_to: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// INVOICES (Story 11.3)
// ============================================================================

/// Invoice entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Invoice {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub unit_id: Uuid,
    pub invoice_number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period_start: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period_end: Option<NaiveDate>,
    pub status: InvoiceStatus,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_date: Option<NaiveDate>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub total: Decimal,
    pub amount_paid: Decimal,
    pub balance_due: Decimal,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_generated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Invoice line item.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct InvoiceItem {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_schedule_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_reading_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Request to create an invoice.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateInvoice {
    pub unit_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period_start: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period_end: Option<NaiveDate>,
    pub due_date: NaiveDate,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub items: Vec<CreateInvoiceItem>,
}

/// Request to create an invoice item.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateInvoiceItem {
    pub description: String,
    #[serde(default = "default_quantity")]
    pub quantity: Decimal,
    pub unit_price: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_schedule_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_reading_id: Option<Uuid>,
}

fn default_quantity() -> Decimal {
    Decimal::ONE
}

// ============================================================================
// PAYMENTS (Story 11.4)
// ============================================================================

/// Payment entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Payment {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub unit_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub payment_method: PaymentMethod,
    pub status: PaymentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_reference: Option<String>,
    pub payment_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to record a payment.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecordPayment {
    pub unit_id: Uuid,
    pub amount: Decimal,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub payment_method: PaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Invoice IDs to allocate this payment to (optional, auto-allocates if empty)
    #[serde(default)]
    pub invoice_ids: Vec<Uuid>,
}

/// Payment allocation to invoice.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PaymentAllocation {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub invoice_id: Uuid,
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
}

/// Unit credit balance.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UnitCreditBalance {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub balance: Decimal,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// ONLINE PAYMENTS (Story 11.5)
// ============================================================================

/// Online payment session.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OnlinePaymentSession {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub invoice_id: Uuid,
    pub provider: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkout_url: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response for online payment initiation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InitiatePaymentResponse {
    pub session_id: Uuid,
    pub checkout_url: String,
    pub expires_at: DateTime<Utc>,
}

// ============================================================================
// PAYMENT REMINDERS (Story 11.6)
// ============================================================================

/// Reminder schedule.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ReminderSchedule {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_before_due: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_after_due: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_template_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification_template: Option<String>,
    pub is_active: bool,
    pub include_sms: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Late fee configuration.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct LateFeeConfig {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub enabled: bool,
    pub grace_period_days: i32,
    pub fee_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_amount: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// RESPONSES
// ============================================================================

/// Response with financial account details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FinancialAccountResponse {
    pub account: FinancialAccount,
    pub recent_transactions: Vec<AccountTransaction>,
}

/// Response with invoice details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InvoiceResponse {
    pub invoice: Invoice,
    pub items: Vec<InvoiceItem>,
    pub payments: Vec<PaymentAllocation>,
}

/// Response listing invoices.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListInvoicesResponse {
    pub invoices: Vec<Invoice>,
    pub total: i64,
}

/// Response with payment details.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaymentResponse {
    pub payment: Payment,
    pub allocations: Vec<PaymentAllocation>,
}

/// Accounts receivable aging report.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountsReceivableReport {
    pub as_of_date: NaiveDate,
    pub entries: Vec<ARReportEntry>,
    pub totals: ARReportTotals,
}

/// Single entry in AR report.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct ARReportEntry {
    pub unit_id: Uuid,
    pub unit_number: String,
    pub current: Decimal,
    pub days_30: Decimal,
    pub days_60: Decimal,
    pub days_90_plus: Decimal,
    pub total: Decimal,
}

/// AR report totals.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ARReportTotals {
    pub current: Decimal,
    pub days_30: Decimal,
    pub days_60: Decimal,
    pub days_90_plus: Decimal,
    pub total: Decimal,
}

// ============================================================================
// Story 11.7 — Financial statement reports
// ============================================================================

/// Single line item in an income statement (revenue or expense).
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct IncomeStatementLine {
    pub category: String,
    pub amount: Decimal,
}

/// Income statement for a date range.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IncomeStatement {
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub revenue: Vec<IncomeStatementLine>,
    pub expenses: Vec<IncomeStatementLine>,
    pub total_revenue: Decimal,
    pub total_expenses: Decimal,
    pub net_income: Decimal,
}

/// One account row in the balance sheet.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct BalanceSheetLine {
    pub account_id: Uuid,
    pub account_name: String,
    pub account_type: String,
    pub balance: Decimal,
}

/// Balance sheet snapshot as of a date.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BalanceSheetReport {
    pub as_of_date: NaiveDate,
    pub accounts: Vec<BalanceSheetLine>,
    pub total_assets: Decimal,
    pub total_liabilities: Decimal,
    pub net_equity: Decimal,
}

/// Single line item in a cash flow report.
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct CashFlowLine {
    pub category: String,
    pub amount: Decimal,
}

/// Cash flow report for a date range.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CashFlowReport {
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub inflows: Vec<CashFlowLine>,
    pub outflows: Vec<CashFlowLine>,
    pub total_inflows: Decimal,
    pub total_outflows: Decimal,
    pub net_cash_flow: Decimal,
}

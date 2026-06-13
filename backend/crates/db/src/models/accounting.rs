//! Native accounting MVP models (PAP-206).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Native contact (CRM).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Contact {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub iban: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status of an issued invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Paid,
    PartiallyPaid,
    Overdue,
}

/// Native issued invoice.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Invoice {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub contact_id: Uuid,
    pub number: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub taxable_supply_date: Option<NaiveDate>,
    pub currency: String,
    pub total_amount: Decimal,
    pub base_amount: Decimal,
    pub vat_amount: Decimal,
    pub variable_symbol: Option<String>,
    pub status: InvoiceStatus,
    pub paid_amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Line item for an invoice.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct InvoiceItem {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub tenant_id: Uuid,
    pub description: String,
    pub qty: Decimal,
    pub unit_price: Decimal,
    pub vat_rate: Decimal,
    pub total_amount: Decimal,
    pub base_amount: Decimal,
    pub vat_amount: Decimal,
}

/// Imported bank statement metadata.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct BankStatement {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub source_filename: String,
    pub imported_at: DateTime<Utc>,
    pub period: Option<String>,
    pub account_iban: String,
}

/// Individual line from a bank statement.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct BankStatementLine {
    pub id: Uuid,
    pub statement_id: Uuid,
    pub tenant_id: Uuid,
    pub booking_date: NaiveDate,
    pub amount: Decimal,
    pub currency: String,
    pub counterparty_iban: Option<String>,
    pub variable_symbol: Option<String>,
    pub raw_ref: Option<String>,
    pub match_state: String,
}

/// State of a payment match decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PaymentMatchState {
    Suggested,
    Confirmed,
    Rejected,
}

/// Decision or suggestion matching a statement line to an invoice.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct PaymentMatch {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub statement_line_id: Uuid,
    pub invoice_id: Uuid,
    pub confidence: Decimal,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub state: PaymentMatchState,
}

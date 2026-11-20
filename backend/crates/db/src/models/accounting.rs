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
///
/// Full lifecycle (UC-ACC-05.17): draft → issued → sent → paid/partially_paid/
/// overdue, with cancelled reachable from issued/sent/overdue while nothing is
/// paid. The DB CHECK (migration 00202) allows the same set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Issued,
    Sent,
    Paid,
    PartiallyPaid,
    Overdue,
    Cancelled,
}

/// VAT rate types for CZ/SK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VatRate {
    /// Zero VAT (non-taxable) - 0%
    Zero,
    /// Reduced rate (10% in CZ, 10% in SK)
    Reduced,
    /// First reduced rate (15% in CZ, currently not used in SK)
    FirstReduced,
    /// Standard rate (21% in CZ, 20% in SK)
    Standard,
}

impl VatRate {
    /// Get the VAT percentage for Czech Republic.
    pub fn cz_percentage(&self) -> Decimal {
        match self {
            VatRate::Zero => Decimal::ZERO,
            VatRate::Reduced => Decimal::from(10),
            VatRate::FirstReduced => Decimal::from(15),
            VatRate::Standard => Decimal::from(21),
        }
    }

    /// Get the VAT percentage for Slovakia.
    pub fn sk_percentage(&self) -> Decimal {
        match self {
            VatRate::Zero => Decimal::ZERO,
            VatRate::Reduced => Decimal::from(10),
            VatRate::FirstReduced => Decimal::from(10), // SK doesn't have 15% rate
            VatRate::Standard => Decimal::from(20),
        }
    }
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

/// Request to create a new invoice.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInvoice {
    pub tenant_id: Uuid,
    pub contact_id: Uuid,
    pub number: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub taxable_supply_date: Option<NaiveDate>,
    pub currency: String,
    pub variable_symbol: Option<String>,
    pub status: Option<InvoiceStatus>,
    pub items: Vec<CreateInvoiceItem>,
    pub country: Option<String>,
}

/// Request to update an existing invoice.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateInvoice {
    pub contact_id: Option<Uuid>,
    pub number: Option<String>,
    pub issue_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub taxable_supply_date: Option<Option<NaiveDate>>,
    pub currency: Option<String>,
    pub variable_symbol: Option<Option<String>>,
    pub status: Option<InvoiceStatus>,
    pub paid_amount: Option<Decimal>,
}

/// Request to create a new invoice item.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInvoiceItem {
    pub description: String,
    pub qty: Decimal,
    pub unit_price: Decimal,
    pub vat_rate: Decimal,
    /// Optional named VAT rate (overrides vat_rate if provided based on tenant country)
    pub vat_rate_type: Option<VatRate>,
}

/// Request to update an existing invoice item.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateInvoiceItem {
    pub description: Option<String>,
    pub qty: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub vat_rate: Option<Decimal>,
    pub vat_rate_type: Option<VatRate>,
}

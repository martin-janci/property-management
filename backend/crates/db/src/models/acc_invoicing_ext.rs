//! ACC sales-invoicing extension models (EPIC-ACC-05).
//!
//! The base `invoice` / `invoice_item` rows remain
//! [`crate::models::accounting::Invoice`] / `InvoiceItem`; these types cover the
//! invoicing extensions in migration 00196 (`acc_document_link`,
//! `acc_attachment`) plus an extended invoice projection that includes the
//! doc-type / correction / advance-settlement / FX columns added by ALTER on
//! `invoice`.
//!
//! NOTE: the base `Invoice` struct (models/accounting.rs) is owned by Foundation
//! and intentionally NOT changed; coders read the new columns via
//! [`AccInvoiceExt`] which `SELECT`s the extended column set.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Document type for an ACC document (UC-ACC-05.4/.5/.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AccDocType {
    Invoice,
    Proforma,
    Advance,
    CreditNote,
}

/// Typed relationship edge between two documents (UC-ACC-05.15).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccDocumentLink {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub from_invoice_id: Uuid,
    pub to_invoice_id: Uuid,
    pub relation: String,
    pub created_at: DateTime<Utc>,
}

/// File attached to a document (UC-ACC-05.13).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccAttachment {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub storage_key: String,
    pub uploaded_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Extended invoice projection including the columns added to `invoice` by
/// migration 00196 (UC-ACC-05.4/.5/.6/.7/.15).
///
/// Field order matches a `SELECT` from `invoice` after the base 00184 columns.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccInvoiceExt {
    // Base 00184 columns
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
    pub status: String,
    pub paid_amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // 00196 extension columns
    pub doc_type: String,
    pub corrected_invoice_id: Option<Uuid>,
    pub advance_settlement: Decimal,
    pub settled_advance_id: Option<Uuid>,
    pub exchange_rate: Option<Decimal>,
    pub base_currency_amount: Option<Decimal>,
    pub numbering_series_id: Option<Uuid>,
    pub bank_account_id: Option<Uuid>,
}

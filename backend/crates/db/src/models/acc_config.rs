//! ACC company & document configuration models (EPIC-ACC-02).
//!
//! Structs match migration 00194 (`acc_company_settings`, `acc_numbering_series`,
//! `acc_unit`, `acc_vat_rate`, `acc_bank_account`, `acc_email_template`,
//! `acc_document_template`). Column names verified against 00194.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Per-company settings driving every document (UC-ACC-02.1/.2/.11).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccCompanySettings {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub legal_name: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub vat_id: Option<String>,
    pub address: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub web: Option<String>,
    pub logo_url: Option<String>,
    pub brand_color: Option<String>,
    pub tax_mode: String,
    pub vat_payer: bool,
    pub base_currency: String,
    pub rounding_mode: String,
    pub default_payment_terms_days: i32,
    pub default_invoice_note: Option<String>,
    pub default_footer: Option<String>,
    pub default_language: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Gapless numbering series per doc type/year (UC-ACC-02.3).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccNumberingSeries {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub doc_type: String,
    pub year: i32,
    pub prefix: String,
    pub format: String,
    pub next_value: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Unit of measure (UC-ACC-02.8).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccUnit {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub code: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Configurable VAT rate (UC-ACC-02.9).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccVatRate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub rate: Decimal,
    pub is_default: bool,
    pub is_active: bool,
    pub effective_from: Option<NaiveDate>,
    pub effective_to: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bank account shown on documents (UC-ACC-02.10).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccBankAccount {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub label: String,
    pub iban: String,
    pub swift: Option<String>,
    pub bank_name: Option<String>,
    pub currency: String,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Templated email for sending documents (UC-ACC-02.6).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccEmailTemplate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: String,
    pub language: String,
    pub subject: String,
    pub body: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Document design/branding template (UC-ACC-02.4).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccDocumentTemplate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub theme: String,
    pub accent_color: Option<String>,
    pub show_logo: bool,
    pub is_default: bool,
    #[schema(value_type = Object)]
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

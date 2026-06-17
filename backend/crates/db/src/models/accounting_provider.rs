//! External accounting provider integration models (PAP-191).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// External accounting provider authentication flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AccountingProviderAuthFlow {
    Ccf,
    Acf,
}

/// External accounting provider connection per tenant.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountingProviderConnection {
    pub tenant_id: Uuid,
    pub auth_flow: AccountingProviderAuthFlow,
    pub provider_account_name: String,
    pub client_id: String,
    pub client_secret_enc: Option<String>,
    pub refresh_token_enc: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// External accounting provider contact snapshot.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountingProviderContact {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub external_id: i64,
    pub company_name: Option<String>,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub email: Option<String>,
    pub iban: Option<String>,
    pub date_last_change: DateTime<Utc>,
    pub raw: serde_json::Value,
    pub synced_at: DateTime<Utc>,
}

/// External accounting provider issued invoice snapshot.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountingProviderIssuedInvoice {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub external_id: i64,
    pub document_number: String,
    pub partner_external_id: i64,
    pub variable_symbol: Option<String>,
    pub iban: Option<String>,
    pub account_number: Option<String>,
    pub bank_code: Option<String>,
    pub currency: Option<String>,
    pub total_with_vat: Decimal,
    pub total_without_vat: Decimal,
    pub payment_status: i16,
    pub date_of_issue: NaiveDate,
    pub date_of_maturity: NaiveDate,
    pub date_of_payment: Option<NaiveDate>,
    pub date_last_change: DateTime<Utc>,
    pub raw: serde_json::Value,
    pub synced_at: DateTime<Utc>,
}

/// External accounting provider incremental sync state.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountingProviderSyncCursor {
    pub tenant_id: Uuid,
    pub entity: String,
    pub last_change_seen: DateTime<Utc>,
    pub last_run_at: DateTime<Utc>,
    pub last_status: Option<String>,
}

/// External accounting provider payment matching result.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccountingProviderPaymentMatchSnapshot {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub invoice_id: Uuid,
    pub bank_movement_ref: String,
    pub matched_by: String,
    pub confidence: Decimal,
    pub amount: Decimal,
    pub matched_at: DateTime<Utc>,
}

//! iDoklad integration models (PAP-191).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// iDoklad authentication flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IdokladAuthFlow {
    Ccf,
    Acf,
}

/// iDoklad connection per tenant.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IdokladConnection {
    pub tenant_id: Uuid,
    pub auth_flow: IdokladAuthFlow,
    pub idoklad_name: String,
    pub client_id: String,
    pub client_secret_enc: Option<String>,
    pub refresh_token_enc: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// iDoklad contact snapshot.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IdokladContact {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub idoklad_id: i64,
    pub company_name: Option<String>,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub email: Option<String>,
    pub iban: Option<String>,
    pub date_last_change: DateTime<Utc>,
    pub raw: serde_json::Value,
    pub synced_at: DateTime<Utc>,
}

/// iDoklad issued invoice snapshot.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IdokladIssuedInvoice {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub idoklad_id: i64,
    pub document_number: String,
    pub partner_idoklad_id: i64,
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

/// iDoklad incremental sync state.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IdokladSyncCursor {
    pub tenant_id: Uuid,
    pub entity: String,
    pub last_change_seen: DateTime<Utc>,
    pub last_run_at: DateTime<Utc>,
    pub last_status: Option<String>,
}

/// iDoklad payment matching result.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct IdokladPaymentMatchSnapshot {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub invoice_id: Uuid,
    pub bank_movement_ref: String,
    pub matched_by: String,
    pub confidence: Decimal,
    pub amount: Decimal,
    pub matched_at: DateTime<Utc>,
}

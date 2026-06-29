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
    /// AES-256-GCM ciphertext — never serialize into an API response.
    #[serde(skip_serializing)]
    pub client_secret_enc: Option<String>,
    /// AES-256-GCM ciphertext — never serialize into an API response.
    #[serde(skip_serializing)]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// PAP-321 F2: the AES-256-GCM ciphertext fields must never appear in a
    /// serialized representation, so that a future provider-connection handler
    /// (PAP-191) cannot accidentally leak encrypted secrets into a response.
    #[test]
    fn provider_connection_does_not_serialize_encrypted_secrets() {
        let conn = AccountingProviderConnection {
            tenant_id: Uuid::nil(),
            auth_flow: AccountingProviderAuthFlow::Acf,
            provider_account_name: "acme".to_string(),
            client_id: "client-123".to_string(),
            client_secret_enc: Some("CIPHERTEXT_SECRET".to_string()),
            refresh_token_enc: Some("CIPHERTEXT_REFRESH".to_string()),
            created_at: DateTime::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap(),
        };

        let json = serde_json::to_string(&conn).expect("serialize");

        assert!(
            !json.contains("client_secret_enc"),
            "client_secret_enc key must be skipped on serialize: {json}"
        );
        assert!(
            !json.contains("refresh_token_enc"),
            "refresh_token_enc key must be skipped on serialize: {json}"
        );
        assert!(
            !json.contains("CIPHERTEXT_SECRET") && !json.contains("CIPHERTEXT_REFRESH"),
            "encrypted secret values must not leak into serialized output: {json}"
        );
        // Non-secret fields are still serialized.
        assert!(
            json.contains("client-123"),
            "client_id must still serialize"
        );
    }
}

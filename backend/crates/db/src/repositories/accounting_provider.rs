//! External accounting provider integration repository (PAP-191).

use crate::models::accounting_provider::{
    AccountingProviderConnection, AccountingProviderContact, AccountingProviderIssuedInvoice,
    AccountingProviderPaymentMatchSnapshot, AccountingProviderSyncCursor,
};
use crate::DbPool;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for external accounting provider integration operations.
#[derive(Clone)]
pub struct AccountingProviderRepository {
    pool: DbPool,
}

impl AccountingProviderRepository {
    /// Create a new AccountingProviderRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // accounting_provider_connection
    // ========================================================================

    pub async fn upsert_connection_rls<'e, E>(
        &self,
        executor: E,
        conn: AccountingProviderConnection,
    ) -> Result<AccountingProviderConnection, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let conn = sqlx::query_as::<_, AccountingProviderConnection>(
            r#"
            INSERT INTO accounting_provider_connection (
                tenant_id, auth_flow, provider_account_name, client_id,
                client_secret_enc, refresh_token_enc, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (tenant_id) DO UPDATE SET
                auth_flow = EXCLUDED.auth_flow,
                provider_account_name = EXCLUDED.provider_account_name,
                client_id = EXCLUDED.client_id,
                client_secret_enc = EXCLUDED.client_secret_enc,
                refresh_token_enc = EXCLUDED.refresh_token_enc,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(conn.tenant_id)
        .bind(conn.auth_flow)
        .bind(conn.provider_account_name)
        .bind(conn.client_id)
        .bind(conn.client_secret_enc)
        .bind(conn.refresh_token_enc)
        .fetch_one(executor)
        .await?;

        Ok(conn)
    }

    pub async fn find_connection_rls<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
    ) -> Result<Option<AccountingProviderConnection>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let conn = sqlx::query_as::<_, AccountingProviderConnection>(
            "SELECT * FROM accounting_provider_connection WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(executor)
        .await?;

        Ok(conn)
    }

    // ========================================================================
    // accounting_provider_contact
    // ========================================================================

    pub async fn upsert_contact_rls<'e, E>(
        &self,
        executor: E,
        contact: AccountingProviderContact,
    ) -> Result<AccountingProviderContact, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let contact = sqlx::query_as::<_, AccountingProviderContact>(
            r#"
            INSERT INTO accounting_provider_contact (
                tenant_id, external_id, company_name, ico, dic, email, iban,
                date_last_change, raw, synced_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            ON CONFLICT (tenant_id, external_id) DO UPDATE SET
                company_name = EXCLUDED.company_name,
                ico = EXCLUDED.ico,
                dic = EXCLUDED.dic,
                email = EXCLUDED.email,
                iban = EXCLUDED.iban,
                date_last_change = EXCLUDED.date_last_change,
                raw = EXCLUDED.raw,
                synced_at = NOW()
            RETURNING *
            "#,
        )
        .bind(contact.tenant_id)
        .bind(contact.external_id)
        .bind(contact.company_name)
        .bind(contact.ico)
        .bind(contact.dic)
        .bind(contact.email)
        .bind(contact.iban)
        .bind(contact.date_last_change)
        .bind(contact.raw)
        .fetch_one(executor)
        .await?;

        Ok(contact)
    }

    // ========================================================================
    // accounting_provider_issued_invoice
    // ========================================================================

    pub async fn upsert_invoice_rls<'e, E>(
        &self,
        executor: E,
        invoice: AccountingProviderIssuedInvoice,
    ) -> Result<AccountingProviderIssuedInvoice, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let invoice = sqlx::query_as::<_, AccountingProviderIssuedInvoice>(
            r#"
            INSERT INTO accounting_provider_issued_invoice (
                tenant_id, external_id, document_number, partner_external_id,
                variable_symbol, iban, account_number, bank_code, currency,
                total_with_vat, total_without_vat, payment_status,
                date_of_issue, date_of_maturity, date_of_payment,
                date_last_change, raw, synced_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NOW())
            ON CONFLICT (tenant_id, external_id) DO UPDATE SET
                document_number = EXCLUDED.document_number,
                partner_external_id = EXCLUDED.partner_external_id,
                variable_symbol = EXCLUDED.variable_symbol,
                iban = EXCLUDED.iban,
                account_number = EXCLUDED.account_number,
                bank_code = EXCLUDED.bank_code,
                currency = EXCLUDED.currency,
                total_with_vat = EXCLUDED.total_with_vat,
                total_without_vat = EXCLUDED.total_without_vat,
                payment_status = EXCLUDED.payment_status,
                date_of_issue = EXCLUDED.date_of_issue,
                date_of_maturity = EXCLUDED.date_of_maturity,
                date_of_payment = EXCLUDED.date_of_payment,
                date_last_change = EXCLUDED.date_last_change,
                raw = EXCLUDED.raw,
                synced_at = NOW()
            RETURNING *
            "#,
        )
        .bind(invoice.tenant_id)
        .bind(invoice.external_id)
        .bind(invoice.document_number)
        .bind(invoice.partner_external_id)
        .bind(invoice.variable_symbol)
        .bind(invoice.iban)
        .bind(invoice.account_number)
        .bind(invoice.bank_code)
        .bind(invoice.currency)
        .bind(invoice.total_with_vat)
        .bind(invoice.total_without_vat)
        .bind(invoice.payment_status)
        .bind(invoice.date_of_issue)
        .bind(invoice.date_of_maturity)
        .bind(invoice.date_of_payment)
        .bind(invoice.date_last_change)
        .bind(invoice.raw)
        .fetch_one(executor)
        .await?;

        Ok(invoice)
    }

    // ========================================================================
    // accounting_provider_sync_cursor
    // ========================================================================

    pub async fn get_sync_cursor_rls<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        entity: &str,
    ) -> Result<Option<AccountingProviderSyncCursor>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let cursor = sqlx::query_as::<_, AccountingProviderSyncCursor>(
            "SELECT * FROM accounting_provider_sync_cursor WHERE tenant_id = $1 AND entity = $2",
        )
        .bind(tenant_id)
        .bind(entity)
        .fetch_optional(executor)
        .await?;

        Ok(cursor)
    }

    pub async fn upsert_sync_cursor_rls<'e, E>(
        &self,
        executor: E,
        cursor: AccountingProviderSyncCursor,
    ) -> Result<AccountingProviderSyncCursor, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let cursor = sqlx::query_as::<_, AccountingProviderSyncCursor>(
            r#"
            INSERT INTO accounting_provider_sync_cursor (
                tenant_id, entity, last_change_seen, last_run_at, last_status
            )
            VALUES ($1, $2, $3, NOW(), $4)
            ON CONFLICT (tenant_id, entity) DO UPDATE SET
                last_change_seen = EXCLUDED.last_change_seen,
                last_run_at = NOW(),
                last_status = EXCLUDED.last_status
            RETURNING *
            "#,
        )
        .bind(cursor.tenant_id)
        .bind(cursor.entity)
        .bind(cursor.last_change_seen)
        .bind(cursor.last_status)
        .fetch_one(executor)
        .await?;

        Ok(cursor)
    }

    // ========================================================================
    // accounting_provider_payment_match_snapshot
    // ========================================================================

    pub async fn create_payment_match_rls<'e, E>(
        &self,
        executor: E,
        match_data: AccountingProviderPaymentMatchSnapshot,
    ) -> Result<AccountingProviderPaymentMatchSnapshot, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let match_data = sqlx::query_as::<_, AccountingProviderPaymentMatchSnapshot>(
            r#"
            INSERT INTO accounting_provider_payment_match_snapshot (
                tenant_id, invoice_id, bank_movement_ref, matched_by,
                confidence, amount, matched_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            RETURNING *
            "#,
        )
        .bind(match_data.tenant_id)
        .bind(match_data.invoice_id)
        .bind(match_data.bank_movement_ref)
        .bind(match_data.matched_by)
        .bind(match_data.confidence)
        .bind(match_data.amount)
        .fetch_one(executor)
        .await?;

        Ok(match_data)
    }
}

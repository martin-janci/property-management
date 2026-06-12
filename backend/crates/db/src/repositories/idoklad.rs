//! iDoklad integration repository (PAP-191).

use crate::models::idoklad::{
    IdokladConnection, IdokladContact, IdokladIssuedInvoice, IdokladSyncCursor,
    IdokladPaymentMatchSnapshot,
};
use crate::DbPool;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for iDoklad integration operations.
#[derive(Clone)]
pub struct IdokladRepository {
    pool: DbPool,
}

impl IdokladRepository {
    /// Create a new IdokladRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // idoklad_connection
    // ========================================================================

    pub async fn upsert_connection_rls<'e, E>(
        &self,
        executor: E,
        conn: IdokladConnection,
    ) -> Result<IdokladConnection, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let conn = sqlx::query_as::<_, IdokladConnection>(
            r#"
            INSERT INTO idoklad_connection (
                tenant_id, auth_flow, idoklad_name, client_id,
                client_secret_enc, refresh_token_enc, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (tenant_id) DO UPDATE SET
                auth_flow = EXCLUDED.auth_flow,
                idoklad_name = EXCLUDED.idoklad_name,
                client_id = EXCLUDED.client_id,
                client_secret_enc = EXCLUDED.client_secret_enc,
                refresh_token_enc = EXCLUDED.refresh_token_enc,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(conn.tenant_id)
        .bind(conn.auth_flow)
        .bind(conn.idoklad_name)
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
    ) -> Result<Option<IdokladConnection>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let conn = sqlx::query_as::<_, IdokladConnection>(
            "SELECT * FROM idoklad_connection WHERE tenant_id = $1",
        )
        .bind(tenant_id)
        .fetch_optional(executor)
        .await?;

        Ok(conn)
    }

    // ========================================================================
    // idoklad_contact
    // ========================================================================

    pub async fn upsert_contact_rls<'e, E>(
        &self,
        executor: E,
        contact: IdokladContact,
    ) -> Result<IdokladContact, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let contact = sqlx::query_as::<_, IdokladContact>(
            r#"
            INSERT INTO idoklad_contact (
                tenant_id, idoklad_id, company_name, ico, dic, email, iban,
                date_last_change, raw, synced_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            ON CONFLICT (tenant_id, idoklad_id) DO UPDATE SET
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
        .bind(contact.idoklad_id)
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
    // idoklad_issued_invoice
    // ========================================================================

    pub async fn upsert_invoice_rls<'e, E>(
        &self,
        executor: E,
        invoice: IdokladIssuedInvoice,
    ) -> Result<IdokladIssuedInvoice, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let invoice = sqlx::query_as::<_, IdokladIssuedInvoice>(
            r#"
            INSERT INTO idoklad_issued_invoice (
                tenant_id, idoklad_id, document_number, partner_idoklad_id,
                variable_symbol, iban, account_number, bank_code, currency,
                total_with_vat, total_without_vat, payment_status,
                date_of_issue, date_of_maturity, date_of_payment,
                date_last_change, raw, synced_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NOW())
            ON CONFLICT (tenant_id, idoklad_id) DO UPDATE SET
                document_number = EXCLUDED.document_number,
                partner_idoklad_id = EXCLUDED.partner_idoklad_id,
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
        .bind(invoice.idoklad_id)
        .bind(invoice.document_number)
        .bind(invoice.partner_idoklad_id)
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
    // idoklad_sync_cursor
    // ========================================================================

    pub async fn get_sync_cursor_rls<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        entity: &str,
    ) -> Result<Option<IdokladSyncCursor>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let cursor = sqlx::query_as::<_, IdokladSyncCursor>(
            "SELECT * FROM idoklad_sync_cursor WHERE tenant_id = $1 AND entity = $2",
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
        cursor: IdokladSyncCursor,
    ) -> Result<IdokladSyncCursor, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let cursor = sqlx::query_as::<_, IdokladSyncCursor>(
            r#"
            INSERT INTO idoklad_sync_cursor (
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
    // idoklad_payment_match_snapshot
    // ========================================================================

    pub async fn create_payment_match_rls<'e, E>(
        &self,
        executor: E,
        match_data: IdokladPaymentMatchSnapshot,
    ) -> Result<IdokladPaymentMatchSnapshot, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let match_data = sqlx::query_as::<_, IdokladPaymentMatchSnapshot>(
            r#"
            INSERT INTO idoklad_payment_match_snapshot (
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

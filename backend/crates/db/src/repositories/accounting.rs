//! Repository for native accounting MVP (PAP-206).

use crate::models::accounting::{
    BankStatement, BankStatementLine, Contact, CreateInvoice, Invoice, InvoiceItem, InvoiceStatus,
    PaymentMatch, UpdateInvoice,
};
use crate::DbPool;
use rust_decimal::Decimal;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

/// Repository for accounting entities.
#[derive(Debug, Clone)]
pub struct AccountingRepository {
    pub pool: DbPool,
}

impl AccountingRepository {
    /// Create a new accounting repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // --- Contacts ---

    /// Find a contact by ID with RLS.
    pub async fn find_contact_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Contact>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Contact>("SELECT * FROM contact WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List all contacts for the current tenant.
    pub async fn list_contacts_rls<'e, E>(&self, executor: E) -> Result<Vec<Contact>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Contact>("SELECT * FROM contact ORDER BY name ASC")
            .fetch_all(executor)
            .await
    }

    // --- Invoices ---

    /// List invoices with RLS.
    pub async fn list_invoices_rls<'e, E>(&self, executor: E) -> Result<Vec<Invoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoice ORDER BY issue_date DESC, created_at DESC",
        )
        .fetch_all(executor)
        .await
    }

    /// Find an invoice by ID with RLS.
    pub async fn find_invoice_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Invoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoice WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find invoice items with RLS.
    pub async fn find_invoice_items_rls<'e, E>(
        &self,
        executor: E,
        invoice_id: Uuid,
    ) -> Result<Vec<InvoiceItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InvoiceItem>(
            "SELECT * FROM invoice_item WHERE invoice_id = $1 ORDER BY id ASC",
        )
        .bind(invoice_id)
        .fetch_all(executor)
        .await
    }

    /// Create an invoice and its items.
    /// Note: This method should be called within a transaction if multiple items are present.
    pub async fn create_invoice_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        data: CreateInvoice,
    ) -> Result<Invoice, sqlx::Error> {
        // Single pass: resolve VAT, round each line to 2dp, and accumulate the
        // header totals from the SAME rounded values. The invoice_item rows are
        // stored as NUMERIC(18,2) (rounded by Postgres on insert), so summing
        // full-precision lines into the header (the old two-loop approach) could
        // diverge from the sum of the stored rows by up to a cent. Rounding once
        // here makes the header always equal the sum of the line rows. (#1522)
        let mut total_amount = Decimal::ZERO;
        let mut base_amount = Decimal::ZERO;
        let mut vat_amount = Decimal::ZERO;
        let mut lines = Vec::with_capacity(data.items.len());

        for mut item in data.items {
            // Resolve VAT rate from type if provided.
            if let Some(rate_type) = item.vat_rate_type {
                item.vat_rate = match data.country.as_deref() {
                    Some("SK") => rate_type.sk_percentage(),
                    _ => rate_type.cz_percentage(), // Default to CZ
                };
            }

            let line_base = (item.qty * item.unit_price).round_dp(2);
            let line_vat = (line_base * (item.vat_rate / Decimal::from(100))).round_dp(2);
            let line_total = line_base + line_vat;

            base_amount += line_base;
            vat_amount += line_vat;
            total_amount += line_total;

            lines.push((item, line_base, line_vat, line_total));
        }

        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoice (
                tenant_id, contact_id, number, issue_date, due_date, taxable_supply_date,
                currency, total_amount, base_amount, vat_amount, variable_symbol, status,
                paid_amount
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(data.tenant_id)
        .bind(data.contact_id)
        .bind(&data.number)
        .bind(data.issue_date)
        .bind(data.due_date)
        .bind(data.taxable_supply_date)
        .bind(&data.currency)
        .bind(total_amount)
        .bind(base_amount)
        .bind(vat_amount)
        .bind(&data.variable_symbol)
        .bind(data.status.unwrap_or(InvoiceStatus::Draft))
        .bind(Decimal::ZERO)
        .fetch_one(&mut *conn)
        .await?;

        // Insert each line from the SAME rounded values used for the header
        // totals above (single source of truth — no re-resolution, no drift).
        for (item, line_base, line_vat, line_total) in lines {
            sqlx::query(
                r#"
                INSERT INTO invoice_item (
                    invoice_id, tenant_id, description, qty, unit_price, vat_rate,
                    total_amount, base_amount, vat_amount
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(invoice.id)
            .bind(data.tenant_id)
            .bind(&item.description)
            .bind(item.qty)
            .bind(item.unit_price)
            .bind(item.vat_rate)
            .bind(line_total)
            .bind(line_base)
            .bind(line_vat)
            .execute(&mut *conn)
            .await?;
        }

        Ok(invoice)
    }

    /// Update an existing invoice.
    pub async fn update_invoice_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateInvoice,
    ) -> Result<Option<Invoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoice
            SET
                contact_id = COALESCE($1, contact_id),
                number = COALESCE($2, number),
                issue_date = COALESCE($3, issue_date),
                due_date = COALESCE($4, due_date),
                taxable_supply_date = CASE WHEN $5 IS NULL THEN taxable_supply_date ELSE $6 END,
                currency = COALESCE($7, currency),
                variable_symbol = CASE WHEN $8 IS NULL THEN variable_symbol ELSE $9 END,
                status = COALESCE($10, status),
                paid_amount = COALESCE($11, paid_amount),
                updated_at = NOW()
            WHERE id = $12
            RETURNING *
            "#,
        )
        .bind(data.contact_id)
        .bind(data.number)
        .bind(data.issue_date)
        .bind(data.due_date)
        .bind(data.taxable_supply_date.is_some())
        .bind(data.taxable_supply_date.flatten())
        .bind(data.currency)
        .bind(data.variable_symbol.is_some())
        .bind(data.variable_symbol.flatten())
        .bind(data.status)
        .bind(data.paid_amount)
        .bind(id)
        .fetch_optional(executor)
        .await
    }

    /// Delete an invoice and its items (items deleted by FK cascade).
    pub async fn delete_invoice_rls<'e, E>(&self, executor: E, id: Uuid) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("DELETE FROM invoice WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(())
    }

    // --- Bank Statements ---

    /// Find a bank statement by ID with RLS.
    pub async fn find_bank_statement_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<BankStatement>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BankStatement>("SELECT * FROM bank_statement WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find a bank statement line by ID with RLS.
    pub async fn find_bank_statement_line_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<BankStatementLine>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BankStatementLine>("SELECT * FROM bank_statement_line WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find bank statement lines with RLS.
    pub async fn find_bank_statement_lines_rls<'e, E>(
        &self,
        executor: E,
        statement_id: Uuid,
    ) -> Result<Vec<BankStatementLine>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BankStatementLine>(
            "SELECT * FROM bank_statement_line WHERE statement_id = $1",
        )
        .bind(statement_id)
        .fetch_all(executor)
        .await
    }

    // --- Payment Matches ---

    /// Find a payment match with RLS.
    pub async fn find_payment_match_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<PaymentMatch>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PaymentMatch>("SELECT * FROM payment_match WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List bank statements for the current tenant.
    pub async fn list_bank_statements_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<BankStatement>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BankStatement>("SELECT * FROM bank_statement ORDER BY imported_at DESC")
            .fetch_all(executor)
            .await
    }

    /// Create a bank statement.
    pub async fn create_bank_statement_rls<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        source_filename: String,
        period: Option<String>,
        account_iban: String,
    ) -> Result<BankStatement, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BankStatement>(
            r#"
            INSERT INTO bank_statement (tenant_id, source_filename, period, account_iban)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(source_filename)
        .bind(period)
        .bind(account_iban)
        .fetch_one(executor)
        .await
    }

    /// Create a bank statement line.
    pub async fn create_bank_statement_line_rls<'e, E>(
        &self,
        executor: E,
        data: BankStatementLine,
    ) -> Result<BankStatementLine, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BankStatementLine>(
            r#"
            INSERT INTO bank_statement_line (
                id, statement_id, tenant_id, booking_date, amount, currency,
                counterparty_iban, variable_symbol, raw_ref, match_state
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(data.id)
        .bind(data.statement_id)
        .bind(data.tenant_id)
        .bind(data.booking_date)
        .bind(data.amount)
        .bind(data.currency)
        .bind(data.counterparty_iban)
        .bind(data.variable_symbol)
        .bind(data.raw_ref)
        .bind(data.match_state)
        .fetch_one(executor)
        .await
    }

    /// List bank statement lines for a specific statement.
    pub async fn list_bank_statement_lines_rls<'e, E>(
        &self,
        executor: E,
        statement_id: Uuid,
    ) -> Result<Vec<BankStatementLine>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BankStatementLine>(
            "SELECT * FROM bank_statement_line WHERE statement_id = $1 ORDER BY booking_date DESC",
        )
        .bind(statement_id)
        .fetch_all(executor)
        .await
    }

    /// Update the match state of a bank statement line.
    pub async fn update_bank_statement_line_match_state_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        match_state: String,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("UPDATE bank_statement_line SET match_state = $1 WHERE id = $2")
            .bind(match_state)
            .bind(id)
            .execute(executor)
            .await?;
        Ok(())
    }

    /// List payment matches for a specific statement line.
    pub async fn list_payment_matches_by_line_rls<'e, E>(
        &self,
        executor: E,
        line_id: Uuid,
    ) -> Result<Vec<PaymentMatch>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PaymentMatch>(
            "SELECT * FROM payment_match WHERE statement_line_id = $1",
        )
        .bind(line_id)
        .fetch_all(executor)
        .await
    }

    /// Create or update a payment match (upsert).
    pub async fn upsert_payment_match_rls<'e, E>(
        &self,
        executor: E,
        data: PaymentMatch,
    ) -> Result<PaymentMatch, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PaymentMatch>(
            r#"
            INSERT INTO payment_match (
                id, tenant_id, statement_line_id, invoice_id, confidence,
                decided_by, decided_at, state
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (statement_line_id, invoice_id) DO UPDATE
            SET
                confidence = EXCLUDED.confidence,
                decided_by = EXCLUDED.decided_by,
                decided_at = EXCLUDED.decided_at,
                state = EXCLUDED.state
            RETURNING *
            "#,
        )
        .bind(data.id)
        .bind(data.tenant_id)
        .bind(data.statement_line_id)
        .bind(data.invoice_id)
        .bind(data.confidence)
        .bind(data.decided_by)
        .bind(data.decided_at)
        .bind(data.state)
        .fetch_one(executor)
        .await
    }

    /// Update invoice paid amount and status based on a match confirmation.
    ///
    /// Cancelled documents are excluded by the WHERE guard: a payment applied
    /// to a cancelled invoice must not resurrect it as paid — callers get
    /// `None` (same as an unknown id) and surface the mismatch (UC-ACC-05.17).
    pub async fn update_invoice_payment_status_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        paid_amount_delta: Decimal,
    ) -> Result<Option<Invoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoice
            SET
                paid_amount = paid_amount + $1,
                status = CASE
                    WHEN paid_amount + $1 >= total_amount THEN 'paid'
                    WHEN paid_amount + $1 > 0 THEN 'partially_paid'
                    -- PAP-325: a negative delta (unapplying a confirmed match)
                    -- can drive paid_amount back to 0; revert to 'issued' rather
                    -- than leaving a stale 'paid'/'partially_paid' status.
                    ELSE 'issued'
                END,
                updated_at = NOW()
            WHERE id = $2 AND status <> 'cancelled'
            RETURNING *
            "#,
        )
        .bind(paid_amount_delta)
        .bind(id)
        .fetch_optional(executor)
        .await
    }
}

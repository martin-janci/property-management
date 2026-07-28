//! Repository for ACC sales-invoicing extensions (EPIC-ACC-05). OWNED BY: BE-Invoicing.
//!
//! RLS-aware, generic-executor pattern (see
//! [`crate::repositories::accounting::AccountingRepository`], which already
//! covers base invoice CRUD on the 00184 `invoice` table). This repo adds the
//! 00196 extensions: doc-type-aware reads via [`AccInvoiceExt`], document links,
//! attachments, issuing (number allocation + immutability), credit notes, and
//! advance settlement.
//!
//! RLS note: the `invoice` / `acc_document_link` / `acc_attachment` policies
//! filter every SELECT by `get_current_org_id()`, so reads never add an explicit
//! `tenant_id = $` clause. INSERTs MUST set `tenant_id` to the request tenant
//! (the policy's WITH CHECK enforces it). All money columns are NUMERIC(18,2);
//! the caller (handlers) computes them via `accounting_core::money`.
//!
//! Column names verified against migrations 00184 + 00196.

use crate::models::acc_invoicing_ext::{AccAttachment, AccDocumentLink, AccInvoiceExt};
use crate::DbPool;
use rust_decimal::Decimal;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

/// The explicit `invoice` column list for the extended projection. Listed
/// explicitly (not `SELECT *`) so the read is stable if a later migration adds
/// columns in a different position — `FromRow` maps by name, and this set is the
/// exact superset [`AccInvoiceExt`] expects (base 00184 cols + 00196 ALTERs).
const INVOICE_EXT_COLUMNS: &str = "id, tenant_id, contact_id, number, issue_date, due_date, \
     taxable_supply_date, currency, total_amount, base_amount, vat_amount, variable_symbol, \
     status, paid_amount, created_at, updated_at, doc_type, corrected_invoice_id, \
     advance_settlement, settled_advance_id, exchange_rate, base_currency_amount, \
     numbering_series_id, bank_account_id";

/// Repository for ACC invoicing extensions (EPIC-ACC-05).
#[derive(Debug, Clone)]
pub struct AccInvoicingRepository {
    pub pool: DbPool,
}

impl AccInvoicingRepository {
    /// Create a new invoicing repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // --- Extended invoice reads (UC-ACC-05.1/.4/.5) ---

    /// List invoices (extended projection) for the current tenant, optionally
    /// filtered by `doc_type`. RLS scopes to the active company (UC-ACC-05.1).
    pub async fn list_invoices_ext_rls<'e, E>(
        &self,
        executor: E,
        doc_type: Option<&str>,
    ) -> Result<Vec<AccInvoiceExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // `$1 IS NULL OR doc_type = $1` keeps one prepared statement for both the
        // unfiltered list and the per-doc-type list (drafts/proformas/etc.).
        let sql = format!(
            "SELECT {INVOICE_EXT_COLUMNS} FROM invoice \
             WHERE ($1::TEXT IS NULL OR doc_type = $1) \
             ORDER BY issue_date DESC, created_at DESC"
        );
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(doc_type)
            .fetch_all(executor)
            .await
    }

    /// Find a single invoice (extended projection) by id (UC-ACC-05.1).
    pub async fn find_invoice_ext_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<AccInvoiceExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!("SELECT {INVOICE_EXT_COLUMNS} FROM invoice WHERE id = $1");
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Issue a draft: set the allocated number, doc_type, numbering series id and
    /// status = 'issued' (UC-ACC-05.1). The number itself is allocated by
    /// BE-Config's atomic `allocate_next_number_rls` inside the caller's
    /// transaction, so concurrent issuance can never skip or duplicate a number.
    ///
    /// Only a `draft` can be issued — the `WHERE ... status = 'draft'` guard
    /// makes the issued document immutable-by-correction (re-issuing returns
    /// `None`, which the handler maps to a 4xx).
    pub async fn issue_invoice_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        number: &str,
        numbering_series_id: Uuid,
    ) -> Result<Option<AccInvoiceExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "UPDATE invoice \
             SET number = $2, numbering_series_id = $3, status = 'issued', updated_at = NOW() \
             WHERE id = $1 AND status = 'draft' \
             RETURNING {INVOICE_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(id)
            .bind(number)
            .bind(numbering_series_id)
            .fetch_optional(executor)
            .await
    }

    /// Mark an issued invoice as `sent` (UC-ACC-05.10). Only an `issued` invoice
    /// advances to `sent`; returns `None` otherwise (idempotent guard).
    pub async fn mark_sent_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<AccInvoiceExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "UPDATE invoice SET status = 'sent', updated_at = NOW() \
             WHERE id = $1 AND status = 'issued' \
             RETURNING {INVOICE_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Cancel an unpaid document (UC-ACC-05.17). Only `issued`/`sent`/`overdue`
    /// documents with a zero `paid_amount` can be cancelled — drafts are simply
    /// deleted, and once any payment landed the correction path is a credit
    /// note (UC-ACC-05.7). Returns `None` when the guard does not match, so a
    /// concurrent payment confirmation can never race a cancellation.
    pub async fn cancel_invoice_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<AccInvoiceExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "UPDATE invoice SET status = 'cancelled', updated_at = NOW() \
             WHERE id = $1 AND status IN ('issued', 'sent', 'overdue') AND paid_amount = 0 \
             RETURNING {INVOICE_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Set FX rate + base-currency amount on a foreign-currency invoice
    /// (UC-ACC-05.4). The handler computes `base_currency_amount` via
    /// `accounting_core::money::to_base_currency`.
    pub async fn set_exchange_rate_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        exchange_rate: Decimal,
        base_currency_amount: Decimal,
    ) -> Result<Option<AccInvoiceExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "UPDATE invoice \
             SET exchange_rate = $2, base_currency_amount = $3, updated_at = NOW() \
             WHERE id = $1 \
             RETURNING {INVOICE_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(id)
            .bind(exchange_rate)
            .bind(base_currency_amount)
            .fetch_optional(executor)
            .await
    }

    /// Create a credit note referencing the corrected invoice (UC-ACC-05.7).
    /// The original stays immutable; this inserts a new `doc_type='credit_note'`
    /// row with `corrected_invoice_id` set and SIGNED (negative) amounts. The
    /// caller (handler) computes the signed totals via
    /// `accounting_core::vat::reverse_groups` and persists the credit-note line
    /// rows separately via [`Self::insert_invoice_item_signed_rls`].
    pub async fn create_credit_note_rls<'e, E>(
        &self,
        executor: E,
        invoice: AccInvoiceExt,
    ) -> Result<AccInvoiceExt, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "INSERT INTO invoice ( \
                tenant_id, contact_id, number, issue_date, due_date, taxable_supply_date, \
                currency, total_amount, base_amount, vat_amount, variable_symbol, status, \
                paid_amount, doc_type, corrected_invoice_id, exchange_rate, base_currency_amount \
             ) VALUES ( \
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'credit_note', $14, $15, $16 \
             ) RETURNING {INVOICE_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(invoice.tenant_id)
            .bind(invoice.contact_id)
            .bind(&invoice.number)
            .bind(invoice.issue_date)
            .bind(invoice.due_date)
            .bind(invoice.taxable_supply_date)
            .bind(&invoice.currency)
            .bind(invoice.total_amount)
            .bind(invoice.base_amount)
            .bind(invoice.vat_amount)
            .bind(&invoice.variable_symbol)
            .bind(&invoice.status)
            .bind(invoice.paid_amount)
            .bind(invoice.corrected_invoice_id)
            .bind(invoice.exchange_rate)
            .bind(invoice.base_currency_amount)
            .fetch_one(executor)
            .await
    }

    /// Insert a (possibly signed) `invoice_item` row for a credit note or
    /// duplicate (UC-ACC-05.7/.16). Used outside the base
    /// `AccountingRepository::create_invoice_rls` flow where signed amounts must
    /// be preserved verbatim rather than recomputed.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_invoice_item_signed_rls<'e, E>(
        &self,
        executor: E,
        invoice_id: Uuid,
        tenant_id: Uuid,
        description: &str,
        qty: Decimal,
        unit_price: Decimal,
        vat_rate: Decimal,
        base_amount: Decimal,
        vat_amount: Decimal,
        total_amount: Decimal,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            "INSERT INTO invoice_item ( \
                invoice_id, tenant_id, description, qty, unit_price, vat_rate, \
                total_amount, base_amount, vat_amount \
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(invoice_id)
        .bind(tenant_id)
        .bind(description)
        .bind(qty)
        .bind(unit_price)
        .bind(vat_rate)
        .bind(total_amount)
        .bind(base_amount)
        .bind(vat_amount)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Record an advance settlement on a final invoice (UC-ACC-05.6): set
    /// `settled_advance_id` + `advance_settlement` (the deducted net amount).
    ///
    /// Guards against double-settlement: the update only applies while
    /// `settled_advance_id IS NULL`, so a second settlement attempt returns
    /// `None` (the handler maps that to a conflict, UC-ACC-05.6 test "double
    /// settlement prevented").
    pub async fn settle_advance_rls<'e, E>(
        &self,
        executor: E,
        final_invoice_id: Uuid,
        advance_invoice_id: Uuid,
        settled_amount: Decimal,
    ) -> Result<Option<AccInvoiceExt>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let sql = format!(
            "UPDATE invoice \
             SET settled_advance_id = $2, advance_settlement = $3, updated_at = NOW() \
             WHERE id = $1 AND settled_advance_id IS NULL \
             RETURNING {INVOICE_EXT_COLUMNS}"
        );
        sqlx::query_as::<_, AccInvoiceExt>(sqlx::AssertSqlSafe(sql.as_str()))
            .bind(final_invoice_id)
            .bind(advance_invoice_id)
            .bind(settled_amount)
            .fetch_optional(executor)
            .await
    }

    // --- Document links (UC-ACC-05.15) ---

    /// List document links from OR to an invoice (UC-ACC-05.15). Returns both
    /// directions so the chain visualization can be assembled by the caller.
    pub async fn list_links_rls<'e, E>(
        &self,
        executor: E,
        invoice_id: Uuid,
    ) -> Result<Vec<AccDocumentLink>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccDocumentLink>(
            "SELECT * FROM acc_document_link \
             WHERE from_invoice_id = $1 OR to_invoice_id = $1 \
             ORDER BY created_at ASC",
        )
        .bind(invoice_id)
        .fetch_all(executor)
        .await
    }

    /// Create a typed document link (UC-ACC-05.15). Idempotent on the unique
    /// `(tenant_id, from_invoice_id, to_invoice_id, relation)` tuple: a duplicate
    /// link is a no-op and the existing row is returned. Cycle-safety
    /// (rejecting `from == to`) is enforced by the handler before calling this.
    pub async fn create_link_rls<'e, E>(
        &self,
        executor: E,
        link: AccDocumentLink,
    ) -> Result<AccDocumentLink, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // ON CONFLICT DO UPDATE (a no-op SET) so RETURNING always yields the row,
        // whether it was just inserted or already existed (DO NOTHING would
        // return no row on conflict, forcing a second SELECT).
        sqlx::query_as::<_, AccDocumentLink>(
            "INSERT INTO acc_document_link (tenant_id, from_invoice_id, to_invoice_id, relation) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (tenant_id, from_invoice_id, to_invoice_id, relation) \
             DO UPDATE SET relation = EXCLUDED.relation \
             RETURNING *",
        )
        .bind(link.tenant_id)
        .bind(link.from_invoice_id)
        .bind(link.to_invoice_id)
        .bind(&link.relation)
        .fetch_one(executor)
        .await
    }

    // --- Attachments (UC-ACC-05.13) ---

    /// List attachments for an invoice (UC-ACC-05.13).
    pub async fn list_attachments_rls<'e, E>(
        &self,
        executor: E,
        invoice_id: Uuid,
    ) -> Result<Vec<AccAttachment>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccAttachment>(
            "SELECT * FROM acc_attachment WHERE invoice_id = $1 ORDER BY created_at DESC",
        )
        .bind(invoice_id)
        .fetch_all(executor)
        .await
    }

    /// Create an attachment record (file already uploaded to object storage)
    /// (UC-ACC-05.13). `tenant_id` is set from the caller's request tenant.
    pub async fn create_attachment_rls<'e, E>(
        &self,
        executor: E,
        attachment: AccAttachment,
    ) -> Result<AccAttachment, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccAttachment>(
            "INSERT INTO acc_attachment ( \
                tenant_id, invoice_id, filename, content_type, size_bytes, storage_key, uploaded_by \
             ) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
        )
        .bind(attachment.tenant_id)
        .bind(attachment.invoice_id)
        .bind(&attachment.filename)
        .bind(&attachment.content_type)
        .bind(attachment.size_bytes)
        .bind(&attachment.storage_key)
        .bind(attachment.uploaded_by)
        .fetch_one(executor)
        .await
    }
}

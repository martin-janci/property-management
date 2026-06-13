//! Repository for native accounting MVP (PAP-206).

use crate::models::accounting::{
    BankStatement, BankStatementLine, Contact, Invoice, InvoiceItem, PaymentMatch,
};
use crate::DbPool;
use sqlx::Postgres;
use uuid::Uuid;

/// Repository for accounting entities.
#[derive(Debug, Clone)]
pub struct AccountingRepository {
    pool: DbPool,
}

impl AccountingRepository {
    /// Create a new accounting repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // --- Contacts ---

    /// Find a contact by ID with RLS.
    pub async fn find_contact_rls(
        &self,
        executor: impl sqlx::Executor<'_, Database = Postgres>,
        id: Uuid,
    ) -> Result<Option<Contact>, sqlx::Error> {
        sqlx::query_as::<_, Contact>("SELECT * FROM contact WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    // --- Invoices ---

    /// Find an invoice by ID with RLS.
    pub async fn find_invoice_rls(
        &self,
        executor: impl sqlx::Executor<'_, Database = Postgres>,
        id: Uuid,
    ) -> Result<Option<Invoice>, sqlx::Error> {
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoice WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find invoice items with RLS.
    pub async fn find_invoice_items_rls(
        &self,
        executor: impl sqlx::Executor<'_, Database = Postgres>,
        invoice_id: Uuid,
    ) -> Result<Vec<InvoiceItem>, sqlx::Error> {
        sqlx::query_as::<_, InvoiceItem>("SELECT * FROM invoice_item WHERE invoice_id = $1")
            .bind(invoice_id)
            .fetch_all(executor)
            .await
    }

    // --- Bank Statements ---

    /// Find a bank statement by ID with RLS.
    pub async fn find_bank_statement_rls(
        &self,
        executor: impl sqlx::Executor<'_, Database = Postgres>,
        id: Uuid,
    ) -> Result<Option<BankStatement>, sqlx::Error> {
        sqlx::query_as::<_, BankStatement>("SELECT * FROM bank_statement WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find bank statement lines with RLS.
    pub async fn find_bank_statement_lines_rls(
        &self,
        executor: impl sqlx::Executor<'_, Database = Postgres>,
        statement_id: Uuid,
    ) -> Result<Vec<BankStatementLine>, sqlx::Error> {
        sqlx::query_as::<_, BankStatementLine>("SELECT * FROM bank_statement_line WHERE statement_id = $1")
            .bind(statement_id)
            .fetch_all(executor)
            .await
    }

    // --- Payment Matches ---

    /// Find a payment match with RLS.
    pub async fn find_payment_match_rls(
        &self,
        executor: impl sqlx::Executor<'_, Database = Postgres>,
        id: Uuid,
    ) -> Result<Option<PaymentMatch>, sqlx::Error> {
        sqlx::query_as::<_, PaymentMatch>("SELECT * FROM payment_match WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }
}

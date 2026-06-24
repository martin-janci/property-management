//! Invoice CRUD, listing & line items (Epic 26).

use super::SubscriptionRepository;
use crate::models::{InvoiceLineItem, InvoiceQueryParams, InvoiceWithDetails, SubscriptionInvoice};
use rust_decimal::Decimal;
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

impl SubscriptionRepository {
    // ==================== Invoices ====================

    /// Create an invoice.
    ///
    /// Uses a database sequence for atomic invoice number generation to
    /// prevent race conditions and duplicate invoice numbers under concurrent
    /// requests. Multi-statement (sequence fetch, then insert), so it takes
    /// `&mut PgConnection`; sequences are not subject to RLS, so running
    /// `nextval` on the context-set connection is fine.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_invoice(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        subscription_id: Option<Uuid>,
        subtotal: Decimal,
        tax_amount: Option<Decimal>,
        total_amount: Decimal,
        currency: &str,
        due_date: chrono::NaiveDate,
    ) -> Result<SubscriptionInvoice, sqlx::Error> {
        // Generate invoice number atomically using database sequence
        let seq: (i64,) = sqlx::query_as("SELECT nextval('invoice_number_seq')")
            .fetch_one(&mut *conn)
            .await
            .unwrap_or((
                // Fallback: use timestamp if sequence doesn't exist
                chrono::Utc::now().timestamp_millis() % 100_000_000,
            ));
        let invoice_number = format!("INV-{:08}", seq.0);

        sqlx::query_as(
            r#"
            INSERT INTO subscription_invoices
                (organization_id, subscription_id, invoice_number, invoice_date, due_date,
                 subtotal, tax_amount, total_amount, currency, status)
            VALUES ($1, $2, $3, CURRENT_DATE, $4, $5, $6, $7, $8, 'open')
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(subscription_id)
        .bind(&invoice_number)
        .bind(due_date)
        .bind(subtotal)
        .bind(tax_amount)
        .bind(total_amount)
        .bind(currency)
        .fetch_one(&mut *conn)
        .await
    }

    /// Find an invoice by ID.
    pub async fn find_invoice_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<SubscriptionInvoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM subscription_invoices WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List invoices for an organization.
    pub async fn list_invoices<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: InvoiceQueryParams,
    ) -> Result<Vec<SubscriptionInvoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM subscription_invoices
            WHERE organization_id = $1
            AND ($2::text IS NULL OR status = $2)
            AND ($3::date IS NULL OR invoice_date >= $3)
            AND ($4::date IS NULL OR invoice_date <= $4)
            ORDER BY invoice_date DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(org_id)
        .bind(&query.status)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// List invoices with details (platform admin).
    ///
    /// Note: under FORCE RLS the result is scoped to the caller's RLS
    /// context.
    pub async fn list_all_invoices<'e, E>(
        &self,
        executor: E,
        query: InvoiceQueryParams,
    ) -> Result<Vec<InvoiceWithDetails>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                id, organization_id, invoice_number, invoice_date, due_date,
                subtotal, tax_amount, total_amount, currency, status, paid_at
            FROM subscription_invoices
            WHERE ($1::text IS NULL OR status = $1)
            AND ($2::date IS NULL OR invoice_date >= $2)
            AND ($3::date IS NULL OR invoice_date <= $3)
            ORDER BY invoice_date DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(&query.status)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// Mark invoice as paid.
    pub async fn mark_invoice_paid<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        payment_method_id: Option<Uuid>,
    ) -> Result<SubscriptionInvoice, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE subscription_invoices SET
                status = 'paid',
                paid_at = NOW(),
                payment_method_id = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(payment_method_id)
        .fetch_one(executor)
        .await
    }

    /// Void an invoice.
    pub async fn void_invoice<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<SubscriptionInvoice, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE subscription_invoices SET
                status = 'void',
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await
    }

    /// Add line items to an invoice.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_invoice_line_item<'e, E>(
        &self,
        executor: E,
        invoice_id: Uuid,
        description: &str,
        quantity: Option<Decimal>,
        unit_price: Decimal,
        amount: Decimal,
        item_type: &str,
        plan_id: Option<Uuid>,
    ) -> Result<InvoiceLineItem, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO invoice_line_items
                (invoice_id, description, quantity, unit_price, amount, item_type, plan_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(invoice_id)
        .bind(description)
        .bind(quantity)
        .bind(unit_price)
        .bind(amount)
        .bind(item_type)
        .bind(plan_id)
        .fetch_one(executor)
        .await
    }

    /// Get line items for an invoice.
    pub async fn get_invoice_line_items<'e, E>(
        &self,
        executor: E,
        invoice_id: Uuid,
    ) -> Result<Vec<InvoiceLineItem>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM invoice_line_items WHERE invoice_id = $1 ORDER BY created_at")
            .bind(invoice_id)
            .fetch_all(executor)
            .await
    }
}

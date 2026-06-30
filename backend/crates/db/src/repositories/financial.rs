//! Financial repository (Epic 11).
//!
//! Provides database operations for financial accounts, transactions,
//! invoices, payments, and related financial management.

use crate::models::financial::{
    ARReportEntry, ARReportTotals, AccountTransaction, AccountsReceivableReport, CreateFeeSchedule,
    CreateFinancialAccount, CreateInvoice, CreateTransaction, FeeSchedule, FinancialAccount,
    FinancialAccountResponse, Invoice, InvoiceItem, InvoiceResponse, InvoiceStatus, LateFeeConfig,
    ListInvoicesResponse, OnlinePaymentSession, Payment, PaymentAllocation, PaymentResponse,
    RecordPayment, ReminderSchedule, TransactionType, UnitCreditBalance, UnitFee,
};
use crate::DbPool;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for financial operations.
#[derive(Clone)]
pub struct FinancialRepository {
    pool: DbPool,
}

impl FinancialRepository {
    /// Create a new FinancialRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // FINANCIAL ACCOUNTS (Story 11.1)
    // ========================================================================

    /// Create a financial account.
    pub async fn create_account(
        &self,
        org_id: Uuid,
        data: CreateFinancialAccount,
    ) -> Result<FinancialAccount, SqlxError> {
        let account = sqlx::query_as::<_, FinancialAccount>(
            r#"
            INSERT INTO financial_accounts (
                organization_id, building_id, unit_id, name, account_type,
                description, currency, balance, opening_balance, is_active
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, true)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(&data.name)
        .bind(data.account_type)
        .bind(&data.description)
        .bind(&data.currency)
        .bind(data.opening_balance)
        .fetch_one(&self.pool)
        .await?;

        Ok(account)
    }

    /// Get a financial account by ID.
    pub async fn get_account(&self, id: Uuid) -> Result<Option<FinancialAccount>, SqlxError> {
        sqlx::query_as::<_, FinancialAccount>("SELECT * FROM financial_accounts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get financial account with recent transactions.
    pub async fn get_account_with_transactions(
        &self,
        id: Uuid,
        limit: i64,
    ) -> Result<Option<FinancialAccountResponse>, SqlxError> {
        let account = self.get_account(id).await?;

        if let Some(account) = account {
            let recent_transactions = sqlx::query_as::<_, AccountTransaction>(
                r#"
                SELECT * FROM account_transactions
                WHERE account_id = $1
                ORDER BY transaction_date DESC, created_at DESC
                LIMIT $2
                "#,
            )
            .bind(id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

            Ok(Some(FinancialAccountResponse {
                account,
                recent_transactions,
            }))
        } else {
            Ok(None)
        }
    }

    /// List financial accounts for an organization.
    pub async fn list_accounts(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<Vec<FinancialAccount>, SqlxError> {
        if let Some(building) = building_id {
            sqlx::query_as::<_, FinancialAccount>(
                r#"
                SELECT * FROM financial_accounts
                WHERE organization_id = $1 AND building_id = $2 AND is_active = true
                ORDER BY name
                "#,
            )
            .bind(org_id)
            .bind(building)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, FinancialAccount>(
                r#"
                SELECT * FROM financial_accounts
                WHERE organization_id = $1 AND is_active = true
                ORDER BY name
                "#,
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
        }
    }

    /// Get unit ledger account.
    pub async fn get_unit_ledger(
        &self,
        unit_id: Uuid,
    ) -> Result<Option<FinancialAccount>, SqlxError> {
        sqlx::query_as::<_, FinancialAccount>(
            r#"
            SELECT * FROM financial_accounts
            WHERE unit_id = $1 AND account_type = 'unit_ledger'
            "#,
        )
        .bind(unit_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create a transaction.
    pub async fn create_transaction(
        &self,
        user_id: Uuid,
        data: CreateTransaction,
    ) -> Result<AccountTransaction, SqlxError> {
        // Get current balance
        let account = self.get_account(data.account_id).await?;
        let current_balance = account.map(|a| a.balance).unwrap_or_default();

        // Calculate new balance
        let new_balance = match data.transaction_type {
            TransactionType::Debit => current_balance - data.amount,
            TransactionType::Credit => current_balance + data.amount,
        };

        let transaction_date = data
            .transaction_date
            .unwrap_or_else(|| Utc::now().date_naive());
        let reference_id = Uuid::new_v4();

        // Create transaction and update balance atomically
        let transaction = sqlx::query_as::<_, AccountTransaction>(
            r#"
            WITH balance_update AS (
                UPDATE financial_accounts SET balance = $1 WHERE id = $2
            )
            INSERT INTO account_transactions (
                account_id, amount, transaction_type, category, description,
                reference_id, counterpart_account_id, invoice_id, payment_id,
                balance_after, transaction_date, recorded_by, notes
            )
            VALUES ($2, $3, $4, $5, $6, $7, $8, $9, $10, $1, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(new_balance)
        .bind(data.account_id)
        .bind(data.amount)
        .bind(data.transaction_type)
        .bind(data.category)
        .bind(&data.description)
        .bind(reference_id)
        .bind(data.counterpart_account_id)
        .bind(data.invoice_id)
        .bind(data.payment_id)
        .bind(transaction_date)
        .bind(user_id)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(transaction)
    }

    /// List transactions for an account.
    pub async fn list_transactions(
        &self,
        account_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AccountTransaction>, SqlxError> {
        sqlx::query_as::<_, AccountTransaction>(
            r#"
            SELECT * FROM account_transactions
            WHERE account_id = $1
            AND ($2::date IS NULL OR transaction_date >= $2)
            AND ($3::date IS NULL OR transaction_date <= $3)
            ORDER BY transaction_date DESC, created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(account_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // FEE SCHEDULES (Story 11.2)
    // ========================================================================

    /// Create a fee schedule.
    pub async fn create_fee_schedule(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateFeeSchedule,
    ) -> Result<FeeSchedule, SqlxError> {
        let effective_from = data
            .effective_from
            .unwrap_or_else(|| Utc::now().date_naive());

        sqlx::query_as::<_, FeeSchedule>(
            r#"
            INSERT INTO fee_schedules (
                organization_id, building_id, name, description, amount,
                currency, frequency, unit_filter, billing_day,
                effective_from, effective_to, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.amount)
        .bind(&data.currency)
        .bind(data.frequency)
        .bind(&data.unit_filter)
        .bind(data.billing_day)
        .bind(effective_from)
        .bind(data.effective_to)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get fee schedule by ID.
    pub async fn get_fee_schedule(&self, id: Uuid) -> Result<Option<FeeSchedule>, SqlxError> {
        sqlx::query_as::<_, FeeSchedule>("SELECT * FROM fee_schedules WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List fee schedules for a building.
    pub async fn list_fee_schedules(
        &self,
        building_id: Uuid,
        active_only: bool,
    ) -> Result<Vec<FeeSchedule>, SqlxError> {
        if active_only {
            sqlx::query_as::<_, FeeSchedule>(
                r#"
                SELECT * FROM fee_schedules
                WHERE building_id = $1 AND is_active = true
                ORDER BY name
                "#,
            )
            .bind(building_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, FeeSchedule>(
                r#"
                SELECT * FROM fee_schedules
                WHERE building_id = $1
                ORDER BY name
                "#,
            )
            .bind(building_id)
            .fetch_all(&self.pool)
            .await
        }
    }

    /// Assign fee to unit.
    pub async fn assign_unit_fee(
        &self,
        unit_id: Uuid,
        fee_schedule_id: Uuid,
        override_amount: Option<Decimal>,
        effective_from: NaiveDate,
        effective_to: Option<NaiveDate>,
    ) -> Result<UnitFee, SqlxError> {
        sqlx::query_as::<_, UnitFee>(
            r#"
            INSERT INTO unit_fees (
                unit_id, fee_schedule_id, override_amount,
                effective_from, effective_to
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (unit_id, fee_schedule_id, effective_from)
            DO UPDATE SET
                override_amount = EXCLUDED.override_amount,
                effective_to = EXCLUDED.effective_to,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(unit_id)
        .bind(fee_schedule_id)
        .bind(override_amount)
        .bind(effective_from)
        .bind(effective_to)
        .fetch_one(&self.pool)
        .await
    }

    /// Get active fees for a unit.
    pub async fn get_unit_fees(
        &self,
        unit_id: Uuid,
        as_of: NaiveDate,
    ) -> Result<Vec<UnitFee>, SqlxError> {
        sqlx::query_as::<_, UnitFee>(
            r#"
            SELECT * FROM unit_fees
            WHERE unit_id = $1
            AND is_active = true
            AND effective_from <= $2
            AND (effective_to IS NULL OR effective_to >= $2)
            "#,
        )
        .bind(unit_id)
        .bind(as_of)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // INVOICES (Story 11.3)
    // ========================================================================

    /// Create an invoice with items.
    pub async fn create_invoice(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateInvoice,
    ) -> Result<InvoiceResponse, SqlxError> {
        let issue_date = Utc::now().date_naive();

        // Calculate totals
        let mut subtotal = Decimal::ZERO;
        let mut tax_amount = Decimal::ZERO;
        for item in &data.items {
            let item_amount = item.quantity * item.unit_price;
            subtotal += item_amount;
            if let Some(tax_rate) = item.tax_rate {
                tax_amount += item_amount * tax_rate / Decimal::from(100);
            }
        }
        let total = subtotal + tax_amount;

        // Create invoice with generated invoice number
        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoices (
                organization_id, unit_id, billing_period_start, billing_period_end,
                issue_date, due_date, subtotal, tax_amount, total, balance_due,
                currency, notes, created_by, invoice_number
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9, $10, $11, $12, generate_invoice_number($1))
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.unit_id)
        .bind(data.billing_period_start)
        .bind(data.billing_period_end)
        .bind(issue_date)
        .bind(data.due_date)
        .bind(subtotal)
        .bind(tax_amount)
        .bind(total)
        .bind(&data.currency)
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Create invoice items
        let mut items = Vec::new();
        for (idx, item_data) in data.items.into_iter().enumerate() {
            let item_amount = item_data.quantity * item_data.unit_price;
            let item_tax = item_data
                .tax_rate
                .map(|r| item_amount * r / Decimal::from(100));

            let item = sqlx::query_as::<_, InvoiceItem>(
                r#"
                INSERT INTO invoice_items (
                    invoice_id, description, quantity, unit_price, amount,
                    tax_rate, tax_amount, fee_schedule_id, meter_reading_id, sort_order
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING *
                "#,
            )
            .bind(invoice.id)
            .bind(&item_data.description)
            .bind(item_data.quantity)
            .bind(item_data.unit_price)
            .bind(item_amount)
            .bind(item_data.tax_rate)
            .bind(item_tax)
            .bind(item_data.fee_schedule_id)
            .bind(item_data.meter_reading_id)
            .bind(idx as i32)
            .fetch_one(&self.pool)
            .await?;

            items.push(item);
        }

        Ok(InvoiceResponse {
            invoice,
            items,
            payments: vec![],
        })
    }

    /// Get invoice by ID.
    pub async fn get_invoice(&self, id: Uuid) -> Result<Option<Invoice>, SqlxError> {
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get invoice with items and payments.
    pub async fn get_invoice_with_details(
        &self,
        id: Uuid,
    ) -> Result<Option<InvoiceResponse>, SqlxError> {
        let invoice = self.get_invoice(id).await?;

        if let Some(invoice) = invoice {
            let items = sqlx::query_as::<_, InvoiceItem>(
                "SELECT * FROM invoice_items WHERE invoice_id = $1 ORDER BY sort_order",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

            let payments = sqlx::query_as::<_, PaymentAllocation>(
                "SELECT * FROM payment_allocations WHERE invoice_id = $1 ORDER BY created_at",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

            Ok(Some(InvoiceResponse {
                invoice,
                items,
                payments,
            }))
        } else {
            Ok(None)
        }
    }

    /// List invoices for a unit.
    pub async fn list_invoices_for_unit(
        &self,
        unit_id: Uuid,
        status: Option<InvoiceStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<ListInvoicesResponse, SqlxError> {
        let invoices = if let Some(s) = status {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT * FROM invoices
                WHERE unit_id = $1 AND status = $2
                ORDER BY issue_date DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(unit_id)
            .bind(s)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT * FROM invoices
                WHERE unit_id = $1
                ORDER BY issue_date DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(unit_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM invoices
            WHERE unit_id = $1 AND ($2::invoice_status IS NULL OR status = $2)
            "#,
        )
        .bind(unit_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListInvoicesResponse {
            invoices,
            total: total.0,
        })
    }

    /// List invoices for an organization.
    pub async fn list_invoices_for_org(
        &self,
        org_id: Uuid,
        status: Option<InvoiceStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<ListInvoicesResponse, SqlxError> {
        let invoices = if let Some(s) = status {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT * FROM invoices
                WHERE organization_id = $1 AND status = $2
                ORDER BY issue_date DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(org_id)
            .bind(s)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Invoice>(
                r#"
                SELECT * FROM invoices
                WHERE organization_id = $1
                ORDER BY issue_date DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(org_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM invoices
            WHERE organization_id = $1 AND ($2::invoice_status IS NULL OR status = $2)
            "#,
        )
        .bind(org_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListInvoicesResponse {
            invoices,
            total: total.0,
        })
    }

    /// Update invoice status.
    pub async fn update_invoice_status(
        &self,
        id: Uuid,
        status: InvoiceStatus,
    ) -> Result<Option<Invoice>, SqlxError> {
        sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_optional(&self.pool)
        .await
    }

    /// Mark invoice as sent.
    pub async fn mark_invoice_sent(&self, id: Uuid) -> Result<Option<Invoice>, SqlxError> {
        sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices SET status = 'sent', sent_at = NOW(), updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // PAYMENTS (Story 11.4)
    // ========================================================================

    /// Record a payment.
    pub async fn record_payment(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: RecordPayment,
    ) -> Result<PaymentResponse, SqlxError> {
        let payment_date = data.payment_date.unwrap_or_else(|| Utc::now().date_naive());

        // Create payment
        let payment = sqlx::query_as::<_, Payment>(
            r#"
            INSERT INTO payments (
                organization_id, unit_id, amount, currency, payment_method,
                status, reference, payment_date, notes, recorded_by
            )
            VALUES ($1, $2, $3, $4, $5, 'completed', $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.unit_id)
        .bind(data.amount)
        .bind(&data.currency)
        .bind(data.payment_method)
        .bind(&data.reference)
        .bind(payment_date)
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Allocate to invoices
        let mut allocations = Vec::new();
        let mut remaining = data.amount;

        if data.invoice_ids.is_empty() {
            // Auto-allocate to oldest unpaid invoices
            let unpaid_invoices = sqlx::query_as::<_, Invoice>(
                r#"
                SELECT * FROM invoices
                WHERE unit_id = $1 AND balance_due > 0
                ORDER BY due_date ASC
                "#,
            )
            .bind(data.unit_id)
            .fetch_all(&self.pool)
            .await?;

            for invoice in unpaid_invoices {
                if remaining <= Decimal::ZERO {
                    break;
                }

                let allocation_amount = remaining.min(invoice.balance_due);
                remaining -= allocation_amount;

                let allocation = self
                    .allocate_payment(payment.id, invoice.id, allocation_amount)
                    .await?;
                allocations.push(allocation);
            }
        } else {
            // Allocate to specified invoices
            for invoice_id in data.invoice_ids {
                if remaining <= Decimal::ZERO {
                    break;
                }

                if let Some(invoice) = self.get_invoice(invoice_id).await? {
                    let allocation_amount = remaining.min(invoice.balance_due);
                    remaining -= allocation_amount;

                    let allocation = self
                        .allocate_payment(payment.id, invoice_id, allocation_amount)
                        .await?;
                    allocations.push(allocation);
                }
            }
        }

        // If there's remaining amount, add to credit balance
        if remaining > Decimal::ZERO {
            self.add_credit_balance(data.unit_id, remaining, &data.currency)
                .await?;
        }

        Ok(PaymentResponse {
            payment,
            allocations,
        })
    }

    /// Allocate payment to invoice.
    async fn allocate_payment(
        &self,
        payment_id: Uuid,
        invoice_id: Uuid,
        amount: Decimal,
    ) -> Result<PaymentAllocation, SqlxError> {
        // Create allocation
        let allocation = sqlx::query_as::<_, PaymentAllocation>(
            r#"
            INSERT INTO payment_allocations (payment_id, invoice_id, amount)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(payment_id)
        .bind(invoice_id)
        .bind(amount)
        .fetch_one(&self.pool)
        .await?;

        // The invoice's amount_paid / balance_due / status are recomputed from the
        // sum of allocations by the `update_invoice_on_allocation` trigger. A manual
        // `balance_due = balance_due - amount` update here double-counts the
        // allocation (the balance goes negative) — so it is intentionally omitted.
        Ok(allocation)
    }

    /// Add credit balance to unit.
    async fn add_credit_balance(
        &self,
        unit_id: Uuid,
        amount: Decimal,
        currency: &str,
    ) -> Result<UnitCreditBalance, SqlxError> {
        sqlx::query_as::<_, UnitCreditBalance>(
            r#"
            INSERT INTO unit_credit_balances (unit_id, balance, currency)
            VALUES ($1, $2, $3)
            ON CONFLICT (unit_id) DO UPDATE
            SET balance = unit_credit_balances.balance + EXCLUDED.balance,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(unit_id)
        .bind(amount)
        .bind(currency)
        .fetch_one(&self.pool)
        .await
    }

    /// Get payment by ID.
    pub async fn get_payment(&self, id: Uuid) -> Result<Option<Payment>, SqlxError> {
        sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get payment with allocations.
    pub async fn get_payment_with_allocations(
        &self,
        id: Uuid,
    ) -> Result<Option<PaymentResponse>, SqlxError> {
        let payment = self.get_payment(id).await?;

        if let Some(payment) = payment {
            let allocations = sqlx::query_as::<_, PaymentAllocation>(
                "SELECT * FROM payment_allocations WHERE payment_id = $1",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

            Ok(Some(PaymentResponse {
                payment,
                allocations,
            }))
        } else {
            Ok(None)
        }
    }

    /// List payments for a unit.
    pub async fn list_payments_for_unit(
        &self,
        unit_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Payment>, SqlxError> {
        sqlx::query_as::<_, Payment>(
            r#"
            SELECT * FROM payments
            WHERE unit_id = $1
            ORDER BY payment_date DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(unit_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    // ------------------------------------------------------------------------
    // Org-wide payment management (Story 11.4 — Payment Management page).
    //
    // The page needs an org-scoped view of all payments plus reconciliation
    // (which payments are still unallocated, allocate one to an invoice, and a
    // bulk auto-match). Every query is scoped on `organization_id` so a payment
    // or invoice from another org is never touched.
    // ------------------------------------------------------------------------

    /// List all payments for an organization, newest first, optionally filtered
    /// by status (compared as text to avoid enum-encode pitfalls).
    pub async fn list_payments(
        &self,
        organization_id: Uuid,
        status: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Payment>, SqlxError> {
        sqlx::query_as::<_, Payment>(
            r#"
            SELECT * FROM payments
            WHERE organization_id = $1
              AND ($2::text IS NULL OR status::text = $2)
            ORDER BY payment_date DESC, created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(organization_id)
        .bind(&status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Count payments for an organization (matching the same status filter as
    /// [`list_payments`](Self::list_payments)) so paginated routes can report a
    /// true total instead of the current page's length.
    pub async fn count_payments(
        &self,
        organization_id: Uuid,
        status: Option<String>,
    ) -> Result<i64, SqlxError> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM payments
            WHERE organization_id = $1
              AND ($2::text IS NULL OR status::text = $2)
            "#,
        )
        .bind(organization_id)
        .bind(&status)
        .fetch_one(&self.pool)
        .await
    }

    /// List payments that still have an unallocated balance (sum of their
    /// allocations is less than the payment amount) — the reconciliation queue.
    pub async fn list_unallocated_payments(
        &self,
        organization_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Payment>, SqlxError> {
        sqlx::query_as::<_, Payment>(
            r#"
            SELECT p.*
            FROM payments p
            LEFT JOIN payment_allocations pa ON pa.payment_id = p.id
            WHERE p.organization_id = $1
            GROUP BY p.id
            HAVING p.amount - COALESCE(SUM(pa.amount), 0) > 0
            ORDER BY p.payment_date DESC, p.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Allocate an already-recorded payment to an invoice (the manual "match"
    /// action). Both the payment and invoice must belong to `organization_id`,
    /// else `Ok(None)` (the route returns 404 — no cross-org leakage). The
    /// allocated amount is clamped to both the payment's remaining unallocated
    /// balance and the invoice's outstanding balance; if nothing can be
    /// allocated, returns `Ok(None)`.
    pub async fn allocate_existing_payment(
        &self,
        organization_id: Uuid,
        payment_id: Uuid,
        invoice_id: Uuid,
        amount: Option<Decimal>,
    ) -> Result<Option<PaymentAllocation>, SqlxError> {
        let mut tx = self.pool.begin().await?;

        let payment = sqlx::query_as::<_, Payment>(
            "SELECT * FROM payments WHERE id = $1 AND organization_id = $2",
        )
        .bind(payment_id)
        .bind(organization_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(payment) = payment else {
            return Ok(None);
        };

        let invoice = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE id = $1 AND organization_id = $2",
        )
        .bind(invoice_id)
        .bind(organization_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(invoice) = invoice else {
            return Ok(None);
        };

        let allocated: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0) FROM payment_allocations WHERE payment_id = $1",
        )
        .bind(payment_id)
        .fetch_one(&mut *tx)
        .await?;
        let payment_remaining = payment.amount - allocated;

        let requested = amount.unwrap_or(payment_remaining);
        let to_allocate = requested.min(payment_remaining).min(invoice.balance_due);
        if to_allocate <= Decimal::ZERO {
            return Ok(None);
        }

        // Inserting the allocation is sufficient: the `update_invoice_on_allocation`
        // trigger recomputes the invoice's amount_paid / balance_due / status from
        // the sum of allocations. Do NOT also update the invoice here or the
        // balance is decremented twice.
        let allocation = sqlx::query_as::<_, PaymentAllocation>(
            r#"
            INSERT INTO payment_allocations (payment_id, invoice_id, amount)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(payment_id)
        .bind(invoice_id)
        .bind(to_allocate)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(allocation))
    }

    /// Bulk auto-match: for every payment in the org with an unallocated
    /// balance, allocate it (oldest-due first) against that unit's outstanding
    /// invoices — the same oldest-first rule [`record_payment`](Self::record_payment)
    /// applies at record time. Returns the number of allocations created.
    pub async fn auto_match_payments(&self, organization_id: Uuid) -> Result<u64, SqlxError> {
        let mut tx = self.pool.begin().await?;

        let payments = sqlx::query_as::<_, Payment>(
            r#"
            SELECT p.*
            FROM payments p
            LEFT JOIN payment_allocations pa ON pa.payment_id = p.id
            WHERE p.organization_id = $1
            GROUP BY p.id
            HAVING p.amount - COALESCE(SUM(pa.amount), 0) > 0
            ORDER BY p.payment_date ASC
            "#,
        )
        .bind(organization_id)
        .fetch_all(&mut *tx)
        .await?;

        let mut created: u64 = 0;
        for payment in payments {
            let allocated: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(SUM(amount), 0) FROM payment_allocations WHERE payment_id = $1",
            )
            .bind(payment.id)
            .fetch_one(&mut *tx)
            .await?;
            let mut remaining = payment.amount - allocated;
            if remaining <= Decimal::ZERO {
                continue;
            }

            let invoices = sqlx::query_as::<_, Invoice>(
                r#"
                SELECT * FROM invoices
                WHERE unit_id = $1 AND organization_id = $2 AND balance_due > 0
                ORDER BY due_date ASC, created_at ASC
                "#,
            )
            .bind(payment.unit_id)
            .bind(organization_id)
            .fetch_all(&mut *tx)
            .await?;

            for invoice in invoices {
                if remaining <= Decimal::ZERO {
                    break;
                }
                let to_allocate = remaining.min(invoice.balance_due);
                if to_allocate <= Decimal::ZERO {
                    continue;
                }
                // The `update_invoice_on_allocation` trigger updates the invoice
                // balance/status; inserting the allocation is all that's needed.
                sqlx::query(
                    "INSERT INTO payment_allocations (payment_id, invoice_id, amount) VALUES ($1, $2, $3)",
                )
                .bind(payment.id)
                .bind(invoice.id)
                .bind(to_allocate)
                .execute(&mut *tx)
                .await?;
                remaining -= to_allocate;
                created += 1;
            }
        }

        tx.commit().await?;
        Ok(created)
    }

    // ========================================================================
    // ONLINE PAYMENTS (Story 11.5)
    // ========================================================================

    /// Create online payment session.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_payment_session(
        &self,
        org_id: Uuid,
        invoice_id: Uuid,
        provider: &str,
        session_id: &str,
        checkout_url: &str,
        amount: Decimal,
        currency: &str,
    ) -> Result<OnlinePaymentSession, SqlxError> {
        sqlx::query_as::<_, OnlinePaymentSession>(
            r#"
            INSERT INTO online_payment_sessions (
                organization_id, invoice_id, provider, session_id,
                checkout_url, amount, currency, status, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', NOW() + INTERVAL '1 hour')
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(invoice_id)
        .bind(provider)
        .bind(session_id)
        .bind(checkout_url)
        .bind(amount)
        .bind(currency)
        .fetch_one(&self.pool)
        .await
    }

    /// Get payment session by provider session ID.
    pub async fn get_payment_session_by_provider_id(
        &self,
        provider_session_id: &str,
    ) -> Result<Option<OnlinePaymentSession>, SqlxError> {
        sqlx::query_as::<_, OnlinePaymentSession>(
            "SELECT * FROM online_payment_sessions WHERE session_id = $1",
        )
        .bind(provider_session_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update payment session status.
    pub async fn update_payment_session_status(
        &self,
        id: Uuid,
        status: &str,
        payment_id: Option<Uuid>,
        error_message: Option<&str>,
    ) -> Result<Option<OnlinePaymentSession>, SqlxError> {
        sqlx::query_as::<_, OnlinePaymentSession>(
            r#"
            UPDATE online_payment_sessions
            SET status = $2, payment_id = $3, error_message = $4, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(payment_id)
        .bind(error_message)
        .fetch_optional(&self.pool)
        .await
    }

    /// Settle an invoice from a confirmed online-gateway payment (Story 11.5).
    ///
    /// Runs in one transaction so a webhook delivery either fully settles or
    /// not at all:
    ///   1. inserts a `payments` row (`payment_method = 'online'`,
    ///      `recorded_by = NULL` — the payer is the gateway, not an internal
    ///      user; `external_reference` = the gateway transaction id),
    ///   2. allocates it to `invoice` (capped at the invoice's `balance_due`),
    ///   3. links the payment back to the originating `online_payment_sessions`
    ///      row and marks the session `completed`.
    ///
    /// The `update_invoice_on_allocation` trigger recomputes the invoice's
    /// `amount_paid`/`balance_due`/`status`, so a fully-covered invoice
    /// transitions to `paid` automatically.
    ///
    /// **Idempotency is enforced atomically**: the first statement in the tx
    /// is a conditional `UPDATE … WHERE status <> 'completed'` that claims the
    /// session row. Stripe is at-least-once and does not serialize deliveries,
    /// so two concurrent duplicates can both pass a pre-tx status read; the
    /// conditional UPDATE row-locks the session, so the loser observes
    /// `rows_affected() == 0` and returns `Ok(None)` (a safe no-op) instead of
    /// inserting a second payment + allocation. Returns `Ok(Some(payment))`
    /// for the delivery that actually settles.
    pub async fn settle_invoice_from_gateway(
        &self,
        session: &OnlinePaymentSession,
        invoice: &Invoice,
        gateway_reference: &str,
    ) -> Result<Option<Payment>, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Atomic idempotency claim. Only the first concurrent delivery flips
        // the row from a non-`completed` state; the conditional predicate
        // row-locks the session so duplicates serialize behind this tx and
        // then no-op. Bail before touching `payments` if we lost the race.
        let claim = sqlx::query(
            r#"
            UPDATE online_payment_sessions
            SET status = 'completed', updated_at = NOW()
            WHERE id = $1 AND status <> 'completed'
            "#,
        )
        .bind(session.id)
        .execute(&mut *tx)
        .await?;

        if claim.rows_affected() == 0 {
            // Another delivery already settled this session — safe no-op.
            tx.rollback().await?;
            return Ok(None);
        }

        let payment = sqlx::query_as::<_, Payment>(
            r#"
            INSERT INTO payments (
                organization_id, unit_id, amount, currency, payment_method,
                status, reference, external_reference, recorded_by
            )
            VALUES ($1, $2, $3, $4, 'online', 'completed', $5, $6, NULL)
            RETURNING *
            "#,
        )
        .bind(session.organization_id)
        .bind(invoice.unit_id)
        .bind(session.amount)
        .bind(&session.currency)
        .bind(&session.session_id)
        .bind(gateway_reference)
        .fetch_one(&mut *tx)
        .await?;

        let allocation_amount = session.amount.min(invoice.balance_due);
        sqlx::query(
            r#"
            INSERT INTO payment_allocations (payment_id, invoice_id, amount)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(payment.id)
        .bind(invoice.id)
        .bind(allocation_amount)
        .execute(&mut *tx)
        .await?;

        // Status was already set to 'completed' by the atomic claim above; here
        // we only link the freshly-recorded payment back to the session.
        sqlx::query(
            r#"
            UPDATE online_payment_sessions
            SET payment_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(session.id)
        .bind(payment.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(payment))
    }

    // ========================================================================
    // REMINDERS (Story 11.6)
    // ========================================================================

    /// Get reminder schedules for organization.
    pub async fn get_reminder_schedules(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<ReminderSchedule>, SqlxError> {
        sqlx::query_as::<_, ReminderSchedule>(
            r#"
            SELECT * FROM reminder_schedules
            WHERE organization_id = $1 AND is_active = true
            ORDER BY COALESCE(days_before_due, 0) DESC, COALESCE(days_after_due, 0) ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get late fee config for organization.
    pub async fn get_late_fee_config(
        &self,
        org_id: Uuid,
    ) -> Result<Option<LateFeeConfig>, SqlxError> {
        sqlx::query_as::<_, LateFeeConfig>(
            "SELECT * FROM late_fee_configs WHERE organization_id = $1",
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get overdue invoices.
    pub async fn get_overdue_invoices(&self, org_id: Uuid) -> Result<Vec<Invoice>, SqlxError> {
        sqlx::query_as::<_, Invoice>(
            r#"
            SELECT * FROM invoices
            WHERE organization_id = $1
            AND status IN ('sent', 'partial')
            AND due_date < CURRENT_DATE
            AND balance_due > 0
            ORDER BY due_date ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // REPORTS (Story 11.7)
    // ========================================================================

    /// Generate accounts receivable aging report.
    pub async fn get_ar_aging_report(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<AccountsReceivableReport, SqlxError> {
        let as_of_date = Utc::now().date_naive();

        // Using a raw query for the aging calculation
        let entries = sqlx::query_as::<_, ARReportEntry>(
            r#"
            SELECT
                u.id as unit_id,
                u.designation AS unit_number,
                COALESCE(SUM(CASE WHEN i.due_date >= $3 THEN i.balance_due ELSE 0 END), 0) as current,
                COALESCE(SUM(CASE WHEN i.due_date < $3 AND i.due_date >= $3 - INTERVAL '30 days' THEN i.balance_due ELSE 0 END), 0) as days_30,
                COALESCE(SUM(CASE WHEN i.due_date < $3 - INTERVAL '30 days' AND i.due_date >= $3 - INTERVAL '60 days' THEN i.balance_due ELSE 0 END), 0) as days_60,
                COALESCE(SUM(CASE WHEN i.due_date < $3 - INTERVAL '60 days' THEN i.balance_due ELSE 0 END), 0) as days_90_plus,
                COALESCE(SUM(i.balance_due), 0) as total
            FROM units u
            JOIN buildings b ON u.building_id = b.id
            LEFT JOIN invoices i ON i.unit_id = u.id AND i.balance_due > 0
            WHERE b.organization_id = $1
            AND ($2::uuid IS NULL OR b.id = $2)
            GROUP BY u.id, u.designation
            HAVING COALESCE(SUM(i.balance_due), 0) > 0
            ORDER BY total DESC
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(as_of_date)
        .fetch_all(&self.pool)
        .await?;

        // Calculate totals
        let mut totals = ARReportTotals {
            current: Decimal::ZERO,
            days_30: Decimal::ZERO,
            days_60: Decimal::ZERO,
            days_90_plus: Decimal::ZERO,
            total: Decimal::ZERO,
        };

        for entry in &entries {
            totals.current += entry.current;
            totals.days_30 += entry.days_30;
            totals.days_60 += entry.days_60;
            totals.days_90_plus += entry.days_90_plus;
            totals.total += entry.total;
        }

        Ok(AccountsReceivableReport {
            as_of_date,
            entries,
            totals,
        })
    }

    /// Income statement: revenue vs expenses from account_transactions over a date range.
    pub async fn get_income_statement(
        &self,
        org_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<crate::models::financial::IncomeStatement, SqlxError> {
        use crate::models::financial::{IncomeStatement, IncomeStatementLine};

        let revenue_rows = sqlx::query_as::<_, IncomeStatementLine>(
            r#"
            SELECT category::text AS category,
                   COALESCE(SUM(amount), 0) AS amount
            FROM account_transactions at_
            JOIN financial_accounts fa ON fa.id = at_.account_id
            WHERE fa.organization_id = $1
              AND at_.transaction_type = 'credit'
              AND at_.transaction_date >= $2
              AND at_.transaction_date <= $3
            GROUP BY category
            ORDER BY amount DESC
            "#,
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        let expense_rows = sqlx::query_as::<_, IncomeStatementLine>(
            r#"
            SELECT category::text AS category,
                   COALESCE(SUM(amount), 0) AS amount
            FROM account_transactions at_
            JOIN financial_accounts fa ON fa.id = at_.account_id
            WHERE fa.organization_id = $1
              AND at_.transaction_type = 'debit'
              AND at_.transaction_date >= $2
              AND at_.transaction_date <= $3
            GROUP BY category
            ORDER BY amount DESC
            "#,
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        let total_revenue = revenue_rows.iter().map(|r| r.amount).sum();
        let total_expenses = expense_rows.iter().map(|r| r.amount).sum();
        let net_income = total_revenue - total_expenses;

        Ok(IncomeStatement {
            from_date: from,
            to_date: to,
            revenue: revenue_rows,
            expenses: expense_rows,
            total_revenue,
            total_expenses,
            net_income,
        })
    }

    /// Balance sheet snapshot: all account balances as of a date.
    pub async fn get_balance_sheet(
        &self,
        org_id: Uuid,
        as_of: NaiveDate,
    ) -> Result<crate::models::financial::BalanceSheetReport, SqlxError> {
        use crate::models::financial::{BalanceSheetLine, BalanceSheetReport};

        let accounts = sqlx::query_as::<_, BalanceSheetLine>(
            r#"
            SELECT fa.id AS account_id,
                   fa.name AS account_name,
                   fa.account_type::text AS account_type,
                   fa.opening_balance
                   + COALESCE(SUM(CASE WHEN at_.transaction_type = 'credit' THEN at_.amount ELSE 0 END), 0)
                   - COALESCE(SUM(CASE WHEN at_.transaction_type = 'debit'  THEN at_.amount ELSE 0 END), 0)
                   AS balance
            FROM financial_accounts fa
            LEFT JOIN account_transactions at_
                   ON at_.account_id = fa.id AND at_.transaction_date <= $2
            WHERE fa.organization_id = $1
              AND fa.is_active = true
            GROUP BY fa.id, fa.name, fa.account_type, fa.opening_balance
            ORDER BY fa.account_type, fa.name
            "#,
        )
        .bind(org_id)
        .bind(as_of)
        .fetch_all(&self.pool)
        .await?;

        let total_assets: Decimal = accounts
            .iter()
            .filter(|a| a.balance > Decimal::ZERO)
            .map(|a| a.balance)
            .sum();
        let total_liabilities: Decimal = accounts
            .iter()
            .filter(|a| a.balance < Decimal::ZERO)
            .map(|a| a.balance.abs())
            .sum();
        let net_equity = total_assets - total_liabilities;

        Ok(BalanceSheetReport {
            as_of_date: as_of,
            accounts,
            total_assets,
            total_liabilities,
            net_equity,
        })
    }

    /// Cash flow report: inflows vs outflows for a date range.
    pub async fn get_cash_flow(
        &self,
        org_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<crate::models::financial::CashFlowReport, SqlxError> {
        use crate::models::financial::{CashFlowLine, CashFlowReport};

        let inflow_rows = sqlx::query_as::<_, CashFlowLine>(
            r#"
            SELECT category::text AS category,
                   COALESCE(SUM(amount), 0) AS amount
            FROM account_transactions at_
            JOIN financial_accounts fa ON fa.id = at_.account_id
            WHERE fa.organization_id = $1
              AND at_.transaction_type = 'credit'
              AND at_.transaction_date >= $2
              AND at_.transaction_date <= $3
            GROUP BY category
            ORDER BY amount DESC
            "#,
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        let outflow_rows = sqlx::query_as::<_, CashFlowLine>(
            r#"
            SELECT category::text AS category,
                   COALESCE(SUM(amount), 0) AS amount
            FROM account_transactions at_
            JOIN financial_accounts fa ON fa.id = at_.account_id
            WHERE fa.organization_id = $1
              AND at_.transaction_type = 'debit'
              AND at_.transaction_date >= $2
              AND at_.transaction_date <= $3
            GROUP BY category
            ORDER BY amount DESC
            "#,
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        let total_inflows = inflow_rows.iter().map(|r| r.amount).sum();
        let total_outflows = outflow_rows.iter().map(|r| r.amount).sum();
        let net_cash_flow = total_inflows - total_outflows;

        Ok(CashFlowReport {
            from_date: from,
            to_date: to,
            inflows: inflow_rows,
            outflows: outflow_rows,
            total_inflows,
            total_outflows,
            net_cash_flow,
        })
    }

    // ========================================================================
    // Story 11.6: Scheduler payment reminder & overdue transition helpers
    // ========================================================================

    /// Find all `sent` or `partial` invoices with `due_date` within the next
    /// `days` days, across all organizations. Used by the scheduler reminder
    /// tick.
    ///
    /// The due/overdue boundary is computed with SQL `CURRENT_DATE`
    /// (`make_interval`) rather than `Utc::now()` in Rust so it agrees with the
    /// `update_invoice_payment_status` trigger (migration 00040), which also
    /// keys off `CURRENT_DATE`. This avoids a CET/CEST tenant flipping
    /// due/overdue a day early or late when process TZ and DB TZ differ
    /// (#1769 finding 2). `partial` invoices are included so partially-paid
    /// invoices get an upcoming-due warning before the overdue escalation
    /// (which already covers `partial`) (#1790 finding 2).
    pub async fn find_invoices_due_for_reminder(
        &self,
        days: i64,
    ) -> Result<Vec<Invoice>, SqlxError> {
        sqlx::query_as::<_, Invoice>(
            r#"
            SELECT * FROM invoices
            WHERE status IN ('sent', 'partial')
              AND due_date > CURRENT_DATE
              AND due_date <= CURRENT_DATE + make_interval(days => $1)
              AND balance_due > 0
              -- Persistent dedup (#1790): skip invoices reminded within the last
              -- 24h so a 60s scheduler tick can't re-spam the whole window.
              AND (last_payment_reminder_at IS NULL
                   OR last_payment_reminder_at < NOW() - INTERVAL '24 hours')
            ORDER BY due_date ASC
            "#,
        )
        .bind(days as i32)
        .fetch_all(&self.pool)
        .await
    }

    /// Stamp `last_payment_reminder_at = NOW()` for an invoice after a reminder
    /// has been dispatched, so subsequent scheduler ticks skip it for the dedup
    /// window (#1790). Mirrors the signature-reminder `touch_signer_reminder`.
    pub async fn mark_payment_reminder_sent(&self, invoice_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("UPDATE invoices SET last_payment_reminder_at = NOW() WHERE id = $1")
            .bind(invoice_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Atomically transitions all `sent` or `partial` invoices whose
    /// `due_date` is strictly before `CURRENT_DATE - grace_period_days` to
    /// status `overdue`. Returns the transitioned invoices so callers can
    /// fire escalation notifications. Safe to call repeatedly (idempotent on
    /// already-overdue rows).
    pub async fn transition_invoices_to_overdue(
        &self,
        grace_period_days: i64,
    ) -> Result<Vec<Invoice>, SqlxError> {
        // Drive the cutoff off SQL `CURRENT_DATE` (matching the
        // `update_invoice_payment_status` trigger in migration 00040) instead of
        // `Utc::now().date_naive()`, so the scheduler and the trigger share one
        // definition of "overdue" regardless of process vs DB timezone (#1769
        // finding 2).
        sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices
            SET status = 'overdue', updated_at = NOW()
            WHERE status IN ('sent', 'partial')
              AND due_date < CURRENT_DATE - make_interval(days => $1)
              AND balance_due > 0
            RETURNING *
            "#,
        )
        .bind(grace_period_days as i32)
        .fetch_all(&self.pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    /// Verify the reminder window predicate: invoices due strictly after today
    /// and on or before today + N days qualify for a reminder.
    #[test]
    fn test_reminder_window_predicate() {
        let today = Utc::now().date_naive();
        let days_before = 7i64;
        let cutoff = today + Duration::days(days_before);

        // The reminder window predicate: due strictly after today, on or before cutoff.
        let in_reminder_window = |due| due > today && due <= cutoff;

        // Due tomorrow — inside window.
        assert!(in_reminder_window(today + Duration::days(1)));

        // Due exactly at cutoff — inside window.
        assert!(in_reminder_window(cutoff));

        // Due today — excluded (already due, not upcoming).
        assert!(!in_reminder_window(today));

        // Due 8 days out — outside window.
        assert!(!in_reminder_window(today + Duration::days(8)));
    }

    /// Verify the overdue grace-period predicate: invoices with due_date
    /// strictly before today - grace_period_days should be transitioned.
    #[test]
    fn test_overdue_grace_period_predicate() {
        let today = Utc::now().date_naive();

        // The overdue predicate: due strictly before the grace cutoff transitions.
        let is_overdue = |due, grace_cutoff| due < grace_cutoff;

        // Grace period = 0: any invoice past due (due_date < today) transitions.
        let grace_cutoff_0 = today - Duration::days(0);
        assert!(is_overdue(today - Duration::days(1), grace_cutoff_0));

        // Due today is NOT included (strict less-than).
        assert!(!is_overdue(today, grace_cutoff_0));

        // Grace period = 3: invoice overdue by 2 days stays pending.
        let grace_cutoff_3 = today - Duration::days(3);
        assert!(!is_overdue(today - Duration::days(2), grace_cutoff_3));

        // Invoice overdue by 4 days transitions.
        assert!(is_overdue(today - Duration::days(4), grace_cutoff_3));
    }
}

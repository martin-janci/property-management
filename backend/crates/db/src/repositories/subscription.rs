//! Subscription and billing repository (Epic 26).
//!
//! # RLS Integration (PAP-112 / PAP-80 / PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the seven org-scoped billing tables this
//! repo touches (`organization_subscriptions`, `payment_methods`,
//! `subscription_invoices`, `invoice_line_items`, `usage_records`,
//! `subscription_events`, `coupon_redemptions`). Under `FORCE` the
//! api-server's owner connection is no longer exempt, so a query issued on a
//! connection without `app.current_org_id` set collapses to deny-all:
//! own-org reads return empty and writes fail the policy `WITH CHECK`.
//!
//! Every method therefore takes an **executor whose connection already has
//! RLS context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds
//! **no pool**, so there is no way to issue a query that bypasses RLS. This
//! mirrors the `work_order.rs` / `vendor.rs` / `llm_document.rs` precedent.
//!
//! Single-statement methods take a generic [`Executor`]; multi-statement
//! methods (`create_subscription`, `create_invoice`, `get_statistics`) and
//! the transactional ones (`set_default_payment_method`, `redeem_coupon`)
//! take `&mut PgConnection` and reborrow. The transactions run on the
//! context-set connection: `set_request_context` sets session-level GUCs, so
//! the RLS context survives `BEGIN`/`COMMIT`.
//!
//! `subscription_plans` and `subscription_coupons` are **not** FORCE-bound
//! (they carry public read policies — plans/coupons are platform-global, not
//! tenant data), so callers without a tenant principal (the public
//! `/plans/public` endpoint) may run those read methods on a plain pool
//! executor.

use crate::models::{
    CancelSubscriptionRequest, ChangePlanRequest, CouponRedemption, CreateOrganizationSubscription,
    CreateSubscriptionCoupon, CreateSubscriptionEvent, CreateSubscriptionPaymentMethod,
    CreateSubscriptionPlan, CreateUsageRecord, InvoiceLineItem, InvoiceQueryParams,
    InvoiceWithDetails, OrganizationSubscription, PlanSubscriptionCount, SubscriptionCoupon,
    SubscriptionEvent, SubscriptionInvoice, SubscriptionPaymentMethod, SubscriptionPlan,
    SubscriptionStatistics, SubscriptionWithPlan, UpdateOrganizationSubscription,
    UpdateSubscriptionCoupon, UpdateSubscriptionPlan, UsageRecord, UsageSummary,
};
use chrono::{Days, Months, Utc};
use rust_decimal::Decimal;
use sqlx::{Connection, Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

/// Repository for subscription and billing operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`)
/// query.
#[derive(Clone)]
pub struct SubscriptionRepository;

impl SubscriptionRepository {
    /// Create a new SubscriptionRepository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set
    /// connection supplied by the caller).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // ==================== Subscription Plans CRUD ====================

    /// Create a new subscription plan.
    pub async fn create_plan<'e, E>(
        &self,
        executor: E,
        data: CreateSubscriptionPlan,
    ) -> Result<SubscriptionPlan, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO subscription_plans
                (name, display_name, description, monthly_price, annual_price, currency,
                 max_buildings, max_units, max_users, max_storage_gb, features, trial_days,
                 sort_order, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(&data.name)
        .bind(&data.display_name)
        .bind(&data.description)
        .bind(data.monthly_price)
        .bind(data.annual_price)
        .bind(data.currency.unwrap_or_else(|| "EUR".to_string()))
        .bind(data.max_buildings)
        .bind(data.max_units)
        .bind(data.max_users)
        .bind(data.max_storage_gb)
        .bind(&data.features)
        .bind(data.trial_days)
        .bind(data.sort_order)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find a subscription plan by ID.
    pub async fn find_plan_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM subscription_plans WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find a subscription plan by name.
    pub async fn find_plan_by_name<'e, E>(
        &self,
        executor: E,
        name: &str,
    ) -> Result<Option<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM subscription_plans WHERE name = $1")
            .bind(name)
            .fetch_optional(executor)
            .await
    }

    /// List all subscription plans.
    pub async fn list_plans<'e, E>(
        &self,
        executor: E,
        active_only: bool,
    ) -> Result<Vec<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if active_only {
            sqlx::query_as(
                "SELECT * FROM subscription_plans WHERE is_active = true ORDER BY sort_order, monthly_price",
            )
            .fetch_all(executor)
            .await
        } else {
            sqlx::query_as("SELECT * FROM subscription_plans ORDER BY sort_order, monthly_price")
                .fetch_all(executor)
                .await
        }
    }

    /// List public subscription plans (for display to customers).
    ///
    /// `subscription_plans` is not FORCE-bound and carries a public read
    /// policy, so the public (unauthenticated) plans endpoint may pass a
    /// plain pool executor here.
    pub async fn list_public_plans<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM subscription_plans WHERE is_active = true AND is_public = true ORDER BY sort_order, monthly_price",
        )
        .fetch_all(executor)
        .await
    }

    /// Update a subscription plan.
    pub async fn update_plan<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateSubscriptionPlan,
    ) -> Result<SubscriptionPlan, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE subscription_plans SET
                display_name = COALESCE($2, display_name),
                description = COALESCE($3, description),
                monthly_price = COALESCE($4, monthly_price),
                annual_price = COALESCE($5, annual_price),
                currency = COALESCE($6, currency),
                max_buildings = COALESCE($7, max_buildings),
                max_units = COALESCE($8, max_units),
                max_users = COALESCE($9, max_users),
                max_storage_gb = COALESCE($10, max_storage_gb),
                features = COALESCE($11, features),
                is_active = COALESCE($12, is_active),
                is_public = COALESCE($13, is_public),
                trial_days = COALESCE($14, trial_days),
                sort_order = COALESCE($15, sort_order),
                metadata = COALESCE($16, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.display_name)
        .bind(&data.description)
        .bind(data.monthly_price)
        .bind(data.annual_price)
        .bind(&data.currency)
        .bind(data.max_buildings)
        .bind(data.max_units)
        .bind(data.max_users)
        .bind(data.max_storage_gb)
        .bind(&data.features)
        .bind(data.is_active)
        .bind(data.is_public)
        .bind(data.trial_days)
        .bind(data.sort_order)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Delete a subscription plan.
    pub async fn delete_plan<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM subscription_plans WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ==================== Organization Subscriptions CRUD ====================

    /// Create a new organization subscription.
    ///
    /// Multi-statement (plan lookup for trial-days, then insert), so it takes
    /// `&mut PgConnection` and runs both statements on the same context-set
    /// connection.
    pub async fn create_subscription(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        data: CreateOrganizationSubscription,
    ) -> Result<OrganizationSubscription, sqlx::Error> {
        // Get plan for trial days calculation.
        let plan = self.find_plan_by_id(&mut *conn, data.plan_id).await?;

        let billing_cycle = data.billing_cycle.unwrap_or_else(|| "monthly".to_string());
        let now = Utc::now();

        // Calculate period end based on billing cycle using calendar months/years
        let period_end = if billing_cycle == "annual" {
            now.checked_add_months(Months::new(12)).unwrap_or(now)
        } else {
            now.checked_add_months(Months::new(1)).unwrap_or(now)
        };

        // Calculate trial dates if starting trial
        let (trial_start, trial_end, is_trial, status) = if data.start_trial.unwrap_or(false)
            && plan.as_ref().is_some_and(|p| p.trial_days.unwrap_or(0) > 0)
        {
            let trial_days = plan
                .as_ref()
                .map(|p| p.trial_days.unwrap_or(14))
                .unwrap_or(14);
            let trial_end = now
                .checked_add_days(Days::new(trial_days as u64))
                .unwrap_or(now);
            (Some(now), Some(trial_end), Some(true), "trialing")
        } else {
            (None, None, Some(false), "active")
        };

        sqlx::query_as(
            r#"
            INSERT INTO organization_subscriptions
                (organization_id, plan_id, status, billing_cycle, current_period_start,
                 current_period_end, trial_start, trial_end, is_trial, payment_method_id, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.plan_id)
        .bind(status)
        .bind(&billing_cycle)
        .bind(now)
        .bind(period_end)
        .bind(trial_start)
        .bind(trial_end)
        .bind(is_trial)
        .bind(data.payment_method_id)
        .bind(&data.metadata)
        .fetch_one(&mut *conn)
        .await
    }

    /// Find an organization's subscription.
    pub async fn find_subscription_by_org<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Option<OrganizationSubscription>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM organization_subscriptions WHERE organization_id = $1 AND status NOT IN ('cancelled', 'expired') ORDER BY created_at DESC LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Find a subscription by ID.
    pub async fn find_subscription_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<OrganizationSubscription>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM organization_subscriptions WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Get subscription with plan details.
    pub async fn get_subscription_with_plan<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Option<SubscriptionWithPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                s.id, s.organization_id, s.status, s.billing_cycle,
                s.current_period_start, s.current_period_end, s.is_trial, s.cancel_at_period_end,
                p.id as plan_id, p.name as plan_name, p.display_name as plan_display_name,
                p.monthly_price, p.annual_price
            FROM organization_subscriptions s
            JOIN subscription_plans p ON p.id = s.plan_id
            WHERE s.organization_id = $1 AND s.status NOT IN ('cancelled', 'expired')
            ORDER BY s.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Update an organization subscription.
    pub async fn update_subscription<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateOrganizationSubscription,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                billing_cycle = COALESCE($2, billing_cycle),
                payment_method_id = COALESCE($3, payment_method_id),
                metadata = COALESCE($4, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.billing_cycle)
        .bind(data.payment_method_id)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Change subscription plan.
    pub async fn change_plan<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: ChangePlanRequest,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                plan_id = $2,
                billing_cycle = COALESCE($3, billing_cycle),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.new_plan_id)
        .bind(&data.billing_cycle)
        .fetch_one(executor)
        .await
    }

    /// Cancel a subscription.
    pub async fn cancel_subscription<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: CancelSubscriptionRequest,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let cancel_at_period_end = data.cancel_at_period_end.unwrap_or(true);
        let status = if cancel_at_period_end {
            "active"
        } else {
            "cancelled"
        };

        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                status = $2,
                cancel_at_period_end = $3,
                cancelled_at = NOW(),
                cancellation_reason = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(cancel_at_period_end)
        .bind(&data.cancellation_reason)
        .fetch_one(executor)
        .await
    }

    /// Reactivate a cancelled subscription.
    pub async fn reactivate_subscription<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                status = 'active',
                cancel_at_period_end = false,
                cancelled_at = NULL,
                cancellation_reason = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await
    }

    /// List all subscriptions (platform admin).
    ///
    /// Note: under FORCE RLS the result is scoped to the caller's RLS
    /// context — the rows visible are those of the org set on the
    /// connection.
    pub async fn list_all_subscriptions<'e, E>(
        &self,
        executor: E,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<SubscriptionWithPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                s.id, s.organization_id, s.status, s.billing_cycle,
                s.current_period_start, s.current_period_end, s.is_trial, s.cancel_at_period_end,
                p.id as plan_id, p.name as plan_name, p.display_name as plan_display_name,
                p.monthly_price, p.annual_price
            FROM organization_subscriptions s
            JOIN subscription_plans p ON p.id = s.plan_id
            WHERE ($1::text IS NULL OR s.status = $1)
            ORDER BY s.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    // ==================== Payment Methods CRUD ====================

    /// Create a payment method.
    pub async fn create_payment_method<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: CreateSubscriptionPaymentMethod,
    ) -> Result<SubscriptionPaymentMethod, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO payment_methods
                (organization_id, method_type, stripe_payment_method_id, is_default,
                 billing_address, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&data.method_type)
        .bind(&data.stripe_payment_method_id)
        .bind(data.is_default.unwrap_or(false))
        .bind(&data.billing_address)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// List payment methods for an organization.
    pub async fn list_payment_methods<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<SubscriptionPaymentMethod>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM payment_methods WHERE organization_id = $1 ORDER BY is_default DESC, created_at DESC",
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Set default payment method.
    ///
    /// Uses a transaction (on the caller's context-set connection) to ensure
    /// atomicity — prevents race conditions where concurrent requests could
    /// result in multiple default payment methods. RLS context is set via
    /// session-level GUCs, so it survives `BEGIN`/`COMMIT`.
    pub async fn set_default_payment_method(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        payment_method_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let mut tx = conn.begin().await?;

        // Reset all to non-default
        sqlx::query("UPDATE payment_methods SET is_default = false WHERE organization_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;

        // Set the specified one as default
        sqlx::query(
            "UPDATE payment_methods SET is_default = true WHERE id = $1 AND organization_id = $2",
        )
        .bind(payment_method_id)
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Delete a payment method.
    pub async fn delete_payment_method<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM payment_methods WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected() > 0)
    }

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

    // ==================== Usage Records ====================

    /// Record a usage metric.
    pub async fn record_usage<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        subscription_id: Option<Uuid>,
        data: CreateUsageRecord,
    ) -> Result<UsageRecord, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO usage_records
                (organization_id, subscription_id, metric_type, quantity, unit, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(subscription_id)
        .bind(&data.metric_type)
        .bind(data.quantity)
        .bind(&data.unit)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Get usage summary for an organization.
    pub async fn get_usage_summary<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        period_start: chrono::DateTime<Utc>,
        period_end: chrono::DateTime<Utc>,
    ) -> Result<Vec<UsageSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                metric_type,
                SUM(quantity) as total_quantity,
                MAX(unit) as unit,
                COUNT(*) as record_count
            FROM usage_records
            WHERE organization_id = $1
            AND recorded_at >= $2 AND recorded_at < $3
            GROUP BY metric_type
            "#,
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(executor)
        .await
    }

    /// Get current usage counts for an organization.
    pub async fn get_current_usage<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<(i64, i64, i64, i64), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Combined query to get all counts in one round-trip
        let counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM buildings WHERE organization_id = $1) as building_count,
                (SELECT COUNT(*) FROM units u JOIN buildings b ON u.building_id = b.id WHERE b.organization_id = $1) as unit_count,
                (SELECT COUNT(*) FROM organization_members WHERE organization_id = $1) as user_count
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await?;

        // Storage would require summing document sizes - using 0 as placeholder
        let storage = 0i64;

        Ok((counts.0, counts.1, counts.2, storage))
    }

    // ==================== Subscription Events ====================

    /// Log a subscription event.
    pub async fn log_event<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        subscription_id: Option<Uuid>,
        actor_id: Option<Uuid>,
        data: CreateSubscriptionEvent,
    ) -> Result<SubscriptionEvent, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO subscription_events
                (organization_id, subscription_id, event_type, description, actor_id,
                 previous_data, new_data, webhook_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(subscription_id)
        .bind(&data.event_type)
        .bind(&data.description)
        .bind(actor_id)
        .bind(&data.previous_data)
        .bind(&data.new_data)
        .bind(&data.webhook_id)
        .fetch_one(executor)
        .await
    }

    /// Get subscription events.
    pub async fn get_events<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        limit: i32,
    ) -> Result<Vec<SubscriptionEvent>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM subscription_events WHERE organization_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(org_id)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    // ==================== Coupons ====================

    /// Create a coupon.
    pub async fn create_coupon<'e, E>(
        &self,
        executor: E,
        data: CreateSubscriptionCoupon,
    ) -> Result<SubscriptionCoupon, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO subscription_coupons
                (code, name, description, discount_type, discount_value, currency, duration,
                 duration_months, max_redemptions, valid_from, valid_until, applicable_plans,
                 min_amount, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(&data.code)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.discount_type)
        .bind(data.discount_value)
        .bind(&data.currency)
        .bind(data.duration.unwrap_or_else(|| "once".to_string()))
        .bind(data.duration_months)
        .bind(data.max_redemptions)
        .bind(data.valid_from)
        .bind(data.valid_until)
        .bind(&data.applicable_plans)
        .bind(data.min_amount)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find a coupon by code.
    pub async fn find_coupon_by_code<'e, E>(
        &self,
        executor: E,
        code: &str,
    ) -> Result<Option<SubscriptionCoupon>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM subscription_coupons WHERE code = $1 AND is_active = true")
            .bind(code)
            .fetch_optional(executor)
            .await
    }

    /// List all coupons.
    pub async fn list_coupons<'e, E>(
        &self,
        executor: E,
        active_only: bool,
    ) -> Result<Vec<SubscriptionCoupon>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if active_only {
            sqlx::query_as(
                "SELECT * FROM subscription_coupons WHERE is_active = true ORDER BY created_at DESC",
            )
            .fetch_all(executor)
            .await
        } else {
            sqlx::query_as("SELECT * FROM subscription_coupons ORDER BY created_at DESC")
                .fetch_all(executor)
                .await
        }
    }

    /// Update a coupon.
    pub async fn update_coupon<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateSubscriptionCoupon,
    ) -> Result<SubscriptionCoupon, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE subscription_coupons SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                max_redemptions = COALESCE($4, max_redemptions),
                valid_from = COALESCE($5, valid_from),
                valid_until = COALESCE($6, valid_until),
                applicable_plans = COALESCE($7, applicable_plans),
                min_amount = COALESCE($8, min_amount),
                is_active = COALESCE($9, is_active),
                metadata = COALESCE($10, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.max_redemptions)
        .bind(data.valid_from)
        .bind(data.valid_until)
        .bind(&data.applicable_plans)
        .bind(data.min_amount)
        .bind(data.is_active)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Redeem a coupon.
    ///
    /// Uses a transaction (on the caller's context-set connection) with
    /// validation to prevent race conditions and over-redemption. Checks
    /// `max_redemptions` before incrementing the count and inserting the
    /// redemption record. The `coupon_redemptions` insert is FORCE-RLS-bound,
    /// so this MUST run on a connection with RLS context set for `org_id`.
    pub async fn redeem_coupon(
        &self,
        conn: &mut PgConnection,
        coupon_id: Uuid,
        org_id: Uuid,
        subscription_id: Option<Uuid>,
        user_id: Uuid,
    ) -> Result<CouponRedemption, sqlx::Error> {
        let mut tx = conn.begin().await?;

        // Check if coupon exists and has remaining redemptions (with row lock)
        let coupon: Option<(i32, Option<i32>)> = sqlx::query_as(
            "SELECT COALESCE(redemption_count, 0), max_redemptions FROM subscription_coupons WHERE id = $1 FOR UPDATE",
        )
        .bind(coupon_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((current_count, max_redemptions)) = coupon {
            if let Some(max) = max_redemptions {
                if current_count >= max {
                    tx.rollback().await?;
                    return Err(sqlx::Error::RowNotFound); // Coupon exhausted
                }
            }
        } else {
            tx.rollback().await?;
            return Err(sqlx::Error::RowNotFound); // Coupon not found
        }

        // Insert redemption record first (this validates FK constraints)
        let redemption: CouponRedemption = sqlx::query_as(
            r#"
            INSERT INTO coupon_redemptions
                (coupon_id, organization_id, subscription_id, redeemed_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(coupon_id)
        .bind(org_id)
        .bind(subscription_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        // Only increment count after successful redemption insert
        sqlx::query(
            "UPDATE subscription_coupons SET redemption_count = COALESCE(redemption_count, 0) + 1 WHERE id = $1",
        )
        .bind(coupon_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(redemption)
    }

    // ==================== Statistics ====================

    /// Get subscription statistics.
    ///
    /// Multi-statement (aggregate stats, then per-plan counts), so it takes
    /// `&mut PgConnection` and runs both queries on the same context-set
    /// connection. Under FORCE RLS the figures are scoped to the caller's
    /// RLS context.
    pub async fn get_statistics(
        &self,
        conn: &mut PgConnection,
    ) -> Result<SubscriptionStatistics, sqlx::Error> {
        // Combined query to get all stats in one round-trip
        let stats: (i64, i64, i64, i64, Option<Decimal>) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM organization_subscriptions) as total,
                (SELECT COUNT(*) FROM organization_subscriptions WHERE status = 'active') as active,
                (SELECT COUNT(*) FROM organization_subscriptions WHERE status = 'trialing') as trial,
                (SELECT COUNT(*) FROM organization_subscriptions WHERE status = 'cancelled') as cancelled,
                (SELECT SUM(
                    CASE WHEN s.billing_cycle = 'annual'
                        THEN p.annual_price / 12
                        ELSE p.monthly_price
                    END
                )
                FROM organization_subscriptions s
                JOIN subscription_plans p ON p.id = s.plan_id
                WHERE s.status = 'active') as mrr
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;

        let monthly_recurring_revenue = stats.4.unwrap_or(Decimal::ZERO);
        let annual_recurring_revenue = monthly_recurring_revenue * Decimal::from(12);

        // Get counts by plan - this requires a separate query
        let by_plan: Vec<PlanSubscriptionCount> = sqlx::query_as(
            r#"
            SELECT p.id as plan_id, p.name as plan_name, COUNT(s.id) as count
            FROM subscription_plans p
            LEFT JOIN organization_subscriptions s ON s.plan_id = p.id AND s.status = 'active'
            GROUP BY p.id, p.name
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok(SubscriptionStatistics {
            total_subscriptions: stats.0,
            active_subscriptions: stats.1,
            trial_subscriptions: stats.2,
            cancelled_subscriptions: stats.3,
            monthly_recurring_revenue,
            annual_recurring_revenue,
            by_plan,
        })
    }
}

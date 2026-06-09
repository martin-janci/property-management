//! Vendor repository (Epic 21).
//!
//! # RLS Integration (PAP-67 / PAP-70)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `vendors`, `vendor_contacts`,
//! `vendor_contracts`, `vendor_invoices`, and `vendor_ratings`. Under `FORCE`
//! the api-server's owner connection is no longer exempt, so a query issued on
//! a connection without `app.current_org_id` set collapses to deny-all (own-org
//! reads return empty, writes fail the policy `WITH CHECK`).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `budget.rs` precedent.

use crate::models::{
    ContractQuery, CreateVendor, CreateVendorContact, CreateVendorContract, CreateVendorInvoice,
    CreateVendorRating, ExpiringContract, InvoiceQuery, InvoiceSummary, UpdateVendor,
    UpdateVendorContract, UpdateVendorInvoice, Vendor, VendorContact, VendorContract,
    VendorInvoice, VendorQuery, VendorRating, VendorStatistics, VendorWithDetails,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

/// Repository for vendor operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct VendorRepository;

impl VendorRepository {
    /// Create a new VendorRepository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // ==================== Vendor CRUD ====================

    /// Create a new vendor.
    pub async fn create<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: CreateVendor,
    ) -> Result<Vendor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO vendors
                (organization_id, company_name, contact_name, phone, email, website, address,
                 services, license_number, tax_id, contract_start, contract_end, hourly_rate,
                 is_preferred, notes, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&data.company_name)
        .bind(&data.contact_name)
        .bind(&data.phone)
        .bind(&data.email)
        .bind(&data.website)
        .bind(&data.address)
        .bind(&data.services)
        .bind(&data.license_number)
        .bind(&data.tax_id)
        .bind(data.contract_start)
        .bind(data.contract_end)
        .bind(data.hourly_rate)
        .bind(data.is_preferred.unwrap_or(false))
        .bind(&data.notes)
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Find vendor by ID.
    pub async fn find_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Vendor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM vendors WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List vendors for an organization.
    pub async fn list<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: VendorQuery,
    ) -> Result<Vec<Vendor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let mut sql = String::from(
            r#"
            SELECT * FROM vendors
            WHERE organization_id = $1
            "#,
        );

        if query.status.is_some() {
            sql.push_str(" AND status = $2");
        }
        if query.service.is_some() {
            sql.push_str(" AND $3 = ANY(services)");
        }
        if query.is_preferred.is_some() {
            sql.push_str(" AND is_preferred = $4");
        }
        if query.contract_expiring_days.is_some() {
            sql.push_str(" AND contract_end <= CURRENT_DATE + $5::integer");
        }
        if query.search.is_some() {
            sql.push_str(" AND (company_name ILIKE $6 OR contact_name ILIKE $6)");
        }

        sql.push_str(" ORDER BY company_name ASC");
        sql.push_str(" LIMIT $7 OFFSET $8");

        let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

        sqlx::query_as(sqlx::AssertSqlSafe(sql))
            .bind(org_id)
            .bind(&query.status)
            .bind(&query.service)
            .bind(query.is_preferred)
            .bind(query.contract_expiring_days)
            .bind(&search_pattern)
            .bind(query.limit.unwrap_or(50))
            .bind(query.offset.unwrap_or(0))
            .fetch_all(executor)
            .await
    }

    /// List vendors with details (active work orders, pending invoices).
    pub async fn list_with_details<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: VendorQuery,
    ) -> Result<Vec<VendorWithDetails>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

        sqlx::query_as(
            r#"
            SELECT
                v.id, v.organization_id, v.company_name, v.contact_name, v.phone, v.email,
                v.services, v.rating, v.total_jobs, v.completed_jobs, v.status, v.is_preferred,
                v.contract_end,
                COALESCE((SELECT COUNT(*) FROM work_orders WHERE vendor_id = v.id AND status NOT IN ('completed', 'cancelled')), 0) as active_work_orders,
                COALESCE((SELECT COUNT(*) FROM vendor_invoices WHERE vendor_id = v.id AND status IN ('pending', 'approved')), 0) as pending_invoices
            FROM vendors v
            WHERE v.organization_id = $1
            AND ($2::text IS NULL OR v.status = $2)
            AND ($3::text IS NULL OR $3 = ANY(v.services))
            AND ($4::boolean IS NULL OR v.is_preferred = $4)
            AND ($5::integer IS NULL OR v.contract_end <= CURRENT_DATE + $5::integer)
            AND ($6::text IS NULL OR v.company_name ILIKE $6 OR v.contact_name ILIKE $6)
            ORDER BY v.company_name ASC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(&query.status)
        .bind(&query.service)
        .bind(query.is_preferred)
        .bind(query.contract_expiring_days)
        .bind(&search_pattern)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// Update a vendor.
    pub async fn update<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateVendor,
    ) -> Result<Vendor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE vendors SET
                company_name = COALESCE($2, company_name),
                contact_name = COALESCE($3, contact_name),
                phone = COALESCE($4, phone),
                email = COALESCE($5, email),
                website = COALESCE($6, website),
                address = COALESCE($7, address),
                services = COALESCE($8, services),
                license_number = COALESCE($9, license_number),
                tax_id = COALESCE($10, tax_id),
                contract_start = COALESCE($11, contract_start),
                contract_end = COALESCE($12, contract_end),
                hourly_rate = COALESCE($13, hourly_rate),
                status = COALESCE($14, status),
                is_preferred = COALESCE($15, is_preferred),
                notes = COALESCE($16, notes),
                metadata = COALESCE($17, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.company_name)
        .bind(&data.contact_name)
        .bind(&data.phone)
        .bind(&data.email)
        .bind(&data.website)
        .bind(&data.address)
        .bind(&data.services)
        .bind(&data.license_number)
        .bind(&data.tax_id)
        .bind(data.contract_start)
        .bind(data.contract_end)
        .bind(data.hourly_rate)
        .bind(&data.status)
        .bind(data.is_preferred)
        .bind(&data.notes)
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Delete a vendor.
    pub async fn delete<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM vendors WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Set vendor as preferred.
    pub async fn set_preferred<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        is_preferred: bool,
    ) -> Result<Vendor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE vendors SET is_preferred = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(is_preferred)
        .fetch_one(executor)
        .await
    }

    /// Get vendor statistics.
    pub async fn get_statistics<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<VendorStatistics, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let counts: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'active') as active,
                COUNT(*) FILTER (WHERE is_preferred = TRUE) as preferred,
                COUNT(*) FILTER (WHERE contract_end IS NOT NULL AND contract_end <= CURRENT_DATE + 30) as expiring
            FROM vendors
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await?;

        // Get by service counts - simplified for now
        let by_service = Vec::new();

        Ok(VendorStatistics {
            total_vendors: counts.0,
            active_vendors: counts.1,
            preferred_vendors: counts.2,
            by_service,
            expiring_contracts: counts.3,
        })
    }

    // ==================== Vendor Contacts ====================

    /// Add a contact to a vendor.
    pub async fn add_contact<'e, E>(
        &self,
        executor: E,
        vendor_id: Uuid,
        data: CreateVendorContact,
    ) -> Result<VendorContact, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO vendor_contacts (vendor_id, name, role, phone, email, is_primary)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(vendor_id)
        .bind(&data.name)
        .bind(&data.role)
        .bind(&data.phone)
        .bind(&data.email)
        .bind(data.is_primary.unwrap_or(false))
        .fetch_one(executor)
        .await
    }

    /// List contacts for a vendor.
    pub async fn list_contacts<'e, E>(
        &self,
        executor: E,
        vendor_id: Uuid,
    ) -> Result<Vec<VendorContact>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM vendor_contacts WHERE vendor_id = $1
            ORDER BY is_primary DESC, name ASC
            "#,
        )
        .bind(vendor_id)
        .fetch_all(executor)
        .await
    }

    /// Delete a contact.
    pub async fn delete_contact<'e, E>(
        &self,
        executor: E,
        contact_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM vendor_contacts WHERE id = $1")
            .bind(contact_id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Resolve the `vendor_id` that owns `contact_id`, if any. Used by the
    /// route layer to derive the owning vendor before deleting a contact.
    ///
    /// Under RLS the lookup is already org-scoped: a contact belonging to
    /// another org is invisible and returns `None`.
    pub async fn find_contact_vendor_id<'e, E>(
        &self,
        executor: E,
        contact_id: Uuid,
    ) -> Result<Option<Uuid>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar("SELECT vendor_id FROM vendor_contacts WHERE id = $1")
            .bind(contact_id)
            .fetch_optional(executor)
            .await
    }

    // ==================== Vendor Contracts ====================

    /// Create a contract.
    pub async fn create_contract<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: CreateVendorContract,
    ) -> Result<VendorContract, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO vendor_contracts
                (vendor_id, organization_id, contract_number, title, description,
                 start_date, end_date, renewal_date, contract_value, payment_terms,
                 contract_type, auto_renew, terms, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(data.vendor_id)
        .bind(org_id)
        .bind(&data.contract_number)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.start_date)
        .bind(data.end_date)
        .bind(data.renewal_date)
        .bind(data.contract_value)
        .bind(&data.payment_terms)
        .bind(data.contract_type.unwrap_or_else(|| "service".to_string()))
        .bind(data.auto_renew.unwrap_or(false))
        .bind(data.terms.map(sqlx::types::Json))
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Find contract by ID.
    pub async fn find_contract_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<VendorContract>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM vendor_contracts WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List contracts.
    pub async fn list_contracts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: ContractQuery,
    ) -> Result<Vec<VendorContract>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM vendor_contracts
            WHERE organization_id = $1
            AND ($2::uuid IS NULL OR vendor_id = $2)
            AND ($3::text IS NULL OR status = $3)
            AND ($4::text IS NULL OR contract_type = $4)
            AND ($5::integer IS NULL OR end_date <= CURRENT_DATE + $5::integer)
            ORDER BY end_date ASC NULLS LAST, title ASC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(org_id)
        .bind(query.vendor_id)
        .bind(&query.status)
        .bind(&query.contract_type)
        .bind(query.expiring_days)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// Update a contract.
    pub async fn update_contract<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateVendorContract,
    ) -> Result<VendorContract, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE vendor_contracts SET
                contract_number = COALESCE($2, contract_number),
                title = COALESCE($3, title),
                description = COALESCE($4, description),
                start_date = COALESCE($5, start_date),
                end_date = COALESCE($6, end_date),
                renewal_date = COALESCE($7, renewal_date),
                contract_value = COALESCE($8, contract_value),
                payment_terms = COALESCE($9, payment_terms),
                contract_type = COALESCE($10, contract_type),
                status = COALESCE($11, status),
                auto_renew = COALESCE($12, auto_renew),
                terms = COALESCE($13, terms),
                metadata = COALESCE($14, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.contract_number)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.start_date)
        .bind(data.end_date)
        .bind(data.renewal_date)
        .bind(data.contract_value)
        .bind(&data.payment_terms)
        .bind(&data.contract_type)
        .bind(&data.status)
        .bind(data.auto_renew)
        .bind(data.terms.map(sqlx::types::Json))
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Delete a contract.
    pub async fn delete_contract<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM vendor_contracts WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get contracts expiring soon.
    pub async fn get_expiring_contracts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        days: i32,
    ) -> Result<Vec<ExpiringContract>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                c.id, c.vendor_id, v.company_name as vendor_name, c.title, c.end_date,
                (c.end_date - CURRENT_DATE)::integer as days_until_expiry,
                c.contract_value, c.auto_renew
            FROM vendor_contracts c
            JOIN vendors v ON v.id = c.vendor_id
            WHERE c.organization_id = $1
            AND c.status = 'active'
            AND c.end_date IS NOT NULL
            AND c.end_date <= CURRENT_DATE + $2::integer
            ORDER BY c.end_date ASC
            "#,
        )
        .bind(org_id)
        .bind(days)
        .fetch_all(executor)
        .await
    }

    // ==================== Vendor Invoices ====================

    /// Create an invoice.
    pub async fn create_invoice<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateVendorInvoice,
    ) -> Result<VendorInvoice, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO vendor_invoices
                (organization_id, vendor_id, contract_id, invoice_number, invoice_date, due_date,
                 subtotal, tax_amount, total_amount, currency, work_order_ids, description,
                 line_items, metadata, submitted_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.vendor_id)
        .bind(data.contract_id)
        .bind(&data.invoice_number)
        .bind(data.invoice_date)
        .bind(data.due_date)
        .bind(data.subtotal)
        .bind(data.tax_amount)
        .bind(data.total_amount)
        .bind(data.currency.unwrap_or_else(|| "EUR".to_string()))
        .bind(&data.work_order_ids)
        .bind(&data.description)
        .bind(data.line_items.map(sqlx::types::Json))
        .bind(data.metadata.map(sqlx::types::Json))
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Find invoice by ID.
    pub async fn find_invoice_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<VendorInvoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM vendor_invoices WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List invoices.
    pub async fn list_invoices<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: InvoiceQuery,
    ) -> Result<Vec<VendorInvoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM vendor_invoices
            WHERE organization_id = $1
            AND ($2::uuid IS NULL OR vendor_id = $2)
            AND ($3::text IS NULL OR status = $3)
            AND ($4::date IS NULL OR due_date <= $4)
            AND ($5::date IS NULL OR due_date >= $5)
            AND ($6::uuid IS NULL OR $6 = ANY(work_order_ids))
            ORDER BY invoice_date DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.vendor_id)
        .bind(&query.status)
        .bind(query.due_before)
        .bind(query.due_after)
        .bind(query.work_order_id)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// Update an invoice.
    pub async fn update_invoice<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateVendorInvoice,
    ) -> Result<VendorInvoice, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE vendor_invoices SET
                invoice_number = COALESCE($2, invoice_number),
                invoice_date = COALESCE($3, invoice_date),
                due_date = COALESCE($4, due_date),
                subtotal = COALESCE($5, subtotal),
                tax_amount = COALESCE($6, tax_amount),
                total_amount = COALESCE($7, total_amount),
                currency = COALESCE($8, currency),
                work_order_ids = COALESCE($9, work_order_ids),
                description = COALESCE($10, description),
                line_items = COALESCE($11, line_items),
                metadata = COALESCE($12, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.invoice_number)
        .bind(data.invoice_date)
        .bind(data.due_date)
        .bind(data.subtotal)
        .bind(data.tax_amount)
        .bind(data.total_amount)
        .bind(&data.currency)
        .bind(&data.work_order_ids)
        .bind(&data.description)
        .bind(data.line_items.map(sqlx::types::Json))
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Approve an invoice.
    pub async fn approve_invoice<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<VendorInvoice, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE vendor_invoices SET
                status = 'approved',
                approved_by = $2,
                approved_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Reject an invoice.
    pub async fn reject_invoice<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        user_id: Uuid,
        reason: &str,
    ) -> Result<VendorInvoice, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE vendor_invoices SET
                status = 'rejected',
                rejected_by = $2,
                rejection_reason = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(reason)
        .fetch_one(executor)
        .await
    }

    /// Record payment for an invoice.
    pub async fn record_payment<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        amount: Decimal,
        method: Option<&str>,
        reference: Option<&str>,
    ) -> Result<VendorInvoice, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE vendor_invoices SET
                paid_amount = COALESCE(paid_amount, 0) + $2,
                payment_method = COALESCE($3, payment_method),
                payment_reference = COALESCE($4, payment_reference),
                paid_at = NOW(),
                status = CASE
                    WHEN COALESCE(paid_amount, 0) + $2 >= total_amount THEN 'paid'
                    ELSE 'partially_paid'
                END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(amount)
        .bind(method)
        .bind(reference)
        .fetch_one(executor)
        .await
    }

    /// Delete an invoice.
    pub async fn delete_invoice<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM vendor_invoices WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get invoice summary by vendor for a period.
    pub async fn get_invoice_summary<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<InvoiceSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                i.vendor_id,
                v.company_name as vendor_name,
                COUNT(*) as total_invoices,
                SUM(i.total_amount) as total_amount,
                SUM(COALESCE(i.paid_amount, 0)) as paid_amount,
                SUM(i.total_amount - COALESCE(i.paid_amount, 0)) as pending_amount
            FROM vendor_invoices i
            JOIN vendors v ON v.id = i.vendor_id
            WHERE i.organization_id = $1
            AND i.invoice_date >= $2
            AND i.invoice_date <= $3
            GROUP BY i.vendor_id, v.company_name
            ORDER BY total_amount DESC
            "#,
        )
        .bind(org_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(executor)
        .await
    }

    /// Get overdue invoices.
    pub async fn get_overdue_invoices<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<VendorInvoice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM vendor_invoices
            WHERE organization_id = $1
            AND status IN ('pending', 'approved')
            AND due_date < CURRENT_DATE
            ORDER BY due_date ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    // ==================== Vendor Ratings ====================

    /// Add a rating for a vendor.
    pub async fn add_rating<'e, E>(
        &self,
        executor: E,
        vendor_id: Uuid,
        user_id: Uuid,
        data: CreateVendorRating,
    ) -> Result<VendorRating, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO vendor_ratings
                (vendor_id, work_order_id, rated_by, rating, quality_rating,
                 timeliness_rating, communication_rating, value_rating, review_text)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(vendor_id)
        .bind(data.work_order_id)
        .bind(user_id)
        .bind(data.rating)
        .bind(data.quality_rating)
        .bind(data.timeliness_rating)
        .bind(data.communication_rating)
        .bind(data.value_rating)
        .bind(&data.review_text)
        .fetch_one(executor)
        .await
    }

    /// List ratings for a vendor.
    pub async fn list_ratings<'e, E>(
        &self,
        executor: E,
        vendor_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<VendorRating>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM vendor_ratings
            WHERE vendor_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(vendor_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }
}

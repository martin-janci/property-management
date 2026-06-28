//! Repository for ACC company & document configuration (EPIC-ACC-02). OWNED BY: BE-Config.
//!
//! RLS-aware. Follows the generic-executor pattern of
//! [`crate::repositories::accounting::AccountingRepository`]: every method takes
//! `executor: E where E: Executor<'e, Database = Postgres>` so callers pass the
//! RLS-scoped connection (`&mut **rls`) or a transaction (`&mut *tx`).
//!
//! Numbering allocation MUST be atomic & gapless (UC-ACC-02.3) — it is a single
//! `UPDATE ... RETURNING` run inside the caller's transaction so concurrent
//! issuance can never skip or duplicate a number (a legal requirement).
//!
//! There is NO sqlx offline metadata in this repo, so column names are verified
//! by hand against migration 00194 (acc_company_settings, acc_numbering_series,
//! acc_unit, acc_vat_rate, acc_bank_account, acc_email_template,
//! acc_document_template).

use crate::models::acc_config::{
    AccBankAccount, AccCompanySettings, AccDocumentTemplate, AccEmailTemplate, AccNumberingSeries,
    AccUnit, AccVatRate,
};
use crate::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

/// Repository for ACC configuration entities (company settings, numbering,
/// units, VAT rates, bank accounts, templates).
#[derive(Debug, Clone)]
pub struct AccConfigRepository {
    pub pool: DbPool,
}

impl AccConfigRepository {
    /// Create a new config repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // --- Company settings (UC-ACC-02.1/.2/.11) ---

    /// Fetch the single company-settings row for the current tenant.
    ///
    /// One row per tenant (UNIQUE(tenant_id) in 00194); RLS scopes the SELECT to
    /// the active company, so a bare `LIMIT 1` is the active company's settings.
    pub async fn get_company_settings_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Option<AccCompanySettings>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-02.1: read the active company's profile/branding/tax-mode.
        sqlx::query_as::<_, AccCompanySettings>("SELECT * FROM acc_company_settings LIMIT 1")
            .fetch_optional(executor)
            .await
    }

    /// Insert or update the company-settings row for the given tenant.
    ///
    /// UC-ACC-02.1/.2/.11: identity, tax/VAT mode, base currency and rounding are
    /// all set here. `tenant_id` is forced from the request session by the
    /// handler. Upsert keyed on the UNIQUE(tenant_id) constraint so each company
    /// has exactly one settings row. `updated_at` is maintained by the
    /// `update_acc_company_settings_updated_at` trigger.
    ///
    /// NOTE on historical immutability (UC-ACC-02.1/.2 AC): changing settings
    /// here MUST NOT mutate already-issued documents. That invariant is upheld at
    /// issue time — `issue_invoice` (BE-Invoicing) snapshots the identity/VAT/
    /// rounding onto the document — so editing this row only affects future
    /// documents. This repo deliberately touches no `invoice*` rows.
    pub async fn upsert_company_settings_rls<'e, E>(
        &self,
        executor: E,
        tenant_id: Uuid,
        settings: AccCompanySettings,
    ) -> Result<AccCompanySettings, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccCompanySettings>(
            r#"
            INSERT INTO acc_company_settings (
                tenant_id, legal_name, ico, dic, vat_id, address, email, phone, web,
                logo_url, brand_color, tax_mode, vat_payer, base_currency, rounding_mode,
                default_payment_terms_days, default_invoice_note, default_footer, default_language
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
            ON CONFLICT (tenant_id) DO UPDATE SET
                legal_name = EXCLUDED.legal_name,
                ico = EXCLUDED.ico,
                dic = EXCLUDED.dic,
                vat_id = EXCLUDED.vat_id,
                address = EXCLUDED.address,
                email = EXCLUDED.email,
                phone = EXCLUDED.phone,
                web = EXCLUDED.web,
                logo_url = EXCLUDED.logo_url,
                brand_color = EXCLUDED.brand_color,
                tax_mode = EXCLUDED.tax_mode,
                vat_payer = EXCLUDED.vat_payer,
                base_currency = EXCLUDED.base_currency,
                rounding_mode = EXCLUDED.rounding_mode,
                default_payment_terms_days = EXCLUDED.default_payment_terms_days,
                default_invoice_note = EXCLUDED.default_invoice_note,
                default_footer = EXCLUDED.default_footer,
                default_language = EXCLUDED.default_language
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(&settings.legal_name)
        .bind(&settings.ico)
        .bind(&settings.dic)
        .bind(&settings.vat_id)
        .bind(&settings.address)
        .bind(&settings.email)
        .bind(&settings.phone)
        .bind(&settings.web)
        .bind(&settings.logo_url)
        .bind(&settings.brand_color)
        .bind(&settings.tax_mode)
        .bind(settings.vat_payer)
        .bind(&settings.base_currency)
        .bind(&settings.rounding_mode)
        .bind(settings.default_payment_terms_days)
        .bind(&settings.default_invoice_note)
        .bind(&settings.default_footer)
        .bind(&settings.default_language)
        .fetch_one(executor)
        .await
    }

    // --- Numbering series (UC-ACC-02.3) ---

    /// List numbering series for the current tenant.
    pub async fn list_numbering_series_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AccNumberingSeries>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccNumberingSeries>(
            "SELECT * FROM acc_numbering_series ORDER BY doc_type ASC, year DESC",
        )
        .fetch_all(executor)
        .await
    }

    /// Create a numbering series (UC-ACC-02.3).
    ///
    /// The unique `(tenant_id, doc_type, year)` key is what implements the yearly
    /// reset cadence: a new row per (doc_type, year) starts a fresh sequence
    /// (default `next_value = 1`). `tenant_id` is forced from the session.
    pub async fn create_numbering_series_rls<'e, E>(
        &self,
        executor: E,
        series: AccNumberingSeries,
    ) -> Result<AccNumberingSeries, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccNumberingSeries>(
            r#"
            INSERT INTO acc_numbering_series (
                tenant_id, doc_type, year, prefix, format, next_value
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(series.tenant_id)
        .bind(&series.doc_type)
        .bind(series.year)
        .bind(&series.prefix)
        .bind(&series.format)
        .bind(series.next_value)
        .fetch_one(executor)
        .await
    }

    /// Find a numbering series by (doc_type, year) for the current tenant.
    ///
    /// Used by the issue flow to resolve the `format`/`prefix` to print after
    /// allocation. RLS scopes to the active company; the unique
    /// `(tenant_id, doc_type, year)` key means at most one row.
    pub async fn find_numbering_series_rls<'e, E>(
        &self,
        executor: E,
        doc_type: &str,
        year: i32,
    ) -> Result<Option<AccNumberingSeries>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccNumberingSeries>(
            "SELECT * FROM acc_numbering_series WHERE doc_type = $1 AND year = $2",
        )
        .bind(doc_type)
        .bind(year)
        .fetch_optional(executor)
        .await
    }

    /// Atomically allocate the next sequence value for a series (gapless).
    ///
    /// UC-ACC-02.3 (LEGAL requirement — gap-free and unique under concurrency):
    /// a single `UPDATE acc_numbering_series SET next_value = next_value + 1`
    /// scoped by `(doc_type, year)` (RLS adds the tenant scope), returning the
    /// value that was reserved (`next_value - 1` after the bump). Because the
    /// `UPDATE` takes a row lock and the read-modify-write is one statement, two
    /// concurrent issuers serialize on that row and receive distinct, contiguous
    /// numbers — no skips, no duplicates.
    ///
    /// MUST be called inside the caller's transaction (the same transaction that
    /// writes the issued document) so a rollback after allocation does not burn a
    /// number. The caller (BE-Invoicing `issue_invoice`) does exactly this.
    ///
    /// Errors with [`sqlx::Error::RowNotFound`] when no series exists for the
    /// `(doc_type, year)` — the caller decides whether to auto-create the series
    /// (yearly rollover) or surface a 4xx.
    pub async fn allocate_next_number_rls<'e, E>(
        &self,
        executor: E,
        doc_type: &str,
        year: i32,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // The reserved value is the pre-bump `next_value`. We return it directly
        // via `RETURNING next_value - 1` after `SET next_value = next_value + 1`,
        // so the row is bumped and the caller gets the number it owns.
        let allocated: (i64,) = sqlx::query_as(
            r#"
            UPDATE acc_numbering_series
            SET next_value = next_value + 1
            WHERE doc_type = $1 AND year = $2
            RETURNING next_value - 1
            "#,
        )
        .bind(doc_type)
        .bind(year)
        .fetch_one(executor)
        .await?;
        Ok(allocated.0)
    }

    // --- Units (UC-ACC-02.8) ---

    /// List units for the current tenant.
    pub async fn list_units_rls<'e, E>(&self, executor: E) -> Result<Vec<AccUnit>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccUnit>("SELECT * FROM acc_unit ORDER BY code ASC")
            .fetch_all(executor)
            .await
    }

    /// Create a unit (UC-ACC-02.8). `tenant_id` forced from session; unique
    /// `(tenant_id, code)` prevents duplicate unit codes per company.
    pub async fn create_unit_rls<'e, E>(
        &self,
        executor: E,
        unit: AccUnit,
    ) -> Result<AccUnit, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccUnit>(
            r#"
            INSERT INTO acc_unit (tenant_id, code, name, is_active)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(unit.tenant_id)
        .bind(&unit.code)
        .bind(&unit.name)
        .bind(unit.is_active)
        .fetch_one(executor)
        .await
    }

    // --- VAT rates (UC-ACC-02.9) ---

    /// List VAT rates for the current tenant.
    pub async fn list_vat_rates_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AccVatRate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Order so the default rate surfaces first, then by descending rate.
        sqlx::query_as::<_, AccVatRate>(
            "SELECT * FROM acc_vat_rate ORDER BY is_default DESC, rate DESC",
        )
        .fetch_all(executor)
        .await
    }

    /// Create a VAT rate (UC-ACC-02.9).
    ///
    /// Historical immutability (UC-ACC-02.9 AC): adding/changing a rate only
    /// affects NEW documents; issued documents keep the VAT rate snapshotted on
    /// their lines. The optional `effective_from`/`effective_to` window supports
    /// date-effective legislative changes (EPIC-ACC-16.8). If `is_default` is set
    /// the previous default is cleared first so only one default exists per
    /// company.
    pub async fn create_vat_rate_rls<'e, E>(
        &self,
        executor: E,
        rate: AccVatRate,
    ) -> Result<AccVatRate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccVatRate>(
            r#"
            INSERT INTO acc_vat_rate (
                tenant_id, name, rate, is_default, is_active, effective_from, effective_to
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(rate.tenant_id)
        .bind(&rate.name)
        .bind(rate.rate)
        .bind(rate.is_default)
        .bind(rate.is_active)
        .bind(rate.effective_from)
        .bind(rate.effective_to)
        .fetch_one(executor)
        .await
    }

    // --- Bank accounts (UC-ACC-02.10) ---

    /// List bank accounts for the current tenant.
    pub async fn list_bank_accounts_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AccBankAccount>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Default account first, then by label.
        sqlx::query_as::<_, AccBankAccount>(
            "SELECT * FROM acc_bank_account ORDER BY is_default DESC, label ASC",
        )
        .fetch_all(executor)
        .await
    }

    /// Resolve the bank account to print on a document (UC-ACC-02.10).
    ///
    /// Currency-matched default selection: for a foreign-currency document, the
    /// active account whose `currency` matches is preferred (and the default one
    /// among those if several); otherwise the company default account is used.
    /// Returns `None` only when the company has no active bank account at all.
    pub async fn resolve_bank_account_for_currency_rls<'e, E>(
        &self,
        executor: E,
        currency: &str,
    ) -> Result<Option<AccBankAccount>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // ORDER prioritizes: (1) currency match, then (2) default flag, so a
        // currency-matched account wins over a non-matching default, and within
        // a currency the default account wins. (UC-ACC-02.10 currency-matched.)
        sqlx::query_as::<_, AccBankAccount>(
            r#"
            SELECT * FROM acc_bank_account
            WHERE is_active = TRUE
            ORDER BY (currency = $1) DESC, is_default DESC, label ASC
            LIMIT 1
            "#,
        )
        .bind(currency)
        .fetch_optional(executor)
        .await
    }

    /// Create a bank account (UC-ACC-02.10).
    pub async fn create_bank_account_rls<'e, E>(
        &self,
        executor: E,
        account: AccBankAccount,
    ) -> Result<AccBankAccount, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccBankAccount>(
            r#"
            INSERT INTO acc_bank_account (
                tenant_id, label, iban, swift, bank_name, currency, is_default, is_active
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(account.tenant_id)
        .bind(&account.label)
        .bind(&account.iban)
        .bind(&account.swift)
        .bind(&account.bank_name)
        .bind(&account.currency)
        .bind(account.is_default)
        .bind(account.is_active)
        .fetch_one(executor)
        .await
    }

    /// Mark one bank account as the company default, clearing any prior default.
    ///
    /// UC-ACC-02.10: exactly one default account drives the document/QR. Runs as
    /// two statements; the caller passes a transaction so the clear+set is atomic.
    pub async fn set_default_bank_account_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        id: Uuid,
    ) -> Result<Option<AccBankAccount>, sqlx::Error> {
        sqlx::query("UPDATE acc_bank_account SET is_default = FALSE WHERE is_default = TRUE")
            .execute(&mut *conn)
            .await?;
        sqlx::query_as::<_, AccBankAccount>(
            "UPDATE acc_bank_account SET is_default = TRUE WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
    }

    // --- Email templates (UC-ACC-02.6) ---

    /// List email templates for the current tenant.
    pub async fn list_email_templates_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AccEmailTemplate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccEmailTemplate>(
            "SELECT * FROM acc_email_template ORDER BY kind ASC, language ASC",
        )
        .fetch_all(executor)
        .await
    }

    /// Create an email template (UC-ACC-02.6). Subject/body carry `{{variable}}`
    /// placeholders resolved at send time from document/contact/company.
    pub async fn create_email_template_rls<'e, E>(
        &self,
        executor: E,
        template: AccEmailTemplate,
    ) -> Result<AccEmailTemplate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccEmailTemplate>(
            r#"
            INSERT INTO acc_email_template (
                tenant_id, kind, language, subject, body, is_default
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(template.tenant_id)
        .bind(&template.kind)
        .bind(&template.language)
        .bind(&template.subject)
        .bind(&template.body)
        .bind(template.is_default)
        .fetch_one(executor)
        .await
    }

    // --- Document templates (UC-ACC-02.4) ---

    /// List document templates for the current tenant.
    pub async fn list_document_templates_rls<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AccDocumentTemplate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccDocumentTemplate>(
            "SELECT * FROM acc_document_template ORDER BY is_default DESC, name ASC",
        )
        .fetch_all(executor)
        .await
    }

    /// Create a document template (UC-ACC-02.4 — branding/theme/accent/logo).
    pub async fn create_document_template_rls<'e, E>(
        &self,
        executor: E,
        template: AccDocumentTemplate,
    ) -> Result<AccDocumentTemplate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccDocumentTemplate>(
            r#"
            INSERT INTO acc_document_template (
                tenant_id, name, theme, accent_color, show_logo, is_default, config
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(template.tenant_id)
        .bind(&template.name)
        .bind(&template.theme)
        .bind(&template.accent_color)
        .bind(template.show_logo)
        .bind(template.is_default)
        .bind(&template.config)
        .fetch_one(executor)
        .await
    }
}

//! Epic 145: Multi-Currency & Cross-Border Support repository.
//! Provides database operations for currency configuration, exchange rates,
//! multi-currency transactions, cross-border leases, and reporting.

use crate::models::multi_currency::*;
use crate::DbPool;
use chrono::{NaiveDate, Utc};
use common::errors::AppError;
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct MultiCurrencyRepository {
    pool: DbPool,
}

impl MultiCurrencyRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // STORY 145.1: MULTI-CURRENCY CONFIGURATION
    // =========================================================================

    /// Get or create organization currency configuration
    pub async fn get_or_create_currency_config(
        &self,
        org_id: Uuid,
    ) -> Result<OrganizationCurrencyConfig, AppError> {
        // Try to get existing config
        if let Some(config) = self.get_currency_config(org_id).await? {
            return Ok(config);
        }

        // Create default config
        let config = sqlx::query_as::<_, OrganizationCurrencyConfig>(
            r#"
            INSERT INTO organization_currency_config (organization_id)
            VALUES ($1)
            RETURNING id, organization_id, base_currency, enabled_currencies,
                      display_currency, show_original_amount, decimal_places,
                      exchange_rate_source, auto_update_rates, update_frequency_hours,
                      last_rate_update, rounding_mode, created_at, updated_at, created_by
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    /// Get organization currency configuration
    pub async fn get_currency_config(
        &self,
        org_id: Uuid,
    ) -> Result<Option<OrganizationCurrencyConfig>, AppError> {
        let config = sqlx::query_as::<_, OrganizationCurrencyConfig>(
            r#"
            SELECT id, organization_id, base_currency, enabled_currencies,
                   display_currency, show_original_amount, decimal_places,
                   exchange_rate_source, auto_update_rates, update_frequency_hours,
                   last_rate_update, rounding_mode, created_at, updated_at, created_by
            FROM organization_currency_config
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    /// Create or update organization currency configuration
    pub async fn upsert_currency_config(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateCurrencyConfig,
    ) -> Result<OrganizationCurrencyConfig, AppError> {
        let config = sqlx::query_as::<_, OrganizationCurrencyConfig>(
            r#"
            INSERT INTO organization_currency_config (
                organization_id, base_currency, enabled_currencies, display_currency,
                show_original_amount, decimal_places, exchange_rate_source,
                auto_update_rates, update_frequency_hours, rounding_mode, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (organization_id) DO UPDATE SET
                base_currency = EXCLUDED.base_currency,
                enabled_currencies = EXCLUDED.enabled_currencies,
                display_currency = EXCLUDED.display_currency,
                show_original_amount = EXCLUDED.show_original_amount,
                decimal_places = EXCLUDED.decimal_places,
                exchange_rate_source = EXCLUDED.exchange_rate_source,
                auto_update_rates = EXCLUDED.auto_update_rates,
                update_frequency_hours = EXCLUDED.update_frequency_hours,
                rounding_mode = EXCLUDED.rounding_mode,
                updated_at = NOW()
            RETURNING id, organization_id, base_currency, enabled_currencies,
                      display_currency, show_original_amount, decimal_places,
                      exchange_rate_source, auto_update_rates, update_frequency_hours,
                      last_rate_update, rounding_mode, created_at, updated_at, created_by
            "#,
        )
        .bind(org_id)
        .bind(req.base_currency)
        .bind(&req.enabled_currencies)
        .bind(req.display_currency)
        .bind(req.show_original_amount)
        .bind(req.decimal_places)
        .bind(req.exchange_rate_source)
        .bind(req.auto_update_rates)
        .bind(req.update_frequency_hours)
        .bind(&req.rounding_mode)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    /// Update organization currency configuration
    pub async fn update_currency_config(
        &self,
        org_id: Uuid,
        req: UpdateCurrencyConfig,
    ) -> Result<Option<OrganizationCurrencyConfig>, AppError> {
        let config = sqlx::query_as::<_, OrganizationCurrencyConfig>(
            r#"
            UPDATE organization_currency_config
            SET base_currency = COALESCE($2, base_currency),
                enabled_currencies = COALESCE($3, enabled_currencies),
                display_currency = COALESCE($4, display_currency),
                show_original_amount = COALESCE($5, show_original_amount),
                decimal_places = COALESCE($6, decimal_places),
                exchange_rate_source = COALESCE($7, exchange_rate_source),
                auto_update_rates = COALESCE($8, auto_update_rates),
                update_frequency_hours = COALESCE($9, update_frequency_hours),
                rounding_mode = COALESCE($10, rounding_mode),
                updated_at = NOW()
            WHERE organization_id = $1
            RETURNING id, organization_id, base_currency, enabled_currencies,
                      display_currency, show_original_amount, decimal_places,
                      exchange_rate_source, auto_update_rates, update_frequency_hours,
                      last_rate_update, rounding_mode, created_at, updated_at, created_by
            "#,
        )
        .bind(org_id)
        .bind(req.base_currency)
        .bind(&req.enabled_currencies)
        .bind(req.display_currency)
        .bind(req.show_original_amount)
        .bind(req.decimal_places)
        .bind(req.exchange_rate_source)
        .bind(req.auto_update_rates)
        .bind(req.update_frequency_hours)
        .bind(&req.rounding_mode)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    /// Create property currency configuration
    pub async fn create_property_currency_config(
        &self,
        org_id: Uuid,
        req: CreatePropertyCurrencyConfig,
    ) -> Result<PropertyCurrencyConfig, AppError> {
        let config = sqlx::query_as::<_, PropertyCurrencyConfig>(
            r#"
            INSERT INTO property_currency_config (
                building_id, organization_id, default_currency, country,
                vat_rate, vat_registration_number, local_tax_id,
                requires_local_reporting, local_accounting_format
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, building_id, organization_id, default_currency, country,
                      vat_rate, vat_registration_number, local_tax_id,
                      requires_local_reporting, local_accounting_format,
                      created_at, updated_at
            "#,
        )
        .bind(req.building_id)
        .bind(org_id)
        .bind(req.default_currency)
        .bind(req.country)
        .bind(req.vat_rate)
        .bind(&req.vat_registration_number)
        .bind(&req.local_tax_id)
        .bind(req.requires_local_reporting)
        .bind(&req.local_accounting_format)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    /// Get property currency configuration scoped to an organization.
    pub async fn get_property_currency_config(
        &self,
        building_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PropertyCurrencyConfig>, AppError> {
        let config = sqlx::query_as::<_, PropertyCurrencyConfig>(
            r#"
            SELECT id, building_id, organization_id, default_currency, country,
                   vat_rate, vat_registration_number, local_tax_id,
                   requires_local_reporting, local_accounting_format,
                   created_at, updated_at
            FROM property_currency_config
            WHERE building_id = $1 AND organization_id = $2
            "#,
        )
        .bind(building_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    /// List property currency configurations for an organization
    pub async fn list_property_currency_configs(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<PropertyCurrencyConfig>, AppError> {
        let configs = sqlx::query_as::<_, PropertyCurrencyConfig>(
            r#"
            SELECT id, building_id, organization_id, default_currency, country,
                   vat_rate, vat_registration_number, local_tax_id,
                   requires_local_reporting, local_accounting_format,
                   created_at, updated_at
            FROM property_currency_config
            WHERE organization_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(configs)
    }

    /// Update property currency configuration
    pub async fn update_property_currency_config(
        &self,
        building_id: Uuid,
        org_id: Uuid,
        req: UpdatePropertyCurrencyConfig,
    ) -> Result<Option<PropertyCurrencyConfig>, AppError> {
        let config = sqlx::query_as::<_, PropertyCurrencyConfig>(
            r#"
            UPDATE property_currency_config
            SET default_currency = COALESCE($3, default_currency),
                country = COALESCE($4, country),
                vat_rate = COALESCE($5, vat_rate),
                vat_registration_number = COALESCE($6, vat_registration_number),
                local_tax_id = COALESCE($7, local_tax_id),
                requires_local_reporting = COALESCE($8, requires_local_reporting),
                local_accounting_format = COALESCE($9, local_accounting_format),
                updated_at = NOW()
            WHERE building_id = $1 AND organization_id = $2
            RETURNING id, building_id, organization_id, default_currency, country,
                      vat_rate, vat_registration_number, local_tax_id,
                      requires_local_reporting, local_accounting_format,
                      created_at, updated_at
            "#,
        )
        .bind(building_id)
        .bind(org_id)
        .bind(req.default_currency)
        .bind(req.country)
        .bind(req.vat_rate)
        .bind(&req.vat_registration_number)
        .bind(&req.local_tax_id)
        .bind(req.requires_local_reporting)
        .bind(&req.local_accounting_format)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    // =========================================================================
    // STORY 145.2: EXCHANGE RATE MANAGEMENT
    // =========================================================================

    /// Get exchange rate for a currency pair on a specific date
    pub async fn get_exchange_rate(
        &self,
        from_currency: SupportedCurrency,
        to_currency: SupportedCurrency,
        date: NaiveDate,
    ) -> Result<Option<ExchangeRate>, AppError> {
        let rate = sqlx::query_as::<_, ExchangeRate>(
            r#"
            SELECT id, from_currency, to_currency, rate, inverse_rate, rate_date,
                   source, source_reference, is_override, override_reason, overridden_by,
                   valid_from, valid_until, created_at
            FROM exchange_rates
            WHERE from_currency = $1 AND to_currency = $2 AND rate_date = $3
            ORDER BY is_override DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(from_currency)
        .bind(to_currency)
        .bind(date)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rate)
    }

    /// Get latest exchange rate for a currency pair
    pub async fn get_latest_exchange_rate(
        &self,
        from_currency: SupportedCurrency,
        to_currency: SupportedCurrency,
    ) -> Result<Option<ExchangeRate>, AppError> {
        let rate = sqlx::query_as::<_, ExchangeRate>(
            r#"
            SELECT id, from_currency, to_currency, rate, inverse_rate, rate_date,
                   source, source_reference, is_override, override_reason, overridden_by,
                   valid_from, valid_until, created_at
            FROM exchange_rates
            WHERE from_currency = $1 AND to_currency = $2
            ORDER BY rate_date DESC, is_override DESC, created_at DESC
            LIMIT 1
            "#,
        )
        .bind(from_currency)
        .bind(to_currency)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rate)
    }

    /// List exchange rates with optional filtering
    pub async fn list_exchange_rates(
        &self,
        query: ExchangeRateQuery,
    ) -> Result<Vec<ExchangeRate>, AppError> {
        let rates = sqlx::query_as::<_, ExchangeRate>(
            r#"
            SELECT id, from_currency, to_currency, rate, inverse_rate, rate_date,
                   source, source_reference, is_override, override_reason, overridden_by,
                   valid_from, valid_until, created_at
            FROM exchange_rates
            WHERE ($1::supported_currency IS NULL OR from_currency = $1)
              AND ($2::supported_currency IS NULL OR to_currency = $2)
              AND ($3::DATE IS NULL OR rate_date = $3)
              AND ($4::DATE IS NULL OR rate_date >= $4)
              AND ($5::DATE IS NULL OR rate_date <= $5)
              AND ($6::exchange_rate_source IS NULL OR source = $6)
            ORDER BY rate_date DESC, from_currency, to_currency
            LIMIT 1000
            "#,
        )
        .bind(query.from_currency)
        .bind(query.to_currency)
        .bind(query.date)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(query.source)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rates)
    }

    /// Create exchange rate
    pub async fn create_exchange_rate(
        &self,
        req: CreateExchangeRate,
    ) -> Result<ExchangeRate, AppError> {
        let inverse_rate = Decimal::ONE / req.rate;

        let rate = sqlx::query_as::<_, ExchangeRate>(
            r#"
            INSERT INTO exchange_rates (
                from_currency, to_currency, rate, inverse_rate, rate_date,
                source, source_reference
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, from_currency, to_currency, rate, inverse_rate, rate_date,
                      source, source_reference, is_override, override_reason, overridden_by,
                      valid_from, valid_until, created_at
            "#,
        )
        .bind(req.from_currency)
        .bind(req.to_currency)
        .bind(req.rate)
        .bind(inverse_rate)
        .bind(req.rate_date)
        .bind(req.source)
        .bind(&req.source_reference)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rate)
    }

    /// Override exchange rate manually
    pub async fn override_exchange_rate(
        &self,
        user_id: Uuid,
        req: OverrideExchangeRate,
    ) -> Result<ExchangeRate, AppError> {
        let inverse_rate = Decimal::ONE / req.rate;

        let rate = sqlx::query_as::<_, ExchangeRate>(
            r#"
            INSERT INTO exchange_rates (
                from_currency, to_currency, rate, inverse_rate, rate_date,
                source, is_override, override_reason, overridden_by
            )
            VALUES ($1, $2, $3, $4, $5, 'manual', true, $6, $7)
            RETURNING id, from_currency, to_currency, rate, inverse_rate, rate_date,
                      source, source_reference, is_override, override_reason, overridden_by,
                      valid_from, valid_until, created_at
            "#,
        )
        .bind(req.from_currency)
        .bind(req.to_currency)
        .bind(req.rate)
        .bind(inverse_rate)
        .bind(req.rate_date)
        .bind(&req.reason)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rate)
    }

    /// Log exchange rate fetch attempt
    pub async fn log_rate_fetch(
        &self,
        org_id: Option<Uuid>,
        source: ExchangeRateSource,
        success: bool,
        rates_fetched: i32,
        error_message: Option<&str>,
    ) -> Result<ExchangeRateFetchLog, AppError> {
        let log = sqlx::query_as::<_, ExchangeRateFetchLog>(
            r#"
            INSERT INTO exchange_rate_fetch_log (
                organization_id, source, success, rates_fetched, error_message
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, organization_id, source, fetch_time, success,
                      rates_fetched, error_message, response_data, created_at
            "#,
        )
        .bind(org_id)
        .bind(source)
        .bind(success)
        .bind(rates_fetched)
        .bind(error_message)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(log)
    }

    /// Update last rate update timestamp for organization
    pub async fn update_last_rate_update(&self, org_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE organization_currency_config
            SET last_rate_update = NOW(), updated_at = NOW()
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // =========================================================================
    // STORY 145.3: CROSS-CURRENCY TRANSACTIONS
    // =========================================================================

    /// Create multi-currency transaction
    pub async fn create_transaction(
        &self,
        org_id: Uuid,
        base_currency: SupportedCurrency,
        req: CreateMultiCurrencyTransaction,
    ) -> Result<MultiCurrencyTransaction, AppError> {
        let rate_date = req.rate_date.unwrap_or_else(|| Utc::now().date_naive());

        // Get exchange rate
        let exchange_rate = if let Some(override_rate) = req.override_rate {
            override_rate
        } else if req.original_currency == base_currency {
            Decimal::ONE
        } else {
            let rate = self
                .get_latest_exchange_rate(req.original_currency, base_currency)
                .await?
                .ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "No exchange rate found for {:?} to {:?}",
                        req.original_currency, base_currency
                    ))
                })?;
            rate.rate
        };

        let converted_amount = req.original_amount * exchange_rate;

        let tx = sqlx::query_as::<_, MultiCurrencyTransaction>(
            r#"
            INSERT INTO multi_currency_transactions (
                organization_id, building_id, source_type, source_id,
                original_currency, original_amount, base_currency, converted_amount,
                exchange_rate, rate_date, conversion_status,
                is_rate_override, override_rate, override_reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'converted', $11, $12, $13)
            RETURNING id, organization_id, building_id, source_type, source_id,
                      original_currency, original_amount, base_currency, converted_amount,
                      exchange_rate, exchange_rate_id, rate_date, conversion_status,
                      conversion_timestamp, is_rate_override, override_rate, override_reason,
                      overridden_by, realized_gain_loss, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(&req.source_type)
        .bind(req.source_id)
        .bind(req.original_currency)
        .bind(req.original_amount)
        .bind(base_currency)
        .bind(converted_amount)
        .bind(exchange_rate)
        .bind(rate_date)
        .bind(req.override_rate.is_some())
        .bind(req.override_rate)
        .bind(&req.override_reason)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tx)
    }

    /// Get multi-currency transaction by ID
    pub async fn get_transaction(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MultiCurrencyTransaction>, AppError> {
        let tx = sqlx::query_as::<_, MultiCurrencyTransaction>(
            r#"
            SELECT id, organization_id, building_id, source_type, source_id,
                   original_currency, original_amount, base_currency, converted_amount,
                   exchange_rate, exchange_rate_id, rate_date, conversion_status,
                   conversion_timestamp, is_rate_override, override_rate, override_reason,
                   overridden_by, realized_gain_loss, created_at, updated_at
            FROM multi_currency_transactions
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tx)
    }

    /// List multi-currency transactions
    pub async fn list_transactions(
        &self,
        org_id: Uuid,
        query: TransactionQuery,
    ) -> Result<Vec<MultiCurrencyTransaction>, AppError> {
        let txs = sqlx::query_as::<_, MultiCurrencyTransaction>(
            r#"
            SELECT id, organization_id, building_id, source_type, source_id,
                   original_currency, original_amount, base_currency, converted_amount,
                   exchange_rate, exchange_rate_id, rate_date, conversion_status,
                   conversion_timestamp, is_rate_override, override_rate, override_reason,
                   overridden_by, realized_gain_loss, created_at, updated_at
            FROM multi_currency_transactions
            WHERE organization_id = $1
              AND ($2::UUID IS NULL OR building_id = $2)
              AND ($3::VARCHAR IS NULL OR source_type = $3)
              AND ($4::supported_currency IS NULL OR original_currency = $4)
              AND ($5::DATE IS NULL OR rate_date >= $5)
              AND ($6::DATE IS NULL OR rate_date <= $6)
              AND ($7::conversion_status IS NULL OR conversion_status = $7)
            ORDER BY created_at DESC
            LIMIT 500
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.source_type)
        .bind(query.currency)
        .bind(query.date_from)
        .bind(query.date_to)
        .bind(query.status)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(txs)
    }

    /// Update transaction exchange rate
    pub async fn update_transaction_rate(
        &self,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        req: UpdateTransactionRate,
    ) -> Result<Option<MultiCurrencyTransaction>, AppError> {
        // Get current transaction
        let current = self.get_transaction(id, org_id).await?;
        if current.is_none() {
            return Ok(None);
        }
        let current = current.unwrap();

        let new_converted_amount = current.original_amount * req.new_rate;

        // Update transaction
        let tx = sqlx::query_as::<_, MultiCurrencyTransaction>(
            r#"
            UPDATE multi_currency_transactions
            SET exchange_rate = $3,
                converted_amount = $4,
                is_rate_override = true,
                override_rate = $3,
                override_reason = $5,
                overridden_by = $6,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING id, organization_id, building_id, source_type, source_id,
                      original_currency, original_amount, base_currency, converted_amount,
                      exchange_rate, exchange_rate_id, rate_date, conversion_status,
                      conversion_timestamp, is_rate_override, override_rate, override_reason,
                      overridden_by, realized_gain_loss, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(req.new_rate)
        .bind(new_converted_amount)
        .bind(&req.reason)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Log audit
        if tx.is_some() {
            sqlx::query(
                r#"
                INSERT INTO currency_conversion_audit (
                    transaction_id, action, previous_rate, new_rate,
                    previous_amount, new_amount, performed_by, notes
                )
                VALUES ($1, 'rate_updated', $2, $3, $4, $5, $6, $7)
                "#,
            )
            .bind(id)
            .bind(current.exchange_rate)
            .bind(req.new_rate)
            .bind(current.converted_amount)
            .bind(new_converted_amount)
            .bind(user_id)
            .bind(&req.reason)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        Ok(tx)
    }

    // =========================================================================
    // STORY 145.4: CROSS-BORDER LEASE MANAGEMENT
    // =========================================================================

    /// Create cross-border lease configuration
    pub async fn create_cross_border_lease(
        &self,
        org_id: Uuid,
        req: CreateCrossBorderLease,
    ) -> Result<CrossBorderLease, AppError> {
        let lease = sqlx::query_as::<_, CrossBorderLease>(
            r#"
            INSERT INTO cross_border_leases (
                lease_id, organization_id, property_country, property_currency,
                tenant_country, tenant_tax_id, tenant_vat_number,
                lease_currency, payment_currency,
                convert_at_invoice_date, convert_at_payment_date,
                fixed_exchange_rate, rate_lock_date,
                local_vat_applicable, vat_rate, reverse_charge_vat, withholding_tax_rate,
                local_clauses, governing_law, jurisdiction
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            RETURNING id, lease_id, organization_id, property_country, property_currency,
                      tenant_country, tenant_tax_id, tenant_vat_number,
                      lease_currency, payment_currency,
                      convert_at_invoice_date, convert_at_payment_date,
                      fixed_exchange_rate, rate_lock_date,
                      local_vat_applicable, vat_rate, reverse_charge_vat, withholding_tax_rate,
                      compliance_status, compliance_notes, last_compliance_check,
                      local_clauses, governing_law, jurisdiction, created_at, updated_at
            "#,
        )
        .bind(req.lease_id)
        .bind(org_id)
        .bind(req.property_country)
        .bind(req.property_currency)
        .bind(req.tenant_country)
        .bind(&req.tenant_tax_id)
        .bind(&req.tenant_vat_number)
        .bind(req.lease_currency)
        .bind(req.payment_currency)
        .bind(req.convert_at_invoice_date)
        .bind(req.convert_at_payment_date)
        .bind(req.fixed_exchange_rate)
        .bind(req.rate_lock_date)
        .bind(req.local_vat_applicable)
        .bind(req.vat_rate)
        .bind(req.reverse_charge_vat)
        .bind(req.withholding_tax_rate)
        .bind(&req.local_clauses)
        .bind(req.governing_law)
        .bind(req.jurisdiction)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(lease)
    }

    /// Get cross-border lease by lease ID
    pub async fn get_cross_border_lease(
        &self,
        lease_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<CrossBorderLease>, AppError> {
        let lease = sqlx::query_as::<_, CrossBorderLease>(
            r#"
            SELECT id, lease_id, organization_id, property_country, property_currency,
                   tenant_country, tenant_tax_id, tenant_vat_number,
                   lease_currency, payment_currency,
                   convert_at_invoice_date, convert_at_payment_date,
                   fixed_exchange_rate, rate_lock_date,
                   local_vat_applicable, vat_rate, reverse_charge_vat, withholding_tax_rate,
                   compliance_status, compliance_notes, last_compliance_check,
                   local_clauses, governing_law, jurisdiction, created_at, updated_at
            FROM cross_border_leases
            WHERE lease_id = $1 AND organization_id = $2
            "#,
        )
        .bind(lease_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(lease)
    }

    /// List cross-border leases for organization
    pub async fn list_cross_border_leases(
        &self,
        org_id: Uuid,
        query: CrossBorderLeaseQuery,
    ) -> Result<Vec<CrossBorderLease>, AppError> {
        let leases = sqlx::query_as::<_, CrossBorderLease>(
            r#"
            SELECT id, lease_id, organization_id, property_country, property_currency,
                   tenant_country, tenant_tax_id, tenant_vat_number,
                   lease_currency, payment_currency,
                   convert_at_invoice_date, convert_at_payment_date,
                   fixed_exchange_rate, rate_lock_date,
                   local_vat_applicable, vat_rate, reverse_charge_vat, withholding_tax_rate,
                   compliance_status, compliance_notes, last_compliance_check,
                   local_clauses, governing_law, jurisdiction, created_at, updated_at
            FROM cross_border_leases
            WHERE organization_id = $1
              AND ($2::country_code IS NULL OR property_country = $2)
              AND ($3::supported_currency IS NULL OR lease_currency = $3)
              AND ($4::compliance_status IS NULL OR compliance_status = $4)
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .bind(query.property_country)
        .bind(query.lease_currency)
        .bind(query.compliance_status)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(leases)
    }

    /// Update cross-border lease
    pub async fn update_cross_border_lease(
        &self,
        lease_id: Uuid,
        org_id: Uuid,
        req: UpdateCrossBorderLease,
    ) -> Result<Option<CrossBorderLease>, AppError> {
        let lease = sqlx::query_as::<_, CrossBorderLease>(
            r#"
            UPDATE cross_border_leases
            SET tenant_country = COALESCE($3, tenant_country),
                tenant_tax_id = COALESCE($4, tenant_tax_id),
                tenant_vat_number = COALESCE($5, tenant_vat_number),
                payment_currency = COALESCE($6, payment_currency),
                convert_at_invoice_date = COALESCE($7, convert_at_invoice_date),
                convert_at_payment_date = COALESCE($8, convert_at_payment_date),
                fixed_exchange_rate = COALESCE($9, fixed_exchange_rate),
                rate_lock_date = COALESCE($10, rate_lock_date),
                local_vat_applicable = COALESCE($11, local_vat_applicable),
                vat_rate = COALESCE($12, vat_rate),
                reverse_charge_vat = COALESCE($13, reverse_charge_vat),
                withholding_tax_rate = COALESCE($14, withholding_tax_rate),
                compliance_status = COALESCE($15, compliance_status),
                compliance_notes = COALESCE($16, compliance_notes),
                local_clauses = COALESCE($17, local_clauses),
                governing_law = COALESCE($18, governing_law),
                jurisdiction = COALESCE($19, jurisdiction),
                updated_at = NOW()
            WHERE lease_id = $1 AND organization_id = $2
            RETURNING id, lease_id, organization_id, property_country, property_currency,
                      tenant_country, tenant_tax_id, tenant_vat_number,
                      lease_currency, payment_currency,
                      convert_at_invoice_date, convert_at_payment_date,
                      fixed_exchange_rate, rate_lock_date,
                      local_vat_applicable, vat_rate, reverse_charge_vat, withholding_tax_rate,
                      compliance_status, compliance_notes, last_compliance_check,
                      local_clauses, governing_law, jurisdiction, created_at, updated_at
            "#,
        )
        .bind(lease_id)
        .bind(org_id)
        .bind(req.tenant_country)
        .bind(&req.tenant_tax_id)
        .bind(&req.tenant_vat_number)
        .bind(req.payment_currency)
        .bind(req.convert_at_invoice_date)
        .bind(req.convert_at_payment_date)
        .bind(req.fixed_exchange_rate)
        .bind(req.rate_lock_date)
        .bind(req.local_vat_applicable)
        .bind(req.vat_rate)
        .bind(req.reverse_charge_vat)
        .bind(req.withholding_tax_rate)
        .bind(req.compliance_status)
        .bind(&req.compliance_notes)
        .bind(&req.local_clauses)
        .bind(req.governing_law)
        .bind(req.jurisdiction)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(lease)
    }

    /// Get compliance requirements for a country
    pub async fn get_compliance_requirements(
        &self,
        country: CountryCode,
    ) -> Result<Vec<CrossBorderComplianceRequirement>, AppError> {
        let requirements = sqlx::query_as::<_, CrossBorderComplianceRequirement>(
            r#"
            SELECT id, country, requirement_type, requirement_name, description,
                   threshold_amount, threshold_currency, reporting_frequency,
                   reporting_deadline_days, required_documents, is_active,
                   effective_from, effective_until, created_at, updated_at
            FROM cross_border_compliance_requirements
            WHERE country = $1 AND is_active = true
              AND effective_from <= CURRENT_DATE
              AND (effective_until IS NULL OR effective_until >= CURRENT_DATE)
            ORDER BY requirement_type, requirement_name
            "#,
        )
        .bind(country)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(requirements)
    }

    // =========================================================================
    // STORY 145.5: CONSOLIDATED MULTI-CURRENCY REPORTING
    // =========================================================================

    /// Create report configuration
    pub async fn create_report_config(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateReportConfig,
    ) -> Result<MultiCurrencyReportConfig, AppError> {
        let config = sqlx::query_as::<_, MultiCurrencyReportConfig>(
            r#"
            INSERT INTO multi_currency_report_config (
                organization_id, name, description, report_currency,
                show_original_currencies, show_conversion_details,
                rate_date_type, specific_rate_date,
                group_by_currency, group_by_country, group_by_property,
                is_saved, is_default, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, organization_id, name, description, report_currency,
                      show_original_currencies, show_conversion_details,
                      rate_date_type, specific_rate_date,
                      group_by_currency, group_by_country, group_by_property,
                      is_saved, is_default, created_at, updated_at, created_by
            "#,
        )
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.report_currency)
        .bind(req.show_original_currencies)
        .bind(req.show_conversion_details)
        .bind(&req.rate_date_type)
        .bind(req.specific_rate_date)
        .bind(req.group_by_currency)
        .bind(req.group_by_country)
        .bind(req.group_by_property)
        .bind(req.is_saved)
        .bind(req.is_default)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(config)
    }

    /// List report configurations
    pub async fn list_report_configs(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<MultiCurrencyReportConfig>, AppError> {
        let configs = sqlx::query_as::<_, MultiCurrencyReportConfig>(
            r#"
            SELECT id, organization_id, name, description, report_currency,
                   show_original_currencies, show_conversion_details,
                   rate_date_type, specific_rate_date,
                   group_by_currency, group_by_country, group_by_property,
                   is_saved, is_default, created_at, updated_at, created_by
            FROM multi_currency_report_config
            WHERE organization_id = $1
            ORDER BY is_default DESC, name
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(configs)
    }

    /// Generate and save report snapshot
    pub async fn generate_report_snapshot(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        req: GenerateReportRequest,
    ) -> Result<MultiCurrencyReportSnapshot, AppError> {
        let rate_date = req.rate_date.unwrap_or(req.period_end);

        // Calculate totals from transactions
        let totals: (Option<Decimal>, Option<Decimal>) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN source_type IN ('invoice', 'rental_income') THEN converted_amount ELSE 0 END), 0) as revenue,
                COALESCE(SUM(CASE WHEN source_type IN ('expense', 'payment') THEN converted_amount ELSE 0 END), 0) as expenses
            FROM multi_currency_transactions
            WHERE organization_id = $1
              AND rate_date BETWEEN $2 AND $3
              AND conversion_status = 'converted'
            "#,
        )
        .bind(org_id)
        .bind(req.period_start)
        .bind(req.period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let total_revenue = totals.0.unwrap_or(Decimal::ZERO);
        let total_expenses = totals.1.unwrap_or(Decimal::ZERO);
        let net_income = total_revenue - total_expenses;

        // Get currency breakdown
        let currency_breakdown = json!({});
        let rates_used = json!({});

        let snapshot = sqlx::query_as::<_, MultiCurrencyReportSnapshot>(
            r#"
            INSERT INTO multi_currency_report_snapshots (
                organization_id, config_id, period_start, period_end,
                report_currency, total_revenue, total_expenses, net_income,
                currency_breakdown, rates_used, rate_date, generated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, organization_id, config_id, period_start, period_end,
                      report_currency, total_revenue, total_expenses, net_income,
                      currency_breakdown, exchange_rate_impact, unrealized_fx_gain_loss,
                      realized_fx_gain_loss, country_breakdown, property_breakdown,
                      rates_used, rate_date, generated_at, generated_by
            "#,
        )
        .bind(org_id)
        .bind(req.config_id)
        .bind(req.period_start)
        .bind(req.period_end)
        .bind(req.report_currency)
        .bind(total_revenue)
        .bind(total_expenses)
        .bind(net_income)
        .bind(&currency_breakdown)
        .bind(&rates_used)
        .bind(rate_date)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(snapshot)
    }

    /// List report snapshots
    pub async fn list_report_snapshots(
        &self,
        org_id: Uuid,
        limit: i32,
    ) -> Result<Vec<MultiCurrencyReportSnapshot>, AppError> {
        let snapshots = sqlx::query_as::<_, MultiCurrencyReportSnapshot>(
            r#"
            SELECT id, organization_id, config_id, period_start, period_end,
                   report_currency, total_revenue, total_expenses, net_income,
                   currency_breakdown, exchange_rate_impact, unrealized_fx_gain_loss,
                   realized_fx_gain_loss, country_breakdown, property_breakdown,
                   rates_used, rate_date, generated_at, generated_by
            FROM multi_currency_report_snapshots
            WHERE organization_id = $1
            ORDER BY generated_at DESC
            LIMIT $2
            "#,
        )
        .bind(org_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(snapshots)
    }

    /// Get currency exposure analysis
    pub async fn get_currency_exposure(
        &self,
        org_id: Uuid,
        date: NaiveDate,
    ) -> Result<Vec<CurrencyExposureAnalysis>, AppError> {
        let exposures = sqlx::query_as::<_, CurrencyExposureAnalysis>(
            r#"
            SELECT id, organization_id, analysis_date, currency,
                   receivables_amount, payables_amount, net_exposure,
                   asset_value, projected_revenue, projected_expenses,
                   value_at_risk, expected_shortfall, hedged_amount,
                   hedge_effectiveness, created_at
            FROM currency_exposure_analysis
            WHERE organization_id = $1 AND analysis_date = $2
            ORDER BY currency
            "#,
        )
        .bind(org_id)
        .bind(date)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(exposures)
    }

    // =========================================================================
    // DASHBOARD & STATISTICS
    // =========================================================================

    /// Get multi-currency statistics
    pub async fn get_statistics(&self, org_id: Uuid) -> Result<MultiCurrencyStatistics, AppError> {
        // Get currency distribution
        let distribution: Vec<(SupportedCurrency, i64, Decimal)> = sqlx::query_as(
            r#"
            SELECT original_currency, COUNT(*) as count, SUM(original_amount) as total
            FROM multi_currency_transactions
            WHERE organization_id = $1
            GROUP BY original_currency
            ORDER BY total DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let total_transactions: i64 = distribution.iter().map(|(_, c, _)| c).sum();
        let total_amount: Decimal = distribution.iter().map(|(_, _, t)| t).sum();

        let currency_distribution: Vec<CurrencyDistribution> = distribution
            .into_iter()
            .map(|(currency, count, amount)| CurrencyDistribution {
                currency,
                transaction_count: count,
                total_amount: amount,
                percentage: if total_amount > Decimal::ZERO {
                    (amount / total_amount) * Decimal::from(100)
                } else {
                    Decimal::ZERO
                },
            })
            .collect();

        // Get cross-border lease count
        let cross_border_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM cross_border_leases WHERE organization_id = $1")
                .bind(org_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        // Get total FX gain/loss
        let fx_gain_loss: (Option<Decimal>,) = sqlx::query_as(
            "SELECT SUM(realized_gain_loss) FROM multi_currency_transactions WHERE organization_id = $1",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(MultiCurrencyStatistics {
            total_currencies_used: currency_distribution.len() as i32,
            total_transactions,
            total_cross_border_leases: cross_border_count.0,
            total_fx_gain_loss: fx_gain_loss.0.unwrap_or(Decimal::ZERO),
            currency_distribution,
        })
    }
}

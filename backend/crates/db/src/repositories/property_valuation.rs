//! Property Valuation repository for Epic 138: Automated Property Valuation Model.
//!
//! Provides database operations for property valuations, AVM models,
//! comparable sales, market data, and valuation reports.

use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, PgPool, Row};
use uuid::Uuid;

use crate::models::property_valuation::*;
use crate::DbPool;

/// Repository for property valuation operations.
#[derive(Clone)]
pub struct PropertyValuationRepository {
    pool: DbPool,
}

impl PropertyValuationRepository {
    /// Create a new PropertyValuationRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get the underlying pool reference.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ========================================================================
    // Valuation Model Operations
    // ========================================================================

    /// Create a new valuation model.
    pub async fn create_model(
        &self,
        org_id: Uuid,
        data: &CreateValuationModel,
        created_by: Uuid,
    ) -> Result<PropertyValuationModel, SqlxError> {
        sqlx::query_as::<_, PropertyValuationModel>(
            r#"
            INSERT INTO property_valuation_models (
                organization_id, name, description, model_type, model_config,
                feature_weights, is_default, created_by
            )
            VALUES ($1, $2, $3, $4::valuation_model_type, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.model_type)
        .bind(&data.model_config)
        .bind(&data.feature_weights)
        .bind(data.is_default)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a valuation model by ID.
    pub async fn get_model(
        &self,
        org_id: Uuid,
        model_id: Uuid,
    ) -> Result<Option<PropertyValuationModel>, SqlxError> {
        sqlx::query_as::<_, PropertyValuationModel>(
            r#"
            SELECT * FROM property_valuation_models
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(model_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List valuation models for an organization.
    pub async fn list_models(
        &self,
        org_id: Uuid,
        active_only: bool,
    ) -> Result<Vec<PropertyValuationModel>, SqlxError> {
        if active_only {
            sqlx::query_as::<_, PropertyValuationModel>(
                r#"
                SELECT * FROM property_valuation_models
                WHERE organization_id = $1 AND is_active = true
                ORDER BY is_default DESC, name ASC
                "#,
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, PropertyValuationModel>(
                r#"
                SELECT * FROM property_valuation_models
                WHERE organization_id = $1
                ORDER BY is_default DESC, name ASC
                "#,
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
        }
    }

    /// Update a valuation model.
    pub async fn update_model(
        &self,
        org_id: Uuid,
        model_id: Uuid,
        data: &UpdateValuationModel,
    ) -> Result<Option<PropertyValuationModel>, SqlxError> {
        sqlx::query_as::<_, PropertyValuationModel>(
            r#"
            UPDATE property_valuation_models
            SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                model_config = COALESCE($5, model_config),
                feature_weights = COALESCE($6, feature_weights),
                r_squared = COALESCE($7, r_squared),
                mean_absolute_error = COALESCE($8, mean_absolute_error),
                mean_percentage_error = COALESCE($9, mean_percentage_error),
                training_sample_size = COALESCE($10, training_sample_size),
                is_active = COALESCE($11, is_active),
                is_default = COALESCE($12, is_default),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(model_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.model_config)
        .bind(&data.feature_weights)
        .bind(data.r_squared.map(Decimal::try_from).and_then(Result::ok))
        .bind(
            data.mean_absolute_error
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.mean_percentage_error
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.training_sample_size)
        .bind(data.is_active)
        .bind(data.is_default)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete a valuation model.
    pub async fn delete_model(&self, org_id: Uuid, model_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM property_valuation_models
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(model_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Property Valuation Operations
    // ========================================================================

    /// Create a new property valuation.
    pub async fn create_valuation(
        &self,
        org_id: Uuid,
        data: &CreatePropertyValuation,
        created_by: Uuid,
    ) -> Result<PropertyValuation, SqlxError> {
        sqlx::query_as::<_, PropertyValuation>(
            r#"
            INSERT INTO avm_property_valuations (
                organization_id, property_id, building_id, model_id,
                valuation_date, effective_date, expiration_date,
                estimated_value, value_range_low, value_range_high,
                confidence_level, confidence_score, price_per_sqm,
                property_condition, effective_age, market_trend,
                methodology_notes, assumptions, created_by
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11::valuation_confidence, $12, $13,
                $14::property_condition, $15, $16::market_trend,
                $17, $18, $19
            )
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.property_id)
        .bind(data.building_id)
        .bind(data.model_id)
        .bind(data.valuation_date)
        .bind(data.effective_date)
        .bind(data.expiration_date)
        .bind(Decimal::try_from(data.estimated_value).unwrap_or_default())
        .bind(
            data.value_range_low
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.value_range_high
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.confidence_level)
        .bind(
            data.confidence_score
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.price_per_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.property_condition)
        .bind(data.effective_age)
        .bind(data.market_trend)
        .bind(&data.methodology_notes)
        .bind(&data.assumptions)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a property valuation by ID.
    pub async fn get_valuation(
        &self,
        org_id: Uuid,
        valuation_id: Uuid,
    ) -> Result<Option<PropertyValuation>, SqlxError> {
        sqlx::query_as::<_, PropertyValuation>(
            r#"
            SELECT * FROM avm_property_valuations
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(valuation_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List valuations for an organization.
    pub async fn list_valuations(
        &self,
        org_id: Uuid,
        property_id: Option<Uuid>,
        status: Option<ValuationStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PropertyValuation>, SqlxError> {
        sqlx::query_as::<_, PropertyValuation>(
            r#"
            SELECT * FROM avm_property_valuations
            WHERE organization_id = $1
              AND ($2::uuid IS NULL OR property_id = $2)
              AND ($3::valuation_status IS NULL OR status = $3)
            ORDER BY valuation_date DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(property_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a property valuation.
    pub async fn update_valuation(
        &self,
        org_id: Uuid,
        valuation_id: Uuid,
        data: &UpdatePropertyValuation,
    ) -> Result<Option<PropertyValuation>, SqlxError> {
        sqlx::query_as::<_, PropertyValuation>(
            r#"
            UPDATE avm_property_valuations
            SET
                status = COALESCE($3::valuation_status, status),
                estimated_value = COALESCE($4, estimated_value),
                value_range_low = COALESCE($5, value_range_low),
                value_range_high = COALESCE($6, value_range_high),
                confidence_level = COALESCE($7::valuation_confidence, confidence_level),
                confidence_score = COALESCE($8, confidence_score),
                market_value = COALESCE($9, market_value),
                insurance_value = COALESCE($10, insurance_value),
                tax_assessed_value = COALESCE($11, tax_assessed_value),
                property_condition = COALESCE($12::property_condition, property_condition),
                market_trend = COALESCE($13::market_trend, market_trend),
                methodology_notes = COALESCE($14, methodology_notes),
                assumptions = COALESCE($15, assumptions),
                limiting_conditions = COALESCE($16, limiting_conditions),
                review_notes = COALESCE($17, review_notes),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(valuation_id)
        .bind(data.status)
        .bind(
            data.estimated_value
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.value_range_low
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.value_range_high
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.confidence_level)
        .bind(
            data.confidence_score
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.market_value
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.insurance_value
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.tax_assessed_value
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.property_condition)
        .bind(data.market_trend)
        .bind(&data.methodology_notes)
        .bind(&data.assumptions)
        .bind(&data.limiting_conditions)
        .bind(&data.review_notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete a property valuation.
    pub async fn delete_valuation(
        &self,
        org_id: Uuid,
        valuation_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM avm_property_valuations
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(valuation_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Approve a valuation.
    pub async fn approve_valuation(
        &self,
        org_id: Uuid,
        valuation_id: Uuid,
        reviewer_id: Uuid,
        notes: Option<String>,
    ) -> Result<Option<PropertyValuation>, SqlxError> {
        sqlx::query_as::<_, PropertyValuation>(
            r#"
            UPDATE avm_property_valuations
            SET
                status = 'approved'::valuation_status,
                reviewed_by = $3,
                reviewed_at = NOW(),
                review_notes = $4,
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(valuation_id)
        .bind(reviewer_id)
        .bind(notes)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Comparable Sales Operations
    // ========================================================================

    /// Create a comparable sale.
    pub async fn create_comparable(
        &self,
        org_id: Uuid,
        data: &CreateComparable,
    ) -> Result<ValuationComparable, SqlxError> {
        sqlx::query_as::<_, ValuationComparable>(
            r#"
            INSERT INTO valuation_comparables (
                organization_id, valuation_id, comparable_property_id,
                external_address, external_city, external_postal_code,
                latitude, longitude, sale_date, sale_price,
                property_type, total_area_sqm, lot_size_sqm,
                year_built, bedrooms, bathrooms, condition,
                distance_km, data_source
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17::property_condition, $18, $19
            )
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.valuation_id)
        .bind(data.comparable_property_id)
        .bind(&data.external_address)
        .bind(&data.external_city)
        .bind(&data.external_postal_code)
        .bind(data.latitude.map(Decimal::try_from).and_then(Result::ok))
        .bind(data.longitude.map(Decimal::try_from).and_then(Result::ok))
        .bind(data.sale_date)
        .bind(Decimal::try_from(data.sale_price).unwrap_or_default())
        .bind(&data.property_type)
        .bind(
            data.total_area_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.lot_size_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.year_built)
        .bind(data.bedrooms)
        .bind(data.bathrooms.map(Decimal::try_from).and_then(Result::ok))
        .bind(data.condition)
        .bind(data.distance_km.map(Decimal::try_from).and_then(Result::ok))
        .bind(&data.data_source)
        .fetch_one(&self.pool)
        .await
    }

    /// Get comparables for a valuation.
    pub async fn list_comparables(
        &self,
        org_id: Uuid,
        valuation_id: Uuid,
    ) -> Result<Vec<ValuationComparable>, SqlxError> {
        sqlx::query_as::<_, ValuationComparable>(
            r#"
            SELECT * FROM valuation_comparables
            WHERE organization_id = $1 AND valuation_id = $2
            ORDER BY similarity_score DESC NULLS LAST, distance_km ASC NULLS LAST
            "#,
        )
        .bind(org_id)
        .bind(valuation_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a comparable.
    pub async fn update_comparable(
        &self,
        org_id: Uuid,
        comparable_id: Uuid,
        data: &UpdateComparable,
    ) -> Result<Option<ValuationComparable>, SqlxError> {
        sqlx::query_as::<_, ValuationComparable>(
            r#"
            UPDATE valuation_comparables
            SET
                sale_price = COALESCE($3, sale_price),
                sale_price_per_sqm = COALESCE($4, sale_price_per_sqm),
                similarity_score = COALESCE($5, similarity_score),
                weight = COALESCE($6, weight),
                gross_adjustment_percent = COALESCE($7, gross_adjustment_percent),
                net_adjustment_percent = COALESCE($8, net_adjustment_percent),
                adjusted_price = COALESCE($9, adjusted_price),
                is_verified = COALESCE($10, is_verified),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(comparable_id)
        .bind(data.sale_price.map(Decimal::try_from).and_then(Result::ok))
        .bind(
            data.sale_price_per_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.similarity_score
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.weight.map(Decimal::try_from).and_then(Result::ok))
        .bind(
            data.gross_adjustment_percent
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.net_adjustment_percent
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.adjusted_price
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.is_verified)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete a comparable.
    pub async fn delete_comparable(
        &self,
        org_id: Uuid,
        comparable_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM valuation_comparables
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(comparable_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Comparable Adjustment Operations
    // ========================================================================

    /// Create a comparable adjustment.
    pub async fn create_adjustment(
        &self,
        org_id: Uuid,
        data: &CreateAdjustment,
    ) -> Result<Option<ComparableAdjustment>, SqlxError> {
        // INSERT ... SELECT from the org's own comparable: a foreign or unknown
        // comparable inserts nothing and yields `None` (404 at the route).
        sqlx::query_as::<_, ComparableAdjustment>(
            r#"
            INSERT INTO comparable_adjustments (
                comparable_id, adjustment_type, adjustment_name,
                subject_value, comparable_value, adjustment_amount,
                adjustment_percent, justification
            )
            SELECT c.id, $2::adjustment_type, $3, $4, $5, $6, $7, $8
            FROM valuation_comparables c
            WHERE c.id = $1 AND c.organization_id = $9
            RETURNING *
            "#,
        )
        .bind(data.comparable_id)
        .bind(data.adjustment_type)
        .bind(&data.adjustment_name)
        .bind(&data.subject_value)
        .bind(&data.comparable_value)
        .bind(Decimal::try_from(data.adjustment_amount).unwrap_or_default())
        .bind(
            data.adjustment_percent
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(&data.justification)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get adjustments for a comparable of `org_id` (empty for a foreign one).
    pub async fn list_adjustments(
        &self,
        org_id: Uuid,
        comparable_id: Uuid,
    ) -> Result<Vec<ComparableAdjustment>, SqlxError> {
        sqlx::query_as::<_, ComparableAdjustment>(
            r#"
            SELECT a.* FROM comparable_adjustments a
            JOIN valuation_comparables c ON c.id = a.comparable_id
            WHERE a.comparable_id = $1 AND c.organization_id = $2
            ORDER BY a.created_at ASC
            "#,
        )
        .bind(comparable_id)
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete an adjustment whose comparable belongs to `org_id`.
    pub async fn delete_adjustment(
        &self,
        org_id: Uuid,
        adjustment_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"DELETE FROM comparable_adjustments a
               USING valuation_comparables c
               WHERE a.id = $1 AND a.comparable_id = c.id AND c.organization_id = $2"#,
        )
        .bind(adjustment_id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Market Data Operations
    // ========================================================================

    /// Create market data entry.
    pub async fn create_market_data(
        &self,
        org_id: Uuid,
        data: &CreateMarketData,
    ) -> Result<ValuationMarketData, SqlxError> {
        sqlx::query_as::<_, ValuationMarketData>(
            r#"
            INSERT INTO valuation_market_data (
                organization_id, region, city, district, neighborhood,
                postal_code, property_type, period_start, period_end,
                median_price, average_price, price_per_sqm_median,
                price_per_sqm_average, total_transactions,
                price_change_percent, market_trend, data_source
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16::market_trend, $17
            )
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&data.region)
        .bind(&data.city)
        .bind(&data.district)
        .bind(&data.neighborhood)
        .bind(&data.postal_code)
        .bind(&data.property_type)
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(
            data.median_price
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.average_price
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.price_per_sqm_median
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.price_per_sqm_average
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.total_transactions)
        .bind(
            data.price_change_percent
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.market_trend)
        .bind(&data.data_source)
        .fetch_one(&self.pool)
        .await
    }

    /// Get market data for a location.
    pub async fn get_market_data(
        &self,
        org_id: Uuid,
        city: Option<&str>,
        district: Option<&str>,
        property_type: Option<&str>,
    ) -> Result<Vec<ValuationMarketData>, SqlxError> {
        sqlx::query_as::<_, ValuationMarketData>(
            r#"
            SELECT * FROM valuation_market_data
            WHERE organization_id = $1
              AND ($2::text IS NULL OR city = $2)
              AND ($3::text IS NULL OR district = $3)
              AND ($4::text IS NULL OR property_type = $4)
            ORDER BY period_end DESC
            LIMIT 12
            "#,
        )
        .bind(org_id)
        .bind(city)
        .bind(district)
        .bind(property_type)
        .fetch_all(&self.pool)
        .await
    }

    /// Update market data.
    pub async fn update_market_data(
        &self,
        org_id: Uuid,
        market_data_id: Uuid,
        data: &UpdateMarketData,
    ) -> Result<Option<ValuationMarketData>, SqlxError> {
        sqlx::query_as::<_, ValuationMarketData>(
            r#"
            UPDATE valuation_market_data
            SET
                median_price = COALESCE($3, median_price),
                average_price = COALESCE($4, average_price),
                price_per_sqm_median = COALESCE($5, price_per_sqm_median),
                price_per_sqm_average = COALESCE($6, price_per_sqm_average),
                total_transactions = COALESCE($7, total_transactions),
                price_change_percent = COALESCE($8, price_change_percent),
                price_change_yoy = COALESCE($9, price_change_yoy),
                market_trend = COALESCE($10::market_trend, market_trend),
                active_listings = COALESCE($11, active_listings),
                months_of_supply = COALESCE($12, months_of_supply),
                is_verified = COALESCE($13, is_verified),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(market_data_id)
        .bind(
            data.median_price
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.average_price
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.price_per_sqm_median
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.price_per_sqm_average
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.total_transactions)
        .bind(
            data.price_change_percent
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.price_change_yoy
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.market_trend)
        .bind(data.active_listings)
        .bind(
            data.months_of_supply
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.is_verified)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Value History Operations
    // ========================================================================

    /// Create value history entry.
    pub async fn create_value_history(
        &self,
        org_id: Uuid,
        data: &CreateValueHistory,
    ) -> Result<PropertyValueHistory, SqlxError> {
        // Calculate change from previous value if provided
        let value_change = data.previous_value.map(|prev| data.estimated_value - prev);
        let value_change_percent = data.previous_value.map(|prev| {
            if prev != 0.0 {
                ((data.estimated_value - prev) / prev) * 100.0
            } else {
                0.0
            }
        });

        sqlx::query_as::<_, PropertyValueHistory>(
            r#"
            INSERT INTO avm_property_value_history (
                organization_id, property_id, valuation_id, record_date,
                estimated_value, price_per_sqm, confidence_level,
                previous_value, value_change, value_change_percent,
                value_source
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7::valuation_confidence, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.property_id)
        .bind(data.valuation_id)
        .bind(data.record_date)
        .bind(Decimal::try_from(data.estimated_value).unwrap_or_default())
        .bind(
            data.price_per_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.confidence_level)
        .bind(
            data.previous_value
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(value_change.map(Decimal::try_from).and_then(Result::ok))
        .bind(
            value_change_percent
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(&data.value_source)
        .fetch_one(&self.pool)
        .await
    }

    /// Get value history for a property.
    pub async fn get_value_history(
        &self,
        org_id: Uuid,
        property_id: Uuid,
        limit: i64,
    ) -> Result<Vec<PropertyValueHistory>, SqlxError> {
        sqlx::query_as::<_, PropertyValueHistory>(
            r#"
            SELECT * FROM avm_property_value_history
            WHERE organization_id = $1 AND property_id = $2
            ORDER BY record_date DESC
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(property_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // Valuation Request Operations
    // ========================================================================

    /// Create a valuation request.
    pub async fn create_request(
        &self,
        org_id: Uuid,
        data: &CreateValuationRequest,
        requested_by: Uuid,
    ) -> Result<ValuationRequest, SqlxError> {
        sqlx::query_as::<_, ValuationRequest>(
            r#"
            INSERT INTO valuation_requests (
                organization_id, property_id, request_type, purpose,
                priority, due_date, requester_notes, requested_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.property_id)
        .bind(&data.request_type)
        .bind(&data.purpose)
        .bind(data.priority)
        .bind(data.due_date)
        .bind(&data.requester_notes)
        .bind(requested_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a valuation request by ID.
    pub async fn get_request(
        &self,
        org_id: Uuid,
        request_id: Uuid,
    ) -> Result<Option<ValuationRequest>, SqlxError> {
        sqlx::query_as::<_, ValuationRequest>(
            r#"
            SELECT * FROM valuation_requests
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(request_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List valuation requests.
    pub async fn list_requests(
        &self,
        org_id: Uuid,
        status: Option<ValuationStatus>,
        assigned_to: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ValuationRequest>, SqlxError> {
        sqlx::query_as::<_, ValuationRequest>(
            r#"
            SELECT * FROM valuation_requests
            WHERE organization_id = $1
              AND ($2::valuation_status IS NULL OR status = $2)
              AND ($3::uuid IS NULL OR assigned_to = $3)
            ORDER BY priority ASC, due_date ASC NULLS LAST
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(status)
        .bind(assigned_to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a valuation request.
    pub async fn update_request(
        &self,
        org_id: Uuid,
        request_id: Uuid,
        data: &UpdateValuationRequest,
    ) -> Result<Option<ValuationRequest>, SqlxError> {
        sqlx::query_as::<_, ValuationRequest>(
            r#"
            UPDATE valuation_requests
            SET
                status = COALESCE($3::valuation_status, status),
                priority = COALESCE($4, priority),
                due_date = COALESCE($5, due_date),
                assigned_to = COALESCE($6, assigned_to),
                assigned_at = CASE WHEN $6 IS NOT NULL THEN NOW() ELSE assigned_at END,
                valuation_id = COALESCE($7, valuation_id),
                internal_notes = COALESCE($8, internal_notes),
                completed_date = CASE WHEN $3 = 'completed' THEN CURRENT_DATE ELSE completed_date END,
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(request_id)
        .bind(data.status)
        .bind(data.priority)
        .bind(data.due_date)
        .bind(data.assigned_to)
        .bind(data.valuation_id)
        .bind(&data.internal_notes)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Property Features Operations
    // ========================================================================

    /// Create property features.
    pub async fn create_features(
        &self,
        org_id: Uuid,
        data: &CreatePropertyFeatures,
        assessed_by: Uuid,
    ) -> Result<PropertyValuationFeatures, SqlxError> {
        sqlx::query_as::<_, PropertyValuationFeatures>(
            r#"
            INSERT INTO property_valuation_features (
                organization_id, property_id, total_area_sqm, living_area_sqm,
                lot_size_sqm, year_built, year_renovated, floors, bedrooms,
                bathrooms, construction_quality, interior_quality,
                exterior_quality, features, has_garage, garage_spaces,
                has_pool, has_basement, condition, condition_score, assessed_by
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19::property_condition, $20, $21
            )
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.property_id)
        .bind(
            data.total_area_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.living_area_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.lot_size_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.year_built)
        .bind(data.year_renovated)
        .bind(data.floors)
        .bind(data.bedrooms)
        .bind(data.bathrooms.map(Decimal::try_from).and_then(Result::ok))
        .bind(data.construction_quality)
        .bind(data.interior_quality)
        .bind(data.exterior_quality)
        .bind(&data.features)
        .bind(data.has_garage)
        .bind(data.garage_spaces)
        .bind(data.has_pool)
        .bind(data.has_basement)
        .bind(data.condition)
        .bind(data.condition_score)
        .bind(assessed_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get current property features.
    pub async fn get_features(
        &self,
        org_id: Uuid,
        property_id: Uuid,
    ) -> Result<Option<PropertyValuationFeatures>, SqlxError> {
        sqlx::query_as::<_, PropertyValuationFeatures>(
            r#"
            SELECT * FROM property_valuation_features
            WHERE organization_id = $1 AND property_id = $2
            ORDER BY recorded_date DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(property_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update property features.
    pub async fn update_features(
        &self,
        org_id: Uuid,
        feature_id: Uuid,
        data: &UpdatePropertyFeatures,
    ) -> Result<Option<PropertyValuationFeatures>, SqlxError> {
        sqlx::query_as::<_, PropertyValuationFeatures>(
            r#"
            UPDATE property_valuation_features
            SET
                total_area_sqm = COALESCE($3, total_area_sqm),
                living_area_sqm = COALESCE($4, living_area_sqm),
                year_renovated = COALESCE($5, year_renovated),
                construction_quality = COALESCE($6, construction_quality),
                interior_quality = COALESCE($7, interior_quality),
                exterior_quality = COALESCE($8, exterior_quality),
                features = COALESCE($9, features),
                has_garage = COALESCE($10, has_garage),
                garage_spaces = COALESCE($11, garage_spaces),
                has_pool = COALESCE($12, has_pool),
                condition = COALESCE($13::property_condition, condition),
                condition_score = COALESCE($14, condition_score),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(feature_id)
        .bind(
            data.total_area_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(
            data.living_area_sqm
                .map(Decimal::try_from)
                .and_then(Result::ok),
        )
        .bind(data.year_renovated)
        .bind(data.construction_quality)
        .bind(data.interior_quality)
        .bind(data.exterior_quality)
        .bind(&data.features)
        .bind(data.has_garage)
        .bind(data.garage_spaces)
        .bind(data.has_pool)
        .bind(data.condition)
        .bind(data.condition_score)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Valuation Report Operations
    // ========================================================================

    /// Create a valuation report.
    pub async fn create_report(
        &self,
        org_id: Uuid,
        data: &CreateValuationReport,
        generated_by: Uuid,
    ) -> Result<ValuationReport, SqlxError> {
        sqlx::query_as::<_, ValuationReport>(
            r#"
            INSERT INTO valuation_reports (
                organization_id, valuation_id, report_type, report_number,
                title, executive_summary, full_report_content, generated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.valuation_id)
        .bind(&data.report_type)
        .bind(&data.report_number)
        .bind(&data.title)
        .bind(&data.executive_summary)
        .bind(&data.full_report_content)
        .bind(generated_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get reports for a valuation.
    pub async fn list_reports(
        &self,
        org_id: Uuid,
        valuation_id: Uuid,
    ) -> Result<Vec<ValuationReport>, SqlxError> {
        sqlx::query_as::<_, ValuationReport>(
            r#"
            SELECT * FROM valuation_reports
            WHERE organization_id = $1 AND valuation_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .bind(valuation_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a report.
    pub async fn update_report(
        &self,
        org_id: Uuid,
        report_id: Uuid,
        data: &UpdateValuationReport,
    ) -> Result<Option<ValuationReport>, SqlxError> {
        sqlx::query_as::<_, ValuationReport>(
            r#"
            UPDATE valuation_reports
            SET
                title = COALESCE($3, title),
                executive_summary = COALESCE($4, executive_summary),
                full_report_content = COALESCE($5, full_report_content),
                file_path = COALESCE($6, file_path),
                file_name = COALESCE($7, file_name),
                file_size_bytes = COALESCE($8, file_size_bytes),
                is_draft = COALESCE($9, is_draft),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(report_id)
        .bind(&data.title)
        .bind(&data.executive_summary)
        .bind(&data.full_report_content)
        .bind(&data.file_path)
        .bind(&data.file_name)
        .bind(data.file_size_bytes)
        .bind(data.is_draft)
        .fetch_optional(&self.pool)
        .await
    }

    /// Sign a report.
    pub async fn sign_report(
        &self,
        org_id: Uuid,
        report_id: Uuid,
        signed_by: Uuid,
    ) -> Result<Option<ValuationReport>, SqlxError> {
        sqlx::query_as::<_, ValuationReport>(
            r#"
            UPDATE valuation_reports
            SET
                is_draft = false,
                is_signed = true,
                signed_by = $3,
                signed_at = NOW(),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(report_id)
        .bind(signed_by)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Audit Log Operations
    // ========================================================================

    /// Create an audit log entry.
    pub async fn create_audit_log(
        &self,
        org_id: Uuid,
        data: &CreateValuationAuditLog,
        performed_by: Uuid,
    ) -> Result<ValuationAuditLog, SqlxError> {
        sqlx::query_as::<_, ValuationAuditLog>(
            r#"
            INSERT INTO valuation_audit_logs (
                organization_id, valuation_id, model_id, action,
                entity_type, entity_id, old_values, new_values,
                reason, ip_address, user_agent, performed_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.valuation_id)
        .bind(data.model_id)
        .bind(&data.action)
        .bind(&data.entity_type)
        .bind(data.entity_id)
        .bind(&data.old_values)
        .bind(&data.new_values)
        .bind(&data.reason)
        .bind(&data.ip_address)
        .bind(&data.user_agent)
        .bind(performed_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get audit logs for a valuation.
    pub async fn get_audit_logs(
        &self,
        org_id: Uuid,
        valuation_id: Uuid,
        limit: i64,
    ) -> Result<Vec<ValuationAuditLog>, SqlxError> {
        sqlx::query_as::<_, ValuationAuditLog>(
            r#"
            SELECT * FROM valuation_audit_logs
            WHERE organization_id = $1 AND valuation_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(valuation_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // Dashboard and Analytics Operations
    // ========================================================================

    /// Get valuation dashboard summary.
    pub async fn get_dashboard(&self, org_id: Uuid) -> Result<ValuationDashboard, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_valuations,
                COUNT(*) FILTER (WHERE status = 'pending_review' OR status = 'in_progress') as pending_valuations,
                COUNT(*) FILTER (WHERE status = 'completed' OR status = 'approved') as completed_valuations,
                COUNT(*) FILTER (WHERE created_at >= DATE_TRUNC('month', CURRENT_DATE)) as valuations_this_month,
                COUNT(*) FILTER (WHERE expiration_date IS NOT NULL AND expiration_date <= CURRENT_DATE + INTERVAL '30 days') as expiring_soon
            FROM avm_property_valuations
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let portfolio_row = sqlx::query(
            r#"
            SELECT SUM(estimated_value) as total_value
            FROM (
                SELECT DISTINCT ON (property_id) estimated_value
                FROM avm_property_valuations
                WHERE organization_id = $1 AND status IN ('completed', 'approved')
                ORDER BY property_id, valuation_date DESC
            ) latest_valuations
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let pending_requests_row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM valuation_requests
            WHERE organization_id = $1 AND status IN ('pending_review', 'in_progress')
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ValuationDashboard {
            total_valuations: row.get::<i64, _>("total_valuations"),
            pending_valuations: row.get::<i64, _>("pending_valuations"),
            completed_valuations: row.get::<i64, _>("completed_valuations"),
            average_confidence: None,
            total_portfolio_value: portfolio_row
                .get::<Option<Decimal>, _>("total_value")
                .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
            valuations_this_month: row.get::<i64, _>("valuations_this_month"),
            expiring_soon: row.get::<i64, _>("expiring_soon"),
            pending_requests: pending_requests_row.get::<i64, _>("count"),
        })
    }

    /// Get expiring valuations.
    pub async fn get_expiring_valuations(
        &self,
        org_id: Uuid,
        days: i32,
    ) -> Result<Vec<PropertyValuation>, SqlxError> {
        sqlx::query_as::<_, PropertyValuation>(
            r#"
            SELECT * FROM avm_property_valuations
            WHERE organization_id = $1
              AND expiration_date IS NOT NULL
              AND expiration_date <= CURRENT_DATE + ($2 || ' days')::INTERVAL
              AND status IN ('completed', 'approved')
            ORDER BY expiration_date ASC
            "#,
        )
        .bind(org_id)
        .bind(days.to_string())
        .fetch_all(&self.pool)
        .await
    }
}

//! Epic 132: Dynamic Rent Pricing & Market Analytics repository.
//! Provides database operations for market data, pricing recommendations, and CMA.

use crate::models::market_pricing::*;
use crate::DbPool;
use common::errors::AppError;
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct MarketPricingRepository {
    pool: DbPool,
}

impl MarketPricingRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ======================== Market Regions (Story 132.1) ========================

    pub async fn create_region(
        &self,
        org_id: Uuid,
        req: CreateMarketRegion,
    ) -> Result<MarketRegion, AppError> {
        let region = sqlx::query_as::<_, MarketRegion>(
            r#"
            INSERT INTO market_regions (organization_id, name, country_code, city, postal_codes,
                                        center_lat, center_lng, radius_km)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, organization_id, name, country_code, city, postal_codes,
                      center_lat, center_lng, radius_km, is_active, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.country_code)
        .bind(&req.city)
        .bind(&req.postal_codes)
        .bind(req.center_lat)
        .bind(req.center_lng)
        .bind(req.radius_km)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(region)
    }

    pub async fn get_region(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<MarketRegion>, AppError> {
        let region = sqlx::query_as::<_, MarketRegion>(
            r#"
            SELECT id, organization_id, name, country_code, city, postal_codes,
                   center_lat, center_lng, radius_km, is_active, created_at, updated_at
            FROM market_regions
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(region)
    }

    pub async fn list_regions(&self, org_id: Uuid) -> Result<Vec<MarketRegion>, AppError> {
        let regions = sqlx::query_as::<_, MarketRegion>(
            r#"
            SELECT id, organization_id, name, country_code, city, postal_codes,
                   center_lat, center_lng, radius_km, is_active, created_at, updated_at
            FROM market_regions
            WHERE organization_id = $1
            ORDER BY name
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(regions)
    }

    pub async fn update_region(
        &self,
        id: Uuid,
        org_id: Uuid,
        req: UpdateMarketRegion,
    ) -> Result<Option<MarketRegion>, AppError> {
        let region = sqlx::query_as::<_, MarketRegion>(
            r#"
            UPDATE market_regions
            SET name = COALESCE($3, name),
                postal_codes = COALESCE($4, postal_codes),
                center_lat = COALESCE($5, center_lat),
                center_lng = COALESCE($6, center_lng),
                radius_km = COALESCE($7, radius_km),
                is_active = COALESCE($8, is_active),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING id, organization_id, name, country_code, city, postal_codes,
                      center_lat, center_lng, radius_km, is_active, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.postal_codes)
        .bind(req.center_lat)
        .bind(req.center_lng)
        .bind(req.radius_km)
        .bind(req.is_active)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(region)
    }

    pub async fn delete_region(&self, id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM market_regions WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ======================== Market Data Points (Story 132.1) ========================

    pub async fn add_data_point(
        &self,
        req: CreateMarketDataPoint,
    ) -> Result<MarketDataPoint, AppError> {
        let data_point = sqlx::query_as::<_, MarketDataPoint>(
            r#"
            INSERT INTO market_data_points (
                region_id, source, source_reference, property_type, size_sqm, rooms, bathrooms,
                floor, has_parking, has_balcony, has_elevator, year_built, monthly_rent, currency,
                latitude, longitude, postal_code, district, listing_date, days_on_market,
                is_furnished, amenities
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
            RETURNING id, region_id, collected_at, source, source_reference, property_type, size_sqm,
                      rooms, bathrooms, floor, has_parking, has_balcony, has_elevator, year_built,
                      monthly_rent, currency, price_per_sqm, latitude, longitude, postal_code, district,
                      listing_date, days_on_market, is_furnished, amenities, created_at
            "#,
        )
        .bind(req.region_id)
        .bind(&req.source)
        .bind(&req.source_reference)
        .bind(&req.property_type)
        .bind(req.size_sqm)
        .bind(req.rooms)
        .bind(req.bathrooms)
        .bind(req.floor)
        .bind(req.has_parking)
        .bind(req.has_balcony)
        .bind(req.has_elevator)
        .bind(req.year_built)
        .bind(req.monthly_rent)
        .bind(&req.currency)
        .bind(req.latitude)
        .bind(req.longitude)
        .bind(&req.postal_code)
        .bind(&req.district)
        .bind(req.listing_date)
        .bind(req.days_on_market)
        .bind(req.is_furnished)
        .bind(&req.amenities)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(data_point)
    }

    pub async fn list_data_points(
        &self,
        org_id: Uuid,
        query: MarketDataQuery,
    ) -> Result<Vec<MarketDataPoint>, AppError> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.page.unwrap_or(0) * limit;

        let data_points = sqlx::query_as::<_, MarketDataPoint>(
            r#"
            SELECT mdp.id, mdp.region_id, mdp.collected_at, mdp.source, mdp.source_reference,
                   mdp.property_type, mdp.size_sqm, mdp.rooms, mdp.bathrooms, mdp.floor,
                   mdp.has_parking, mdp.has_balcony, mdp.has_elevator, mdp.year_built,
                   mdp.monthly_rent, mdp.currency, mdp.price_per_sqm, mdp.latitude, mdp.longitude,
                   mdp.postal_code, mdp.district, mdp.listing_date, mdp.days_on_market,
                   mdp.is_furnished, mdp.amenities, mdp.created_at
            FROM market_data_points mdp
            JOIN market_regions mr ON mr.id = mdp.region_id
            WHERE mr.organization_id = $1
              AND ($2::uuid IS NULL OR mdp.region_id = $2)
              AND ($3::text IS NULL OR mdp.property_type = $3)
              AND ($4::numeric IS NULL OR mdp.size_sqm >= $4)
              AND ($5::numeric IS NULL OR mdp.size_sqm <= $5)
              AND ($6::numeric IS NULL OR mdp.monthly_rent >= $6)
              AND ($7::numeric IS NULL OR mdp.monthly_rent <= $7)
              AND ($8::int IS NULL OR mdp.rooms = $8)
              AND ($9::timestamptz IS NULL OR mdp.collected_at >= $9)
              AND ($10::timestamptz IS NULL OR mdp.collected_at <= $10)
            ORDER BY mdp.collected_at DESC
            LIMIT $11 OFFSET $12
            "#,
        )
        .bind(org_id)
        .bind(query.region_id)
        .bind(&query.property_type)
        .bind(query.min_size_sqm)
        .bind(query.max_size_sqm)
        .bind(query.min_rent)
        .bind(query.max_rent)
        .bind(query.rooms)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(data_points)
    }

    // ======================== Market Statistics (Story 132.1) ========================

    pub async fn get_market_statistics(
        &self,
        region_id: Uuid,
        property_type: Option<String>,
    ) -> Result<Option<MarketStatistics>, AppError> {
        let stats = sqlx::query_as::<_, MarketStatistics>(
            r#"
            SELECT id, region_id, property_type, period_start, period_end,
                   avg_rent, median_rent, min_rent, max_rent,
                   avg_price_per_sqm, median_price_per_sqm,
                   vacancy_rate, avg_days_on_market, sample_size,
                   rent_change_pct, rent_change_vs_last_year, currency, created_at
            FROM market_statistics
            WHERE region_id = $1
              AND ($2::text IS NULL OR property_type = $2)
            ORDER BY period_end DESC
            LIMIT 1
            "#,
        )
        .bind(region_id)
        .bind(&property_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(stats)
    }

    pub async fn get_market_statistics_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<MarketStatistics>, AppError> {
        let stats = sqlx::query_as::<_, MarketStatistics>(
            r#"
            SELECT id, region_id, property_type, period_start, period_end,
                   avg_rent, median_rent, min_rent, max_rent,
                   avg_price_per_sqm, median_price_per_sqm,
                   vacancy_rate, avg_days_on_market, sample_size,
                   rent_change_pct, rent_change_vs_last_year, currency, created_at
            FROM market_statistics
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(stats)
    }

    pub async fn generate_statistics(
        &self,
        req: GenerateStatisticsRequest,
    ) -> Result<MarketStatistics, AppError> {
        // Calculate statistics from data points
        let stats = sqlx::query_as::<_, MarketStatistics>(
            r#"
            WITH data AS (
                SELECT monthly_rent, price_per_sqm, days_on_market
                FROM market_data_points
                WHERE region_id = $1
                  AND ($2::text IS NULL OR property_type = $2)
                  AND collected_at >= $3
                  AND collected_at <= $4
            ),
            aggregates AS (
                SELECT
                    AVG(monthly_rent) as avg_rent,
                    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY monthly_rent) as median_rent,
                    MIN(monthly_rent) as min_rent,
                    MAX(monthly_rent) as max_rent,
                    AVG(price_per_sqm) as avg_price_per_sqm,
                    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY price_per_sqm) as median_price_per_sqm,
                    AVG(days_on_market) as avg_days_on_market,
                    COUNT(*) as sample_size
                FROM data
            )
            INSERT INTO market_statistics (
                region_id, property_type, period_start, period_end,
                avg_rent, median_rent, min_rent, max_rent,
                avg_price_per_sqm, median_price_per_sqm, avg_days_on_market, sample_size
            )
            SELECT
                $1, COALESCE($2, 'all'), $3, $4,
                COALESCE(a.avg_rent, 0), COALESCE(a.median_rent, 0),
                COALESCE(a.min_rent, 0), COALESCE(a.max_rent, 0),
                COALESCE(a.avg_price_per_sqm, 0), COALESCE(a.median_price_per_sqm, 0),
                a.avg_days_on_market, COALESCE(a.sample_size, 0)::int
            FROM aggregates a
            ON CONFLICT (region_id, property_type, period_start, period_end)
            DO UPDATE SET
                avg_rent = EXCLUDED.avg_rent,
                median_rent = EXCLUDED.median_rent,
                min_rent = EXCLUDED.min_rent,
                max_rent = EXCLUDED.max_rent,
                avg_price_per_sqm = EXCLUDED.avg_price_per_sqm,
                median_price_per_sqm = EXCLUDED.median_price_per_sqm,
                avg_days_on_market = EXCLUDED.avg_days_on_market,
                sample_size = EXCLUDED.sample_size,
                created_at = NOW()
            RETURNING id, region_id, property_type, period_start, period_end,
                      avg_rent, median_rent, min_rent, max_rent,
                      avg_price_per_sqm, median_price_per_sqm,
                      vacancy_rate, avg_days_on_market, sample_size,
                      rent_change_pct, rent_change_vs_last_year, currency, created_at
            "#,
        )
        .bind(req.region_id)
        .bind(&req.property_type)
        .bind(req.period_start)
        .bind(req.period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(stats)
    }

    // ======================== Pricing Recommendations (Story 132.2) ========================

    #[allow(clippy::too_many_arguments)]
    pub async fn create_recommendation(
        &self,
        unit_id: Uuid,
        min_price: Decimal,
        optimal_price: Decimal,
        max_price: Decimal,
        currency: &str,
        confidence_score: Decimal,
        factors: serde_json::Value,
        comparables_count: i32,
        market_stats_id: Option<Uuid>,
    ) -> Result<PricingRecommendation, AppError> {
        let recommendation = sqlx::query_as::<_, PricingRecommendation>(
            r#"
            INSERT INTO pricing_recommendations (
                unit_id, min_price, optimal_price, max_price, currency,
                confidence_score, factors, comparables_count, market_stats_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, unit_id, generated_at, min_price, optimal_price, max_price, currency,
                      confidence_score, status, expires_at, factors, comparables_count, market_stats_id,
                      accepted_price, accepted_at, accepted_by, rejection_reason, created_at, updated_at
            "#,
        )
        .bind(unit_id)
        .bind(min_price)
        .bind(optimal_price)
        .bind(max_price)
        .bind(currency)
        .bind(confidence_score)
        .bind(&factors)
        .bind(comparables_count)
        .bind(market_stats_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(recommendation)
    }

    pub async fn get_recommendation(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PricingRecommendation>, AppError> {
        let recommendation = sqlx::query_as::<_, PricingRecommendation>(
            r#"
            SELECT pr.id, pr.unit_id, pr.generated_at, pr.min_price, pr.optimal_price, pr.max_price,
                   pr.currency, pr.confidence_score, pr.status, pr.expires_at, pr.factors,
                   pr.comparables_count, pr.market_stats_id, pr.accepted_price, pr.accepted_at,
                   pr.accepted_by, pr.rejection_reason, pr.created_at, pr.updated_at
            FROM pricing_recommendations pr
            JOIN units u ON u.id = pr.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE pr.id = $1 AND b.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(recommendation)
    }

    pub async fn get_latest_recommendation_for_unit(
        &self,
        unit_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PricingRecommendation>, AppError> {
        let recommendation = sqlx::query_as::<_, PricingRecommendation>(
            r#"
            SELECT pr.id, pr.unit_id, pr.generated_at, pr.min_price, pr.optimal_price, pr.max_price,
                   pr.currency, pr.confidence_score, pr.status, pr.expires_at, pr.factors,
                   pr.comparables_count, pr.market_stats_id, pr.accepted_price, pr.accepted_at,
                   pr.accepted_by, pr.rejection_reason, pr.created_at, pr.updated_at
            FROM pricing_recommendations pr
            JOIN units u ON u.id = pr.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE pr.unit_id = $1 AND b.organization_id = $2
            ORDER BY pr.generated_at DESC
            LIMIT 1
            "#,
        )
        .bind(unit_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(recommendation)
    }

    pub async fn list_pending_recommendations(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<PricingRecommendationSummary>, AppError> {
        let recommendations = sqlx::query_as::<_, PricingRecommendationSummary>(
            r#"
            SELECT pr.id, pr.unit_id, pr.optimal_price, pr.confidence_score, pr.status,
                   pr.generated_at, pr.expires_at
            FROM pricing_recommendations pr
            JOIN units u ON u.id = pr.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE b.organization_id = $1
              AND pr.status = 'pending'
              AND pr.expires_at > NOW()
            ORDER BY pr.confidence_score DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(recommendations)
    }

    pub async fn accept_recommendation(
        &self,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        accepted_price: Decimal,
    ) -> Result<Option<PricingRecommendation>, AppError> {
        let recommendation = sqlx::query_as::<_, PricingRecommendation>(
            r#"
            UPDATE pricing_recommendations pr
            SET status = 'accepted',
                accepted_price = $3,
                accepted_at = NOW(),
                accepted_by = $4,
                updated_at = NOW()
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            WHERE pr.id = $1 AND u.id = pr.unit_id AND b.organization_id = $2
            RETURNING pr.id, pr.unit_id, pr.generated_at, pr.min_price, pr.optimal_price, pr.max_price,
                      pr.currency, pr.confidence_score, pr.status, pr.expires_at, pr.factors,
                      pr.comparables_count, pr.market_stats_id, pr.accepted_price, pr.accepted_at,
                      pr.accepted_by, pr.rejection_reason, pr.created_at, pr.updated_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(accepted_price)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(recommendation)
    }

    pub async fn reject_recommendation(
        &self,
        id: Uuid,
        org_id: Uuid,
        reason: &str,
    ) -> Result<Option<PricingRecommendation>, AppError> {
        let recommendation = sqlx::query_as::<_, PricingRecommendation>(
            r#"
            UPDATE pricing_recommendations pr
            SET status = 'rejected',
                rejection_reason = $3,
                updated_at = NOW()
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            WHERE pr.id = $1 AND u.id = pr.unit_id AND b.organization_id = $2
            RETURNING pr.id, pr.unit_id, pr.generated_at, pr.min_price, pr.optimal_price, pr.max_price,
                      pr.currency, pr.confidence_score, pr.status, pr.expires_at, pr.factors,
                      pr.comparables_count, pr.market_stats_id, pr.accepted_price, pr.accepted_at,
                      pr.accepted_by, pr.rejection_reason, pr.created_at, pr.updated_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(recommendation)
    }

    // ======================== Unit Pricing History (Story 132.3) ========================

    pub async fn record_price_change(
        &self,
        req: RecordPriceChange,
        user_id: Uuid,
    ) -> Result<UnitPricingHistory, AppError> {
        // Close any active pricing record
        sqlx::query(
            r#"
            UPDATE unit_pricing_history
            SET end_date = $2
            WHERE unit_id = $1 AND end_date IS NULL
            "#,
        )
        .bind(req.unit_id)
        .bind(req.effective_date)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Create new pricing record
        let history = sqlx::query_as::<_, UnitPricingHistory>(
            r#"
            INSERT INTO unit_pricing_history (
                unit_id, effective_date, monthly_rent, currency, recommendation_id, change_reason, changed_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, unit_id, effective_date, end_date, monthly_rent, currency,
                      recommendation_id, change_reason, changed_by, created_at
            "#,
        )
        .bind(req.unit_id)
        .bind(req.effective_date)
        .bind(req.monthly_rent)
        .bind(&req.currency)
        .bind(req.recommendation_id)
        .bind(&req.change_reason)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(history)
    }

    pub async fn get_pricing_history(
        &self,
        unit_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<UnitPricingHistory>, AppError> {
        let history = sqlx::query_as::<_, UnitPricingHistory>(
            r#"
            SELECT uph.id, uph.unit_id, uph.effective_date, uph.end_date, uph.monthly_rent,
                   uph.currency, uph.recommendation_id, uph.change_reason, uph.changed_by, uph.created_at
            FROM unit_pricing_history uph
            JOIN units u ON u.id = uph.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE uph.unit_id = $1 AND b.organization_id = $2
            ORDER BY uph.effective_date DESC
            "#,
        )
        .bind(unit_id)
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(history)
    }

    pub async fn get_current_rent(&self, unit_id: Uuid) -> Result<Option<Decimal>, AppError> {
        let rent = sqlx::query_scalar::<_, Decimal>(
            r#"
            SELECT monthly_rent
            FROM unit_pricing_history
            WHERE unit_id = $1 AND end_date IS NULL
            ORDER BY effective_date DESC
            LIMIT 1
            "#,
        )
        .bind(unit_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rent)
    }

    // ======================== Comparative Market Analysis (Story 132.4) ========================

    pub async fn create_cma(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateComparativeMarketAnalysis,
    ) -> Result<ComparativeMarketAnalysis, AppError> {
        let cma = sqlx::query_as::<_, ComparativeMarketAnalysis>(
            r#"
            INSERT INTO comparative_market_analyses (
                organization_id, created_by, name, region_id, property_type, properties_compared, analysis_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, organization_id, created_by, name, region_id, property_type,
                      avg_price_per_sqm, avg_rental_yield, appreciation_trend,
                      analysis_data, properties_compared, is_archived, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(&req.name)
        .bind(req.region_id)
        .bind(&req.property_type)
        .bind(&req.properties_to_compare)
        .bind(json!({}))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(cma)
    }

    pub async fn get_cma(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<ComparativeMarketAnalysis>, AppError> {
        let cma = sqlx::query_as::<_, ComparativeMarketAnalysis>(
            r#"
            SELECT id, organization_id, created_by, name, region_id, property_type,
                   avg_price_per_sqm, avg_rental_yield, appreciation_trend,
                   analysis_data, properties_compared, is_archived, created_at, updated_at
            FROM comparative_market_analyses
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(cma)
    }

    pub async fn list_cmas(&self, org_id: Uuid) -> Result<Vec<CmaSummary>, AppError> {
        let cmas = sqlx::query_as::<_, CmaSummary>(
            r#"
            SELECT id, name, property_type, avg_price_per_sqm, avg_rental_yield,
                   COALESCE(array_length(properties_compared, 1), 0) as properties_count,
                   created_at
            FROM comparative_market_analyses
            WHERE organization_id = $1 AND is_archived = false
            ORDER BY created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(cmas)
    }

    pub async fn add_property_to_cma(
        &self,
        cma_id: Uuid,
        req: AddCmaProperty,
    ) -> Result<CmaPropertyComparison, AppError> {
        // Calculate price_per_sqm and rental_yield if possible
        let price_per_sqm = if let Some(rent) = req.monthly_rent {
            Some(rent / req.size_sqm)
        } else if let Some(price) = req.sale_price {
            Some(price / req.size_sqm)
        } else {
            None
        };

        let rental_yield = match (req.monthly_rent, req.sale_price) {
            (Some(rent), Some(price)) if price > Decimal::ZERO => {
                Some((rent * Decimal::new(12, 0) / price) * Decimal::new(100, 0))
            }
            _ => None,
        };

        let comparison = sqlx::query_as::<_, CmaPropertyComparison>(
            r#"
            INSERT INTO cma_property_comparisons (
                cma_id, unit_id, address, property_type, size_sqm, rooms, year_built,
                monthly_rent, sale_price, price_per_sqm, rental_yield, currency,
                distance_km, notes, source, source_url
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING id, cma_id, unit_id, address, property_type, size_sqm, rooms, year_built,
                      monthly_rent, sale_price, price_per_sqm, rental_yield, currency,
                      distance_km, similarity_score, notes, source, source_url, created_at
            "#,
        )
        .bind(cma_id)
        .bind(req.unit_id)
        .bind(&req.address)
        .bind(&req.property_type)
        .bind(req.size_sqm)
        .bind(req.rooms)
        .bind(req.year_built)
        .bind(req.monthly_rent)
        .bind(req.sale_price)
        .bind(price_per_sqm)
        .bind(rental_yield)
        .bind(&req.currency)
        .bind(req.distance_km)
        .bind(&req.notes)
        .bind(&req.source)
        .bind(&req.source_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(comparison)
    }

    pub async fn get_cma_properties(
        &self,
        cma_id: Uuid,
    ) -> Result<Vec<CmaPropertyComparison>, AppError> {
        let properties = sqlx::query_as::<_, CmaPropertyComparison>(
            r#"
            SELECT id, cma_id, unit_id, address, property_type, size_sqm, rooms, year_built,
                   monthly_rent, sale_price, price_per_sqm, rental_yield, currency,
                   distance_km, similarity_score, notes, source, source_url, created_at
            FROM cma_property_comparisons
            WHERE cma_id = $1
            ORDER BY price_per_sqm DESC NULLS LAST
            "#,
        )
        .bind(cma_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(properties)
    }

    pub async fn update_cma(
        &self,
        id: Uuid,
        org_id: Uuid,
        req: UpdateComparativeMarketAnalysis,
    ) -> Result<Option<ComparativeMarketAnalysis>, AppError> {
        let cma = sqlx::query_as::<_, ComparativeMarketAnalysis>(
            r#"
            UPDATE comparative_market_analyses
            SET name = COALESCE($3, name),
                is_archived = COALESCE($4, is_archived),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING id, organization_id, created_by, name, region_id, property_type,
                      avg_price_per_sqm, avg_rental_yield, appreciation_trend,
                      analysis_data, properties_compared, is_archived, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&req.name)
        .bind(req.is_archived)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(cma)
    }

    pub async fn delete_cma(&self, id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query(
            "DELETE FROM comparative_market_analyses WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ======================== CMA With Details (Story 132.4) ========================

    /// Get a full CMA with all properties and computed analysis summary.
    /// Story 132.4: Comparative Market Analysis Tool with aggregated metrics.
    pub async fn get_cma_with_details(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<CmaWithProperties>, AppError> {
        // Get the CMA
        let cma = match self.get_cma(id, org_id).await? {
            Some(c) => c,
            None => return Ok(None),
        };

        // Get all properties
        let properties = self.get_cma_properties(id).await?;

        // Compute analysis summary
        let summary = self.compute_cma_summary(&properties);

        Ok(Some(CmaWithProperties {
            analysis: cma,
            properties,
            summary,
        }))
    }

    /// Compute analysis summary from CMA properties.
    fn compute_cma_summary(&self, properties: &[CmaPropertyComparison]) -> CmaAnalysisSummary {
        if properties.is_empty() {
            return CmaAnalysisSummary {
                total_properties: 0,
                avg_size_sqm: Decimal::ZERO,
                avg_rent: None,
                avg_price_per_sqm: None,
                price_range: None,
                rental_yield_range: None,
            };
        }

        let total = properties.len() as i32;

        // Calculate average size
        let total_size: Decimal = properties.iter().map(|p| p.size_sqm).sum();
        let avg_size_sqm = total_size / Decimal::from(total);

        // Calculate average rent (only from properties with rent)
        let rents: Vec<Decimal> = properties.iter().filter_map(|p| p.monthly_rent).collect();
        let avg_rent = if !rents.is_empty() {
            let sum: Decimal = rents.iter().sum();
            Some(sum / Decimal::from(rents.len() as i32))
        } else {
            None
        };

        // Calculate average price per sqm
        let prices_per_sqm: Vec<Decimal> =
            properties.iter().filter_map(|p| p.price_per_sqm).collect();
        let avg_price_per_sqm = if !prices_per_sqm.is_empty() {
            let sum: Decimal = prices_per_sqm.iter().sum();
            Some(sum / Decimal::from(prices_per_sqm.len() as i32))
        } else {
            None
        };

        // Calculate price range
        let price_range = if !prices_per_sqm.is_empty() {
            let min = prices_per_sqm
                .iter()
                .min()
                .cloned()
                .unwrap_or(Decimal::ZERO);
            let max = prices_per_sqm
                .iter()
                .max()
                .cloned()
                .unwrap_or(Decimal::ZERO);
            Some(PriceRange { min, max })
        } else {
            None
        };

        // Calculate rental yield range
        let yields: Vec<Decimal> = properties.iter().filter_map(|p| p.rental_yield).collect();
        let rental_yield_range = if !yields.is_empty() {
            let min = yields.iter().min().cloned().unwrap_or(Decimal::ZERO);
            let max = yields.iter().max().cloned().unwrap_or(Decimal::ZERO);
            Some(YieldRange { min, max })
        } else {
            None
        };

        CmaAnalysisSummary {
            total_properties: total,
            avg_size_sqm,
            avg_rent,
            avg_price_per_sqm,
            price_range,
            rental_yield_range,
        }
    }

    /// Remove a property from a CMA.
    pub async fn remove_property_from_cma(
        &self,
        cma_id: Uuid,
        property_id: Uuid,
    ) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM cma_property_comparisons WHERE id = $1 AND cma_id = $2")
                .bind(property_id)
                .bind(cma_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// Recalculate and update CMA aggregate metrics based on properties.
    pub async fn recalculate_cma_metrics(&self, cma_id: Uuid) -> Result<(), AppError> {
        let properties = self.get_cma_properties(cma_id).await?;
        let summary = self.compute_cma_summary(&properties);

        sqlx::query(
            r#"
            UPDATE comparative_market_analyses
            SET avg_price_per_sqm = $2,
                avg_rental_yield = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(cma_id)
        .bind(summary.avg_price_per_sqm)
        .bind(
            summary
                .rental_yield_range
                .map(|r| (r.min + r.max) / Decimal::new(2, 0)),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ======================== Dashboard (Story 132.3) ========================

    /// Get market comparables for pricing analysis.
    ///
    /// # Arguments
    /// * `region_id` - The market region to search in
    /// * `property_type` - Type of property (apartment, house, etc.)
    /// * `size_sqm` - Size in square meters for similarity matching
    /// * `limit` - Maximum number of comparables to return
    /// * `max_age_days` - Maximum age of market data to consider (default: 90 days)
    pub async fn get_market_comparables(
        &self,
        region_id: Uuid,
        property_type: &str,
        size_sqm: Decimal,
        limit: i32,
        max_age_days: Option<i32>,
    ) -> Result<Vec<MarketComparable>, AppError> {
        let age_days = max_age_days.unwrap_or(90);
        let comparables = sqlx::query_as::<_, MarketComparable>(
            r#"
            SELECT
                COALESCE(district, postal_code, 'Unknown') as address,
                property_type,
                size_sqm,
                monthly_rent,
                price_per_sqm,
                0.0::numeric as distance_km,
                -- Continuous similarity score based on size difference with exponential decay
                (100 * EXP(-ABS(size_sqm - $3) / GREATEST($3 * 0.2, 10)))::numeric as similarity_score
            FROM market_data_points
            WHERE region_id = $1 AND property_type = $2
              AND collected_at > NOW() - ($5 || ' days')::interval
            ORDER BY ABS(size_sqm - $3)
            LIMIT $4
            "#,
        )
        .bind(region_id)
        .bind(property_type)
        .bind(size_sqm)
        .bind(limit)
        .bind(age_days)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(comparables)
    }

    // ======================== Dashboard (Story 132.3) ========================

    /// Get market trends over time for a region.
    pub async fn get_market_trends(
        &self,
        region_id: Uuid,
        months: i32,
    ) -> Result<Vec<MarketTrendPoint>, AppError> {
        // Generate monthly trend data from market statistics
        let trends = sqlx::query_as::<_, MarketTrendPoint>(
            r#"
            SELECT
                period_end as date,
                avg_rent,
                avg_price_per_sqm
            FROM market_statistics
            WHERE region_id = $1
              AND period_end >= NOW() - ($2 || ' months')::interval
            ORDER BY period_end ASC
            "#,
        )
        .bind(region_id)
        .bind(months)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(trends)
    }

    /// Get units with active pricing recommendations.
    pub async fn get_units_with_recommendations(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<Vec<UnitRecommendationSummary>, AppError> {
        let summaries = sqlx::query_as::<_, UnitRecommendationSummary>(
            r#"
            SELECT
                pr.unit_id,
                COALESCE(u.designation, 'Unit') as unit_name,
                COALESCE(
                    (SELECT monthly_rent FROM unit_pricing_history
                     WHERE unit_id = u.id AND end_date IS NULL
                     ORDER BY effective_date DESC LIMIT 1),
                    0
                ) as current_rent,
                pr.optimal_price as recommended_rent,
                CASE
                    WHEN COALESCE(
                        (SELECT monthly_rent FROM unit_pricing_history
                         WHERE unit_id = u.id AND end_date IS NULL
                         ORDER BY effective_date DESC LIMIT 1),
                        0
                    ) > 0 THEN
                        ((pr.optimal_price - COALESCE(
                            (SELECT monthly_rent FROM unit_pricing_history
                             WHERE unit_id = u.id AND end_date IS NULL
                             ORDER BY effective_date DESC LIMIT 1),
                            0
                        )) / COALESCE(
                            (SELECT monthly_rent FROM unit_pricing_history
                             WHERE unit_id = u.id AND end_date IS NULL
                             ORDER BY effective_date DESC LIMIT 1),
                            1
                        )) * 100
                    ELSE 0
                END as difference_pct,
                pr.confidence_score
            FROM pricing_recommendations pr
            JOIN units u ON u.id = pr.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE b.organization_id = $1
              AND pr.status = 'pending'
              AND pr.expires_at > NOW()
              AND ($2::uuid IS NULL OR b.id = $2)
            ORDER BY pr.confidence_score DESC
            LIMIT 50
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(summaries)
    }

    /// Get portfolio pricing summary.
    pub async fn get_portfolio_summary(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<PortfolioPricingSummary, AppError> {
        // Get unit count
        let total_units: i64 = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT COUNT(*)
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            WHERE b.organization_id = $1
              AND u.status = 'active'
              AND ($2::uuid IS NULL OR b.id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0);

        // Get average current rent from pricing history
        let avg_rent: Decimal = sqlx::query_scalar::<_, Option<Decimal>>(
            r#"
            SELECT COALESCE(AVG(uph.monthly_rent), 0)::numeric
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            INNER JOIN LATERAL (
                SELECT monthly_rent
                FROM unit_pricing_history
                WHERE unit_id = u.id AND end_date IS NULL
                ORDER BY effective_date DESC
                LIMIT 1
            ) uph ON true
            WHERE b.organization_id = $1
              AND u.status = 'active'
              AND ($2::uuid IS NULL OR b.id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(Decimal::ZERO);

        // Get average recommended rent from recommendations
        let market_avg_rent: Decimal = sqlx::query_scalar::<_, Option<Decimal>>(
            r#"
            SELECT COALESCE(AVG(pr.optimal_price), $3)::numeric
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            INNER JOIN LATERAL (
                SELECT optimal_price
                FROM pricing_recommendations
                WHERE unit_id = u.id AND status = 'pending' AND expires_at > NOW()
                ORDER BY generated_at DESC
                LIMIT 1
            ) pr ON true
            WHERE b.organization_id = $1
              AND u.status = 'active'
              AND ($2::uuid IS NULL OR b.id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(avg_rent) // fallback to current avg if no recommendations
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(avg_rent);

        // Calculate units below/above market based on pricing recommendations
        let (units_below_market, units_above_market): (i64, i64) = sqlx::query_as::<_, (i64, i64)>(
            r#"
                SELECT
                    COALESCE(SUM(CASE
                        WHEN pr.optimal_price > u.current_rent THEN 1 ELSE 0
                    END), 0)::bigint AS units_below_market,
                    COALESCE(SUM(CASE
                        WHEN pr.optimal_price < u.current_rent THEN 1 ELSE 0
                    END), 0)::bigint AS units_above_market
                FROM units u
                JOIN buildings b ON b.id = u.building_id
                LEFT JOIN LATERAL (
                    SELECT optimal_price
                    FROM pricing_recommendations
                    WHERE unit_id = u.id
                      AND status = 'pending'
                      AND expires_at > NOW()
                    ORDER BY generated_at DESC
                    LIMIT 1
                ) pr ON true
                WHERE b.organization_id = $1
                  AND u.status = 'active'
                  AND ($2::uuid IS NULL OR b.id = $2)
                  AND pr.optimal_price IS NOT NULL
                "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0, 0));

        let units_below_market = units_below_market as i32;
        let units_above_market = units_above_market as i32;

        // Calculate portfolio vs market percentage
        let portfolio_vs_market_pct = if market_avg_rent > Decimal::ZERO {
            ((avg_rent - market_avg_rent) / market_avg_rent) * Decimal::new(100, 0)
        } else {
            Decimal::ZERO
        };

        // Calculate potential revenue increase (for units below market)
        let potential_revenue_increase = if market_avg_rent > avg_rent {
            (market_avg_rent - avg_rent) * Decimal::from(units_below_market as u32)
        } else {
            Decimal::ZERO
        };

        Ok(PortfolioPricingSummary {
            total_units: total_units as i32,
            avg_rent,
            market_avg_rent,
            portfolio_vs_market_pct,
            units_below_market,
            units_above_market,
            potential_revenue_increase,
        })
    }

    /// Get vacancy trend data.
    ///
    /// Note: Currently returns the current vacancy rate for all months as historical
    /// occupancy tracking is not yet implemented. Future versions should track
    /// occupancy_status changes over time to provide actual historical trends.
    pub async fn get_vacancy_trends(
        &self,
        org_id: Uuid,
        months: i32,
    ) -> Result<Vec<VacancyTrendPoint>, AppError> {
        // Get current vacancy rate - future: query historical occupancy_status changes
        let current_vacancy: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT (COUNT(CASE WHEN u.occupancy_status = 'vacant' THEN 1 END)::numeric /
                    NULLIF(COUNT(*)::numeric, 0)) * 100
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            WHERE b.organization_id = $1
              AND u.status = 'active'
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let vacancy_rate = current_vacancy.unwrap_or(Decimal::ZERO);

        // Generate date series with current vacancy rate
        // TODO: Once historical occupancy tracking is implemented, query actual historical data
        let trends: Vec<VacancyTrendPoint> = (0..months)
            .rev()
            .map(|n| {
                let date =
                    chrono::Utc::now().date_naive() - chrono::Duration::days(i64::from(n) * 30);
                VacancyTrendPoint { date, vacancy_rate }
            })
            .collect();

        Ok(trends)
    }
}

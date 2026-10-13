//! Epic 140: Multi-Property Portfolio Analytics repository.
//! Provides database operations for portfolio benchmarks, metrics, trends, and alerts.

use crate::models::portfolio_analytics::*;
use crate::DbPool;
use chrono::NaiveDate;
use common::errors::AppError;
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Clone)]
pub struct PortfolioAnalyticsRepository {
    pool: DbPool,
}

impl PortfolioAnalyticsRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ======================== Benchmarks (Story 140.1) ========================

    pub async fn create_benchmark(
        &self,
        org_id: Uuid,
        req: CreatePortfolioBenchmark,
    ) -> Result<PortfolioBenchmark, AppError> {
        let benchmark = sqlx::query_as::<_, PortfolioBenchmark>(
            r#"
            INSERT INTO portfolio_benchmarks (
                organization_id, name, description, category, target_value,
                min_acceptable, max_acceptable, scope, property_type, region,
                is_industry_standard, source_name, source_year
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, organization_id, name, description, category,
                      target_value, min_acceptable, max_acceptable, scope,
                      property_type, region, is_industry_standard, source_name,
                      source_year, is_active, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.category)
        .bind(req.target_value)
        .bind(req.min_acceptable)
        .bind(req.max_acceptable)
        .bind(&req.scope)
        .bind(&req.property_type)
        .bind(&req.region)
        .bind(req.is_industry_standard)
        .bind(&req.source_name)
        .bind(req.source_year)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(benchmark)
    }

    pub async fn get_benchmark(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PortfolioBenchmark>, AppError> {
        let benchmark = sqlx::query_as::<_, PortfolioBenchmark>(
            r#"
            SELECT id, organization_id, name, description, category,
                   target_value, min_acceptable, max_acceptable, scope,
                   property_type, region, is_industry_standard, source_name,
                   source_year, is_active, created_at, updated_at
            FROM portfolio_benchmarks
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(benchmark)
    }

    pub async fn list_benchmarks(&self, org_id: Uuid) -> Result<Vec<PortfolioBenchmark>, AppError> {
        let benchmarks = sqlx::query_as::<_, PortfolioBenchmark>(
            r#"
            SELECT id, organization_id, name, description, category,
                   target_value, min_acceptable, max_acceptable, scope,
                   property_type, region, is_industry_standard, source_name,
                   source_year, is_active, created_at, updated_at
            FROM portfolio_benchmarks
            WHERE organization_id = $1
            ORDER BY category, name
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(benchmarks)
    }

    pub async fn update_benchmark(
        &self,
        id: Uuid,
        org_id: Uuid,
        req: UpdatePortfolioBenchmark,
    ) -> Result<Option<PortfolioBenchmark>, AppError> {
        let benchmark = sqlx::query_as::<_, PortfolioBenchmark>(
            r#"
            UPDATE portfolio_benchmarks
            SET name = COALESCE($3, name),
                description = COALESCE($4, description),
                target_value = COALESCE($5, target_value),
                min_acceptable = COALESCE($6, min_acceptable),
                max_acceptable = COALESCE($7, max_acceptable),
                is_active = COALESCE($8, is_active),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING id, organization_id, name, description, category,
                      target_value, min_acceptable, max_acceptable, scope,
                      property_type, region, is_industry_standard, source_name,
                      source_year, is_active, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.target_value)
        .bind(req.min_acceptable)
        .bind(req.max_acceptable)
        .bind(req.is_active)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(benchmark)
    }

    pub async fn delete_benchmark(&self, id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM portfolio_benchmarks WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ======================== Property Metrics (Story 140.2) ========================

    pub async fn upsert_property_metrics(
        &self,
        org_id: Uuid,
        req: CreatePropertyMetrics,
    ) -> Result<PropertyPerformanceMetrics, AppError> {
        // Org-scoped write (issue #2946): persist `organization_id` and guard the
        // target building against the caller's org via `WHERE EXISTS`. A building
        // owned by another org matches no row, so the INSERT..SELECT writes nothing
        // and `fetch_one` returns RowNotFound — a cross-tenant upsert can neither
        // create nor overwrite a foreign org's metrics.
        let metrics = sqlx::query_as::<_, PropertyPerformanceMetrics>(
            r#"
            INSERT INTO property_performance_metrics (
                organization_id, building_id, period_start, period_end, period_type,
                total_units, occupied_units, average_lease_term_months, tenant_turnover_rate,
                gross_rental_income, other_income, operating_expenses, currency,
                revenue_per_unit, expense_per_unit, expense_ratio, collection_rate,
                maintenance_requests, avg_resolution_time_hours, maintenance_cost,
                tenant_satisfaction_score, complaints_count, estimated_value, cap_rate
            )
            SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24
            WHERE EXISTS (
                SELECT 1 FROM buildings b WHERE b.id = $2 AND b.organization_id = $1
            )
            ON CONFLICT (building_id, period_start, period_end, period_type)
            DO UPDATE SET
                total_units = EXCLUDED.total_units,
                occupied_units = EXCLUDED.occupied_units,
                average_lease_term_months = EXCLUDED.average_lease_term_months,
                tenant_turnover_rate = EXCLUDED.tenant_turnover_rate,
                gross_rental_income = EXCLUDED.gross_rental_income,
                other_income = EXCLUDED.other_income,
                operating_expenses = EXCLUDED.operating_expenses,
                revenue_per_unit = EXCLUDED.revenue_per_unit,
                expense_per_unit = EXCLUDED.expense_per_unit,
                expense_ratio = EXCLUDED.expense_ratio,
                collection_rate = EXCLUDED.collection_rate,
                maintenance_requests = EXCLUDED.maintenance_requests,
                avg_resolution_time_hours = EXCLUDED.avg_resolution_time_hours,
                maintenance_cost = EXCLUDED.maintenance_cost,
                tenant_satisfaction_score = EXCLUDED.tenant_satisfaction_score,
                complaints_count = EXCLUDED.complaints_count,
                estimated_value = EXCLUDED.estimated_value,
                cap_rate = EXCLUDED.cap_rate
            RETURNING id, building_id, period_start, period_end, period_type,
                      total_units, occupied_units, occupancy_rate, average_lease_term_months,
                      tenant_turnover_rate, gross_rental_income, other_income, total_revenue,
                      operating_expenses, net_operating_income, currency, revenue_per_unit,
                      expense_per_unit, expense_ratio, collection_rate, maintenance_requests,
                      avg_resolution_time_hours, maintenance_cost, tenant_satisfaction_score,
                      complaints_count, estimated_value, cap_rate, created_at
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(req.period_start)
        .bind(req.period_end)
        .bind(&req.period_type)
        .bind(req.total_units)
        .bind(req.occupied_units)
        .bind(req.average_lease_term_months)
        .bind(req.tenant_turnover_rate)
        .bind(req.gross_rental_income)
        .bind(req.other_income)
        .bind(req.operating_expenses)
        .bind(&req.currency)
        .bind(req.revenue_per_unit)
        .bind(req.expense_per_unit)
        .bind(req.expense_ratio)
        .bind(req.collection_rate)
        .bind(req.maintenance_requests)
        .bind(req.avg_resolution_time_hours)
        .bind(req.maintenance_cost)
        .bind(req.tenant_satisfaction_score)
        .bind(req.complaints_count)
        .bind(req.estimated_value)
        .bind(req.cap_rate)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(metrics)
    }

    pub async fn get_property_metrics(
        &self,
        org_id: Uuid,
        building_id: Uuid,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Option<PropertyPerformanceMetrics>, AppError> {
        // Org-scoped by `organization_id` so a foreign org's building_id resolves
        // to no row (surfaced as 404), never the cross-tenant metrics (issue #2946).
        let metrics = sqlx::query_as::<_, PropertyPerformanceMetrics>(
            r#"
            SELECT id, building_id, period_start, period_end, period_type,
                   total_units, occupied_units, occupancy_rate, average_lease_term_months,
                   tenant_turnover_rate, gross_rental_income, other_income, total_revenue,
                   operating_expenses, net_operating_income, currency, revenue_per_unit,
                   expense_per_unit, expense_ratio, collection_rate, maintenance_requests,
                   avg_resolution_time_hours, maintenance_cost, tenant_satisfaction_score,
                   complaints_count, estimated_value, cap_rate, created_at
            FROM property_performance_metrics
            WHERE building_id = $1 AND period_start = $2 AND period_end = $3
              AND organization_id = $4
            "#,
        )
        .bind(building_id)
        .bind(period_start)
        .bind(period_end)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(metrics)
    }

    pub async fn list_property_metrics(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
        period_type: Option<AggregationPeriod>,
    ) -> Result<Vec<PropertyPerformanceMetrics>, AppError> {
        let metrics = sqlx::query_as::<_, PropertyPerformanceMetrics>(
            r#"
            SELECT ppm.id, ppm.building_id, ppm.period_start, ppm.period_end, ppm.period_type,
                   ppm.total_units, ppm.occupied_units, ppm.occupancy_rate, ppm.average_lease_term_months,
                   ppm.tenant_turnover_rate, ppm.gross_rental_income, ppm.other_income, ppm.total_revenue,
                   ppm.operating_expenses, ppm.net_operating_income, ppm.currency, ppm.revenue_per_unit,
                   ppm.expense_per_unit, ppm.expense_ratio, ppm.collection_rate, ppm.maintenance_requests,
                   ppm.avg_resolution_time_hours, ppm.maintenance_cost, ppm.tenant_satisfaction_score,
                   ppm.complaints_count, ppm.estimated_value, ppm.cap_rate, ppm.created_at
            FROM property_performance_metrics ppm
            JOIN buildings b ON b.id = ppm.building_id
            WHERE b.organization_id = $1
              AND ($2::UUID IS NULL OR ppm.building_id = $2)
              AND ($3::aggregation_period IS NULL OR ppm.period_type = $3)
            ORDER BY ppm.period_start DESC
            LIMIT 100
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(period_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(metrics)
    }

    // ======================== Portfolio Metrics (Story 140.3) ========================

    pub async fn get_portfolio_metrics(
        &self,
        org_id: Uuid,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Option<PortfolioAggregatedMetrics>, AppError> {
        let metrics = sqlx::query_as::<_, PortfolioAggregatedMetrics>(
            r#"
            SELECT id, organization_id, period_start, period_end, period_type,
                   total_buildings, total_units, total_sqm, occupied_units,
                   portfolio_occupancy_rate, total_revenue, total_expenses, total_noi,
                   currency, avg_rent_per_unit, avg_rent_per_sqm, avg_expense_per_unit,
                   avg_cap_rate, revenue_growth_pct, expense_growth_pct, noi_growth_pct,
                   estimated_portfolio_value, buildings_by_type, revenue_by_region, created_at
            FROM portfolio_aggregated_metrics
            WHERE organization_id = $1 AND period_start = $2 AND period_end = $3
            "#,
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(metrics)
    }

    pub async fn calculate_portfolio_metrics(
        &self,
        org_id: Uuid,
        period_start: NaiveDate,
        period_end: NaiveDate,
        period_type: AggregationPeriod,
    ) -> Result<PortfolioAggregatedMetrics, AppError> {
        // Aggregate from property metrics
        let metrics = sqlx::query_as::<_, PortfolioAggregatedMetrics>(
            r#"
            INSERT INTO portfolio_aggregated_metrics (
                organization_id, period_start, period_end, period_type,
                total_buildings, total_units, occupied_units,
                portfolio_occupancy_rate, total_revenue, total_expenses, total_noi,
                currency, avg_rent_per_unit, avg_cap_rate
            )
            SELECT
                $1 as organization_id,
                $2 as period_start,
                $3 as period_end,
                $4 as period_type,
                COUNT(DISTINCT ppm.building_id)::INTEGER as total_buildings,
                COALESCE(SUM(ppm.total_units), 0)::INTEGER as total_units,
                COALESCE(SUM(ppm.occupied_units), 0)::INTEGER as occupied_units,
                CASE WHEN SUM(ppm.total_units) > 0
                     THEN (SUM(ppm.occupied_units)::DECIMAL / SUM(ppm.total_units)) * 100
                     ELSE 0 END as portfolio_occupancy_rate,
                COALESCE(SUM(ppm.total_revenue), 0) as total_revenue,
                COALESCE(SUM(ppm.operating_expenses), 0) as total_expenses,
                COALESCE(SUM(ppm.net_operating_income), 0) as total_noi,
                'EUR' as currency,
                CASE WHEN SUM(ppm.total_units) > 0
                     THEN SUM(ppm.total_revenue) / SUM(ppm.total_units)
                     ELSE 0 END as avg_rent_per_unit,
                AVG(ppm.cap_rate) as avg_cap_rate
            FROM property_performance_metrics ppm
            JOIN buildings b ON b.id = ppm.building_id
            WHERE b.organization_id = $1
              AND ppm.period_start >= $2 AND ppm.period_end <= $3
            ON CONFLICT (organization_id, period_start, period_end, period_type)
            DO UPDATE SET
                total_buildings = EXCLUDED.total_buildings,
                total_units = EXCLUDED.total_units,
                occupied_units = EXCLUDED.occupied_units,
                portfolio_occupancy_rate = EXCLUDED.portfolio_occupancy_rate,
                total_revenue = EXCLUDED.total_revenue,
                total_expenses = EXCLUDED.total_expenses,
                total_noi = EXCLUDED.total_noi,
                avg_rent_per_unit = EXCLUDED.avg_rent_per_unit,
                avg_cap_rate = EXCLUDED.avg_cap_rate
            RETURNING id, organization_id, period_start, period_end, period_type,
                      total_buildings, total_units, total_sqm, occupied_units,
                      portfolio_occupancy_rate, total_revenue, total_expenses, total_noi,
                      currency, avg_rent_per_unit, avg_rent_per_sqm, avg_expense_per_unit,
                      avg_cap_rate, revenue_growth_pct, expense_growth_pct, noi_growth_pct,
                      estimated_portfolio_value, buildings_by_type, revenue_by_region, created_at
            "#,
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .bind(&period_type)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(metrics)
    }

    pub async fn get_portfolio_summary(&self, org_id: Uuid) -> Result<PortfolioSummary, AppError> {
        // Get latest portfolio metrics
        let row = sqlx::query_as::<_, (i64, i64, i64, Decimal, Decimal, Decimal)>(
            r#"
            SELECT
                COUNT(DISTINCT b.id) as total_buildings,
                COALESCE(SUM(u.unit_count), 0) as total_units,
                COALESCE(SUM(u.occupied_count), 0) as occupied_units,
                COALESCE(pam.total_revenue, 0) as total_revenue,
                COALESCE(pam.total_noi, 0) as total_noi,
                COALESCE(pam.estimated_portfolio_value, 0) as estimated_value
            FROM buildings b
            LEFT JOIN LATERAL (
                SELECT COUNT(*) as unit_count,
                       COUNT(*) FILTER (WHERE status = 'occupied') as occupied_count
                FROM units WHERE building_id = b.id
            ) u ON true
            LEFT JOIN portfolio_aggregated_metrics pam ON pam.organization_id = b.organization_id
                AND pam.period_end = (SELECT MAX(period_end) FROM portfolio_aggregated_metrics WHERE organization_id = $1)
            WHERE b.organization_id = $1
            GROUP BY pam.total_revenue, pam.total_noi, pam.estimated_portfolio_value
            "#,
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let (
            total_buildings,
            total_units,
            occupied_units,
            total_revenue,
            total_noi,
            estimated_value,
        ) = row.unwrap_or((0, 0, 0, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));

        let occupancy_rate = if total_units > 0 {
            Decimal::from(occupied_units * 100) / Decimal::from(total_units)
        } else {
            Decimal::ZERO
        };

        Ok(PortfolioSummary {
            total_buildings: total_buildings as i32,
            total_units: total_units as i32,
            occupied_units: occupied_units as i32,
            occupancy_rate,
            total_revenue,
            total_noi,
            estimated_value,
            currency: "EUR".to_string(),
        })
    }

    // ======================== Property Comparisons (Story 140.4) ========================

    pub async fn create_comparison(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        req: CreatePropertyComparison,
    ) -> Result<PortfolioPropertyComparison, AppError> {
        let comparison = sqlx::query_as::<_, PortfolioPropertyComparison>(
            r#"
            INSERT INTO property_comparisons (
                organization_id, name, description, building_ids,
                comparison_period_start, comparison_period_end,
                metrics_to_compare, is_saved, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, organization_id, name, description, building_ids,
                      comparison_period_start, comparison_period_end,
                      metrics_to_compare, comparison_results, rankings,
                      is_saved, created_by, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.building_ids)
        .bind(req.comparison_period_start)
        .bind(req.comparison_period_end)
        .bind(&req.metrics_to_compare)
        .bind(req.is_saved)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(comparison)
    }

    pub async fn get_comparison(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PortfolioPropertyComparison>, AppError> {
        let comparison = sqlx::query_as::<_, PortfolioPropertyComparison>(
            r#"
            SELECT id, organization_id, name, description, building_ids,
                   comparison_period_start, comparison_period_end,
                   metrics_to_compare, comparison_results, rankings,
                   is_saved, created_by, created_at, updated_at
            FROM property_comparisons
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(comparison)
    }

    pub async fn list_comparisons(
        &self,
        org_id: Uuid,
        saved_only: bool,
    ) -> Result<Vec<PortfolioPropertyComparison>, AppError> {
        let comparisons = sqlx::query_as::<_, PortfolioPropertyComparison>(
            r#"
            SELECT id, organization_id, name, description, building_ids,
                   comparison_period_start, comparison_period_end,
                   metrics_to_compare, comparison_results, rankings,
                   is_saved, created_by, created_at, updated_at
            FROM property_comparisons
            WHERE organization_id = $1 AND ($2 = false OR is_saved = true)
            ORDER BY updated_at DESC
            "#,
        )
        .bind(org_id)
        .bind(saved_only)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(comparisons)
    }

    pub async fn delete_comparison(&self, id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM property_comparisons WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ======================== Trends (Story 140.5) ========================

    pub async fn record_trend(
        &self,
        org_id: Uuid,
        req: RecordTrend,
    ) -> Result<PortfolioTrend, AppError> {
        // Get previous value
        let previous = sqlx::query_scalar::<_, Decimal>(
            r#"
            SELECT value FROM portfolio_trends
            WHERE organization_id = $1
              AND COALESCE(building_id, '00000000-0000-0000-0000-000000000000') = COALESCE($2, '00000000-0000-0000-0000-000000000000')
              AND metric_name = $3
              AND recorded_at < $4
            ORDER BY recorded_at DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(&req.metric_name)
        .bind(req.recorded_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let change_pct = previous.map(|prev| {
            if prev != Decimal::ZERO {
                ((req.value - prev) / prev) * Decimal::from(100)
            } else {
                Decimal::ZERO
            }
        });

        let trend = sqlx::query_as::<_, PortfolioTrend>(
            r#"
            INSERT INTO portfolio_trends (
                organization_id, building_id, metric_name, recorded_at,
                value, previous_value, change_pct, period_type, currency, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (organization_id, building_id, metric_name, recorded_at, period_type)
            DO UPDATE SET value = EXCLUDED.value, previous_value = EXCLUDED.previous_value,
                          change_pct = EXCLUDED.change_pct, notes = EXCLUDED.notes
            RETURNING id, organization_id, building_id, metric_name, recorded_at,
                      value, previous_value, change_pct, period_type, currency, notes, created_at
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(&req.metric_name)
        .bind(req.recorded_at)
        .bind(req.value)
        .bind(previous)
        .bind(change_pct)
        .bind(&req.period_type)
        .bind(&req.currency)
        .bind(&req.notes)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(trend)
    }

    pub async fn get_trends(
        &self,
        org_id: Uuid,
        metric_name: &str,
        building_id: Option<Uuid>,
        from_date: Option<NaiveDate>,
        to_date: Option<NaiveDate>,
    ) -> Result<Vec<PortfolioTrend>, AppError> {
        let trends = sqlx::query_as::<_, PortfolioTrend>(
            r#"
            SELECT id, organization_id, building_id, metric_name, recorded_at,
                   value, previous_value, change_pct, period_type, currency, notes, created_at
            FROM portfolio_trends
            WHERE organization_id = $1
              AND metric_name = $2
              AND ($3::UUID IS NULL OR building_id = $3)
              AND ($4::DATE IS NULL OR recorded_at >= $4)
              AND ($5::DATE IS NULL OR recorded_at <= $5)
            ORDER BY recorded_at ASC
            "#,
        )
        .bind(org_id)
        .bind(metric_name)
        .bind(building_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(trends)
    }

    // ======================== Alert Rules (Story 140.6) ========================

    pub async fn create_alert_rule(
        &self,
        org_id: Uuid,
        req: CreateAlertRule,
    ) -> Result<PortfolioAlertRule, AppError> {
        let rule = sqlx::query_as::<_, PortfolioAlertRule>(
            r#"
            INSERT INTO portfolio_alert_rules (
                organization_id, name, description, metric_name, category,
                operator, threshold_value, scope, building_id,
                notify_roles, notify_users, notification_frequency
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, organization_id, name, description, metric_name, category,
                      operator, threshold_value, scope, building_id,
                      notify_roles, notify_users, notification_frequency,
                      is_active, last_triggered_at, trigger_count, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.metric_name)
        .bind(&req.category)
        .bind(&req.operator)
        .bind(req.threshold_value)
        .bind(&req.scope)
        .bind(req.building_id)
        .bind(&req.notify_roles)
        .bind(&req.notify_users)
        .bind(&req.notification_frequency)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rule)
    }

    pub async fn get_alert_rule(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PortfolioAlertRule>, AppError> {
        let rule = sqlx::query_as::<_, PortfolioAlertRule>(
            r#"
            SELECT id, organization_id, name, description, metric_name, category,
                   operator, threshold_value, scope, building_id,
                   notify_roles, notify_users, notification_frequency,
                   is_active, last_triggered_at, trigger_count, created_at, updated_at
            FROM portfolio_alert_rules
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rule)
    }

    pub async fn list_alert_rules(
        &self,
        org_id: Uuid,
        active_only: bool,
    ) -> Result<Vec<PortfolioAlertRule>, AppError> {
        let rules = sqlx::query_as::<_, PortfolioAlertRule>(
            r#"
            SELECT id, organization_id, name, description, metric_name, category,
                   operator, threshold_value, scope, building_id,
                   notify_roles, notify_users, notification_frequency,
                   is_active, last_triggered_at, trigger_count, created_at, updated_at
            FROM portfolio_alert_rules
            WHERE organization_id = $1 AND ($2 = false OR is_active = true)
            ORDER BY name
            "#,
        )
        .bind(org_id)
        .bind(active_only)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rules)
    }

    pub async fn update_alert_rule(
        &self,
        id: Uuid,
        org_id: Uuid,
        req: UpdateAlertRule,
    ) -> Result<Option<PortfolioAlertRule>, AppError> {
        let rule = sqlx::query_as::<_, PortfolioAlertRule>(
            r#"
            UPDATE portfolio_alert_rules
            SET name = COALESCE($3, name),
                description = COALESCE($4, description),
                threshold_value = COALESCE($5, threshold_value),
                operator = COALESCE($6, operator),
                notify_roles = COALESCE($7, notify_roles),
                notify_users = COALESCE($8, notify_users),
                notification_frequency = COALESCE($9, notification_frequency),
                is_active = COALESCE($10, is_active),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING id, organization_id, name, description, metric_name, category,
                      operator, threshold_value, scope, building_id,
                      notify_roles, notify_users, notification_frequency,
                      is_active, last_triggered_at, trigger_count, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&req.name)
        .bind(&req.description)
        .bind(req.threshold_value)
        .bind(&req.operator)
        .bind(&req.notify_roles)
        .bind(&req.notify_users)
        .bind(&req.notification_frequency)
        .bind(req.is_active)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rule)
    }

    pub async fn delete_alert_rule(&self, id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM portfolio_alert_rules WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ======================== Alerts (Story 140.6) ========================

    pub async fn list_alerts(
        &self,
        org_id: Uuid,
        unread_only: bool,
        unresolved_only: bool,
        limit: i32,
    ) -> Result<Vec<PortfolioAlert>, AppError> {
        let alerts = sqlx::query_as::<_, PortfolioAlert>(
            r#"
            SELECT id, rule_id, organization_id, building_id,
                   metric_name, current_value, threshold_value, deviation_pct,
                   severity, title, message, is_read, is_resolved,
                   resolved_at, resolved_by, resolution_notes, notifications_sent, created_at
            FROM portfolio_alerts
            WHERE organization_id = $1
              AND ($2 = false OR is_read = false)
              AND ($3 = false OR is_resolved = false)
            ORDER BY created_at DESC
            LIMIT $4
            "#,
        )
        .bind(org_id)
        .bind(unread_only)
        .bind(unresolved_only)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alerts)
    }

    pub async fn get_alert(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PortfolioAlert>, AppError> {
        let alert = sqlx::query_as::<_, PortfolioAlert>(
            r#"
            SELECT id, rule_id, organization_id, building_id,
                   metric_name, current_value, threshold_value, deviation_pct,
                   severity, title, message, is_read, is_resolved,
                   resolved_at, resolved_by, resolution_notes, notifications_sent, created_at
            FROM portfolio_alerts
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(alert)
    }

    pub async fn acknowledge_alert(
        &self,
        id: Uuid,
        org_id: Uuid,
        is_read: bool,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE portfolio_alerts SET is_read = $3 WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .bind(is_read)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn resolve_alert(
        &self,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        notes: Option<String>,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE portfolio_alerts
            SET is_resolved = true, resolved_at = NOW(), resolved_by = $3, resolution_notes = $4
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(user_id)
        .bind(&notes)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn get_alert_stats(&self, org_id: Uuid) -> Result<AlertStats, AppError> {
        let stats = sqlx::query_as::<_, (i64, i64, i64, i64)>(
            r#"
            SELECT
                COUNT(*) as total_alerts,
                COUNT(*) FILTER (WHERE is_read = false) as unread_alerts,
                COUNT(*) FILTER (WHERE is_resolved = false) as unresolved_alerts,
                COUNT(*) FILTER (WHERE severity = 'critical' AND is_resolved = false) as critical_alerts
            FROM portfolio_alerts
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(AlertStats {
            total_alerts: stats.0,
            unread_alerts: stats.1,
            unresolved_alerts: stats.2,
            critical_alerts: stats.3,
        })
    }
}

//! Energy & Sustainability repository (Epic 65).
//!
//! Provides database operations for energy performance certificates,
//! carbon emissions, sustainability scores, and utility benchmarking.

use crate::models::energy::{
    BenchmarkAlert, BenchmarkAlertsQuery, BenchmarkDashboard, BenchmarkMetricType, BenchmarkQuery,
    BuildingBenchmark, CalculateBenchmark, CarbonDashboard, CarbonEmission, CarbonTarget,
    CreateBenchmarkAlert, CreateCarbonEmission, CreateCarbonTarget,
    CreateEnergyPerformanceCertificate, CreateSustainabilityScore, EmissionSourceType,
    EnergyPerformanceCertificate, EnergyRating, HeatingType, InsulationRating,
    ListBenchmarkAlertsResponse, ListBenchmarksResponse, ListEmissionsResponse, ListEpcsResponse,
    MonthlyEmission, RatingCount, SourceEmission, SustainabilityFilter, SustainabilityScore,
    UpdateEnergyPerformanceCertificate, UpdateSustainabilityScore,
};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/// Repository for energy and sustainability operations.
#[derive(Clone)]
pub struct EnergyRepository {
    pool: PgPool,
}

impl EnergyRepository {
    /// Create a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // ENERGY PERFORMANCE CERTIFICATES (Story 65.1)
    // ========================================================================

    /// Create an EPC for a unit.
    pub async fn create_epc(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateEnergyPerformanceCertificate,
    ) -> Result<EnergyPerformanceCertificate, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO energy_performance_certificates (
                organization_id, unit_id, rating, certificate_number, valid_until,
                annual_energy_kwh, annual_co2_kg, primary_energy_kwh_per_sqm,
                issued_by, issued_at, document_url, notes, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.unit_id)
        .bind(data.rating)
        .bind(&data.certificate_number)
        .bind(data.valid_until)
        .bind(data.annual_energy_kwh)
        .bind(data.annual_co2_kg)
        .bind(data.primary_energy_kwh_per_sqm)
        .bind(&data.issued_by)
        .bind(data.issued_at)
        .bind(&data.document_url)
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get EPC by ID.
    pub async fn get_epc(
        &self,
        id: Uuid,
    ) -> Result<Option<EnergyPerformanceCertificate>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM energy_performance_certificates WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get EPC for a unit.
    pub async fn get_epc_for_unit(
        &self,
        unit_id: Uuid,
    ) -> Result<Option<EnergyPerformanceCertificate>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT * FROM energy_performance_certificates
            WHERE unit_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(unit_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List EPCs for a building.
    pub async fn list_epcs_for_building(
        &self,
        building_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<ListEpcsResponse, sqlx::Error> {
        let epcs: Vec<EnergyPerformanceCertificate> = sqlx::query_as(
            r#"
            SELECT e.* FROM energy_performance_certificates e
            JOIN units u ON e.unit_id = u.id
            WHERE u.building_id = $1
            ORDER BY u.designation, e.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(building_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM energy_performance_certificates e
            JOIN units u ON e.unit_id = u.id
            WHERE u.building_id = $1
            "#,
        )
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListEpcsResponse { epcs, total })
    }

    /// Update an EPC.
    pub async fn update_epc(
        &self,
        id: Uuid,
        data: UpdateEnergyPerformanceCertificate,
    ) -> Result<EnergyPerformanceCertificate, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE energy_performance_certificates SET
                rating = COALESCE($2, rating),
                certificate_number = COALESCE($3, certificate_number),
                valid_until = COALESCE($4, valid_until),
                annual_energy_kwh = COALESCE($5, annual_energy_kwh),
                annual_co2_kg = COALESCE($6, annual_co2_kg),
                primary_energy_kwh_per_sqm = COALESCE($7, primary_energy_kwh_per_sqm),
                issued_by = COALESCE($8, issued_by),
                issued_at = COALESCE($9, issued_at),
                document_url = COALESCE($10, document_url),
                notes = COALESCE($11, notes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.rating)
        .bind(data.certificate_number)
        .bind(data.valid_until)
        .bind(data.annual_energy_kwh)
        .bind(data.annual_co2_kg)
        .bind(data.primary_energy_kwh_per_sqm)
        .bind(data.issued_by)
        .bind(data.issued_at)
        .bind(data.document_url)
        .bind(data.notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete an EPC.
    pub async fn delete_epc(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM energy_performance_certificates WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get EPC rating distribution for a building.
    pub async fn get_rating_distribution(
        &self,
        building_id: Uuid,
    ) -> Result<Vec<RatingCount>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT e.rating, COUNT(*) as count
            FROM energy_performance_certificates e
            JOIN units u ON e.unit_id = u.id
            WHERE u.building_id = $1
            GROUP BY e.rating
            ORDER BY e.rating
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // CARBON EMISSIONS (Story 65.2)
    // ========================================================================

    /// Record a carbon emission.
    pub async fn create_emission(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateCarbonEmission,
    ) -> Result<CarbonEmission, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO carbon_emissions (
                organization_id, building_id, year, month, source_type,
                co2_kg, energy_kwh, notes, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (building_id, year, month, source_type) DO UPDATE SET
                co2_kg = EXCLUDED.co2_kg,
                energy_kwh = EXCLUDED.energy_kwh,
                notes = EXCLUDED.notes
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.year)
        .bind(data.month)
        .bind(data.source_type)
        .bind(data.co2_kg)
        .bind(data.energy_kwh)
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get carbon dashboard for a building.
    pub async fn get_carbon_dashboard(
        &self,
        building_id: Uuid,
        year: i32,
    ) -> Result<CarbonDashboard, sqlx::Error> {
        // Get building name
        let (building_name,): (String,) =
            sqlx::query_as("SELECT name FROM buildings WHERE id = $1")
                .bind(building_id)
                .fetch_one(&self.pool)
                .await?;

        // Get total CO2 for current year
        let (total_co2_kg,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(co2_kg), 0)
            FROM carbon_emissions
            WHERE building_id = $1 AND year = $2
            "#,
        )
        .bind(building_id)
        .bind(year)
        .fetch_one(&self.pool)
        .await?;

        // Get total CO2 for previous year
        let (total_co2_kg_previous_year,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(co2_kg), 0)
            FROM carbon_emissions
            WHERE building_id = $1 AND year = $2
            "#,
        )
        .bind(building_id)
        .bind(year - 1)
        .fetch_one(&self.pool)
        .await?;

        let total_co2 = total_co2_kg.unwrap_or(Decimal::ZERO);
        let total_co2_prev = total_co2_kg_previous_year.unwrap_or(Decimal::ZERO);

        let change_percentage = if total_co2_prev > Decimal::ZERO {
            ((total_co2 - total_co2_prev) / total_co2_prev) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Get monthly emissions
        let monthly_emissions: Vec<MonthlyEmission> = sqlx::query_as(
            r#"
            SELECT year, month, SUM(co2_kg) as co2_kg
            FROM carbon_emissions
            WHERE building_id = $1 AND year = $2
            GROUP BY year, month
            ORDER BY month
            "#,
        )
        .bind(building_id)
        .bind(year)
        .fetch_all(&self.pool)
        .await?;

        // Get emissions by source
        let source_emissions: Vec<(EmissionSourceType, Decimal)> = sqlx::query_as(
            r#"
            SELECT source_type, SUM(co2_kg) as co2_kg
            FROM carbon_emissions
            WHERE building_id = $1 AND year = $2
            GROUP BY source_type
            ORDER BY co2_kg DESC
            "#,
        )
        .bind(building_id)
        .bind(year)
        .fetch_all(&self.pool)
        .await?;

        let emissions_by_source: Vec<SourceEmission> = source_emissions
            .iter()
            .map(|(source_type, co2_kg)| {
                let percentage = if total_co2 > Decimal::ZERO {
                    (*co2_kg / total_co2) * Decimal::from(100)
                } else {
                    Decimal::ZERO
                };
                SourceEmission {
                    source_type: *source_type,
                    co2_kg: *co2_kg,
                    percentage,
                }
            })
            .collect();

        // Get building area for per-sqm calculation
        let (floor_area,): (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT SUM(floor_area)
            FROM units
            WHERE building_id = $1
            "#,
        )
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        let co2_per_sqm = if let Some(area) = floor_area {
            if area > Decimal::ZERO {
                total_co2 / area
            } else {
                Decimal::ZERO
            }
        } else {
            Decimal::ZERO
        };

        // Get target
        let target: Option<(Decimal,)> = sqlx::query_as(
            r#"
            SELECT target_co2_kg
            FROM carbon_targets
            WHERE building_id = $1 AND year = $2
            "#,
        )
        .bind(building_id)
        .bind(year)
        .fetch_optional(&self.pool)
        .await?;

        let target_co2_kg = target.map(|t| t.0);
        let on_track = target_co2_kg.map(|t| total_co2 <= t).unwrap_or(true);

        Ok(CarbonDashboard {
            building_id,
            building_name,
            total_co2_kg: total_co2,
            total_co2_kg_previous_year: total_co2_prev,
            change_percentage,
            monthly_emissions,
            emissions_by_source,
            co2_per_sqm,
            target_co2_kg,
            on_track,
        })
    }

    /// List emissions for a building.
    pub async fn list_emissions(
        &self,
        building_id: Uuid,
        year: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<ListEmissionsResponse, sqlx::Error> {
        let current_year = Utc::now().year();
        let filter_year = year.unwrap_or(current_year);

        let emissions: Vec<CarbonEmission> = sqlx::query_as(
            r#"
            SELECT * FROM carbon_emissions
            WHERE building_id = $1 AND year = $2
            ORDER BY month DESC, source_type
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(building_id)
        .bind(filter_year)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM carbon_emissions
            WHERE building_id = $1 AND year = $2
            "#,
        )
        .bind(building_id)
        .bind(filter_year)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListEmissionsResponse { emissions, total })
    }

    /// Set carbon target for a building.
    pub async fn set_carbon_target(
        &self,
        org_id: Uuid,
        data: CreateCarbonTarget,
    ) -> Result<CarbonTarget, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO carbon_targets (
                organization_id, building_id, year, target_co2_kg,
                baseline_co2_kg, baseline_year, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (building_id, year) DO UPDATE SET
                target_co2_kg = EXCLUDED.target_co2_kg,
                baseline_co2_kg = EXCLUDED.baseline_co2_kg,
                baseline_year = EXCLUDED.baseline_year,
                notes = EXCLUDED.notes
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.year)
        .bind(data.target_co2_kg)
        .bind(data.baseline_co2_kg)
        .bind(data.baseline_year)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await
    }

    // ========================================================================
    // SUSTAINABILITY SCORES (Story 65.3)
    // ========================================================================

    /// Create or update sustainability score for a listing.
    pub async fn upsert_sustainability_score(
        &self,
        org_id: Uuid,
        data: CreateSustainabilityScore,
    ) -> Result<SustainabilityScore, sqlx::Error> {
        // Calculate score based on features
        let score = self.calculate_sustainability_score(&data);

        sqlx::query_as(
            r#"
            INSERT INTO sustainability_scores (
                organization_id, listing_id, score, has_solar, has_heat_pump,
                insulation_rating, heating_type, has_ev_charging, has_rainwater_harvesting,
                has_smart_thermostat, has_led_lighting, has_double_glazing,
                renewable_energy_percentage, annual_energy_cost_estimate, energy_rating, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            ON CONFLICT (listing_id) DO UPDATE SET
                score = EXCLUDED.score,
                has_solar = EXCLUDED.has_solar,
                has_heat_pump = EXCLUDED.has_heat_pump,
                insulation_rating = EXCLUDED.insulation_rating,
                heating_type = EXCLUDED.heating_type,
                has_ev_charging = EXCLUDED.has_ev_charging,
                has_rainwater_harvesting = EXCLUDED.has_rainwater_harvesting,
                has_smart_thermostat = EXCLUDED.has_smart_thermostat,
                has_led_lighting = EXCLUDED.has_led_lighting,
                has_double_glazing = EXCLUDED.has_double_glazing,
                renewable_energy_percentage = EXCLUDED.renewable_energy_percentage,
                annual_energy_cost_estimate = EXCLUDED.annual_energy_cost_estimate,
                energy_rating = EXCLUDED.energy_rating,
                notes = EXCLUDED.notes,
                calculated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.listing_id)
        .bind(score)
        .bind(data.has_solar)
        .bind(data.has_heat_pump)
        .bind(data.insulation_rating)
        .bind(data.heating_type)
        .bind(data.has_ev_charging)
        .bind(data.has_rainwater_harvesting)
        .bind(data.has_smart_thermostat)
        .bind(data.has_led_lighting)
        .bind(data.has_double_glazing)
        .bind(data.renewable_energy_percentage)
        .bind(data.annual_energy_cost_estimate)
        .bind(data.energy_rating)
        .bind(&data.notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Calculate sustainability score from features.
    fn calculate_sustainability_score(&self, data: &CreateSustainabilityScore) -> i32 {
        let mut score = 30; // Base score

        // Solar panels: +15
        if data.has_solar {
            score += 15;
        }

        // Heat pump: +15
        if data.has_heat_pump {
            score += 15;
        }

        // Insulation rating
        score += match data.insulation_rating {
            InsulationRating::Excellent => 15,
            InsulationRating::Good => 10,
            InsulationRating::Average => 5,
            InsulationRating::Poor => 0,
            InsulationRating::None => -5,
        };

        // Heating type
        score += match data.heating_type {
            HeatingType::Solar | HeatingType::Geothermal => 10,
            HeatingType::HeatPump | HeatingType::DistrictHeating => 8,
            HeatingType::Biomass => 6,
            HeatingType::Electric => 4,
            HeatingType::Gas => 2,
            HeatingType::Oil => 0,
            HeatingType::None => 0,
        };

        // Additional features
        if data.has_ev_charging.unwrap_or(false) {
            score += 5;
        }
        if data.has_rainwater_harvesting.unwrap_or(false) {
            score += 5;
        }
        if data.has_smart_thermostat.unwrap_or(false) {
            score += 3;
        }
        if data.has_led_lighting.unwrap_or(false) {
            score += 3;
        }
        if data.has_double_glazing.unwrap_or(false) {
            score += 5;
        }

        // Energy rating bonus
        if let Some(rating) = &data.energy_rating {
            score += match rating {
                EnergyRating::A => 10,
                EnergyRating::B => 7,
                EnergyRating::C => 4,
                EnergyRating::D => 0,
                EnergyRating::E => -3,
                EnergyRating::F => -5,
                EnergyRating::G => -10,
            };
        }

        // Clamp to 1-100
        score.clamp(1, 100)
    }

    /// Get sustainability score for a listing.
    pub async fn get_sustainability_score(
        &self,
        listing_id: Uuid,
    ) -> Result<Option<SustainabilityScore>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM sustainability_scores WHERE listing_id = $1")
            .bind(listing_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Update sustainability score.
    pub async fn update_sustainability_score(
        &self,
        listing_id: Uuid,
        data: UpdateSustainabilityScore,
    ) -> Result<SustainabilityScore, sqlx::Error> {
        // First get existing score to recalculate
        let existing = self.get_sustainability_score(listing_id).await?;

        if let Some(existing) = existing {
            // Create updated data for score calculation
            let create_data = CreateSustainabilityScore {
                listing_id,
                has_solar: data.has_solar.unwrap_or(existing.has_solar),
                has_heat_pump: data.has_heat_pump.unwrap_or(existing.has_heat_pump),
                insulation_rating: data.insulation_rating.unwrap_or(existing.insulation_rating),
                heating_type: data.heating_type.unwrap_or(existing.heating_type),
                has_ev_charging: data.has_ev_charging.or(existing.has_ev_charging),
                has_rainwater_harvesting: data
                    .has_rainwater_harvesting
                    .or(existing.has_rainwater_harvesting),
                has_smart_thermostat: data.has_smart_thermostat.or(existing.has_smart_thermostat),
                has_led_lighting: data.has_led_lighting.or(existing.has_led_lighting),
                has_double_glazing: data.has_double_glazing.or(existing.has_double_glazing),
                renewable_energy_percentage: data
                    .renewable_energy_percentage
                    .or(existing.renewable_energy_percentage),
                annual_energy_cost_estimate: data
                    .annual_energy_cost_estimate
                    .or(existing.annual_energy_cost_estimate),
                energy_rating: data.energy_rating.or(existing.energy_rating),
                notes: data.notes.clone().or(existing.notes),
            };

            let score = self.calculate_sustainability_score(&create_data);

            sqlx::query_as(
                r#"
                UPDATE sustainability_scores SET
                    score = $2,
                    has_solar = COALESCE($3, has_solar),
                    has_heat_pump = COALESCE($4, has_heat_pump),
                    insulation_rating = COALESCE($5, insulation_rating),
                    heating_type = COALESCE($6, heating_type),
                    has_ev_charging = COALESCE($7, has_ev_charging),
                    has_rainwater_harvesting = COALESCE($8, has_rainwater_harvesting),
                    has_smart_thermostat = COALESCE($9, has_smart_thermostat),
                    has_led_lighting = COALESCE($10, has_led_lighting),
                    has_double_glazing = COALESCE($11, has_double_glazing),
                    renewable_energy_percentage = COALESCE($12, renewable_energy_percentage),
                    annual_energy_cost_estimate = COALESCE($13, annual_energy_cost_estimate),
                    energy_rating = COALESCE($14, energy_rating),
                    notes = COALESCE($15, notes),
                    calculated_at = NOW()
                WHERE listing_id = $1
                RETURNING *
                "#,
            )
            .bind(listing_id)
            .bind(score)
            .bind(data.has_solar)
            .bind(data.has_heat_pump)
            .bind(data.insulation_rating)
            .bind(data.heating_type)
            .bind(data.has_ev_charging)
            .bind(data.has_rainwater_harvesting)
            .bind(data.has_smart_thermostat)
            .bind(data.has_led_lighting)
            .bind(data.has_double_glazing)
            .bind(data.renewable_energy_percentage)
            .bind(data.annual_energy_cost_estimate)
            .bind(data.energy_rating)
            .bind(data.notes)
            .fetch_one(&self.pool)
            .await
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    /// Search listings with sustainability filter.
    pub async fn filter_listings_by_sustainability(
        &self,
        org_id: Uuid,
        filter: SustainabilityFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        let listings: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT s.listing_id
            FROM sustainability_scores s
            JOIN listings l ON s.listing_id = l.id
            WHERE l.organization_id = $1
                AND ($2::int IS NULL OR s.score >= $2)
                AND ($3::bool IS NULL OR s.has_solar = $3)
                AND ($4::bool IS NULL OR s.has_heat_pump = $4)
                AND ($5::bool IS NULL OR s.has_ev_charging = $5)
                AND ($6::energy_rating IS NULL OR s.energy_rating <= $6)
            ORDER BY s.score DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(filter.min_score)
        .bind(filter.has_solar)
        .bind(filter.has_heat_pump)
        .bind(filter.has_ev_charging)
        .bind(filter.min_energy_rating)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(listings.into_iter().map(|(id,)| id).collect())
    }

    // ========================================================================
    // UTILITY BENCHMARKING (Story 65.4)
    // ========================================================================

    /// Calculate and store benchmark for a building.
    pub async fn calculate_benchmark(
        &self,
        org_id: Uuid,
        data: CalculateBenchmark,
    ) -> Result<BuildingBenchmark, sqlx::Error> {
        // Get building's metric value
        let value = self
            .get_metric_value(
                data.building_id,
                data.metric_type,
                data.period_start,
                data.period_end,
            )
            .await?;

        // Get comparable buildings' values
        let comparables = self
            .get_comparable_values(
                org_id,
                data.building_id,
                data.metric_type,
                data.period_start,
                data.period_end,
                data.region.as_deref(),
                data.building_type.as_deref(),
            )
            .await?;

        let comparable_count = comparables.len() as i32;

        // Calculate percentile (percentage of values less than or equal to this value)
        let percentile = if comparable_count > 0 {
            let at_or_below = comparables.iter().filter(|v| **v <= value).count();
            ((at_or_below as f64 / comparable_count as f64) * 100.0) as i32
        } else {
            50 // Default to median if no comparables
        };

        // Store benchmark
        sqlx::query_as(
            r#"
            INSERT INTO building_benchmarks (
                organization_id, building_id, metric_type, value, percentile,
                comparable_buildings_count, period_start, period_end, region, building_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.metric_type)
        .bind(value)
        .bind(percentile)
        .bind(comparable_count)
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(&data.region)
        .bind(&data.building_type)
        .fetch_one(&self.pool)
        .await
    }

    /// Get metric value for a building.
    async fn get_metric_value(
        &self,
        building_id: Uuid,
        metric_type: BenchmarkMetricType,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Decimal, sqlx::Error> {
        let (floor_area,): (Option<Decimal>,) =
            sqlx::query_as("SELECT SUM(floor_area) FROM units WHERE building_id = $1")
                .bind(building_id)
                .fetch_one(&self.pool)
                .await?;

        let area = floor_area.unwrap_or(Decimal::from(1));

        match metric_type {
            BenchmarkMetricType::ElectricityPerSqm => {
                let (consumption,): (Option<Decimal>,) = sqlx::query_as(
                    r#"
                    SELECT SUM(mr.consumption)
                    FROM meter_readings mr
                    JOIN meters m ON mr.meter_id = m.id
                    WHERE m.building_id = $1
                        AND m.meter_type = 'electricity'
                        AND mr.reading_date >= $2
                        AND mr.reading_date <= $3
                    "#,
                )
                .bind(building_id)
                .bind(period_start)
                .bind(period_end)
                .fetch_one(&self.pool)
                .await?;

                Ok(consumption.unwrap_or(Decimal::ZERO) / area)
            }
            BenchmarkMetricType::GasPerSqm => {
                let (consumption,): (Option<Decimal>,) = sqlx::query_as(
                    r#"
                    SELECT SUM(mr.consumption)
                    FROM meter_readings mr
                    JOIN meters m ON mr.meter_id = m.id
                    WHERE m.building_id = $1
                        AND m.meter_type = 'gas'
                        AND mr.reading_date >= $2
                        AND mr.reading_date <= $3
                    "#,
                )
                .bind(building_id)
                .bind(period_start)
                .bind(period_end)
                .fetch_one(&self.pool)
                .await?;

                Ok(consumption.unwrap_or(Decimal::ZERO) / area)
            }
            BenchmarkMetricType::CarbonPerSqm => {
                let start_year = period_start.year();
                let end_year = period_end.year();

                let (co2,): (Option<Decimal>,) = sqlx::query_as(
                    r#"
                    SELECT SUM(co2_kg)
                    FROM carbon_emissions
                    WHERE building_id = $1
                        AND year >= $2
                        AND year <= $3
                    "#,
                )
                .bind(building_id)
                .bind(start_year)
                .bind(end_year)
                .fetch_one(&self.pool)
                .await?;

                Ok(co2.unwrap_or(Decimal::ZERO) / area)
            }
            _ => Ok(Decimal::ZERO),
        }
    }

    /// Get comparable values from other buildings.
    #[allow(clippy::too_many_arguments)]
    async fn get_comparable_values(
        &self,
        org_id: Uuid,
        exclude_building_id: Uuid,
        metric_type: BenchmarkMetricType,
        period_start: NaiveDate,
        period_end: NaiveDate,
        _region: Option<&str>,
        _building_type: Option<&str>,
    ) -> Result<Vec<Decimal>, sqlx::Error> {
        // Get all buildings in organization except the target
        let buildings: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM buildings
            WHERE organization_id = $1 AND id != $2
            "#,
        )
        .bind(org_id)
        .bind(exclude_building_id)
        .fetch_all(&self.pool)
        .await?;

        let mut values = Vec::new();
        for (building_id,) in buildings {
            if let Ok(value) = self
                .get_metric_value(building_id, metric_type, period_start, period_end)
                .await
            {
                if value > Decimal::ZERO {
                    values.push(value);
                }
            }
        }

        Ok(values)
    }

    /// Get benchmark dashboard for a building.
    pub async fn get_benchmark_dashboard(
        &self,
        building_id: Uuid,
        query: BenchmarkQuery,
    ) -> Result<BenchmarkDashboard, sqlx::Error> {
        // Get building name
        let (building_name,): (String,) =
            sqlx::query_as("SELECT name FROM buildings WHERE id = $1")
                .bind(building_id)
                .fetch_one(&self.pool)
                .await?;

        // Get benchmarks
        let benchmarks: Vec<BuildingBenchmark> = sqlx::query_as(
            r#"
            SELECT * FROM building_benchmarks
            WHERE building_id = $1
                AND ($2::date IS NULL OR period_start >= $2)
                AND ($3::date IS NULL OR period_end <= $3)
                AND ($4::benchmark_metric_type IS NULL OR metric_type = $4)
            ORDER BY calculated_at DESC
            LIMIT 20
            "#,
        )
        .bind(building_id)
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.metric_type)
        .fetch_all(&self.pool)
        .await?;

        // Get unresolved alerts
        let alerts: Vec<BenchmarkAlert> = sqlx::query_as(
            r#"
            SELECT * FROM benchmark_alerts
            WHERE building_id = $1 AND is_resolved = false
            ORDER BY created_at DESC
            LIMIT 10
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        // Calculate overall percentile (average of all metrics)
        let overall_percentile = if !benchmarks.is_empty() {
            benchmarks.iter().map(|b| b.percentile).sum::<i32>() / benchmarks.len() as i32
        } else {
            50
        };

        let metrics_above_average = benchmarks.iter().filter(|b| b.percentile > 50).count() as i32;
        let metrics_below_average = benchmarks.iter().filter(|b| b.percentile < 50).count() as i32;

        // Generate improvement suggestions
        let improvement_suggestions = self.generate_improvement_suggestions(&benchmarks);

        Ok(BenchmarkDashboard {
            building_id,
            building_name,
            benchmarks,
            alerts,
            overall_percentile,
            metrics_above_average,
            metrics_below_average,
            improvement_suggestions,
        })
    }

    /// Generate improvement suggestions based on benchmarks.
    fn generate_improvement_suggestions(&self, benchmarks: &[BuildingBenchmark]) -> Vec<String> {
        let mut suggestions = Vec::new();

        for benchmark in benchmarks {
            if benchmark.percentile < 25 {
                let suggestion = match benchmark.metric_type {
                    BenchmarkMetricType::ElectricityPerSqm => {
                        "Consider LED lighting upgrades and smart energy management systems to reduce electricity consumption."
                    }
                    BenchmarkMetricType::GasPerSqm => {
                        "Evaluate insulation improvements and heating system efficiency to reduce gas usage."
                    }
                    BenchmarkMetricType::CarbonPerSqm => {
                        "Explore renewable energy options and carbon offset programs to reduce carbon footprint."
                    }
                    BenchmarkMetricType::WaterPerPerson => {
                        "Install low-flow fixtures and consider rainwater harvesting to reduce water consumption."
                    }
                    _ => continue,
                };
                suggestions.push(suggestion.to_string());
            }
        }

        suggestions
    }

    /// List benchmarks for a building.
    pub async fn list_benchmarks(
        &self,
        building_id: Uuid,
        query: BenchmarkQuery,
        limit: i64,
        offset: i64,
    ) -> Result<ListBenchmarksResponse, sqlx::Error> {
        let benchmarks: Vec<BuildingBenchmark> = sqlx::query_as(
            r#"
            SELECT * FROM building_benchmarks
            WHERE building_id = $1
                AND ($2::date IS NULL OR period_start >= $2)
                AND ($3::date IS NULL OR period_end <= $3)
                AND ($4::benchmark_metric_type IS NULL OR metric_type = $4)
            ORDER BY calculated_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(building_id)
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.metric_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM building_benchmarks
            WHERE building_id = $1
                AND ($2::date IS NULL OR period_start >= $2)
                AND ($3::date IS NULL OR period_end <= $3)
                AND ($4::benchmark_metric_type IS NULL OR metric_type = $4)
            "#,
        )
        .bind(building_id)
        .bind(query.period_start)
        .bind(query.period_end)
        .bind(query.metric_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListBenchmarksResponse { benchmarks, total })
    }

    /// Create benchmark alert.
    pub async fn create_benchmark_alert(
        &self,
        org_id: Uuid,
        data: CreateBenchmarkAlert,
    ) -> Result<BenchmarkAlert, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO benchmark_alerts (
                organization_id, building_id, metric_type, current_value,
                benchmark_value, deviation_percentage, severity, message
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.metric_type)
        .bind(data.current_value)
        .bind(data.benchmark_value)
        .bind(data.deviation_percentage)
        .bind(data.severity)
        .bind(&data.message)
        .fetch_one(&self.pool)
        .await
    }

    /// List benchmark alerts for a building.
    pub async fn list_benchmark_alerts(
        &self,
        building_id: Uuid,
        query: BenchmarkAlertsQuery,
    ) -> Result<ListBenchmarkAlertsResponse, sqlx::Error> {
        let alerts: Vec<BenchmarkAlert> = sqlx::query_as(
            r#"
            SELECT * FROM benchmark_alerts
            WHERE building_id = $1
                AND ($2::bool = false OR is_resolved = false)
                AND ($3::benchmark_alert_severity IS NULL OR severity = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(building_id)
        .bind(query.unresolved_only)
        .bind(query.severity)
        .bind(query.limit)
        .bind(query.offset)
        .fetch_all(&self.pool)
        .await?;

        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM benchmark_alerts
            WHERE building_id = $1
                AND ($2::bool = false OR is_resolved = false)
                AND ($3::benchmark_alert_severity IS NULL OR severity = $3)
            "#,
        )
        .bind(building_id)
        .bind(query.unresolved_only)
        .bind(query.severity)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListBenchmarkAlertsResponse { alerts, total })
    }

    /// Resolve a benchmark alert.
    /// Get a single benchmark alert by id (used for tenant-ownership checks
    /// before resolving it).
    pub async fn get_benchmark_alert(
        &self,
        id: Uuid,
    ) -> Result<Option<BenchmarkAlert>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM benchmark_alerts WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn resolve_benchmark_alert(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<BenchmarkAlert, sqlx::Error> {
        sqlx::query_as(
            r#"
            UPDATE benchmark_alerts SET
                is_resolved = true,
                resolved_at = NOW(),
                resolved_by = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }
}

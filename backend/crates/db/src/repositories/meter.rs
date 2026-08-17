//! Meter repository (Epic 12).
//!
//! Provides database operations for meters, readings, utility bills,
//! and consumption tracking.

use crate::models::meter::{
    ConsumptionAggregate, ConsumptionDataPoint, ConsumptionHistoryResponse, CreateSubmissionWindow,
    CreateUtilityBill, DistributeUtilityBill, ListMetersResponse, ListReadingsResponse, Meter,
    MeterReading, MeterResponse, MissingReadingAlert, ReadingSource, ReadingStatus,
    ReadingSubmissionWindow, ReadingValidationRule, RegisterMeter, ReplaceMeter,
    SmartMeterProvider, SmartMeterReadingLog, SubmitReading, UtilityBill, UtilityBillDistribution,
    UtilityBillResponse, ValidateReading,
};
use crate::DbPool;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Repository for meter operations.
#[derive(Clone)]
pub struct MeterRepository {
    pool: DbPool,
}

impl MeterRepository {
    /// Create a new MeterRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // METERS (Story 12.1)
    // ========================================================================

    /// Register a new meter.
    pub async fn register_meter(
        &self,
        org_id: Uuid,
        data: RegisterMeter,
    ) -> Result<Meter, SqlxError> {
        sqlx::query_as::<_, Meter>(
            r#"
            INSERT INTO meters (
                organization_id, building_id, unit_id, meter_number, meter_type,
                location, description, initial_reading, current_reading, unit_of_measure,
                is_shared, installed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(&data.meter_number)
        .bind(data.meter_type)
        .bind(&data.location)
        .bind(&data.description)
        .bind(data.initial_reading)
        .bind(&data.unit_of_measure)
        .bind(data.is_shared)
        .bind(data.installed_at)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a meter by ID.
    pub async fn get_meter(&self, id: Uuid) -> Result<Option<Meter>, SqlxError> {
        sqlx::query_as::<_, Meter>("SELECT * FROM meters WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get meter by meter number.
    pub async fn get_meter_by_number(
        &self,
        org_id: Uuid,
        meter_number: &str,
    ) -> Result<Option<Meter>, SqlxError> {
        sqlx::query_as::<_, Meter>(
            "SELECT * FROM meters WHERE organization_id = $1 AND meter_number = $2",
        )
        .bind(org_id)
        .bind(meter_number)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get meter with recent readings.
    pub async fn get_meter_with_readings(
        &self,
        id: Uuid,
        limit: i64,
    ) -> Result<Option<MeterResponse>, SqlxError> {
        let meter = self.get_meter(id).await?;

        if let Some(meter) = meter {
            let recent_readings = sqlx::query_as::<_, MeterReading>(
                r#"
                SELECT * FROM meter_readings
                WHERE meter_id = $1
                ORDER BY reading_date DESC, created_at DESC
                LIMIT $2
                "#,
            )
            .bind(id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

            Ok(Some(MeterResponse {
                meter,
                recent_readings,
            }))
        } else {
            Ok(None)
        }
    }

    /// List meters for a building.
    pub async fn list_meters_for_building(
        &self,
        building_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<ListMetersResponse, SqlxError> {
        let meters = sqlx::query_as::<_, Meter>(
            r#"
            SELECT * FROM meters
            WHERE building_id = $1 AND is_active = true
            ORDER BY meter_number
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(building_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM meters WHERE building_id = $1 AND is_active = true",
        )
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListMetersResponse {
            meters,
            total: total.0,
        })
    }

    /// List meters for a unit.
    pub async fn list_meters_for_unit(&self, unit_id: Uuid) -> Result<Vec<Meter>, SqlxError> {
        sqlx::query_as::<_, Meter>(
            r#"
            SELECT * FROM meters
            WHERE unit_id = $1 AND is_active = true
            ORDER BY meter_type, meter_number
            "#,
        )
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Replace a meter.
    pub async fn replace_meter(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: ReplaceMeter,
    ) -> Result<Meter, SqlxError> {
        // Get the old meter
        let old_meter = self
            .get_meter(id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        // Decommission old meter
        sqlx::query(
            r#"
            UPDATE meters
            SET is_active = false,
                decommissioned_at = CURRENT_DATE,
                current_reading = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(data.final_reading)
        .execute(&self.pool)
        .await?;

        // Create new meter
        let new_meter = sqlx::query_as::<_, Meter>(
            r#"
            INSERT INTO meters (
                organization_id, building_id, unit_id, meter_number, meter_type,
                location, description, initial_reading, current_reading, unit_of_measure,
                is_shared, installed_at, replaced_meter_id, replacement_reading
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $9, $10, CURRENT_DATE, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(old_meter.building_id)
        .bind(old_meter.unit_id)
        .bind(&data.new_meter_number)
        .bind(old_meter.meter_type)
        .bind(data.new_location.as_ref().or(old_meter.location.as_ref()))
        .bind(&old_meter.description)
        .bind(data.new_initial_reading)
        .bind(&old_meter.unit_of_measure)
        .bind(old_meter.is_shared)
        .bind(id)
        .bind(data.final_reading)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_meter)
    }

    // ========================================================================
    // METER READINGS (Story 12.2)
    // ========================================================================

    /// Submit a meter reading.
    pub async fn submit_reading(
        &self,
        user_id: Uuid,
        data: SubmitReading,
    ) -> Result<MeterReading, SqlxError> {
        let reading_date = data.reading_date.unwrap_or_else(|| Utc::now().date_naive());

        // Get previous reading for consumption calculation
        let previous = sqlx::query_as::<_, MeterReading>(
            r#"
            SELECT * FROM meter_readings
            WHERE meter_id = $1 AND status = 'approved'
            ORDER BY reading_date DESC
            LIMIT 1
            "#,
        )
        .bind(data.meter_id)
        .fetch_optional(&self.pool)
        .await?;

        let consumption = previous.as_ref().map(|p| data.reading - p.reading);
        let previous_reading_id = previous.map(|p| p.id);

        // Determine initial status based on source
        let status = match data.source {
            ReadingSource::Automatic => ReadingStatus::Approved,
            _ => ReadingStatus::Pending,
        };

        let reading = sqlx::query_as::<_, MeterReading>(
            r#"
            INSERT INTO meter_readings (
                meter_id, reading, reading_date, reading_time, source,
                photo_url, status, consumption, previous_reading_id, submitted_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(data.meter_id)
        .bind(data.reading)
        .bind(reading_date)
        .bind(data.reading_time)
        .bind(data.source)
        .bind(&data.photo_url)
        .bind(status)
        .bind(consumption)
        .bind(previous_reading_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Update meter's current reading if approved
        if status == ReadingStatus::Approved {
            sqlx::query(
                r#"
                UPDATE meters
                SET current_reading = $2, last_reading_date = $3, updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(data.meter_id)
            .bind(data.reading)
            .bind(reading_date)
            .execute(&self.pool)
            .await?;
        }

        Ok(reading)
    }

    /// Get reading by ID.
    pub async fn get_reading(&self, id: Uuid) -> Result<Option<MeterReading>, SqlxError> {
        sqlx::query_as::<_, MeterReading>("SELECT * FROM meter_readings WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List readings for a meter.
    pub async fn list_readings_for_meter(
        &self,
        meter_id: Uuid,
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        limit: i64,
        offset: i64,
    ) -> Result<ListReadingsResponse, SqlxError> {
        let readings = sqlx::query_as::<_, MeterReading>(
            r#"
            SELECT * FROM meter_readings
            WHERE meter_id = $1
            AND ($2::date IS NULL OR reading_date >= $2)
            AND ($3::date IS NULL OR reading_date <= $3)
            ORDER BY reading_date DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(meter_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM meter_readings
            WHERE meter_id = $1
            AND ($2::date IS NULL OR reading_date >= $2)
            AND ($3::date IS NULL OR reading_date <= $3)
            "#,
        )
        .bind(meter_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        Ok(ListReadingsResponse {
            readings,
            total: total.0,
        })
    }

    // ========================================================================
    // SUBMISSION WINDOWS
    // ========================================================================

    /// Create a reading submission window.
    pub async fn create_submission_window(
        &self,
        org_id: Uuid,
        data: CreateSubmissionWindow,
    ) -> Result<ReadingSubmissionWindow, SqlxError> {
        sqlx::query_as::<_, ReadingSubmissionWindow>(
            r#"
            INSERT INTO reading_submission_windows (
                organization_id, building_id, name, description,
                billing_period_start, billing_period_end,
                submission_start, submission_end
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.billing_period_start)
        .bind(data.billing_period_end)
        .bind(data.submission_start)
        .bind(data.submission_end)
        .fetch_one(&self.pool)
        .await
    }

    /// Get open submission window for building.
    pub async fn get_open_submission_window(
        &self,
        building_id: Uuid,
    ) -> Result<Option<ReadingSubmissionWindow>, SqlxError> {
        sqlx::query_as::<_, ReadingSubmissionWindow>(
            r#"
            SELECT * FROM reading_submission_windows
            WHERE building_id = $1
            AND is_open = true
            AND is_finalized = false
            AND submission_start <= CURRENT_DATE
            AND submission_end >= CURRENT_DATE
            "#,
        )
        .bind(building_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find all open submission windows closing within `days_before` days.
    ///
    /// Used by the scheduler to dispatch meter-reading reminders (Story 12.2).
    pub async fn find_windows_closing_soon(
        &self,
        days_before: i64,
    ) -> Result<Vec<ReadingSubmissionWindow>, SqlxError> {
        let cutoff = chrono::Utc::now().date_naive() + chrono::Duration::days(days_before);
        sqlx::query_as::<_, ReadingSubmissionWindow>(
            r#"
            SELECT * FROM reading_submission_windows
            WHERE is_open = true
              AND is_finalized = false
              AND submission_start <= CURRENT_DATE
              AND submission_end >= CURRENT_DATE
              AND submission_end <= $1
            ORDER BY submission_end ASC
            "#,
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // READING VALIDATION (Story 12.3)
    // ========================================================================

    /// Validate (approve/reject) a reading.
    pub async fn validate_reading(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: ValidateReading,
    ) -> Result<Option<MeterReading>, SqlxError> {
        let reading = sqlx::query_as::<_, MeterReading>(
            r#"
            UPDATE meter_readings
            SET status = $2,
                reading = COALESCE($3, reading),
                validated_by = $4,
                validated_at = NOW(),
                validation_notes = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.status)
        .bind(data.corrected_reading)
        .bind(user_id)
        .bind(&data.notes)
        .fetch_optional(&self.pool)
        .await?;

        // If approved, update meter's current reading
        if let Some(ref r) = reading {
            if r.status == ReadingStatus::Approved {
                sqlx::query(
                    r#"
                    UPDATE meters
                    SET current_reading = $2, last_reading_date = $3, updated_at = NOW()
                    WHERE id = $1
                    "#,
                )
                .bind(r.meter_id)
                .bind(r.reading)
                .bind(r.reading_date)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(reading)
    }

    /// Get readings pending approval.
    pub async fn get_pending_readings(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<Vec<MeterReading>, SqlxError> {
        sqlx::query_as::<_, MeterReading>(
            r#"
            SELECT r.*
            FROM meter_readings r
            JOIN meters m ON r.meter_id = m.id
            WHERE m.organization_id = $1
            AND ($2::uuid IS NULL OR m.building_id = $2)
            AND r.status = 'pending'
            ORDER BY r.reading_date ASC
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await
    }

    /// List pending meter readings for a single unit, honouring the caller's
    /// RLS session context.
    ///
    /// Unlike [`get_pending_readings`] (org/building-wide, own-pool), this is
    /// scoped to one resident's unit and runs on the caller-supplied,
    /// context-set connection so tenant isolation is enforced. Used by the
    /// voice assistant's "check meter readings" intent, which must answer with
    /// the resident's own pending readings rather than fabricating an empty
    /// result.
    pub async fn get_pending_readings_for_unit_rls<'e, E>(
        &self,
        executor: E,
        unit_id: Uuid,
    ) -> Result<Vec<MeterReading>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, MeterReading>(
            r#"
            SELECT r.*
            FROM meter_readings r
            JOIN meters m ON r.meter_id = m.id
            WHERE m.unit_id = $1
              AND r.status = 'pending'
            ORDER BY r.reading_date ASC
            "#,
        )
        .bind(unit_id)
        .fetch_all(executor)
        .await
    }

    /// Get validation rules for organization.
    pub async fn get_validation_rules(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<ReadingValidationRule>, SqlxError> {
        sqlx::query_as::<_, ReadingValidationRule>(
            "SELECT * FROM reading_validation_rules WHERE organization_id = $1 AND is_active = true",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // UTILITY BILLS (Story 12.4)
    // ========================================================================

    /// Create a utility bill.
    pub async fn create_utility_bill(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateUtilityBill,
    ) -> Result<UtilityBill, SqlxError> {
        sqlx::query_as::<_, UtilityBill>(
            r#"
            INSERT INTO utility_bills (
                organization_id, building_id, bill_number, meter_type,
                provider_name, period_start, period_end, total_amount,
                total_consumption, currency, distribution_method, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.bill_number)
        .bind(data.meter_type)
        .bind(&data.provider_name)
        .bind(data.period_start)
        .bind(data.period_end)
        .bind(data.total_amount)
        .bind(data.total_consumption)
        .bind(&data.currency)
        .bind(data.distribution_method)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get utility bill by ID.
    pub async fn get_utility_bill(&self, id: Uuid) -> Result<Option<UtilityBill>, SqlxError> {
        sqlx::query_as::<_, UtilityBill>("SELECT * FROM utility_bills WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Get utility bill with distributions.
    pub async fn get_utility_bill_with_distributions(
        &self,
        id: Uuid,
    ) -> Result<Option<UtilityBillResponse>, SqlxError> {
        let bill = self.get_utility_bill(id).await?;

        if let Some(bill) = bill {
            let distributions = sqlx::query_as::<_, UtilityBillDistribution>(
                "SELECT * FROM utility_bill_distributions WHERE utility_bill_id = $1",
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

            Ok(Some(UtilityBillResponse {
                bill,
                distributions,
            }))
        } else {
            Ok(None)
        }
    }

    /// Distribute utility bill to units.
    pub async fn distribute_utility_bill(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: DistributeUtilityBill,
    ) -> Result<UtilityBillResponse, SqlxError> {
        let bill = self
            .get_utility_bill(id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        // Get all units in the building
        let units: Vec<(Uuid, Decimal, i32)> = sqlx::query_as(
            r#"
            SELECT u.id, u.floor_area, COALESCE(pm.count, 1)
            FROM units u
            LEFT JOIN (
                SELECT unit_id, SUM(count) as count
                FROM person_months
                WHERE month = date_trunc('month', $2::date)
                GROUP BY unit_id
            ) pm ON pm.unit_id = u.id
            WHERE u.building_id = $1 AND u.status = 'active'
            "#,
        )
        .bind(bill.building_id)
        .bind(bill.period_start)
        .fetch_all(&self.pool)
        .await?;

        // Calculate distribution based on method
        let mut distributions = Vec::new();
        let total_units = units.len() as i32;
        let total_area: Decimal = units.iter().map(|(_, a, _)| *a).sum();
        let total_occupants: i32 = units.iter().map(|(_, _, o)| *o).sum();

        for (unit_id, area, occupants) in units {
            // Check for override
            let override_amount = data
                .unit_overrides
                .iter()
                .find(|o| o.unit_id == unit_id)
                .map(|o| o.amount);

            let amount = if let Some(amt) = override_amount {
                amt
            } else {
                match data.distribution_method {
                    crate::models::meter::DistributionMethod::Equal => {
                        bill.total_amount / Decimal::from(total_units)
                    }
                    crate::models::meter::DistributionMethod::Area => {
                        if total_area > Decimal::ZERO {
                            bill.total_amount * area / total_area
                        } else {
                            bill.total_amount / Decimal::from(total_units)
                        }
                    }
                    crate::models::meter::DistributionMethod::Occupants => {
                        if total_occupants > 0 {
                            bill.total_amount * Decimal::from(occupants)
                                / Decimal::from(total_occupants)
                        } else {
                            bill.total_amount / Decimal::from(total_units)
                        }
                    }
                    _ => bill.total_amount / Decimal::from(total_units),
                }
            };

            let distribution = sqlx::query_as::<_, UtilityBillDistribution>(
                r#"
                INSERT INTO utility_bill_distributions (
                    utility_bill_id, unit_id, amount,
                    area_factor, occupant_factor
                )
                VALUES ($1, $2, $3, $4, $5)
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(unit_id)
            .bind(amount)
            .bind(area)
            .bind(occupants)
            .fetch_one(&self.pool)
            .await?;

            distributions.push(distribution);
        }

        // Mark bill as distributed
        let updated_bill = sqlx::query_as::<_, UtilityBill>(
            r#"
            UPDATE utility_bills
            SET is_distributed = true,
                distributed_at = NOW(),
                distributed_by = $2,
                distribution_method = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(data.distribution_method)
        .fetch_one(&self.pool)
        .await?;

        Ok(UtilityBillResponse {
            bill: updated_bill,
            distributions,
        })
    }

    // ========================================================================
    // CONSUMPTION ANALYTICS (Story 12.5)
    // ========================================================================

    /// Get consumption history for a meter.
    pub async fn get_consumption_history(
        &self,
        meter_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<ConsumptionHistoryResponse, SqlxError> {
        let meter = self
            .get_meter(meter_id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        let readings = sqlx::query_as::<_, MeterReading>(
            r#"
            SELECT * FROM meter_readings
            WHERE meter_id = $1
            AND status = 'approved'
            AND reading_date >= $2
            AND reading_date <= $3
            ORDER BY reading_date ASC
            "#,
        )
        .bind(meter_id)
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;

        let history: Vec<ConsumptionDataPoint> = readings
            .iter()
            .map(|r| ConsumptionDataPoint {
                date: r.reading_date,
                reading: r.reading,
                consumption: r.consumption.unwrap_or_default(),
            })
            .collect();

        // Get building average
        let building_avg: Option<(Decimal,)> = sqlx::query_as(
            r#"
            SELECT AVG(r.consumption) as avg_consumption
            FROM meter_readings r
            JOIN meters m ON r.meter_id = m.id
            WHERE m.building_id = (SELECT building_id FROM meters WHERE id = $1)
            AND m.meter_type = (SELECT meter_type FROM meters WHERE id = $1)
            AND r.status = 'approved'
            AND r.reading_date >= $2
            AND r.reading_date <= $3
            "#,
        )
        .bind(meter_id)
        .bind(from)
        .bind(to)
        .fetch_optional(&self.pool)
        .await?;

        Ok(ConsumptionHistoryResponse {
            meter_id,
            meter_type: meter.meter_type,
            unit_of_measure: meter.unit_of_measure,
            history,
            building_average: building_avg.map(|a| a.0),
            comparison: None, // Could be computed
        })
    }

    /// Get consumption aggregates.
    pub async fn get_consumption_aggregates(
        &self,
        meter_id: Uuid,
        year: i32,
    ) -> Result<Vec<ConsumptionAggregate>, SqlxError> {
        sqlx::query_as::<_, ConsumptionAggregate>(
            r#"
            SELECT * FROM consumption_aggregates
            WHERE meter_id = $1 AND year = $2
            ORDER BY month
            "#,
        )
        .bind(meter_id)
        .bind(year)
        .fetch_all(&self.pool)
        .await
    }

    // ========================================================================
    // SMART METERS (Story 12.6)
    // ========================================================================

    /// Get smart meter providers for organization.
    pub async fn get_smart_meter_providers(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<SmartMeterProvider>, SqlxError> {
        sqlx::query_as::<_, SmartMeterProvider>(
            "SELECT * FROM smart_meter_providers WHERE organization_id = $1",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get smart meter provider by ID.
    pub async fn get_smart_meter_provider(
        &self,
        id: Uuid,
    ) -> Result<Option<SmartMeterProvider>, SqlxError> {
        sqlx::query_as::<_, SmartMeterProvider>("SELECT * FROM smart_meter_providers WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Ingest smart meter reading.
    pub async fn ingest_smart_meter_reading(
        &self,
        provider_id: Uuid,
        meter_id: Uuid,
        reading: Decimal,
        reading_timestamp: chrono::DateTime<Utc>,
        raw_data: Option<serde_json::Value>,
    ) -> Result<SmartMeterReadingLog, SqlxError> {
        // Create log entry
        let log = sqlx::query_as::<_, SmartMeterReadingLog>(
            r#"
            INSERT INTO smart_meter_reading_logs (
                meter_id, provider_id, reading, reading_timestamp, raw_data
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(meter_id)
        .bind(provider_id)
        .bind(reading)
        .bind(reading_timestamp)
        .bind(&raw_data)
        .fetch_one(&self.pool)
        .await?;

        // Create actual meter reading
        let meter_reading = self
            .submit_reading(
                Uuid::nil(), // System submission
                SubmitReading {
                    meter_id,
                    reading,
                    reading_date: Some(reading_timestamp.date_naive()),
                    reading_time: Some(reading_timestamp.time()),
                    source: ReadingSource::Automatic,
                    photo_url: None,
                },
            )
            .await?;

        // Update log with meter reading ID
        sqlx::query(
            r#"
            UPDATE smart_meter_reading_logs
            SET meter_reading_id = $2, processed = true
            WHERE id = $1
            "#,
        )
        .bind(log.id)
        .bind(meter_reading.id)
        .execute(&self.pool)
        .await?;

        Ok(log)
    }

    /// Update provider sync status.
    pub async fn update_provider_sync_status(
        &self,
        id: Uuid,
        error: Option<&str>,
    ) -> Result<(), SqlxError> {
        if let Some(err) = error {
            sqlx::query(
                r#"
                UPDATE smart_meter_providers
                SET last_error = $2, error_count = error_count + 1, updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(err)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE smart_meter_providers
                SET last_sync_at = NOW(), last_error = NULL, error_count = 0, updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Get missing reading alerts.
    pub async fn get_missing_reading_alerts(
        &self,
        org_id: Uuid,
        unresolved_only: bool,
    ) -> Result<Vec<MissingReadingAlert>, SqlxError> {
        if unresolved_only {
            sqlx::query_as::<_, MissingReadingAlert>(
                r#"
                SELECT a.* FROM missing_reading_alerts a
                JOIN meters m ON a.meter_id = m.id
                WHERE m.organization_id = $1 AND a.is_resolved = false
                ORDER BY a.expected_date DESC
                "#,
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, MissingReadingAlert>(
                r#"
                SELECT a.* FROM missing_reading_alerts a
                JOIN meters m ON a.meter_id = m.id
                WHERE m.organization_id = $1
                ORDER BY a.expected_date DESC
                "#,
            )
            .bind(org_id)
            .fetch_all(&self.pool)
            .await
        }
    }

    /// Resolve missing reading alert.
    /// Get a single missing-reading alert by id (used for tenant-ownership
    /// checks before resolving it).
    pub async fn get_missing_alert(
        &self,
        id: Uuid,
    ) -> Result<Option<MissingReadingAlert>, SqlxError> {
        sqlx::query_as::<_, MissingReadingAlert>(
            "SELECT * FROM missing_reading_alerts WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn resolve_missing_alert(
        &self,
        id: Uuid,
        user_id: Uuid,
        notes: Option<&str>,
    ) -> Result<Option<MissingReadingAlert>, SqlxError> {
        sqlx::query_as::<_, MissingReadingAlert>(
            r#"
            UPDATE missing_reading_alerts
            SET is_resolved = true, resolved_at = NOW(), resolved_by = $2, resolution_notes = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(notes)
        .fetch_optional(&self.pool)
        .await
    }

    // ========================================================================
    // Epic 55: Reporting Analytics
    // ========================================================================

    /// Get consumption report data (Epic 55, Story 55.4).
    pub async fn get_consumption_report(
        &self,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        utility_type: Option<&str>,
        from_date: chrono::NaiveDate,
        to_date: chrono::NaiveDate,
    ) -> Result<crate::models::reports::ConsumptionReportData, SqlxError> {
        use rust_decimal::Decimal;

        // Get summary data
        let (total_consumption, total_cost, meter_count): (Option<Decimal>, Option<Decimal>, i64) =
            sqlx::query_as(
                r#"
            SELECT
                COALESCE(SUM(mr.consumption), 0),
                COALESCE(SUM(mr.cost), 0),
                COUNT(DISTINCT m.id)
            FROM meter_readings mr
            JOIN meters m ON mr.meter_id = m.id
            JOIN units u ON m.unit_id = u.id
            JOIN buildings b ON u.building_id = b.id
            WHERE b.organization_id = $1
              AND ($2::uuid IS NULL OR u.building_id = $2)
              AND ($3::text IS NULL OR m.meter_type::text = $3)
              AND mr.reading_date >= $4 AND mr.reading_date <= $5
            "#,
            )
            .bind(organization_id)
            .bind(building_id)
            .bind(utility_type)
            .bind(from_date)
            .bind(to_date)
            .fetch_one(&self.pool)
            .await?;

        // Get unit count for average
        let (unit_count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT m.unit_id)
            FROM meters m
            JOIN units u ON m.unit_id = u.id
            JOIN buildings b ON u.building_id = b.id
            WHERE b.organization_id = $1
              AND ($2::uuid IS NULL OR u.building_id = $2)
              AND ($3::text IS NULL OR m.meter_type::text = $3)
            "#,
        )
        .bind(organization_id)
        .bind(building_id)
        .bind(utility_type)
        .fetch_one(&self.pool)
        .await?;

        let total_consumption = total_consumption.unwrap_or(Decimal::ZERO);
        let total_cost = total_cost.unwrap_or(Decimal::ZERO);
        let average_consumption_per_unit = if unit_count > 0 {
            total_consumption / Decimal::from(unit_count)
        } else {
            Decimal::ZERO
        };

        Ok(crate::models::reports::ConsumptionReportData {
            summary: crate::models::reports::ConsumptionSummary {
                total_consumption,
                total_cost,
                meter_count,
                average_consumption_per_unit,
            },
            by_utility_type: vec![], // Could be expanded with per-type breakdown
            by_unit: vec![],         // Could be expanded with per-unit details
        })
    }

    /// Detect consumption anomalies (Epic 55, Story 55.4).
    pub async fn detect_consumption_anomalies(
        &self,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        from_date: chrono::NaiveDate,
        to_date: chrono::NaiveDate,
    ) -> Result<Vec<crate::models::reports::ConsumptionAnomaly>, SqlxError> {
        // Detect readings that deviate more than 50% from the average
        let anomalies = sqlx::query_as::<_, crate::models::reports::ConsumptionAnomaly>(
            r#"
            WITH avg_consumption AS (
                SELECT
                    m.id as meter_id,
                    AVG(mr.consumption) as avg_consumption
                FROM meter_readings mr
                JOIN meters m ON mr.meter_id = m.id
                JOIN units u ON m.unit_id = u.id
                JOIN buildings b ON u.building_id = b.id
                WHERE b.organization_id = $1
                  AND ($2::uuid IS NULL OR u.building_id = $2)
                GROUP BY m.id
            )
            SELECT
                u.id as unit_id,
                u.designation as unit_designation,
                m.id as meter_id,
                m.meter_type::text as utility_type,
                mr.reading_date,
                mr.consumption,
                ac.avg_consumption as expected_consumption,
                CASE
                    WHEN ac.avg_consumption > 0 THEN
                        ABS(mr.consumption - ac.avg_consumption) / ac.avg_consumption * 100.0
                    ELSE 0.0
                END as deviation_percentage,
                CASE
                    WHEN ABS(mr.consumption - ac.avg_consumption) / NULLIF(ac.avg_consumption, 0) * 100.0 > 100 THEN 'high'
                    WHEN ABS(mr.consumption - ac.avg_consumption) / NULLIF(ac.avg_consumption, 0) * 100.0 > 50 THEN 'medium'
                    ELSE 'low'
                END as severity
            FROM meter_readings mr
            JOIN meters m ON mr.meter_id = m.id
            JOIN units u ON m.unit_id = u.id
            JOIN buildings b ON u.building_id = b.id
            JOIN avg_consumption ac ON m.id = ac.meter_id
            WHERE b.organization_id = $1
              AND ($2::uuid IS NULL OR u.building_id = $2)
              AND mr.reading_date >= $3 AND mr.reading_date <= $4
              AND ac.avg_consumption > 0
              AND ABS(mr.consumption - ac.avg_consumption) / ac.avg_consumption * 100.0 > 50
            ORDER BY deviation_percentage DESC
            LIMIT 100
            "#,
        )
        .bind(organization_id)
        .bind(building_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(&self.pool)
        .await?;

        Ok(anomalies)
    }

    // ========================================================================
    // OCR METER CORRECTIONS (Story 128.1)
    // ========================================================================

    /// Persist an OCR correction feedback record.
    pub async fn create_ocr_correction(
        &self,
        user_id: Uuid,
        data: crate::models::meter::CreateOcrCorrection,
    ) -> Result<crate::models::meter::OcrMeterCorrection, SqlxError> {
        sqlx::query_as::<_, crate::models::meter::OcrMeterCorrection>(
            r#"
            INSERT INTO ocr_meter_corrections (
                organization_id, submitted_by, meter_reading_id,
                original_value, corrected_value, image_url, bounding_box
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(user_id)
        .bind(data.meter_reading_id)
        .bind(data.original_value)
        .bind(data.corrected_value)
        .bind(&data.image_url)
        .bind(data.bounding_box)
        .fetch_one(&self.pool)
        .await
    }

    /// Submit a meter reading with a photo URL and optional OCR value.
    pub async fn submit_photo_reading(
        &self,
        user_id: Uuid,
        meter_id: Uuid,
        reading: rust_decimal::Decimal,
        photo_url: String,
        ocr_reading: Option<rust_decimal::Decimal>,
    ) -> Result<MeterReading, SqlxError> {
        let reading_date = Utc::now().date_naive();

        let previous = sqlx::query_as::<_, MeterReading>(
            r#"
            SELECT * FROM meter_readings
            WHERE meter_id = $1 AND status = 'approved'
            ORDER BY reading_date DESC
            LIMIT 1
            "#,
        )
        .bind(meter_id)
        .fetch_optional(&self.pool)
        .await?;

        let consumption = previous.as_ref().map(|p| reading - p.reading);
        let previous_reading_id = previous.map(|p| p.id);

        sqlx::query_as::<_, MeterReading>(
            r#"
            INSERT INTO meter_readings (
                meter_id, reading, reading_date, source,
                photo_url, ocr_reading, status, consumption,
                previous_reading_id, submitted_by
            )
            VALUES ($1, $2, $3, 'photo', $4, $5, 'pending', $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(meter_id)
        .bind(reading)
        .bind(reading_date)
        .bind(&photo_url)
        .bind(ocr_reading)
        .bind(consumption)
        .bind(previous_reading_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
    }
}

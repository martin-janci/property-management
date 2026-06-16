//! IoT sensor repository (Epic 14).
//!
//! # RLS Integration (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on every sensor table (`sensors`,
//! `sensor_readings`, `sensor_alerts`, `sensor_thresholds`,
//! `sensor_threshold_templates`, `sensor_fault_correlations`). Under `FORCE`
//! the api-server's owner connection is no longer exempt, so a query issued on
//! a connection without `app.current_org_id` set collapses to deny-all (own-org
//! reads return empty, writes fail).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `budget.rs` precedent.

use crate::models::{
    AggregatedReading, AlertQuery, CreateSensor, CreateSensorAlert, CreateSensorFaultCorrelation,
    CreateSensorReading, CreateSensorThreshold, ReadingQuery, Sensor, SensorAlert,
    SensorFaultCorrelation, SensorQuery, SensorReading, SensorThreshold, SensorThresholdTemplate,
    SensorTypeCount, UpdateSensor, UpdateSensorThreshold,
};
use chrono::{DateTime, Utc};
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

/// Repository for IoT sensor operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone, Default)]
pub struct SensorRepository;

impl SensorRepository {
    pub fn new() -> Self {
        Self
    }

    // ========================================================================
    // Sensor CRUD
    // ========================================================================

    /// Create a new sensor.
    pub async fn create<'e, E>(
        &self,
        executor: E,
        data: CreateSensor,
    ) -> Result<Sensor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO sensors
                (organization_id, building_id, unit_id, name, sensor_type, location,
                 location_description, connection_type, connection_config, unit_of_measurement,
                 data_interval_seconds, manufacturer, model, firmware_version, serial_number,
                 installed_at, metadata, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(data.name)
        .bind(data.sensor_type)
        .bind(data.location)
        .bind(data.location_description)
        .bind(data.connection_type.unwrap_or_else(|| "api".to_string()))
        .bind(sqlx::types::Json(
            data.connection_config.unwrap_or_default(),
        ))
        .bind(data.unit_of_measurement)
        .bind(data.data_interval_seconds)
        .bind(data.manufacturer)
        .bind(data.model)
        .bind(data.firmware_version)
        .bind(data.serial_number)
        .bind(data.installed_at)
        .bind(sqlx::types::Json(data.metadata.unwrap_or_default()))
        .bind(data.created_by)
        .fetch_one(executor)
        .await
    }

    /// Get sensor by ID.
    pub async fn find_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Sensor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM sensors WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List sensors with filters.
    pub async fn list<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: SensorQuery,
    ) -> Result<Vec<Sensor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT * FROM sensors
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND ($3::uuid IS NULL OR unit_id = $3)
                AND ($4::text IS NULL OR sensor_type = $4)
                AND ($5::text IS NULL OR status = $5)
                AND ($6::text IS NULL OR name ILIKE '%' || $6 || '%')
            ORDER BY name
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.sensor_type)
        .bind(query.status)
        .bind(query.search)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update a sensor.
    pub async fn update<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateSensor,
    ) -> Result<Sensor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sensors SET
                name = COALESCE($2, name),
                location = COALESCE($3, location),
                location_description = COALESCE($4, location_description),
                connection_type = COALESCE($5, connection_type),
                connection_config = COALESCE($6, connection_config),
                unit_of_measurement = COALESCE($7, unit_of_measurement),
                data_interval_seconds = COALESCE($8, data_interval_seconds),
                status = COALESCE($9, status),
                manufacturer = COALESCE($10, manufacturer),
                model = COALESCE($11, model),
                firmware_version = COALESCE($12, firmware_version),
                metadata = COALESCE($13, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.name)
        .bind(data.location)
        .bind(data.location_description)
        .bind(data.connection_type)
        .bind(data.connection_config.map(sqlx::types::Json))
        .bind(data.unit_of_measurement)
        .bind(data.data_interval_seconds)
        .bind(data.status)
        .bind(data.manufacturer)
        .bind(data.model)
        .bind(data.firmware_version)
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_one(executor)
        .await
    }

    /// Delete a sensor.
    pub async fn delete<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM sensors WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Update sensor status and timestamps.
    pub async fn update_status<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<Sensor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sensors SET
                status = $2,
                last_seen_at = NOW(),
                last_error = $3,
                error_count = CASE WHEN $2 = 'error' THEN error_count + 1 ELSE 0 END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(last_error)
        .fetch_one(executor)
        .await
    }

    /// Get sensors count by type.
    pub async fn count_by_type<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<SensorTypeCount>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT sensor_type, COUNT(*) as count
            FROM sensors
            WHERE organization_id = $1
            GROUP BY sensor_type
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    // ========================================================================
    // Sensor Readings
    // ========================================================================

    /// Create a sensor reading.
    ///
    /// Multi-statement (INSERT reading + UPDATE parent sensor), so it takes a
    /// connection and reborrows it for each query.
    pub async fn create_reading(
        &self,
        conn: &mut PgConnection,
        data: CreateSensorReading,
    ) -> Result<SensorReading, sqlx::Error> {
        // Also update the sensor's last_reading_at
        let reading: SensorReading = sqlx::query_as(
            r#"
            INSERT INTO sensor_readings (sensor_id, value, unit, quality, raw_data, timestamp)
            VALUES ($1, $2, $3, $4, $5, COALESCE($6, NOW()))
            RETURNING *
            "#,
        )
        .bind(data.sensor_id)
        .bind(data.value)
        .bind(data.unit)
        .bind(data.quality.unwrap_or_else(|| "good".to_string()))
        .bind(data.raw_data.map(sqlx::types::Json))
        .bind(data.timestamp)
        .fetch_one(&mut *conn)
        .await?;

        // Update sensor status
        sqlx::query(
            "UPDATE sensors SET last_reading_at = $2, last_seen_at = NOW(), status = 'active' WHERE id = $1",
        )
        .bind(data.sensor_id)
        .bind(reading.timestamp)
        .execute(&mut *conn)
        .await?;

        Ok(reading)
    }

    /// Get readings for a sensor.
    pub async fn list_readings<'e, E>(
        &self,
        executor: E,
        sensor_id: Uuid,
        query: ReadingQuery,
    ) -> Result<Vec<SensorReading>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(100);
        let from = query
            .from_time
            .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(24));
        let to = query.to_time.unwrap_or_else(Utc::now);

        sqlx::query_as(
            r#"
            SELECT * FROM sensor_readings
            WHERE sensor_id = $1
                AND timestamp >= $2
                AND timestamp <= $3
            ORDER BY timestamp DESC
            LIMIT $4
            "#,
        )
        .bind(sensor_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    /// Get aggregated readings.
    pub async fn list_aggregated_readings<'e, E>(
        &self,
        executor: E,
        sensor_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        interval: &str,
    ) -> Result<Vec<AggregatedReading>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let interval_sql = match interval {
            "minute" => "date_trunc('minute', timestamp)",
            "hour" => "date_trunc('hour', timestamp)",
            "day" => "date_trunc('day', timestamp)",
            _ => "date_trunc('hour', timestamp)",
        };

        let query = format!(
            r#"
            SELECT
                {} as period,
                MIN(value) as min_value,
                MAX(value) as max_value,
                AVG(value) as avg_value,
                COUNT(*) as count
            FROM sensor_readings
            WHERE sensor_id = $1
                AND timestamp >= $2
                AND timestamp <= $3
            GROUP BY period
            ORDER BY period DESC
            "#,
            interval_sql
        );

        sqlx::query_as(sqlx::AssertSqlSafe(query))
            .bind(sensor_id)
            .bind(from)
            .bind(to)
            .fetch_all(executor)
            .await
    }

    /// Get latest reading for a sensor.
    pub async fn get_latest_reading<'e, E>(
        &self,
        executor: E,
        sensor_id: Uuid,
    ) -> Result<Option<SensorReading>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM sensor_readings
            WHERE sensor_id = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(sensor_id)
        .fetch_optional(executor)
        .await
    }

    // ========================================================================
    // Thresholds
    // ========================================================================

    /// Create a threshold.
    pub async fn create_threshold<'e, E>(
        &self,
        executor: E,
        data: CreateSensorThreshold,
    ) -> Result<SensorThreshold, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO sensor_thresholds
                (sensor_id, metric, comparison, warning_value, warning_high,
                 critical_value, critical_high, alert_cooldown_minutes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.sensor_id)
        .bind(data.metric.unwrap_or_else(|| "value".to_string()))
        .bind(data.comparison)
        .bind(data.warning_value)
        .bind(data.warning_high)
        .bind(data.critical_value)
        .bind(data.critical_high)
        .bind(data.alert_cooldown_minutes)
        .fetch_one(executor)
        .await
    }

    /// Get thresholds for a sensor.
    pub async fn list_thresholds<'e, E>(
        &self,
        executor: E,
        sensor_id: Uuid,
    ) -> Result<Vec<SensorThreshold>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM sensor_thresholds WHERE sensor_id = $1 ORDER BY metric")
            .bind(sensor_id)
            .fetch_all(executor)
            .await
    }

    /// Update a threshold.
    pub async fn update_threshold<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateSensorThreshold,
    ) -> Result<SensorThreshold, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sensor_thresholds SET
                comparison = COALESCE($2, comparison),
                warning_value = COALESCE($3, warning_value),
                warning_high = COALESCE($4, warning_high),
                critical_value = COALESCE($5, critical_value),
                critical_high = COALESCE($6, critical_high),
                enabled = COALESCE($7, enabled),
                alert_cooldown_minutes = COALESCE($8, alert_cooldown_minutes),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.comparison)
        .bind(data.warning_value)
        .bind(data.warning_high)
        .bind(data.critical_value)
        .bind(data.critical_high)
        .bind(data.enabled)
        .bind(data.alert_cooldown_minutes)
        .fetch_one(executor)
        .await
    }

    /// Delete a threshold.
    pub async fn delete_threshold<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM sensor_thresholds WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get threshold templates.
    pub async fn list_threshold_templates<'e, E>(
        &self,
        executor: E,
        org_id: Option<Uuid>,
        sensor_type: Option<&str>,
    ) -> Result<Vec<SensorThresholdTemplate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM sensor_threshold_templates
            WHERE (organization_id IS NULL OR organization_id = $1)
                AND ($2::text IS NULL OR sensor_type = $2)
            ORDER BY is_default DESC, name
            "#,
        )
        .bind(org_id)
        .bind(sensor_type)
        .fetch_all(executor)
        .await
    }

    // ========================================================================
    // Alerts
    // ========================================================================

    /// Create an alert.
    pub async fn create_alert<'e, E>(
        &self,
        executor: E,
        data: CreateSensorAlert,
    ) -> Result<SensorAlert, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO sensor_alerts
                (sensor_id, threshold_id, severity, triggered_value, threshold_value, message)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(data.sensor_id)
        .bind(data.threshold_id)
        .bind(data.severity)
        .bind(data.triggered_value)
        .bind(data.threshold_value)
        .bind(data.message)
        .fetch_one(executor)
        .await
    }

    /// List alerts with filters.
    pub async fn list_alerts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: AlertQuery,
    ) -> Result<Vec<SensorAlert>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as(
            r#"
            SELECT a.* FROM sensor_alerts a
            JOIN sensors s ON s.id = a.sensor_id
            WHERE s.organization_id = $1
                AND ($2::uuid IS NULL OR a.sensor_id = $2)
                AND ($3::uuid IS NULL OR s.building_id = $3)
                AND ($4::text IS NULL OR a.severity = $4)
                AND ($5::boolean IS NULL OR (a.resolved_at IS NOT NULL) = $5)
                AND ($6::boolean IS NULL OR (a.acknowledged_at IS NOT NULL) = $6)
                AND ($7::timestamptz IS NULL OR a.triggered_at >= $7)
                AND ($8::timestamptz IS NULL OR a.triggered_at <= $8)
            ORDER BY a.triggered_at DESC
            LIMIT $9 OFFSET $10
            "#,
        )
        .bind(org_id)
        .bind(query.sensor_id)
        .bind(query.building_id)
        .bind(query.severity)
        .bind(query.resolved)
        .bind(query.acknowledged)
        .bind(query.from_time)
        .bind(query.to_time)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Resolve an alert.
    /// resolved_value is optional - if None, only resolved_at is set.
    pub async fn resolve_alert<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        resolved_value: Option<f64>,
    ) -> Result<SensorAlert, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sensor_alerts SET
                resolved_at = NOW(),
                resolved_value = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(resolved_value)
        .fetch_one(executor)
        .await
    }

    /// Acknowledge an alert.
    pub async fn acknowledge_alert<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<SensorAlert, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sensor_alerts SET
                acknowledged_at = NOW(),
                acknowledged_by = $2
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Count unresolved alerts.
    pub async fn count_unresolved_alerts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM sensor_alerts a
            JOIN sensors s ON s.id = a.sensor_id
            WHERE s.organization_id = $1 AND a.resolved_at IS NULL
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await?;
        Ok(result.0)
    }

    // ========================================================================
    // Correlations
    // ========================================================================

    /// Create a sensor-fault correlation.
    pub async fn create_correlation<'e, E>(
        &self,
        executor: E,
        data: CreateSensorFaultCorrelation,
    ) -> Result<SensorFaultCorrelation, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO sensor_fault_correlations
                (sensor_id, fault_id, correlation_type, confidence, sensor_data_start,
                 sensor_data_end, summary, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.sensor_id)
        .bind(data.fault_id)
        .bind(
            data.correlation_type
                .unwrap_or_else(|| "manual".to_string()),
        )
        .bind(data.confidence)
        .bind(data.sensor_data_start)
        .bind(data.sensor_data_end)
        .bind(data.summary)
        .bind(data.created_by)
        .fetch_one(executor)
        .await
    }

    /// Get correlations for a fault.
    pub async fn list_correlations_for_fault<'e, E>(
        &self,
        executor: E,
        fault_id: Uuid,
    ) -> Result<Vec<SensorFaultCorrelation>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM sensor_fault_correlations WHERE fault_id = $1")
            .bind(fault_id)
            .fetch_all(executor)
            .await
    }

    /// Get correlations for a sensor.
    pub async fn list_correlations_for_sensor<'e, E>(
        &self,
        executor: E,
        sensor_id: Uuid,
    ) -> Result<Vec<SensorFaultCorrelation>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM sensor_fault_correlations WHERE sensor_id = $1")
            .bind(sensor_id)
            .fetch_all(executor)
            .await
    }

    /// Delete a correlation.
    pub async fn delete_correlation<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM sensor_fault_correlations WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Get sensors near a location (for auto-correlation).
    pub async fn get_sensors_for_building<'e, E>(
        &self,
        executor: E,
        building_id: Uuid,
    ) -> Result<Vec<Sensor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM sensors
            WHERE building_id = $1 AND status = 'active'
            ORDER BY name
            "#,
        )
        .bind(building_id)
        .fetch_all(executor)
        .await
    }

    /// Create batch readings using efficient bulk insert.
    ///
    /// Multi-statement (bulk INSERT + UPDATE parent sensor), so it takes a
    /// connection and reborrows it for each query.
    pub async fn create_batch_readings(
        &self,
        conn: &mut PgConnection,
        sensor_id: Uuid,
        readings: Vec<crate::models::SingleReading>,
    ) -> Result<i64, sqlx::Error> {
        if readings.is_empty() {
            return Ok(0);
        }

        let count = readings.len() as i64;

        // Build bulk INSERT with multiple VALUES for efficiency (single round-trip)
        let mut values_parts = Vec::with_capacity(readings.len());
        let mut param_idx = 1;

        for _ in &readings {
            values_parts.push(format!(
                "(${}, ${}, ${}, ${}, ${})",
                param_idx,
                param_idx + 1,
                param_idx + 2,
                param_idx + 3,
                param_idx + 4
            ));
            param_idx += 5;
        }

        let query = format!(
            "INSERT INTO sensor_readings (sensor_id, value, unit, quality, timestamp) VALUES {}",
            values_parts.join(", ")
        );

        let mut query_builder = sqlx::query(sqlx::AssertSqlSafe(query));
        for reading in &readings {
            query_builder = query_builder
                .bind(sensor_id)
                .bind(reading.value)
                .bind(&reading.unit)
                .bind(reading.quality.as_deref().unwrap_or("good"))
                .bind(reading.timestamp);
        }

        query_builder.execute(&mut *conn).await?;

        // Update sensor status
        sqlx::query(
            "UPDATE sensors SET last_reading_at = NOW(), last_seen_at = NOW(), status = 'active' WHERE id = $1",
        )
        .bind(sensor_id)
        .execute(&mut *conn)
        .await?;

        Ok(count)
    }

    /// Apply threshold template to a sensor.
    ///
    /// Multi-statement (SELECT template + upsert threshold), so it takes a
    /// connection and reborrows it for each query.
    pub async fn apply_threshold_template(
        &self,
        conn: &mut PgConnection,
        template_id: Uuid,
        sensor_id: Uuid,
    ) -> Result<SensorThreshold, sqlx::Error> {
        // Get the template first
        let template: SensorThresholdTemplate =
            sqlx::query_as("SELECT * FROM sensor_threshold_templates WHERE id = $1")
                .bind(template_id)
                .fetch_one(&mut *conn)
                .await?;

        // Create threshold from template
        sqlx::query_as(
            r#"
            INSERT INTO sensor_thresholds
                (sensor_id, metric, comparison, warning_value, warning_high,
                 critical_value, critical_high)
            VALUES ($1, 'value', $2, $3, $4, $5, $6)
            ON CONFLICT (sensor_id, metric) DO UPDATE SET
                comparison = EXCLUDED.comparison,
                warning_value = EXCLUDED.warning_value,
                warning_high = EXCLUDED.warning_high,
                critical_value = EXCLUDED.critical_value,
                critical_high = EXCLUDED.critical_high,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(sensor_id)
        .bind(&template.comparison)
        .bind(template.warning_value)
        .bind(template.warning_high)
        .bind(template.critical_value)
        .bind(template.critical_high)
        .fetch_one(&mut *conn)
        .await
    }

    /// Get dashboard data for an organization.
    ///
    /// Multi-statement (six aggregate reads), so it takes a connection and
    /// reborrows it for each query.
    pub async fn get_dashboard(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        building_id: Option<Uuid>,
    ) -> Result<crate::models::SensorDashboard, sqlx::Error> {
        // Get sensor counts
        let (total,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sensors WHERE organization_id = $1 AND ($2::uuid IS NULL OR building_id = $2)",
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        let (active,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sensors WHERE organization_id = $1 AND status = 'active' AND ($2::uuid IS NULL OR building_id = $2)",
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        let (offline,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sensors WHERE organization_id = $1 AND status = 'offline' AND ($2::uuid IS NULL OR building_id = $2)",
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        // Count unresolved alerts
        let (unresolved_alerts,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM sensor_alerts a
            JOIN sensors s ON s.id = a.sensor_id
            WHERE s.organization_id = $1 AND a.resolved_at IS NULL
                AND ($2::uuid IS NULL OR s.building_id = $2)
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_one(&mut *conn)
        .await?;

        // Get sensors by type
        let sensors_by_type: Vec<SensorTypeCount> = sqlx::query_as(
            r#"
            SELECT sensor_type, COUNT(*) as count
            FROM sensors
            WHERE organization_id = $1 AND ($2::uuid IS NULL OR building_id = $2)
            GROUP BY sensor_type
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await?;

        // Get recent alerts
        let recent_alerts: Vec<SensorAlert> = sqlx::query_as(
            r#"
            SELECT a.* FROM sensor_alerts a
            JOIN sensors s ON s.id = a.sensor_id
            WHERE s.organization_id = $1 AND ($2::uuid IS NULL OR s.building_id = $2)
            ORDER BY a.triggered_at DESC
            LIMIT 10
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .fetch_all(&mut *conn)
        .await?;

        Ok(crate::models::SensorDashboard {
            total_sensors: total,
            active_sensors: active,
            offline_sensors: offline,
            unresolved_alerts,
            sensors_by_type,
            recent_alerts,
        })
    }
}

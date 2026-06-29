//! Reading writes (single + batch), listing & aggregation.

use super::SensorRepository;
use crate::models::{AggregatedReading, CreateSensorReading, ReadingQuery, SensorReading};
use chrono::{DateTime, Utc};
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

impl SensorRepository {
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
}

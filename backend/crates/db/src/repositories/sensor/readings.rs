//! Reading writes (single + batch), listing & aggregation.

use super::SensorRepository;
use crate::models::{AggregatedReading, CreateSensorReading, ReadingQuery, SensorReading};
use chrono::{DateTime, Utc};
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

impl SensorRepository {
    /// Create a sensor reading, scoped to `org_id`.
    ///
    /// The sensor-in-org guard is folded INTO the INSERT (`INSERT … SELECT …
    /// WHERE EXISTS(… sensors WHERE id=$sensor AND organization_id=$org)`), so a
    /// cross-org or missing sensor writes zero rows and returns `Ok(None)` in a
    /// single statement — no separate `SELECT EXISTS` round-trip and no
    /// check-then-insert TOCTOU window (issue #3305). This matches the
    /// folded-predicate pattern already used by the sensor CRUD queries and
    /// holds even when the connection's role bypasses RLS.
    ///
    /// Multi-statement (INSERT reading + UPDATE parent sensor), so it takes a
    /// connection and reborrows it for each query. The parent UPDATE runs only
    /// when a row was actually inserted.
    pub async fn create_reading(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        data: CreateSensorReading,
    ) -> Result<Option<SensorReading>, sqlx::Error> {
        let reading: Option<SensorReading> = sqlx::query_as(
            r#"
            INSERT INTO sensor_readings (sensor_id, value, unit, quality, raw_data, timestamp)
            SELECT $1, $2, $3, $4, $5, COALESCE($6, NOW())
            WHERE EXISTS (
                SELECT 1 FROM sensors WHERE id = $1 AND organization_id = $7
            )
            RETURNING *
            "#,
        )
        .bind(data.sensor_id)
        .bind(data.value)
        .bind(data.unit)
        .bind(data.quality.unwrap_or_else(|| "good".to_string()))
        .bind(data.raw_data.map(sqlx::types::Json))
        .bind(data.timestamp)
        .bind(org_id)
        .fetch_optional(&mut *conn)
        .await?;

        let Some(reading) = reading else {
            return Ok(None);
        };

        // Update sensor status (the parent sensor is guaranteed in-org by the
        // folded guard above).
        sqlx::query(
            "UPDATE sensors SET last_reading_at = $2, last_seen_at = NOW(), status = 'active' WHERE id = $1",
        )
        .bind(data.sensor_id)
        .bind(reading.timestamp)
        .execute(&mut *conn)
        .await?;

        Ok(Some(reading))
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

    /// Create batch readings using an efficient bulk insert, scoped to `org_id`.
    ///
    /// Like [`create_reading`](Self::create_reading), the sensor-in-org guard is
    /// folded into the bulk INSERT (`INSERT … SELECT … FROM (VALUES …) WHERE
    /// EXISTS(…)`), so a cross-org or missing sensor writes zero rows. Returns
    /// `Ok(None)` when a non-empty batch wrote nothing (sensor not in org →
    /// 404), `Ok(Some(n))` for the number of rows inserted otherwise (issue
    /// #3305). The VALUES parameters are explicitly cast because the derived
    /// table's column types can't be inferred from the INSERT target columns.
    ///
    /// Multi-statement (bulk INSERT + UPDATE parent sensor), so it takes a
    /// connection and reborrows it for each query. The parent UPDATE runs only
    /// when rows were actually inserted.
    pub async fn create_batch_readings(
        &self,
        conn: &mut PgConnection,
        sensor_id: Uuid,
        org_id: Uuid,
        readings: Vec<crate::models::SingleReading>,
    ) -> Result<Option<i64>, sqlx::Error> {
        if readings.is_empty() {
            return Ok(Some(0));
        }

        // Build the VALUES rows for the derived table; casts give Postgres the
        // column types it can't infer through the EXISTS-guarded SELECT.
        let mut values_parts = Vec::with_capacity(readings.len());
        let mut param_idx = 1;

        for _ in &readings {
            values_parts.push(format!(
                "(${}::uuid, ${}::float8, ${}::text, ${}::text, ${}::timestamptz)",
                param_idx,
                param_idx + 1,
                param_idx + 2,
                param_idx + 3,
                param_idx + 4
            ));
            param_idx += 5;
        }

        // The sensor-id and org-id binds for the EXISTS guard follow the row binds.
        let query = format!(
            "INSERT INTO sensor_readings (sensor_id, value, unit, quality, timestamp) \
             SELECT v.sensor_id, v.value, v.unit, v.quality, v.timestamp \
             FROM (VALUES {}) AS v(sensor_id, value, unit, quality, timestamp) \
             WHERE EXISTS (SELECT 1 FROM sensors WHERE id = ${} AND organization_id = ${})",
            values_parts.join(", "),
            param_idx,
            param_idx + 1,
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
        query_builder = query_builder.bind(sensor_id).bind(org_id);

        let inserted = query_builder.execute(&mut *conn).await?.rows_affected() as i64;
        if inserted == 0 {
            // Non-empty batch wrote nothing: the sensor is not in the caller's org.
            return Ok(None);
        }

        // Update sensor status (the parent sensor is guaranteed in-org).
        sqlx::query(
            "UPDATE sensors SET last_reading_at = NOW(), last_seen_at = NOW(), status = 'active' WHERE id = $1",
        )
        .bind(sensor_id)
        .execute(&mut *conn)
        .await?;

        Ok(Some(inserted))
    }
}

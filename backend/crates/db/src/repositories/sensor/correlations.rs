//! Sensor-fault correlation CRUD & lookups.

use super::SensorRepository;
use crate::models::{CreateSensorFaultCorrelation, SensorFaultCorrelation};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl SensorRepository {
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
}

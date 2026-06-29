//! Alert CRUD, resolve/acknowledge & counts.

use super::SensorRepository;
use crate::models::{AlertQuery, CreateSensorAlert, SensorAlert};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl SensorRepository {
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
}

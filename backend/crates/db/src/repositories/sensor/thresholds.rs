//! Threshold CRUD, templates & template application.

use super::SensorRepository;
use crate::models::{
    CreateSensorThreshold, SensorThreshold, SensorThresholdTemplate, UpdateSensorThreshold,
};
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

impl SensorRepository {
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
}

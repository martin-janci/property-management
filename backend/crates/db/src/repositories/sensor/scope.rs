//! Organization-scope checks for sensor resources.
//!
//! Handlers address sensors, thresholds, alerts and correlations by id. These
//! helpers resolve the owning organization through `sensors.organization_id`
//! so a handler can reject (404) an id that belongs to another organization
//! before reading or mutating it, independent of whether RLS is enforced on the
//! connection (e.g. a superuser/owner role bypasses `FORCE ROW LEVEL SECURITY`).

use super::SensorRepository;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl SensorRepository {
    /// `true` iff the sensor exists and belongs to `org_id`.
    pub async fn sensor_in_org<'e, E>(
        &self,
        executor: E,
        sensor_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sensors WHERE id = $1 AND organization_id = $2)",
        )
        .bind(sensor_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// `true` iff the threshold's sensor belongs to `org_id`.
    pub async fn threshold_in_org<'e, E>(
        &self,
        executor: E,
        threshold_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM sensor_thresholds t
                JOIN sensors s ON s.id = t.sensor_id
                WHERE t.id = $1 AND s.organization_id = $2
            )
            "#,
        )
        .bind(threshold_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// `true` iff the alert's sensor belongs to `org_id`.
    pub async fn alert_in_org<'e, E>(
        &self,
        executor: E,
        alert_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM sensor_alerts a
                JOIN sensors s ON s.id = a.sensor_id
                WHERE a.id = $1 AND s.organization_id = $2
            )
            "#,
        )
        .bind(alert_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// `true` iff the correlation's sensor belongs to `org_id`.
    pub async fn correlation_in_org<'e, E>(
        &self,
        executor: E,
        correlation_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM sensor_fault_correlations c
                JOIN sensors s ON s.id = c.sensor_id
                WHERE c.id = $1 AND s.organization_id = $2
            )
            "#,
        )
        .bind(correlation_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// `true` iff the threshold template is global or owned by `org_id`.
    pub async fn template_visible_to_org<'e, E>(
        &self,
        executor: E,
        template_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM sensor_threshold_templates
                WHERE id = $1 AND (organization_id IS NULL OR organization_id = $2)
            )
            "#,
        )
        .bind(template_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }

    /// `true` iff the fault belongs to `org_id`.
    pub async fn fault_in_org<'e, E>(
        &self,
        executor: E,
        fault_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM faults WHERE id = $1 AND organization_id = $2)",
        )
        .bind(fault_id)
        .bind(org_id)
        .fetch_one(executor)
        .await
    }
}

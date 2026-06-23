//! Usage records & current-usage counts (Epic 26).

use super::SubscriptionRepository;
use crate::models::{CreateUsageRecord, UsageRecord, UsageSummary};
use chrono::Utc;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl SubscriptionRepository {
    // ==================== Usage Records ====================

    /// Record a usage metric.
    pub async fn record_usage<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        subscription_id: Option<Uuid>,
        data: CreateUsageRecord,
    ) -> Result<UsageRecord, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO usage_records
                (organization_id, subscription_id, metric_type, quantity, unit, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(subscription_id)
        .bind(&data.metric_type)
        .bind(data.quantity)
        .bind(&data.unit)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Get usage summary for an organization.
    pub async fn get_usage_summary<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        period_start: chrono::DateTime<Utc>,
        period_end: chrono::DateTime<Utc>,
    ) -> Result<Vec<UsageSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                metric_type,
                SUM(quantity) as total_quantity,
                MAX(unit) as unit,
                COUNT(*) as record_count
            FROM usage_records
            WHERE organization_id = $1
            AND recorded_at >= $2 AND recorded_at < $3
            GROUP BY metric_type
            "#,
        )
        .bind(org_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(executor)
        .await
    }

    /// Get current usage counts for an organization.
    pub async fn get_current_usage<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<(i64, i64, i64, i64), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Combined query to get all counts in one round-trip
        let counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM buildings WHERE organization_id = $1) as building_count,
                (SELECT COUNT(*) FROM units u JOIN buildings b ON u.building_id = b.id WHERE b.organization_id = $1) as unit_count,
                (SELECT COUNT(*) FROM organization_members WHERE organization_id = $1) as user_count
            "#,
        )
        .bind(org_id)
        .fetch_one(executor)
        .await?;

        // Storage would require summing document sizes - using 0 as placeholder
        let storage = 0i64;

        Ok((counts.0, counts.1, counts.2, storage))
    }
}

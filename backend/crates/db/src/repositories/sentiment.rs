//! Sentiment analysis repository (Epic 13, Story 13.2).
//!
//! # RLS Integration (PAP-67 / PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `sentiment_trends`, `sentiment_alerts`, and
//! `sentiment_thresholds`. Under `FORCE` the api-server's owner connection is no
//! longer exempt, so a query issued on a connection without `app.current_org_id`
//! set collapses to deny-all (own-org reads return empty, writes fail the policy
//! `WITH CHECK`).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `vendor.rs` precedent.

use crate::models::{
    CreateSentimentAlert, SentimentAlert, SentimentThresholds, SentimentTrend, SentimentTrendQuery,
    UpdateSentimentThresholds, UpsertSentimentTrend,
};
use chrono::NaiveDate;
use sqlx::{Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

/// Repository for sentiment analysis operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct SentimentRepository;

impl SentimentRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    /// Upsert a sentiment trend record.
    pub async fn upsert_trend<'e, E>(
        &self,
        executor: E,
        data: UpsertSentimentTrend,
    ) -> Result<SentimentTrend, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO sentiment_trends
                (organization_id, building_id, date, avg_sentiment, message_count, negative_count, neutral_count, positive_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (organization_id, building_id, date) DO UPDATE SET
                avg_sentiment = EXCLUDED.avg_sentiment,
                message_count = EXCLUDED.message_count,
                negative_count = EXCLUDED.negative_count,
                neutral_count = EXCLUDED.neutral_count,
                positive_count = EXCLUDED.positive_count,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.building_id)
        .bind(data.date)
        .bind(data.avg_sentiment)
        .bind(data.message_count)
        .bind(data.negative_count)
        .bind(data.neutral_count)
        .bind(data.positive_count)
        .fetch_one(executor)
        .await
    }

    /// Get sentiment trends.
    pub async fn list_trends<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: SentimentTrendQuery,
    ) -> Result<Vec<SentimentTrend>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(30);
        let from_date = query
            .from_date
            .unwrap_or_else(|| chrono::Utc::now().date_naive() - chrono::Duration::days(30));
        let to_date = query
            .to_date
            .unwrap_or_else(|| chrono::Utc::now().date_naive());

        if let Some(building_id) = query.building_id {
            sqlx::query_as(
                r#"
                SELECT * FROM sentiment_trends
                WHERE organization_id = $1 AND building_id = $2
                    AND date >= $3 AND date <= $4
                ORDER BY date DESC
                LIMIT $5
                "#,
            )
            .bind(org_id)
            .bind(building_id)
            .bind(from_date)
            .bind(to_date)
            .bind(limit)
            .fetch_all(executor)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT * FROM sentiment_trends
                WHERE organization_id = $1 AND building_id IS NULL
                    AND date >= $2 AND date <= $3
                ORDER BY date DESC
                LIMIT $4
                "#,
            )
            .bind(org_id)
            .bind(from_date)
            .bind(to_date)
            .bind(limit)
            .fetch_all(executor)
            .await
        }
    }

    /// Create a sentiment alert.
    pub async fn create_alert<'e, E>(
        &self,
        executor: E,
        data: CreateSentimentAlert,
    ) -> Result<SentimentAlert, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO sentiment_alerts
                (organization_id, building_id, alert_type, threshold_breached, current_sentiment, previous_sentiment, sample_message_ids)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.building_id)
        .bind(data.alert_type)
        .bind(data.threshold_breached)
        .bind(data.current_sentiment)
        .bind(data.previous_sentiment)
        .bind(&data.sample_message_ids)
        .fetch_one(executor)
        .await
    }

    /// List alerts for organization.
    pub async fn list_alerts<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        acknowledged: Option<bool>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SentimentAlert>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        match acknowledged {
            Some(ack) => {
                sqlx::query_as(
                    r#"
                    SELECT * FROM sentiment_alerts
                    WHERE organization_id = $1 AND acknowledged = $2
                    ORDER BY created_at DESC
                    LIMIT $3 OFFSET $4
                    "#,
                )
                .bind(org_id)
                .bind(ack)
                .bind(limit)
                .bind(offset)
                .fetch_all(executor)
                .await
            }
            None => {
                sqlx::query_as(
                    r#"
                    SELECT * FROM sentiment_alerts
                    WHERE organization_id = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                )
                .bind(org_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(executor)
                .await
            }
        }
    }

    /// Acknowledge an alert — tenant-scoped.
    ///
    /// `org_id` is the authoritative tenant from `RlsConnection::tenant_id()`.
    /// The `WHERE id = $1 AND organization_id = $2` clause plus the active
    /// `FORCE` RLS policy both scope the write to the caller's org, so a caller
    /// in org B cannot acknowledge an alert that belongs to org A; `fetch_one`
    /// returns `RowNotFound` when either the alert does not exist or belongs to
    /// a different tenant (callers should surface both as 404).
    pub async fn acknowledge_alert<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<SentimentAlert, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE sentiment_alerts
            SET acknowledged = TRUE, acknowledged_by = $3, acknowledged_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Get or create sentiment thresholds for organization.
    ///
    /// Multi-statement (SELECT then conditional INSERT), so it takes a
    /// `&mut PgConnection` and reborrows for each query rather than a generic
    /// by-value executor.
    pub async fn get_thresholds(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> Result<SentimentThresholds, sqlx::Error> {
        // Try to get existing
        let existing: Option<SentimentThresholds> =
            sqlx::query_as("SELECT * FROM sentiment_thresholds WHERE organization_id = $1")
                .bind(org_id)
                .fetch_optional(&mut *conn)
                .await?;

        match existing {
            Some(t) => Ok(t),
            None => {
                // Create default
                sqlx::query_as(
                    r#"
                    INSERT INTO sentiment_thresholds (organization_id)
                    VALUES ($1)
                    RETURNING *
                    "#,
                )
                .bind(org_id)
                .fetch_one(&mut *conn)
                .await
            }
        }
    }

    /// Update sentiment thresholds.
    pub async fn update_thresholds<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: UpdateSentimentThresholds,
    ) -> Result<SentimentThresholds, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Upsert: a bare UPDATE ... RETURNING + fetch_one raised `RowNotFound`
        // (→ 500) for an org that never materialized a thresholds row, whereas
        // `get_thresholds` auto-creates one. Insert defaults on first write and
        // COALESCE-merge on conflict so PUT /thresholds is create-or-update.
        // The INSERT-side literal defaults mirror migration 00043.
        sqlx::query_as(
            r#"
            INSERT INTO sentiment_thresholds (
                organization_id, negative_threshold, alert_on_spike,
                spike_threshold_change, min_messages_for_alert, enabled
            )
            VALUES (
                $1,
                COALESCE($2, -0.3),
                COALESCE($3, TRUE),
                COALESCE($4, 0.2),
                COALESCE($5, 5),
                COALESCE($6, TRUE)
            )
            ON CONFLICT (organization_id) DO UPDATE SET
                negative_threshold = COALESCE($2, sentiment_thresholds.negative_threshold),
                alert_on_spike = COALESCE($3, sentiment_thresholds.alert_on_spike),
                spike_threshold_change = COALESCE($4, sentiment_thresholds.spike_threshold_change),
                min_messages_for_alert = COALESCE($5, sentiment_thresholds.min_messages_for_alert),
                enabled = COALESCE($6, sentiment_thresholds.enabled),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.negative_threshold)
        .bind(data.alert_on_spike)
        .bind(data.spike_threshold_change)
        .bind(data.min_messages_for_alert)
        .bind(data.enabled)
        .fetch_one(executor)
        .await
    }

    /// Get organization average sentiment for a date range.
    pub async fn get_org_average_sentiment<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<f64, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result: (Option<f64>,) = sqlx::query_as(
            r#"
            SELECT AVG(avg_sentiment) as avg
            FROM sentiment_trends
            WHERE organization_id = $1 AND building_id IS NULL
                AND date >= $2 AND date <= $3
            "#,
        )
        .bind(org_id)
        .bind(from_date)
        .bind(to_date)
        .fetch_one(executor)
        .await?;

        Ok(result.0.unwrap_or(0.0))
    }
}

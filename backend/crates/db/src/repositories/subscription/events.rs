//! Subscription event log (Epic 26).

use super::SubscriptionRepository;
use crate::models::{CreateSubscriptionEvent, SubscriptionEvent};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl SubscriptionRepository {
    // ==================== Subscription Events ====================

    /// Log a subscription event.
    pub async fn log_event<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        subscription_id: Option<Uuid>,
        actor_id: Option<Uuid>,
        data: CreateSubscriptionEvent,
    ) -> Result<SubscriptionEvent, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO subscription_events
                (organization_id, subscription_id, event_type, description, actor_id,
                 previous_data, new_data, webhook_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(subscription_id)
        .bind(&data.event_type)
        .bind(&data.description)
        .bind(actor_id)
        .bind(&data.previous_data)
        .bind(&data.new_data)
        .bind(&data.webhook_id)
        .fetch_one(executor)
        .await
    }

    /// Get subscription events.
    pub async fn get_events<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        limit: i32,
    ) -> Result<Vec<SubscriptionEvent>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM subscription_events WHERE organization_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(org_id)
        .bind(limit)
        .fetch_all(executor)
        .await
    }
}

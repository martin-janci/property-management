//! Subscription plan CRUD (Epic 26).

use super::SubscriptionRepository;
use crate::models::{CreateSubscriptionPlan, SubscriptionPlan, UpdateSubscriptionPlan};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

impl SubscriptionRepository {
    // ==================== Subscription Plans CRUD ====================

    /// Create a new subscription plan.
    pub async fn create_plan<'e, E>(
        &self,
        executor: E,
        data: CreateSubscriptionPlan,
    ) -> Result<SubscriptionPlan, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO subscription_plans
                (name, display_name, description, monthly_price, annual_price, currency,
                 max_buildings, max_units, max_users, max_storage_gb, features, trial_days,
                 sort_order, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(&data.name)
        .bind(&data.display_name)
        .bind(&data.description)
        .bind(data.monthly_price)
        .bind(data.annual_price)
        .bind(data.currency.unwrap_or_else(|| "EUR".to_string()))
        .bind(data.max_buildings)
        .bind(data.max_units)
        .bind(data.max_users)
        .bind(data.max_storage_gb)
        .bind(&data.features)
        .bind(data.trial_days)
        .bind(data.sort_order)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find a subscription plan by ID.
    pub async fn find_plan_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM subscription_plans WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find a subscription plan by name.
    pub async fn find_plan_by_name<'e, E>(
        &self,
        executor: E,
        name: &str,
    ) -> Result<Option<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM subscription_plans WHERE name = $1")
            .bind(name)
            .fetch_optional(executor)
            .await
    }

    /// List all subscription plans.
    pub async fn list_plans<'e, E>(
        &self,
        executor: E,
        active_only: bool,
    ) -> Result<Vec<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if active_only {
            sqlx::query_as(
                "SELECT * FROM subscription_plans WHERE is_active = true ORDER BY sort_order, monthly_price",
            )
            .fetch_all(executor)
            .await
        } else {
            sqlx::query_as("SELECT * FROM subscription_plans ORDER BY sort_order, monthly_price")
                .fetch_all(executor)
                .await
        }
    }

    /// List public subscription plans (for display to customers).
    ///
    /// `subscription_plans` is not FORCE-bound and carries a public read
    /// policy, so the public (unauthenticated) plans endpoint may pass a
    /// plain pool executor here.
    pub async fn list_public_plans<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<SubscriptionPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM subscription_plans WHERE is_active = true AND is_public = true ORDER BY sort_order, monthly_price",
        )
        .fetch_all(executor)
        .await
    }

    /// Update a subscription plan.
    pub async fn update_plan<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateSubscriptionPlan,
    ) -> Result<SubscriptionPlan, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE subscription_plans SET
                display_name = COALESCE($2, display_name),
                description = COALESCE($3, description),
                monthly_price = COALESCE($4, monthly_price),
                annual_price = COALESCE($5, annual_price),
                currency = COALESCE($6, currency),
                max_buildings = COALESCE($7, max_buildings),
                max_units = COALESCE($8, max_units),
                max_users = COALESCE($9, max_users),
                max_storage_gb = COALESCE($10, max_storage_gb),
                features = COALESCE($11, features),
                is_active = COALESCE($12, is_active),
                is_public = COALESCE($13, is_public),
                trial_days = COALESCE($14, trial_days),
                sort_order = COALESCE($15, sort_order),
                metadata = COALESCE($16, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.display_name)
        .bind(&data.description)
        .bind(data.monthly_price)
        .bind(data.annual_price)
        .bind(&data.currency)
        .bind(data.max_buildings)
        .bind(data.max_units)
        .bind(data.max_users)
        .bind(data.max_storage_gb)
        .bind(&data.features)
        .bind(data.is_active)
        .bind(data.is_public)
        .bind(data.trial_days)
        .bind(data.sort_order)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Delete a subscription plan.
    pub async fn delete_plan<'e, E>(&self, executor: E, id: Uuid) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM subscription_plans WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

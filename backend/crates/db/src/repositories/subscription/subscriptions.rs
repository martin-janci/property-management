//! Organization subscription CRUD & lifecycle (Epic 26).

use super::SubscriptionRepository;
use crate::models::{
    CancelSubscriptionRequest, ChangePlanRequest, CreateOrganizationSubscription,
    OrganizationSubscription, SubscriptionWithPlan, UpdateOrganizationSubscription,
};
use chrono::{Days, Months, Utc};
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

impl SubscriptionRepository {
    // ==================== Organization Subscriptions CRUD ====================

    /// Create a new organization subscription.
    ///
    /// Multi-statement (plan lookup for trial-days, then insert), so it takes
    /// `&mut PgConnection` and runs both statements on the same context-set
    /// connection.
    pub async fn create_subscription(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        data: CreateOrganizationSubscription,
    ) -> Result<OrganizationSubscription, sqlx::Error> {
        // Get plan for trial days calculation.
        let plan = self.find_plan_by_id(&mut *conn, data.plan_id).await?;

        let billing_cycle = data.billing_cycle.unwrap_or_else(|| "monthly".to_string());
        let now = Utc::now();

        // Calculate period end based on billing cycle using calendar months/years
        let period_end = if billing_cycle == "annual" {
            now.checked_add_months(Months::new(12)).unwrap_or(now)
        } else {
            now.checked_add_months(Months::new(1)).unwrap_or(now)
        };

        // Calculate trial dates if starting trial
        let (trial_start, trial_end, is_trial, status) = if data.start_trial.unwrap_or(false)
            && plan.as_ref().is_some_and(|p| p.trial_days.unwrap_or(0) > 0)
        {
            let trial_days = plan
                .as_ref()
                .map(|p| p.trial_days.unwrap_or(14))
                .unwrap_or(14);
            let trial_end = now
                .checked_add_days(Days::new(trial_days as u64))
                .unwrap_or(now);
            (Some(now), Some(trial_end), Some(true), "trialing")
        } else {
            (None, None, Some(false), "active")
        };

        sqlx::query_as(
            r#"
            INSERT INTO organization_subscriptions
                (organization_id, plan_id, status, billing_cycle, current_period_start,
                 current_period_end, trial_start, trial_end, is_trial, payment_method_id, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.plan_id)
        .bind(status)
        .bind(&billing_cycle)
        .bind(now)
        .bind(period_end)
        .bind(trial_start)
        .bind(trial_end)
        .bind(is_trial)
        .bind(data.payment_method_id)
        .bind(&data.metadata)
        .fetch_one(&mut *conn)
        .await
    }

    /// Find an organization's subscription.
    pub async fn find_subscription_by_org<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Option<OrganizationSubscription>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM organization_subscriptions WHERE organization_id = $1 AND status NOT IN ('cancelled', 'expired') ORDER BY created_at DESC LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Find a subscription by ID.
    pub async fn find_subscription_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<OrganizationSubscription>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM organization_subscriptions WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Get subscription with plan details.
    pub async fn get_subscription_with_plan<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Option<SubscriptionWithPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                s.id, s.organization_id, s.status, s.billing_cycle,
                s.current_period_start, s.current_period_end, s.is_trial, s.cancel_at_period_end,
                p.id as plan_id, p.name as plan_name, p.display_name as plan_display_name,
                p.monthly_price, p.annual_price
            FROM organization_subscriptions s
            JOIN subscription_plans p ON p.id = s.plan_id
            WHERE s.organization_id = $1 AND s.status NOT IN ('cancelled', 'expired')
            ORDER BY s.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Update an organization subscription.
    pub async fn update_subscription<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateOrganizationSubscription,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                billing_cycle = COALESCE($2, billing_cycle),
                payment_method_id = COALESCE($3, payment_method_id),
                metadata = COALESCE($4, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.billing_cycle)
        .bind(data.payment_method_id)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Change subscription plan.
    pub async fn change_plan<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: ChangePlanRequest,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                plan_id = $2,
                billing_cycle = COALESCE($3, billing_cycle),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.new_plan_id)
        .bind(&data.billing_cycle)
        .fetch_one(executor)
        .await
    }

    /// Cancel a subscription.
    pub async fn cancel_subscription<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: CancelSubscriptionRequest,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let cancel_at_period_end = data.cancel_at_period_end.unwrap_or(true);
        let status = if cancel_at_period_end {
            "active"
        } else {
            "cancelled"
        };

        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                status = $2,
                cancel_at_period_end = $3,
                cancelled_at = NOW(),
                cancellation_reason = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(cancel_at_period_end)
        .bind(&data.cancellation_reason)
        .fetch_one(executor)
        .await
    }

    /// Reactivate a cancelled subscription.
    pub async fn reactivate_subscription<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<OrganizationSubscription, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE organization_subscriptions SET
                status = 'active',
                cancel_at_period_end = false,
                cancelled_at = NULL,
                cancellation_reason = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await
    }

    /// List all subscriptions (platform admin).
    ///
    /// Note: under FORCE RLS the result is scoped to the caller's RLS
    /// context — the rows visible are those of the org set on the
    /// connection.
    pub async fn list_all_subscriptions<'e, E>(
        &self,
        executor: E,
        status: Option<&str>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<SubscriptionWithPlan>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                s.id, s.organization_id, s.status, s.billing_cycle,
                s.current_period_start, s.current_period_end, s.is_trial, s.cancel_at_period_end,
                p.id as plan_id, p.name as plan_name, p.display_name as plan_display_name,
                p.monthly_price, p.annual_price
            FROM organization_subscriptions s
            JOIN subscription_plans p ON p.id = s.plan_id
            WHERE ($1::text IS NULL OR s.status = $1)
            ORDER BY s.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }
}

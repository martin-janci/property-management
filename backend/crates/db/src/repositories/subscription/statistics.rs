//! Aggregate subscription statistics (Epic 26).

use super::SubscriptionRepository;
use crate::models::{PlanSubscriptionCount, SubscriptionStatistics};
use rust_decimal::Decimal;
use sqlx::PgConnection;

impl SubscriptionRepository {
    // ==================== Statistics ====================

    /// Get subscription statistics.
    ///
    /// Multi-statement (aggregate stats, then per-plan counts), so it takes
    /// `&mut PgConnection` and runs both queries on the same context-set
    /// connection. Under FORCE RLS the figures are scoped to the caller's
    /// RLS context.
    pub async fn get_statistics(
        &self,
        conn: &mut PgConnection,
    ) -> Result<SubscriptionStatistics, sqlx::Error> {
        // Combined query to get all stats in one round-trip
        let stats: (i64, i64, i64, i64, Option<Decimal>) = sqlx::query_as(
            r#"
            SELECT
                (SELECT COUNT(*) FROM organization_subscriptions) as total,
                (SELECT COUNT(*) FROM organization_subscriptions WHERE status = 'active') as active,
                (SELECT COUNT(*) FROM organization_subscriptions WHERE status = 'trialing') as trial,
                (SELECT COUNT(*) FROM organization_subscriptions WHERE status = 'cancelled') as cancelled,
                (SELECT SUM(
                    CASE WHEN s.billing_cycle = 'annual'
                        THEN p.annual_price / 12
                        ELSE p.monthly_price
                    END
                )
                FROM organization_subscriptions s
                JOIN subscription_plans p ON p.id = s.plan_id
                WHERE s.status = 'active') as mrr
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;

        let monthly_recurring_revenue = stats.4.unwrap_or(Decimal::ZERO);
        let annual_recurring_revenue = monthly_recurring_revenue * Decimal::from(12);

        // Get counts by plan - this requires a separate query
        let by_plan: Vec<PlanSubscriptionCount> = sqlx::query_as(
            r#"
            SELECT p.id as plan_id, p.name as plan_name, COUNT(s.id) as count
            FROM subscription_plans p
            LEFT JOIN organization_subscriptions s ON s.plan_id = p.id AND s.status = 'active'
            GROUP BY p.id, p.name
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        Ok(SubscriptionStatistics {
            total_subscriptions: stats.0,
            active_subscriptions: stats.1,
            trial_subscriptions: stats.2,
            cancelled_subscriptions: stats.3,
            monthly_recurring_revenue,
            annual_recurring_revenue,
            by_plan,
        })
    }
}

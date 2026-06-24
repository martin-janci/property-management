//! Coupon CRUD & redemption (Epic 26).

use super::SubscriptionRepository;
use crate::models::{
    CouponRedemption, CreateSubscriptionCoupon, SubscriptionCoupon, UpdateSubscriptionCoupon,
};
use sqlx::{Connection, Executor, PgConnection, Postgres};
use uuid::Uuid;

impl SubscriptionRepository {
    // ==================== Coupons ====================

    /// Create a coupon.
    pub async fn create_coupon<'e, E>(
        &self,
        executor: E,
        data: CreateSubscriptionCoupon,
    ) -> Result<SubscriptionCoupon, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO subscription_coupons
                (code, name, description, discount_type, discount_value, currency, duration,
                 duration_months, max_redemptions, valid_from, valid_until, applicable_plans,
                 min_amount, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(&data.code)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.discount_type)
        .bind(data.discount_value)
        .bind(&data.currency)
        .bind(data.duration.unwrap_or_else(|| "once".to_string()))
        .bind(data.duration_months)
        .bind(data.max_redemptions)
        .bind(data.valid_from)
        .bind(data.valid_until)
        .bind(&data.applicable_plans)
        .bind(data.min_amount)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find a coupon by code.
    pub async fn find_coupon_by_code<'e, E>(
        &self,
        executor: E,
        code: &str,
    ) -> Result<Option<SubscriptionCoupon>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM subscription_coupons WHERE code = $1 AND is_active = true")
            .bind(code)
            .fetch_optional(executor)
            .await
    }

    /// List all coupons.
    pub async fn list_coupons<'e, E>(
        &self,
        executor: E,
        active_only: bool,
    ) -> Result<Vec<SubscriptionCoupon>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if active_only {
            sqlx::query_as(
                "SELECT * FROM subscription_coupons WHERE is_active = true ORDER BY created_at DESC",
            )
            .fetch_all(executor)
            .await
        } else {
            sqlx::query_as("SELECT * FROM subscription_coupons ORDER BY created_at DESC")
                .fetch_all(executor)
                .await
        }
    }

    /// Update a coupon.
    pub async fn update_coupon<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateSubscriptionCoupon,
    ) -> Result<SubscriptionCoupon, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE subscription_coupons SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                max_redemptions = COALESCE($4, max_redemptions),
                valid_from = COALESCE($5, valid_from),
                valid_until = COALESCE($6, valid_until),
                applicable_plans = COALESCE($7, applicable_plans),
                min_amount = COALESCE($8, min_amount),
                is_active = COALESCE($9, is_active),
                metadata = COALESCE($10, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.max_redemptions)
        .bind(data.valid_from)
        .bind(data.valid_until)
        .bind(&data.applicable_plans)
        .bind(data.min_amount)
        .bind(data.is_active)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Redeem a coupon.
    ///
    /// Uses a transaction (on the caller's context-set connection) with
    /// validation to prevent race conditions and over-redemption. Checks
    /// `max_redemptions` before incrementing the count and inserting the
    /// redemption record. The `coupon_redemptions` insert is FORCE-RLS-bound,
    /// so this MUST run on a connection with RLS context set for `org_id`.
    pub async fn redeem_coupon(
        &self,
        conn: &mut PgConnection,
        coupon_id: Uuid,
        org_id: Uuid,
        subscription_id: Option<Uuid>,
        user_id: Uuid,
    ) -> Result<CouponRedemption, sqlx::Error> {
        let mut tx = conn.begin().await?;

        // Check if coupon exists and has remaining redemptions (with row lock)
        let coupon: Option<(i32, Option<i32>)> = sqlx::query_as(
            "SELECT COALESCE(redemption_count, 0), max_redemptions FROM subscription_coupons WHERE id = $1 FOR UPDATE",
        )
        .bind(coupon_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((current_count, max_redemptions)) = coupon {
            if let Some(max) = max_redemptions {
                if current_count >= max {
                    tx.rollback().await?;
                    return Err(sqlx::Error::RowNotFound); // Coupon exhausted
                }
            }
        } else {
            tx.rollback().await?;
            return Err(sqlx::Error::RowNotFound); // Coupon not found
        }

        // Insert redemption record first (this validates FK constraints)
        let redemption: CouponRedemption = sqlx::query_as(
            r#"
            INSERT INTO coupon_redemptions
                (coupon_id, organization_id, subscription_id, redeemed_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(coupon_id)
        .bind(org_id)
        .bind(subscription_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        // Only increment count after successful redemption insert
        sqlx::query(
            "UPDATE subscription_coupons SET redemption_count = COALESCE(redemption_count, 0) + 1 WHERE id = $1",
        )
        .bind(coupon_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(redemption)
    }
}

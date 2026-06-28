//! Payment method CRUD & default selection (Epic 26).

use super::SubscriptionRepository;
use crate::models::{CreateSubscriptionPaymentMethod, SubscriptionPaymentMethod};
use sqlx::{Connection, Executor, PgConnection, Postgres};
use uuid::Uuid;

impl SubscriptionRepository {
    // ==================== Payment Methods CRUD ====================

    /// Create a payment method.
    pub async fn create_payment_method<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: CreateSubscriptionPaymentMethod,
    ) -> Result<SubscriptionPaymentMethod, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO payment_methods
                (organization_id, method_type, stripe_payment_method_id, is_default,
                 billing_address, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&data.method_type)
        .bind(&data.stripe_payment_method_id)
        .bind(data.is_default.unwrap_or(false))
        .bind(&data.billing_address)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// List payment methods for an organization.
    pub async fn list_payment_methods<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<SubscriptionPaymentMethod>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM payment_methods WHERE organization_id = $1 ORDER BY is_default DESC, created_at DESC",
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Set default payment method.
    ///
    /// Uses a transaction (on the caller's context-set connection) to ensure
    /// atomicity — prevents race conditions where concurrent requests could
    /// result in multiple default payment methods. RLS context is set via
    /// session-level GUCs, so it survives `BEGIN`/`COMMIT`.
    pub async fn set_default_payment_method(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        payment_method_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        let mut tx = conn.begin().await?;

        // Reset all to non-default
        sqlx::query("UPDATE payment_methods SET is_default = false WHERE organization_id = $1")
            .bind(org_id)
            .execute(&mut *tx)
            .await?;

        // Set the specified one as default
        sqlx::query(
            "UPDATE payment_methods SET is_default = true WHERE id = $1 AND organization_id = $2",
        )
        .bind(payment_method_id)
        .bind(org_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Delete a payment method.
    pub async fn delete_payment_method<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM payment_methods WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected() > 0)
    }
}

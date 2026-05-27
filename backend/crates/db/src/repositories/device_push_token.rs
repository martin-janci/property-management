// Epic 8A-3: Mobile OS push notification registration
// Repository for device_push_tokens — FCM (Android) and APNs (iOS) token management.
//
// Uses runtime `sqlx::query_as` / `sqlx::query` (not the compile-time macros) so
// the crate compiles without a live DATABASE_URL, consistent with other repositories.

use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

use crate::models::DevicePushToken;

#[derive(Clone)]
pub struct DevicePushTokenRepository {
    pool: PgPool,
}

impl DevicePushTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // -- RLS-aware methods (called within a session that has set app.current_user_id) --

    /// Upsert a device push token for the authenticated user.
    ///
    /// Uses `ON CONFLICT (token) DO UPDATE` so the same token sent on every app open
    /// simply refreshes `last_seen_at` without creating duplicates.  Device transfers
    /// (same physical token, new owner) are handled by updating `user_id` and
    /// `last_seen_at` atomically.
    pub async fn upsert_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        token: &str,
        platform: &str,
        app_id: Option<&str>,
        device_name: Option<&str>,
    ) -> Result<DevicePushToken, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DevicePushToken>(
            r#"
            INSERT INTO device_push_tokens (user_id, token, platform, app_id, device_name, last_seen_at)
            VALUES ($1, $2, $3, $4, $5, now())
            ON CONFLICT (token) DO UPDATE
                SET user_id      = EXCLUDED.user_id,
                    platform     = EXCLUDED.platform,
                    app_id       = EXCLUDED.app_id,
                    device_name  = EXCLUDED.device_name,
                    last_seen_at = now()
            RETURNING id, user_id, token, platform, app_id, device_name, last_seen_at, created_at
            "#,
        )
        .bind(user_id)
        .bind(token)
        .bind(platform)
        .bind(app_id)
        .bind(device_name)
        .fetch_one(executor)
        .await
    }

    /// Delete a specific device push token that belongs to the authenticated user.
    ///
    /// Returns `true` if a row was deleted, `false` if the token was not found
    /// (idempotent -- callers can treat both as success).
    pub async fn delete_token_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        token: &str,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM device_push_tokens WHERE user_id = $1 AND token = $2")
                .bind(user_id)
                .bind(token)
                .execute(executor)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    // -- Service-role methods (no RLS -- used for fan-out notification delivery) --

    /// Fetch all registered push tokens for a user.  Called by the notification
    /// delivery pipeline to fan-out to every device owned by the user.
    ///
    /// # Caller contract
    /// Uses the raw pool — must be called from a service-role context where
    /// `app.current_user_id` is **not** set.  The `device_push_tokens_service_policy`
    /// RLS policy allows SELECT for such connections.  Do not call from a
    /// request handler; use the `_rls` variants there.
    pub async fn get_tokens_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<DevicePushToken>, sqlx::Error> {
        sqlx::query_as::<_, DevicePushToken>(
            r#"
            SELECT id, user_id, token, platform, app_id, device_name, last_seen_at, created_at
            FROM device_push_tokens
            WHERE user_id = $1
            ORDER BY last_seen_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Remove all push tokens for a user (e.g. on account deletion or logout-all).
    ///
    /// # Caller contract
    /// Same as `get_tokens_for_user` — service-role pool only; `app.current_user_id`
    /// must not be set on the connection.
    pub async fn delete_all_for_user(&self, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM device_push_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete a single push token by its raw token string (service-role, no RLS).
    ///
    /// Used by the push fanout worker to evict tokens that FCM/APNs has reported
    /// as `NOT_REGISTERED` / stale.  Only deletes the token if it belongs to the
    /// given `user_id`, providing an extra guard against cross-user eviction bugs.
    ///
    /// # Caller contract
    /// Service-role pool only — `app.current_user_id` must not be set on the
    /// connection.  Do not call from a request handler; use `delete_token_rls` there.
    pub async fn delete_stale_token(
        &self,
        user_id: Uuid,
        token: &str,
    ) -> Result<bool, sqlx::Error> {
        // sqlx::query (runtime) rather than sqlx::query! (compile-time) is intentional:
        // this DELETE returns no rows, so there is no return type for the macro to verify,
        // and keeping it consistent with the other service-role methods in this file lets
        // the crate compile without a live DATABASE_URL (SQLX_OFFLINE = true).
        let result =
            sqlx::query("DELETE FROM device_push_tokens WHERE user_id = $1 AND token = $2")
                .bind(user_id)
                .bind(token)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }
}

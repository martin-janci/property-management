//! Portal password reset token repository (Reality Portal UC-44.3).
//!
//! Mirrors `password_reset.rs`. The two are intentionally kept separate
//! because portal users (`portal_users` table on the reality-server)
//! are a distinct identity from PM users (`users` table on the
//! api-server) and the foreign keys must point at different tables.

use crate::models::portal_password_reset::{
    CreatePortalPasswordResetToken, PortalPasswordResetToken,
};
use crate::DbPool;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for portal password reset token operations.
#[derive(Clone)]
pub struct PortalPasswordResetRepository {
    pool: DbPool,
}

impl PortalPasswordResetRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert a new reset token row.
    pub async fn create(
        &self,
        data: CreatePortalPasswordResetToken,
    ) -> Result<PortalPasswordResetToken, SqlxError> {
        let token = sqlx::query_as::<_, PortalPasswordResetToken>(
            r#"
            INSERT INTO portal_password_reset_tokens (portal_user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(data.portal_user_id)
        .bind(&data.token_hash)
        .bind(data.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Look up an unused token by its SHA-256 hash.
    pub async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PortalPasswordResetToken>, SqlxError> {
        let token = sqlx::query_as::<_, PortalPasswordResetToken>(
            r#"
            SELECT * FROM portal_password_reset_tokens
            WHERE token_hash = $1 AND used_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Mark a single token as used (post password update).
    pub async fn mark_used(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE portal_password_reset_tokens SET used_at = NOW()
            WHERE id = $1 AND used_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Invalidate every outstanding token for a user (called when a new
    /// reset is requested or the password is changed by other means).
    pub async fn invalidate_user_tokens(&self, portal_user_id: Uuid) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE portal_password_reset_tokens SET used_at = NOW()
            WHERE portal_user_id = $1 AND used_at IS NULL
            "#,
        )
        .bind(portal_user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Cleanup job: delete tokens that are expired or were consumed more
    /// than a day ago. Compares `used_at` (not `created_at`) so a token
    /// that was minted long before being used still gets the full 24-hour
    /// post-use retention window.
    pub async fn cleanup_expired(&self) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM portal_password_reset_tokens
            WHERE expires_at < NOW()
               OR used_at < NOW() - INTERVAL '1 day'
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

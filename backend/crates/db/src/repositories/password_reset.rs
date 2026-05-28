//! Password reset token repository (Epic 1, Story 1.4).

use crate::models::password_reset::{CreatePasswordResetToken, PasswordResetToken};
use crate::DbPool;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for password reset token operations.
#[derive(Clone)]
pub struct PasswordResetRepository {
    pool: DbPool,
}

impl PasswordResetRepository {
    /// Create a new PasswordResetRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new password reset token.
    pub async fn create(
        &self,
        data: CreatePasswordResetToken,
    ) -> Result<PasswordResetToken, SqlxError> {
        let token = sqlx::query_as::<_, PasswordResetToken>(
            r#"
            INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(&data.token_hash)
        .bind(data.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find password reset token by hash.
    pub async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<PasswordResetToken>, SqlxError> {
        let token = sqlx::query_as::<_, PasswordResetToken>(
            r#"
            SELECT * FROM password_reset_tokens
            WHERE token_hash = $1 AND used_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Mark token as used.
    pub async fn mark_used(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE password_reset_tokens SET used_at = NOW()
            WHERE id = $1 AND used_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Invalidate all unused tokens for a user (when new token is created or password is changed).
    pub async fn invalidate_user_tokens(&self, user_id: Uuid) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE password_reset_tokens SET used_at = NOW()
            WHERE user_id = $1 AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Cleanup expired and used tokens.
    pub async fn cleanup_expired(&self) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM password_reset_tokens
            WHERE expires_at < NOW() OR (used_at IS NOT NULL AND created_at < NOW() - INTERVAL '1 day')
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Count password-reset tokens issued for a user in the last `window`
    /// minutes. Used by `forgot_password` to rate-limit reset-email
    /// floods to a victim address (P1-07). Counts ALL tokens — used,
    /// unused, expired-but-not-yet-cleaned — because each one represents
    /// an email that was sent. Returns 0 if the user_id is unknown
    /// (caller has already short-circuited the unknown-email case).
    pub async fn count_recent_for_user(
        &self,
        user_id: Uuid,
        window_minutes: i32,
    ) -> Result<i64, SqlxError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM password_reset_tokens
            WHERE user_id = $1
              AND created_at > NOW() - make_interval(mins => $2)
            "#,
        )
        .bind(user_id)
        .bind(window_minutes)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }
}

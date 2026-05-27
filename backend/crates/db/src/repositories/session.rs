//! Session repository (Epic 1, Story 1.2).

use crate::models::refresh_token::{CreateRefreshToken, RateLimitStatus, RefreshToken};
use crate::DbPool;
use chrono::{DateTime, Duration, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for session operations (refresh tokens and login attempts).
#[derive(Clone)]
pub struct SessionRepository {
    pool: DbPool,
}

impl SessionRepository {
    /// Create a new SessionRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ==================== Refresh Tokens ====================

    /// Create a new refresh token.
    pub async fn create_refresh_token(
        &self,
        data: CreateRefreshToken,
    ) -> Result<RefreshToken, SqlxError> {
        let token = sqlx::query_as::<_, RefreshToken>(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at, user_agent, ip_address, device_info)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(&data.token_hash)
        .bind(data.expires_at)
        .bind(&data.user_agent)
        .bind(data.ip_address)
        .bind(&data.device_info)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find refresh token by hash.
    pub async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, SqlxError> {
        let token = sqlx::query_as::<_, RefreshToken>(
            r#"
            SELECT * FROM refresh_tokens
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find refresh token by hash *regardless of revocation status*.
    /// Used by the refresh handler to detect replay of an already-revoked
    /// token (P1-01 / RFC 9700 family invalidation). `find_by_token_hash`
    /// hides revoked rows, which prevented the caller from distinguishing
    /// "never existed" from "was revoked"; the latter case should trigger
    /// a security response, not just a 401.
    pub async fn find_by_token_hash_any_status(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, SqlxError> {
        let token = sqlx::query_as::<_, RefreshToken>(
            r#"
            SELECT * FROM refresh_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find all active sessions for a user.
    pub async fn find_user_sessions(&self, user_id: Uuid) -> Result<Vec<RefreshToken>, SqlxError> {
        let tokens = sqlx::query_as::<_, RefreshToken>(
            r#"
            SELECT * FROM refresh_tokens
            WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
            ORDER BY last_used_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tokens)
    }

    /// Update last_used_at for a token.
    pub async fn touch_token(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE refresh_tokens SET last_used_at = NOW()
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke a specific refresh token.
    pub async fn revoke_token(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE refresh_tokens SET revoked_at = NOW()
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke all refresh tokens for a user (except optionally one).
    pub async fn revoke_all_user_tokens(
        &self,
        user_id: Uuid,
        except_id: Option<Uuid>,
    ) -> Result<u64, SqlxError> {
        let result = if let Some(except) = except_id {
            sqlx::query(
                r#"
                UPDATE refresh_tokens SET revoked_at = NOW()
                WHERE user_id = $1 AND id != $2 AND revoked_at IS NULL
                "#,
            )
            .bind(user_id)
            .bind(except)
            .execute(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                UPDATE refresh_tokens SET revoked_at = NOW()
                WHERE user_id = $1 AND revoked_at IS NULL
                "#,
            )
            .bind(user_id)
            .execute(&self.pool)
            .await?
        };

        Ok(result.rows_affected())
    }

    /// Cleanup expired and old revoked tokens.
    pub async fn cleanup_expired_tokens(&self) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM refresh_tokens
            WHERE expires_at < NOW() OR revoked_at < NOW() - INTERVAL '7 days'
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // ==================== Login Attempts (Rate Limiting) ====================

    /// Record a login attempt.
    pub async fn record_login_attempt(
        &self,
        email: &str,
        ip_address: &str,
        success: bool,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            INSERT INTO login_attempts (email, ip_address, success)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(email)
        .bind(ip_address)
        .bind(success)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check rate limit status for an email.
    pub async fn check_rate_limit(&self, email: &str) -> Result<RateLimitStatus, SqlxError> {
        let window_start = Utc::now() - Duration::minutes(RateLimitStatus::LOCKOUT_MINUTES);

        // Count failed attempts in the lockout window
        let failed_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM login_attempts
            WHERE LOWER(email) = LOWER($1) AND success = false AND attempt_at > $2
            "#,
        )
        .bind(email)
        .bind(window_start)
        .fetch_one(&self.pool)
        .await?;

        let is_locked = failed_count >= RateLimitStatus::MAX_FAILED_ATTEMPTS;

        // If locked, find when the lockout expires
        let lockout_remaining = if is_locked {
            let oldest_in_window = sqlx::query_scalar::<_, DateTime<Utc>>(
                r#"
                SELECT MIN(attempt_at) FROM login_attempts
                WHERE LOWER(email) = LOWER($1) AND success = false AND attempt_at > $2
                "#,
            )
            .bind(email)
            .bind(window_start)
            .fetch_optional(&self.pool)
            .await?;

            oldest_in_window.map(|oldest| {
                let unlock_at = oldest + Duration::minutes(RateLimitStatus::LOCKOUT_MINUTES);
                (unlock_at - Utc::now()).num_seconds().max(0)
            })
        } else {
            None
        };

        Ok(RateLimitStatus {
            failed_attempts: failed_count,
            is_locked,
            lockout_remaining_secs: lockout_remaining,
        })
    }

    // Note: Clear successful login resets the failed attempt counter.
    // This is implicit - successful logins are recorded but don't clear old failures.
    // The window-based approach handles this automatically.

    /// Cleanup old login attempts.
    pub async fn cleanup_old_attempts(&self) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM login_attempts WHERE attempt_at < NOW() - INTERVAL '1 hour'
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

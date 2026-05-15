//! MFA-recency check.
//!
//! Phase 5 mandates: every capability-gated admin call requires the user to
//! have MFA-verified within the last 15 minutes. The window is short on
//! purpose — leak #21 ("super-admin = single catastrophic point") is
//! survivable iff a stolen JWT cannot be re-used long after the last
//! interactive verify.
//!
//! Reads from `two_factor_auth_verifications` (migration 00138).

use async_trait::async_trait;
use chrono::Duration;
use sqlx::Error as SqlxError;
use uuid::Uuid;

use crate::AdminError;

/// How recent an MFA verification must be. 15 minutes.
pub const RECENT_MFA_WINDOW: Duration = Duration::minutes(15);

/// Trait abstraction over the verification store. Tests inject a fake.
#[async_trait]
pub trait MfaRecency: Send + Sync {
    /// Has `user_id` MFA-verified within `RECENT_MFA_WINDOW`?
    async fn recent_mfa(&self, user_id: Uuid) -> Result<bool, AdminError>;
}

/// Postgres-backed implementation.
#[derive(Clone)]
pub struct PgMfaRecency {
    pool: db::DbPool,
}

impl PgMfaRecency {
    pub fn new(pool: db::DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MfaRecency for PgMfaRecency {
    async fn recent_mfa(&self, user_id: Uuid) -> Result<bool, AdminError> {
        // The `INTERVAL` literal mirrors `RECENT_MFA_WINDOW`. Keep them
        // in sync if the constant changes.
        let row: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT TRUE
            FROM two_factor_auth_verifications
            WHERE user_id = $1
              AND verified_at > NOW() - INTERVAL '15 minutes'
            ORDER BY verified_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e: SqlxError| AdminError::Internal(format!("mfa_recency: {}", e)))?;
        Ok(row.is_some())
    }
}

/// Test fixture: always returns `false` (i.e. no recent MFA). Tests that
/// want to exercise the success path can use [`AlwaysFreshMfa`].
#[derive(Default, Clone)]
pub struct NoopMfaRecency;

#[async_trait]
impl MfaRecency for NoopMfaRecency {
    async fn recent_mfa(&self, _user_id: Uuid) -> Result<bool, AdminError> {
        Ok(false)
    }
}

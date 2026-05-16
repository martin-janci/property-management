//! User invite repository (Phase 2 — Identity Unification).
//!
//! Single-use, expiring, email-bound invites — the only sanctioned path to
//! create a `user_memberships` row (defends leak #9). Tokens are stored as
//! SHA-256 hex hashes; plaintext is shown to the inviter ONCE at creation.

use crate::models::membership::UserInvite;
use crate::DbPool;
use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Result of `consume_by_token`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumeInviteOutcome {
    /// Invite consumed; returns the (now-accepted) invite row.
    Accepted(UserInvite),
    /// Token did not match any invite.
    UnknownToken,
    /// Invite has already been accepted (single-use guard).
    AlreadyAccepted,
    /// Invite expired before acceptance.
    Expired,
    /// The accepting user's email does not match the invite's email
    /// (case-insensitive). Email-bound guard.
    EmailMismatch,
}

#[derive(Clone)]
pub struct UserInviteRepository {
    pool: DbPool,
}

impl UserInviteRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Hash a plaintext token to its storage form (SHA-256 hex). Pure, no I/O.
    pub fn hash_token(token: &str) -> String {
        let mut h = Sha256::new();
        h.update(token.as_bytes());
        hex::encode(h.finalize())
    }

    /// Insert a new invite. Caller supplies the plaintext token (so they can
    /// surface it once to the inviter). Only the hash is stored.
    pub async fn create(
        &self,
        email: &str,
        organization_id: Uuid,
        role: &str,
        invited_by: Option<Uuid>,
        plaintext_token: &str,
        ttl: Duration,
    ) -> Result<UserInvite, SqlxError> {
        let token_hash = Self::hash_token(plaintext_token);
        let expires_at = Utc::now() + ttl;
        sqlx::query_as::<_, UserInvite>(
            r#"
            INSERT INTO user_invites (
                email, organization_id, role, invited_by, token_hash, expires_at
            )
            VALUES (LOWER($1), $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(organization_id)
        .bind(role)
        .bind(invited_by)
        .bind(&token_hash)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
    }

    /// Atomically consume an invite token. The accepting user's id and email
    /// are required so we can enforce the email-bound and single-use guards.
    pub async fn consume_by_token(
        &self,
        plaintext_token: &str,
        accepting_user_id: Uuid,
        accepting_email: &str,
    ) -> Result<ConsumeInviteOutcome, SqlxError> {
        let token_hash = Self::hash_token(plaintext_token);
        let mut tx = self.pool.begin().await?;

        // SELECT ... FOR UPDATE to serialize concurrent acceptance attempts.
        let invite: Option<UserInvite> = sqlx::query_as::<_, UserInvite>(
            r#"
            SELECT * FROM user_invites WHERE token_hash = $1 FOR UPDATE
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(invite) = invite else {
            // No row — release the implicit transaction and report.
            tx.rollback().await.ok();
            return Ok(ConsumeInviteOutcome::UnknownToken);
        };

        if invite.is_accepted() {
            tx.rollback().await.ok();
            return Ok(ConsumeInviteOutcome::AlreadyAccepted);
        }
        if invite.is_expired() {
            tx.rollback().await.ok();
            return Ok(ConsumeInviteOutcome::Expired);
        }
        if invite.email.to_lowercase() != accepting_email.to_lowercase() {
            tx.rollback().await.ok();
            return Ok(ConsumeInviteOutcome::EmailMismatch);
        }

        // Mark accepted in the same transaction. The PK lock above guarantees
        // only one acceptor wins.
        let updated = sqlx::query_as::<_, UserInvite>(
            r#"
            UPDATE user_invites
               SET accepted_at = NOW(),
                   accepted_by = $1
             WHERE id = $2
               AND accepted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(accepting_user_id)
        .bind(invite.id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(accepted) = updated else {
            // Race lost — somebody else accepted between our SELECT and UPDATE.
            tx.rollback().await.ok();
            return Ok(ConsumeInviteOutcome::AlreadyAccepted);
        };

        tx.commit().await?;
        Ok(ConsumeInviteOutcome::Accepted(accepted))
    }

    /// Mark all not-yet-accepted invites whose `expires_at` is in the past as
    /// rejected (for the test/cleanup path). Returns the count touched.
    ///
    /// We model expiry via the timestamp itself (no `status` column) — this
    /// helper exists to bulk-reject in tests/cron without changing schema.
    pub async fn expire_pending(&self) -> Result<u64, SqlxError> {
        let r = sqlx::query(
            r#"
            UPDATE user_invites
               SET expires_at = NOW() - INTERVAL '1 second'
             WHERE accepted_at IS NULL
               AND expires_at <= NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(r.rows_affected())
    }

    /// Look up an invite by id (admin / test introspection).
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<UserInvite>, SqlxError> {
        sqlx::query_as::<_, UserInvite>(r#"SELECT * FROM user_invites WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Look up an invite row by its hashed token (for tests).
    pub async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<UserInvite>, SqlxError> {
        sqlx::query_as::<_, UserInvite>(r#"SELECT * FROM user_invites WHERE token_hash = $1"#)
            .bind(token_hash)
            .fetch_optional(&self.pool)
            .await
    }

    /// Convenience: build an `expires_at` `TTL` from the current clock.
    pub fn default_ttl() -> Duration {
        Duration::days(7)
    }

    /// Convenience used by a couple of tests.
    pub fn now() -> DateTime<Utc> {
        Utc::now()
    }
}

//! Short-lived impersonation tokens.
//!
//! When an operator with `Capability::UsersImpersonate` "impersonates" a
//! tenant user, we issue a 15-minute opaque token. The token's mere presence
//! (carried as a cookie / header) tells the frontend to render a sticky
//! banner — leak #21 calls out impersonation as one of the things that must
//! be impossible to *hide*.
//!
//! Storage: `impersonation_tokens` table (migration 00138). We persist a
//! HASH of the token, not the token itself, mirroring password-reset patterns.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use rand::TryRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Error as SqlxError;
use std::sync::Arc;
use uuid::Uuid;

use crate::{AdminError, AuditOutcome, AuditWriter, Capability};

/// 15-minute TTL — same window as `RECENT_MFA_WINDOW`.
pub const IMPERSONATION_TTL: Duration = Duration::minutes(15);

/// Persisted impersonation grant.
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ImpersonationToken {
    pub id: Uuid,
    pub token_hash: String,
    pub actor_id: Uuid,
    pub target_user_id: Uuid,
    pub justifying_capability: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// What `start_impersonation` returns: the persisted record + the *plain*
/// token to hand to the client. The plain token is only ever in memory.
#[derive(Debug, Clone)]
pub struct IssuedImpersonationToken {
    pub record: ImpersonationToken,
    pub plain_token: String,
}

/// Trait so tests can plug a fake.
#[async_trait]
pub trait ImpersonationService: Send + Sync {
    async fn start_impersonation(
        &self,
        actor: Uuid,
        target: Uuid,
        capability: Capability,
    ) -> Result<IssuedImpersonationToken, AdminError>;

    async fn end_impersonation(&self, token_id: Uuid, actor: Uuid) -> Result<(), AdminError>;

    /// Resolve a presented token to its record. Returns `None` if expired,
    /// ended, or unknown.
    async fn resolve(&self, plain_token: &str) -> Result<Option<ImpersonationToken>, AdminError>;
}

/// Postgres-backed implementation.
#[derive(Clone)]
pub struct PgImpersonationService {
    pool: db::DbPool,
    audit: Arc<dyn AuditWriter>,
}

impl PgImpersonationService {
    pub fn new(pool: db::DbPool, audit: Arc<dyn AuditWriter>) -> Self {
        Self { pool, audit }
    }
}

fn map_sqlx(err: SqlxError) -> AdminError {
    AdminError::Internal(format!("impersonation: {}", err))
}

/// Generate a cryptographically random 256-bit token, encoded hex.
fn generate_token() -> String {
    let mut buf = [0u8; 32];
    // rand 0.10: SysRng is the OS entropy source; `try_fill` returns a Result.
    rand::rngs::SysRng
        .try_fill_bytes(&mut buf[..])
        .expect("OS RNG is available");
    hex::encode(buf)
}

/// Hash a token for storage.
fn hash_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

#[async_trait]
impl ImpersonationService for PgImpersonationService {
    async fn start_impersonation(
        &self,
        actor: Uuid,
        target: Uuid,
        capability: Capability,
    ) -> Result<IssuedImpersonationToken, AdminError> {
        if actor == target {
            return Err(AdminError::Internal(
                "no_self_impersonation: actor must differ from target".into(),
            ));
        }

        let plain = generate_token();
        let hash = hash_token(&plain);
        let expires_at = Utc::now() + IMPERSONATION_TTL;

        let record = sqlx::query_as::<_, ImpersonationToken>(
            r#"
            INSERT INTO impersonation_tokens
                (token_hash, actor_id, target_user_id, justifying_capability, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, token_hash, actor_id, target_user_id, justifying_capability,
                      issued_at, expires_at, ended_at
            "#,
        )
        .bind(&hash)
        .bind(actor)
        .bind(target)
        .bind(capability.as_str())
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;

        // Audit: link impersonator → target. Target is the target_id; the
        // capability that justified it is in `details.capability`.
        let _ = self
            .audit
            .record(
                Some(actor),
                Capability::UsersImpersonate,
                AuditOutcome::Allowed,
                Some("user"),
                Some(target),
                Some(&serde_json::json!({
                    "impersonation_token_id": record.id,
                    "expires_at": record.expires_at,
                    "justifying_capability": capability.as_str(),
                })),
                None,
                None,
            )
            .await;

        Ok(IssuedImpersonationToken {
            record,
            plain_token: plain,
        })
    }

    async fn end_impersonation(&self, token_id: Uuid, actor: Uuid) -> Result<(), AdminError> {
        // Scope the update to the session owner: a caller may only end an
        // impersonation session they themselves started. Without the
        // `actor_id = $2` predicate any admin could terminate another admin's
        // active session by guessing the token id.
        let row = sqlx::query_as::<_, (Uuid,)>(
            r#"
            UPDATE impersonation_tokens
            SET ended_at = NOW()
            WHERE id = $1 AND actor_id = $2 AND ended_at IS NULL
            RETURNING target_user_id
            "#,
        )
        .bind(token_id)
        .bind(actor)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        if let Some((target,)) = row {
            let _ = self
                .audit
                .record(
                    Some(actor),
                    Capability::UsersImpersonate,
                    AuditOutcome::Allowed,
                    Some("user"),
                    Some(target),
                    Some(&serde_json::json!({
                        "impersonation_token_id": token_id,
                        "action": "end",
                    })),
                    None,
                    None,
                )
                .await;
        }

        Ok(())
    }

    async fn resolve(&self, plain_token: &str) -> Result<Option<ImpersonationToken>, AdminError> {
        let hash = hash_token(plain_token);
        let row = sqlx::query_as::<_, ImpersonationToken>(
            r#"
            SELECT id, token_hash, actor_id, target_user_id, justifying_capability,
                   issued_at, expires_at, ended_at
            FROM impersonation_tokens
            WHERE token_hash = $1
              AND ended_at IS NULL
              AND expires_at > NOW()
            "#,
        )
        .bind(&hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_64_hex() {
        let t = generate_token();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn token_hash_is_stable() {
        let h1 = hash_token("abc");
        let h2 = hash_token("abc");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}

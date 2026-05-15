//! Audit writer.
//!
//! Every `RequireCapability` extraction — successful OR denied — writes an
//! audit row. Captured fields: actor, action (capability), target, payload
//! HASH (SHA-256 hex; raw payload never persisted), ip, user agent.
//!
//! We persist to the existing `audit_logs` table (migration 00025). The
//! Phase 5 enforcement pass adds a trigger (migration 00138) that REJECTs
//! UPDATE/DELETE on `audit_logs`, so rows here are *truly* append-only.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Error as SqlxError;
use uuid::Uuid;

use crate::{AdminError, Capability};

/// Outcome of an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    /// The action was allowed and executed.
    Allowed,
    /// The action was denied (capability missing, MFA stale, etc.). We
    /// audit denials too — leak #21 includes "noisy abuse from a stolen
    /// credential" as a thing operators must be able to *see*.
    Denied,
}

impl AuditOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditOutcome::Allowed => "allowed",
            AuditOutcome::Denied => "denied",
        }
    }
}

/// What we hand back to the caller after writing a row. Useful in tests.
#[derive(Debug, Clone)]
pub struct RecordedAuditEvent {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Trait so tests can plug a counter / collector in place of Postgres.
#[async_trait]
pub trait AuditWriter: Send + Sync {
    /// Record a capability invocation.
    ///
    /// Arguments:
    ///   * `actor_id` — the platform principal making the call.
    ///   * `capability` — what was attempted.
    ///   * `outcome` — allowed/denied.
    ///   * `target_type` / `target_id` — optional resource pointer (e.g.
    ///     `("organization", org_id)`).
    ///   * `payload` — the request body. We HASH this; we do NOT store it.
    ///     This protects against accidental PII exposure in audit reads.
    ///   * `ip` / `user_agent` — request metadata.
    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        actor_id: Option<Uuid>,
        capability: Capability,
        outcome: AuditOutcome,
        target_type: Option<&str>,
        target_id: Option<Uuid>,
        payload: Option<&serde_json::Value>,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<RecordedAuditEvent, AdminError>;
}

/// SHA-256 hash a JSON payload. Returns hex.
pub fn hash_payload(payload: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(payload).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(&bytes);
    hex::encode(h.finalize())
}

/// Postgres-backed implementation that writes to `audit_logs`. We map the
/// Phase 5 surface (capability + outcome) onto the `audit_action` enum's
/// `resource_accessed` slot and stash the rich detail in `details` JSONB.
#[derive(Clone)]
pub struct PgAuditWriter {
    pool: db::DbPool,
}

impl PgAuditWriter {
    pub fn new(pool: db::DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditWriter for PgAuditWriter {
    async fn record(
        &self,
        actor_id: Option<Uuid>,
        capability: Capability,
        outcome: AuditOutcome,
        target_type: Option<&str>,
        target_id: Option<Uuid>,
        payload: Option<&serde_json::Value>,
        ip: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<RecordedAuditEvent, AdminError> {
        let payload_hash = payload.map(hash_payload);
        let details = serde_json::json!({
            "capability": capability.as_str(),
            "outcome": outcome.as_str(),
            "payload_hash": payload_hash,
            "source": "admin_core",
        });

        // We use the existing `audit_action` enum's `resource_accessed` slot
        // because Phase 5 actions are read/write of *resources* through the
        // capability gate. The richer "what capability + outcome" detail
        // lives in the JSONB. Adding a new enum value here would conflict
        // with Phase 2 (which owns the auth-action surface).
        let row: (Uuid, DateTime<Utc>) = sqlx::query_as(
            r#"
            INSERT INTO audit_logs
                (user_id, action, resource_type, resource_id, details, ip_address, user_agent)
            VALUES ($1, 'resource_accessed'::audit_action, $2, $3, $4, $5, $6)
            RETURNING id, created_at
            "#,
        )
        .bind(actor_id)
        .bind(target_type)
        .bind(target_id)
        .bind(details)
        .bind(ip)
        .bind(user_agent)
        .fetch_one(&self.pool)
        .await
        .map_err(|e: SqlxError| AdminError::Internal(format!("audit_writer: {}", e)))?;

        Ok(RecordedAuditEvent {
            id: row.0,
            created_at: row.1,
        })
    }
}

/// No-op audit writer for unit tests that do not care about persistence.
#[derive(Default, Clone)]
pub struct NoopAuditWriter;

#[async_trait]
impl AuditWriter for NoopAuditWriter {
    async fn record(
        &self,
        _actor_id: Option<Uuid>,
        _capability: Capability,
        _outcome: AuditOutcome,
        _target_type: Option<&str>,
        _target_id: Option<Uuid>,
        _payload: Option<&serde_json::Value>,
        _ip: Option<&str>,
        _user_agent: Option<&str>,
    ) -> Result<RecordedAuditEvent, AdminError> {
        Ok(RecordedAuditEvent {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_hash_is_stable_and_hex() {
        let v = serde_json::json!({ "a": 1, "b": "x" });
        let h1 = hash_payload(&v);
        let h2 = hash_payload(&v);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn payload_hash_differs_for_different_input() {
        let a = serde_json::json!({ "a": 1 });
        let b = serde_json::json!({ "a": 2 });
        assert_ne!(hash_payload(&a), hash_payload(&b));
    }
}

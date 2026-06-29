//! ACC platform / security models (EPIC-ACC-16).
//!
//! Structs match migration 00202 (`acc_tag`, `acc_share_link`, `acc_audit_log`,
//! `acc_two_factor`), with `acc_share_link.token` renamed to `token_hash` in
//! 00204. Column names verified against those migrations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Document tag (UC-ACC-05.14). Generic over an entity ref.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccTag {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub tag: String,
    pub created_at: DateTime<Utc>,
}

/// Capability-token share link for a read-only invoice view (UC-ACC-05.11).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccShareLink {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub invoice_id: Uuid,
    /// SHA-256 hash of the capability token (the raw token is shown once on
    /// creation and never persisted in clear). Lookups hash the presented token.
    pub token_hash: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Append-only audit-trail entry (UC-ACC-16.7 / 01.10).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccAuditLog {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    #[schema(value_type = Object)]
    pub diff: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Per-user TOTP 2FA enrollment for ACC (UC-ACC-16.1 / 01.9).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AccTwoFactor {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub secret: String,
    pub enabled: bool,
    pub confirmed_at: Option<DateTime<Utc>>,
    #[schema(value_type = Object)]
    pub recovery_codes: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

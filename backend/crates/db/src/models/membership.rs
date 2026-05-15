//! User membership models (Phase 2 — Identity Unification).
//!
//! `UserMembership` mirrors a row in the `user_memberships` table — the new
//! authorization spine. Authority is resolved per-request from active rows
//! here intersected with the host-resolved organization (defends leaks #10/#11).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A single (user, organization, role) grant.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserMembership {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub role: String,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl UserMembership {
    /// True when the row represents an active grant right now.
    pub fn is_active(&self) -> bool {
        if self.revoked_at.is_some() {
            return false;
        }
        match self.expires_at {
            Some(exp) => exp > Utc::now(),
            None => true,
        }
    }
}

/// Data required to create a membership grant.
#[derive(Debug, Clone)]
pub struct GrantMembership {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub role: String,
    pub granted_by: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// User invite (used by `user_invites` table; see migration 00130).
#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize)]
pub struct UserInvite {
    pub id: Uuid,
    pub email: String,
    pub organization_id: Uuid,
    pub role: String,
    pub invited_by: Option<Uuid>,
    /// SHA-256 hex of the token. Plaintext is shown ONCE at creation time.
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl UserInvite {
    pub fn is_accepted(&self) -> bool {
        self.accepted_at.is_some()
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }

    /// Acceptable invites are: not yet accepted AND not expired.
    pub fn is_redeemable(&self) -> bool {
        !self.is_accepted() && !self.is_expired()
    }
}

/// User merge collision row (`user_merge_collisions` table; see migration 00131).
///
/// Written by the portal_users → users merge path when an email collision is
/// detected. Read access is super-admin only via RLS (defends leak #7).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserMergeCollision {
    pub id: Uuid,
    pub source_table: String,
    pub source_id: Uuid,
    pub target_email: String,
    pub payload: serde_json::Value,
    pub status: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

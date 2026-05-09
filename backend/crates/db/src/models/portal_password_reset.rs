//! Portal password reset token models (Reality Portal UC-44.3).
//!
//! Mirrors `password_reset.rs` but for `portal_users` rather than the
//! Property Management `users` table.

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Portal password reset token row.
#[derive(Debug, Clone, FromRow)]
pub struct PortalPasswordResetToken {
    pub id: Uuid,
    pub portal_user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl PortalPasswordResetToken {
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_used()
    }
}

/// Data for creating a new portal password reset token.
#[derive(Debug, Clone)]
pub struct CreatePortalPasswordResetToken {
    pub portal_user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

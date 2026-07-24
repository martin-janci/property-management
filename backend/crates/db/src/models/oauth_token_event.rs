//! OAuth token-usage analytics models (Epic 10A — data audit).
//!
//! These models back the `oauth_token_events` table, which records
//! issuance / refresh / revocation of first-party OAuth tokens per client so
//! the platform-admin ecosystem-health dashboard can surface OAuth usage.
//! The table is system-scoped (no org / RLS) like the other `oauth_*` tables.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Lifecycle event tracked for per-client OAuth token-usage analytics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthTokenEventType {
    /// A new access (and optionally refresh) token was minted.
    Issued,
    /// A refresh token was exchanged for a new access token.
    Refreshed,
    /// An access or refresh token was revoked.
    Revoked,
}

impl OAuthTokenEventType {
    /// Stable database representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Issued => "issued",
            Self::Refreshed => "refreshed",
            Self::Revoked => "revoked",
        }
    }

    /// Parse from the database representation.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "issued" => Some(Self::Issued),
            "refreshed" => Some(Self::Refreshed),
            "revoked" => Some(Self::Revoked),
            _ => None,
        }
    }
}

/// Which token an event concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthTokenKind {
    /// Short-lived access token.
    Access,
    /// Long-lived refresh token.
    Refresh,
}

impl OAuthTokenKind {
    /// Stable database representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Refresh => "refresh",
        }
    }

    /// Parse from the database representation.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "access" => Some(Self::Access),
            "refresh" => Some(Self::Refresh),
            _ => None,
        }
    }
}

/// A recorded OAuth token lifecycle event (row of `oauth_token_events`).
#[derive(Debug, Clone, FromRow)]
pub struct OAuthTokenEvent {
    pub id: Uuid,
    pub event_type: String,
    pub client_id: String,
    pub user_id: Option<Uuid>,
    pub token_type: String,
    pub grant_type: Option<String>,
    pub scopes: sqlx::types::Json<Vec<String>>,
    pub created_at: DateTime<Utc>,
}

/// Data for recording a new token event.
#[derive(Debug, Clone)]
pub struct CreateOAuthTokenEvent {
    pub event_type: OAuthTokenEventType,
    pub client_id: String,
    pub user_id: Option<Uuid>,
    pub token_type: OAuthTokenKind,
    /// OAuth `grant_type` (e.g. `authorization_code`, `refresh_token`); absent
    /// for revocations.
    pub grant_type: Option<String>,
    pub scopes: Vec<String>,
}

impl CreateOAuthTokenEvent {
    /// Convenience constructor for an issuance event.
    pub fn issued(
        client_id: impl Into<String>,
        user_id: Option<Uuid>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            event_type: OAuthTokenEventType::Issued,
            client_id: client_id.into(),
            user_id,
            token_type: OAuthTokenKind::Access,
            grant_type: Some("authorization_code".to_string()),
            scopes,
        }
    }

    /// Convenience constructor for a refresh event.
    pub fn refreshed(
        client_id: impl Into<String>,
        user_id: Option<Uuid>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            event_type: OAuthTokenEventType::Refreshed,
            client_id: client_id.into(),
            user_id,
            token_type: OAuthTokenKind::Access,
            grant_type: Some("refresh_token".to_string()),
            scopes,
        }
    }

    /// Convenience constructor for a revocation event.
    pub fn revoked(
        client_id: impl Into<String>,
        user_id: Option<Uuid>,
        token_type: OAuthTokenKind,
    ) -> Self {
        Self {
            event_type: OAuthTokenEventType::Revoked,
            client_id: client_id.into(),
            user_id,
            token_type,
            grant_type: None,
            scopes: Vec::new(),
        }
    }
}

/// Per-client rollup of token-usage counts for the ecosystem-health dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OAuthClientTokenUsage {
    pub client_id: String,
    /// Client display name (NULL if the client row was removed).
    pub client_name: Option<String>,
    pub issued_count: i64,
    pub refreshed_count: i64,
    pub revoked_count: i64,
    /// Timestamp of the most recent event for this client in the window.
    pub last_event_at: Option<DateTime<Utc>>,
}

/// Whole-ecosystem token-usage totals over a time window.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct OAuthTokenEventTotals {
    pub issued_count: i64,
    pub refreshed_count: i64,
    pub revoked_count: i64,
    /// Distinct clients that produced at least one event in the window.
    pub active_clients: i64,
}

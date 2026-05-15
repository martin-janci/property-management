//! Agency domain model (Phase 1: Tenant Resolution Keystone).
//!
//! An `agency_domains` row maps an inbound Host header (a subdomain OR a full
//! custom domain) to exactly one organization — the RLS tenant. The
//! host-resolution middleware queries this table on a system (non-RLS)
//! connection before any tenant context is established.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Kind of domain mapping: a platform subdomain or a customer-owned custom domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AgencyDomainKind {
    Subdomain,
    Custom,
}

impl AgencyDomainKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgencyDomainKind::Subdomain => "subdomain",
            AgencyDomainKind::Custom => "custom",
        }
    }
}

impl std::fmt::Display for AgencyDomainKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Verification lifecycle for a domain mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AgencyDomainVerificationState {
    Pending,
    Verifying,
    Verified,
    Failed,
}

impl AgencyDomainVerificationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgencyDomainVerificationState::Pending => "pending",
            AgencyDomainVerificationState::Verifying => "verifying",
            AgencyDomainVerificationState::Verified => "verified",
            AgencyDomainVerificationState::Failed => "failed",
        }
    }

    /// Whether this state permits the domain to resolve to its tenant.
    pub fn is_verified(&self) -> bool {
        matches!(self, AgencyDomainVerificationState::Verified)
    }
}

impl std::fmt::Display for AgencyDomainVerificationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Agency domain entity from the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AgencyDomain {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub host: String,
    /// `subdomain` | `custom` (stored as text; see [`AgencyDomainKind`]).
    pub kind: String,
    /// `pending` | `verifying` | `verified` | `failed` (see [`AgencyDomainVerificationState`]).
    pub verification_state: String,
    pub verification_token: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub cert_enabled: bool,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AgencyDomain {
    /// Whether this domain currently resolves to its tenant.
    pub fn is_verified(&self) -> bool {
        self.verification_state == "verified"
    }

    /// Get kind as enum (defaults to `Subdomain` for unknown values).
    pub fn kind_enum(&self) -> AgencyDomainKind {
        match self.kind.as_str() {
            "custom" => AgencyDomainKind::Custom,
            _ => AgencyDomainKind::Subdomain,
        }
    }

    /// Get verification state as enum (defaults to `Pending` for unknown values).
    pub fn verification_state_enum(&self) -> AgencyDomainVerificationState {
        match self.verification_state.as_str() {
            "verifying" => AgencyDomainVerificationState::Verifying,
            "verified" => AgencyDomainVerificationState::Verified,
            "failed" => AgencyDomainVerificationState::Failed,
            _ => AgencyDomainVerificationState::Pending,
        }
    }
}

/// Data for creating a new agency domain mapping.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAgencyDomain {
    pub organization_id: Uuid,
    pub host: String,
    /// Defaults to `subdomain` when omitted.
    #[serde(default)]
    pub kind: Option<AgencyDomainKind>,
    /// Defaults to `false` when omitted.
    #[serde(default)]
    pub is_primary: bool,
}

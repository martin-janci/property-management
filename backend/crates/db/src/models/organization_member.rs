//! Organization member model (Epic 2A, Story 2A.5).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Organization membership status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MembershipStatus {
    Pending,
    Active,
    Suspended,
    Removed,
}

impl MembershipStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MembershipStatus::Pending => "pending",
            MembershipStatus::Active => "active",
            MembershipStatus::Suspended => "suspended",
            MembershipStatus::Removed => "removed",
        }
    }
}

impl std::fmt::Display for MembershipStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Organization member entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Option<Uuid>,
    pub role_type: String,
    pub status: String,
    pub invited_by: Option<Uuid>,
    pub invited_at: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OrganizationMember {
    /// Check if membership is active.
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    /// Check if membership is pending.
    pub fn is_pending(&self) -> bool {
        self.status == "pending"
    }

    /// Get status as enum.
    pub fn status_enum(&self) -> MembershipStatus {
        match self.status.as_str() {
            "pending" => MembershipStatus::Pending,
            "active" => MembershipStatus::Active,
            "suspended" => MembershipStatus::Suspended,
            "removed" => MembershipStatus::Removed,
            _ => MembershipStatus::Pending,
        }
    }
}

/// Data for creating a new organization membership.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateOrganizationMember {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Option<Uuid>,
    pub role_type: String,
    pub invited_by: Option<Uuid>,
}

/// Data for updating an organization membership.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateOrganizationMember {
    pub role_id: Option<Uuid>,
    pub role_type: Option<String>,
    pub status: Option<MembershipStatus>,
}

/// Organization member with user details (for list views).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct OrganizationMemberWithUser {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Option<Uuid>,
    pub role_type: String,
    pub status: String,
    pub joined_at: Option<DateTime<Utc>>,
    // User fields
    pub user_email: String,
    pub user_name: String,
    // Role fields (populated via LEFT JOIN roles to avoid N+1 in list views).
    // `None` when the member has no role assignment.
    pub role_name: Option<String>,
}

/// User's membership in an organization (for user profile views).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UserOrganizationMembership {
    pub membership_id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub organization_slug: String,
    pub organization_logo_url: Option<String>,
    pub role_type: String,
    pub role_name: Option<String>,
    pub status: String,
    pub joined_at: Option<DateTime<Utc>>,
}

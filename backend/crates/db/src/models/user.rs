//! User model (Epic 1, Story 1.1).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// User status enumeration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum UserStatus {
    Pending,
    Active,
    Suspended,
    Deleted,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Pending => "pending",
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
            UserStatus::Deleted => "deleted",
        }
    }
}

impl std::fmt::Display for UserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Supported locales for email templates.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum Locale {
    #[sqlx(rename = "sk")]
    Slovak,
    #[sqlx(rename = "cs")]
    Czech,
    #[sqlx(rename = "de")]
    German,
    #[sqlx(rename = "en")]
    #[default]
    English,
}

impl Locale {
    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::Slovak => "sk",
            Locale::Czech => "cs",
            Locale::German => "de",
            Locale::English => "en",
        }
    }

    /// Parse locale from string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sk" | "sk-sk" => Locale::Slovak,
            "cs" | "cs-cz" => Locale::Czech,
            "de" | "de-de" | "de-at" | "de-ch" => Locale::German,
            _ => Locale::English,
        }
    }
}

/// Profile visibility for neighbor display (Story 6.6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileVisibility {
    /// Full name and unit shown to neighbors
    #[default]
    Visible,
    /// Anonymous display as "Resident of Unit X"
    Hidden,
    /// Name shown but no contact info unless connected
    ContactsOnly,
}

impl ProfileVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileVisibility::Visible => "visible",
            ProfileVisibility::Hidden => "hidden",
            ProfileVisibility::ContactsOnly => "contacts_only",
        }
    }

    /// Parse from database string.
    pub fn parse(s: &str) -> Self {
        match s {
            "visible" => ProfileVisibility::Visible,
            "hidden" => ProfileVisibility::Hidden,
            "contacts_only" => ProfileVisibility::ContactsOnly,
            _ => ProfileVisibility::Visible,
        }
    }
}

/// User entity from database.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub phone: Option<String>,
    pub status: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub invited_at: Option<DateTime<Utc>>,
    pub invited_by: Option<Uuid>,
    pub suspended_at: Option<DateTime<Utc>>,
    pub suspended_by: Option<Uuid>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub locale: String,
    /// Privacy: profile visibility to neighbors (Story 6.6)
    pub profile_visibility: String,
    /// Privacy: show contact info to neighbors (Story 6.6)
    pub show_contact_info: bool,
    /// GDPR: scheduled deletion timestamp (Story 9.4)
    pub scheduled_deletion_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Check if user account is active and verified.
    pub fn is_active(&self) -> bool {
        self.status == "active" && self.email_verified_at.is_some()
    }

    /// Check if user can log in.
    pub fn can_login(&self) -> bool {
        self.status == "active"
    }

    /// Check if email is verified.
    pub fn is_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    /// Get status as enum.
    pub fn status_enum(&self) -> UserStatus {
        match self.status.as_str() {
            "pending" => UserStatus::Pending,
            "active" => UserStatus::Active,
            "suspended" => UserStatus::Suspended,
            "deleted" => UserStatus::Deleted,
            _ => UserStatus::Pending,
        }
    }

    /// Get locale as enum.
    pub fn locale_enum(&self) -> Locale {
        Locale::parse(&self.locale)
    }

    /// Get profile visibility as enum (Story 6.6).
    pub fn visibility_enum(&self) -> ProfileVisibility {
        ProfileVisibility::parse(&self.profile_visibility)
    }

    /// Check if account is scheduled for deletion (Story 9.4).
    pub fn is_scheduled_for_deletion(&self) -> bool {
        self.scheduled_deletion_at.is_some()
    }

    /// Check if email is verified (helper for GDPR export).
    pub fn email_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }
}

/// Data for creating a new user.
#[derive(Debug, Clone)]
pub struct CreateUser {
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub phone: Option<String>,
    pub locale: Locale,
}

/// Data for updating a user.
#[derive(Debug, Clone, Default)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub locale: Option<Locale>,
}

/// Email verification token entity.
#[derive(Debug, Clone, FromRow)]
pub struct EmailVerificationToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl EmailVerificationToken {
    /// Check if token is expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Check if token has been used.
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    /// Check if token is valid (not expired and not used).
    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_used()
    }
}

// ============================================================================
// Neighbor Models (Story 6.6)
// ============================================================================

/// Privacy-aware view of a neighbor.
/// Respects the neighbor's profile_visibility and show_contact_info settings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NeighborView {
    pub user_id: Uuid,
    /// Display name - full name if visible, "Resident" if hidden
    pub display_name: String,
    /// Unit identifier (e.g., "Apt 4B")
    pub unit_label: String,
    /// Whether the profile is visible (vs hidden/anonymous)
    pub is_visible: bool,
    /// Contact email (only if show_contact_info is true and visibility is not hidden)
    pub email: Option<String>,
    /// Contact phone (only if show_contact_info is true and visibility is not hidden)
    pub phone: Option<String>,
    /// Resident type (owner, tenant, etc.)
    pub resident_type: String,
}

/// Row for neighbor query (before privacy transformation).
#[derive(Debug, Clone, FromRow)]
pub struct NeighborRow {
    pub user_id: Uuid,
    pub user_name: String,
    pub user_email: String,
    pub user_phone: Option<String>,
    pub profile_visibility: String,
    pub show_contact_info: bool,
    pub unit_id: Uuid,
    pub unit_number: String,
    pub building_name: Option<String>,
    pub resident_type: String,
}

impl NeighborRow {
    /// Transform into privacy-aware NeighborView.
    pub fn into_neighbor_view(self) -> NeighborView {
        let visibility = ProfileVisibility::parse(&self.profile_visibility);

        let (display_name, is_visible) = match visibility {
            ProfileVisibility::Hidden => (format!("Resident of {}", self.unit_number), false),
            _ => (self.user_name, true),
        };

        let (email, phone) = if visibility != ProfileVisibility::Hidden && self.show_contact_info {
            (Some(self.user_email), self.user_phone)
        } else {
            (None, None)
        };

        NeighborView {
            user_id: self.user_id,
            display_name,
            unit_label: self.unit_number,
            is_visible,
            email,
            phone,
            resident_type: self.resident_type,
        }
    }
}

/// User's privacy settings (Story 6.6).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PrivacySettings {
    pub profile_visibility: ProfileVisibility,
    pub show_contact_info: bool,
}

/// Request to update privacy settings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePrivacySettings {
    pub profile_visibility: Option<ProfileVisibility>,
    pub show_contact_info: Option<bool>,
}

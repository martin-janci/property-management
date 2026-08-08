//! Announcement model (Epic 6: Announcements & Communication).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Announcement status enum values.
pub mod announcement_status {
    pub const DRAFT: &str = "draft";
    pub const SCHEDULED: &str = "scheduled";
    pub const PUBLISHED: &str = "published";
    pub const ARCHIVED: &str = "archived";

    pub const ALL: &[&str] = &[DRAFT, SCHEDULED, PUBLISHED, ARCHIVED];
}

/// Announcement target type enum values.
pub mod target_type {
    pub const ALL: &str = "all";
    pub const BUILDING: &str = "building";
    pub const UNITS: &str = "units";
    pub const ROLES: &str = "roles";

    pub const ALL_TYPES: &[&str] = &[ALL, BUILDING, UNITS, ROLES];
}

// ============================================================================
// Announcement
// ============================================================================

/// Announcement entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Announcement {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub content: String,
    pub target_type: String,
    pub target_ids: serde_json::Value,
    pub status: String,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub pinned_at: Option<DateTime<Utc>>,
    pub pinned_by: Option<Uuid>,
    pub comments_enabled: bool,
    pub acknowledgment_required: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Announcement {
    /// Check if announcement is in draft status.
    pub fn is_draft(&self) -> bool {
        self.status == announcement_status::DRAFT
    }

    /// Check if announcement is published.
    pub fn is_published(&self) -> bool {
        self.status == announcement_status::PUBLISHED
    }

    /// Check if announcement is scheduled.
    pub fn is_scheduled(&self) -> bool {
        self.status == announcement_status::SCHEDULED
    }

    /// Check if announcement can be edited.
    pub fn can_edit(&self) -> bool {
        self.status == announcement_status::DRAFT || self.status == announcement_status::SCHEDULED
    }

    /// Check if announcement is currently visible to users.
    pub fn is_visible(&self) -> bool {
        self.status == announcement_status::PUBLISHED
    }
}

/// Summary view of an announcement.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementSummary {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub target_type: String,
    pub published_at: Option<DateTime<Utc>>,
    pub pinned: bool,
    pub comments_enabled: bool,
    pub acknowledgment_required: bool,
}

/// Announcement with additional details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementWithDetails {
    #[serde(flatten)]
    pub announcement: Announcement,
    pub author_name: String,
    pub read_count: i64,
    pub acknowledged_count: i64,
    pub comment_count: i64,
    pub attachment_count: i64,
}

/// Data for creating an announcement.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAnnouncement {
    pub organization_id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub content: String,
    pub target_type: String,
    pub target_ids: Vec<Uuid>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub comments_enabled: Option<bool>,
    pub acknowledgment_required: Option<bool>,
}

/// Data for updating an announcement.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAnnouncement {
    pub title: Option<String>,
    pub content: Option<String>,
    pub target_type: Option<String>,
    pub target_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub comments_enabled: Option<bool>,
    pub acknowledgment_required: Option<bool>,
}

/// Data for publishing an announcement.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublishAnnouncement {
    /// If true, publish immediately. If false, use scheduled_at.
    pub immediate: bool,
}

/// Data for pinning an announcement.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PinAnnouncement {
    pub pinned: bool,
}

// ============================================================================
// Announcement Attachment
// ============================================================================

/// Announcement attachment entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementAttachment {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub file_key: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

/// Data for creating an attachment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAnnouncementAttachment {
    pub announcement_id: Uuid,
    pub file_key: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
}

// ============================================================================
// Announcement Read (Foundation for Story 6.2)
// ============================================================================

/// Announcement read record entity.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementRead {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub read_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
}

/// Data for marking an announcement as read.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarkAnnouncementRead {
    pub announcement_id: Uuid,
    pub user_id: Uuid,
}

/// Data for acknowledging an announcement.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgeAnnouncement {
    pub announcement_id: Uuid,
    pub user_id: Uuid,
}

// ============================================================================
// Query types
// ============================================================================

/// Query for listing announcements.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct AnnouncementListQuery {
    pub status: Option<Vec<String>>,
    pub target_type: Option<String>,
    pub author_id: Option<Uuid>,
    pub pinned: Option<bool>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Statistics for announcements.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementStatistics {
    pub total: i64,
    pub published: i64,
    pub draft: i64,
    pub scheduled: i64,
    pub archived: i64,
}

/// Acknowledgment statistics for a single announcement (Story 6.2).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgmentStats {
    pub announcement_id: Uuid,
    pub total_targeted: i64,
    pub read_count: i64,
    pub acknowledged_count: i64,
    pub pending_count: i64,
}

/// Delivered / read / acknowledged fan-out metrics for a single announcement,
/// implicitly bucketed by its **targeting scope** (`scope` = the announcement's
/// `target_type`: `all` | `building` | `units` | `roles`).
///
/// `delivered` is the *scope-aware* audience — the set of active organization
/// members the announcement's targeting actually resolves to, computed with the
/// same cross-tenant-safe joins the notification fan-out uses (issue #2484).
/// This is deliberately narrower than [`AcknowledgmentStats::total_targeted`],
/// which counts every active org member regardless of `target_type` and so
/// over-counts a building/unit/role-targeted announcement. `read` and
/// `acknowledged` come from `announcement_reads` for the same announcement.
///
/// Emitting delivered per scope from the real targeting SQL (rather than a
/// pure-Rust re-model of the audience) is the data-quality guard #2484 asks for:
/// the count the metric reports is the count the query actually resolves, so a
/// regression in the cross-tenant scoping surfaces as a metric mismatch.
#[derive(Debug, Clone, PartialEq, Eq, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementFanoutMetrics {
    pub announcement_id: Uuid,
    /// The announcement's `target_type` — the targeting scope the counts apply to.
    pub scope: String,
    /// Scope-aware audience size (cross-tenant-safe), i.e. the fan-out reach.
    pub delivered: i64,
    /// Number of users who have a read record for this announcement.
    pub read: i64,
    /// Number of users who acknowledged this announcement.
    pub acknowledged: i64,
}

/// Individual user's acknowledgment status for an announcement (Story 6.2).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct UserAcknowledgmentStatus {
    pub user_id: Uuid,
    pub user_name: String,
    pub read_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Announcement Comments (Story 6.3)
// ============================================================================

/// Announcement comment entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementComment {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub ai_training_consent: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub deletion_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AnnouncementComment {
    /// Check if the comment is deleted (soft-deleted).
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Check if this is a top-level comment.
    pub fn is_top_level(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this is a reply to another comment.
    pub fn is_reply(&self) -> bool {
        self.parent_id.is_some()
    }
}

/// Comment with author information for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommentWithAuthor {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub author_name: String,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Nested replies (max 1 level deep).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replies: Option<Vec<CommentWithAuthor>>,
}

/// Row struct for comment with author query.
#[derive(Debug, Clone, FromRow)]
pub struct CommentWithAuthorRow {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub author_name: String,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CommentWithAuthorRow {
    /// Convert to CommentWithAuthor.
    pub fn into_comment_with_author(
        self,
        replies: Option<Vec<CommentWithAuthor>>,
    ) -> CommentWithAuthor {
        CommentWithAuthor {
            id: self.id,
            announcement_id: self.announcement_id,
            user_id: self.user_id,
            parent_id: self.parent_id,
            content: if self.deleted_at.is_some() {
                "[deleted]".to_string()
            } else {
                self.content
            },
            author_name: if self.deleted_at.is_some() {
                "[deleted]".to_string()
            } else {
                self.author_name
            },
            is_deleted: self.deleted_at.is_some(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            replies,
        }
    }
}

/// Data for creating a comment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateComment {
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub ai_training_consent: bool,
}

/// Data for deleting (soft-delete) a comment.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteComment {
    pub comment_id: Uuid,
    pub deleted_by: Uuid,
    pub deletion_reason: Option<String>,
}

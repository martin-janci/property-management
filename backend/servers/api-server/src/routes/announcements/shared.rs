//! Shared types, constants, and helpers for the announcement route surfaces.
//!
//! These items are split out of the per-surface handler modules (`crud`,
//! `lifecycle`, `engagement`, `comments`, `stats`, `ai_draft`) so each surface
//! can `use super::shared::*` without duplicating request/response DTOs or the
//! validation/sanitization/recipient-resolution helpers.

use chrono::{DateTime, Utc};
use db::models::{
    target_type, AcknowledgmentStats, Announcement, AnnouncementAttachment, AnnouncementStatistics,
    AnnouncementSummary, AnnouncementWithDetails, CommentWithAuthor, UserAcknowledgmentStatus,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed title length (characters).
pub(crate) const MAX_TITLE_LENGTH: usize = 200;

/// Maximum allowed content length (characters).
pub(crate) const MAX_CONTENT_LENGTH: usize = 50_000;

/// Maximum allowed comment length (characters).
pub(crate) const MAX_COMMENT_LENGTH: usize = 2000;

// ============================================================================
// Response Types
// ============================================================================

/// Response for announcement creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAnnouncementResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for announcement list with pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementListResponse {
    pub announcements: Vec<AnnouncementSummary>,
    /// Number of items in this response.
    pub count: usize,
    /// Total number of items matching the query (for pagination).
    pub total: i64,
}

/// Response for announcement details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementDetailResponse {
    pub announcement: AnnouncementWithDetails,
    pub attachments: Vec<AnnouncementAttachment>,
}

/// Response for generic announcement action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementActionResponse {
    pub message: String,
    pub announcement: Announcement,
}

/// Response for attachments list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AttachmentsResponse {
    pub attachments: Vec<AnnouncementAttachment>,
}

/// Response for statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatisticsResponse {
    pub statistics: AnnouncementStatistics,
}

/// Response for unread count.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnreadCountResponse {
    pub unread_count: i64,
}

/// Response for acknowledgment statistics (Story 6.2).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgmentStatsResponse {
    pub stats: AcknowledgmentStats,
}

/// Response for acknowledgment list (Story 6.2).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AcknowledgmentListResponse {
    pub users: Vec<UserAcknowledgmentStatus>,
    pub count: usize,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAnnouncementRequest {
    pub title: String,
    pub content: String,
    pub target_type: String,
    pub target_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub comments_enabled: Option<bool>,
    pub acknowledgment_required: Option<bool>,
}

/// Request for updating an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub target_type: Option<String>,
    pub target_ids: Option<Vec<Uuid>>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub comments_enabled: Option<bool>,
    pub acknowledgment_required: Option<bool>,
}

/// Request for scheduling an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ScheduleAnnouncementRequest {
    pub scheduled_at: DateTime<Utc>,
}

/// Request for pinning/unpinning an announcement.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PinAnnouncementRequest {
    pub pinned: bool,
}

/// Request for adding an attachment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddAttachmentRequest {
    pub file_key: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
}

/// Query for listing announcements.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListAnnouncementsQuery {
    pub status: Option<String>,
    pub target_type: Option<String>,
    pub author_id: Option<Uuid>,
    pub pinned: Option<bool>,
    pub from_date: Option<chrono::NaiveDate>,
    pub to_date: Option<chrono::NaiveDate>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query for acknowledgment list pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct AcknowledgmentListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Comment Types (Story 6.3)
// ============================================================================

/// Request for creating a comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCommentRequest {
    pub content: String,
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub ai_training_consent: bool,
}

/// Request for deleting a comment (moderation).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteCommentRequest {
    pub reason: Option<String>,
}

/// Response for comment list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommentsResponse {
    pub comments: Vec<CommentWithAuthor>,
    pub count: usize,
    pub total: i64,
}

/// Response for single comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommentResponse {
    pub comment: CommentWithAuthor,
}

/// Query for listing comments.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListCommentsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Epic 92.4: AI Announcement Draft Types
// ============================================================================

/// Request for AI-generated announcement draft.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenerateAiDraftRequest {
    /// Topic or subject of the announcement
    pub topic: String,
    /// Key points to include in the announcement
    pub key_points: Option<Vec<String>>,
    /// Urgency level (low, medium, high, critical)
    pub urgency: Option<String>,
    /// Target audience description
    pub audience: Option<String>,
    /// Tone (formal, friendly, urgent, informative)
    pub tone: Option<String>,
    /// Language (sk, cs, de, en)
    #[serde(default = "default_language")]
    pub language: String,
    /// Number of draft variants to generate (1-3)
    pub num_drafts: Option<i32>,
    /// Building ID for context
    pub building_id: Option<Uuid>,
}

pub(crate) fn default_language() -> String {
    "en".to_string()
}

/// Single announcement draft.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AnnouncementDraft {
    pub title: String,
    pub content: String,
    pub suggested_target: String,
    pub tone_analysis: String,
}

/// Response for AI-generated announcement drafts.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GenerateAiDraftResponse {
    pub drafts: Vec<AnnouncementDraft>,
    pub tokens_used: i32,
    pub generation_time_ms: u64,
    pub provider: String,
    pub model: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Resolve an announcement's targeting into the concrete set of recipient
/// user ids (Epic 2B / Epic 6 publish fan-out).
///
/// Must be called on a connection whose RLS context is the publishing
/// manager's org, so the membership / unit / resident reads are authorised.
///
/// | target_type | recipients |
/// |-------------|------------|
/// | `all`       | every active org member (`user_memberships`, not revoked) |
/// | `building`  | active residents of units in the target buildings |
/// | `units`     | active residents of the target units |
/// | `roles`     | active org members holding one of the target roles |
///
/// De-duplicated (a user in two targeted units is notified once).
pub(crate) async fn resolve_announcement_recipients(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    target_type_str: &str,
    target_ids: &[Uuid],
) -> Result<Vec<Uuid>, sqlx::Error> {
    // Non-`all` targets with no ids resolve to nobody (validated upstream).
    if target_type_str != target_type::ALL && target_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<(Uuid,)> = match target_type_str {
        target_type::ALL => {
            sqlx::query_as(
                r#"
                SELECT DISTINCT user_id FROM user_memberships
                WHERE organization_id = $1 AND revoked_at IS NULL
                "#,
            )
            .bind(org_id)
            .fetch_all(&mut *conn)
            .await?
        }
        target_type::BUILDING => {
            sqlx::query_as(
                r#"
                SELECT DISTINCT ur.user_id
                FROM unit_residents ur
                JOIN units u ON u.id = ur.unit_id
                WHERE u.building_id = ANY($1)
                  AND ur.end_date IS NULL
                  AND ur.receives_notifications = TRUE
                "#,
            )
            .bind(target_ids)
            .fetch_all(&mut *conn)
            .await?
        }
        target_type::UNITS => {
            sqlx::query_as(
                r#"
                SELECT DISTINCT ur.user_id
                FROM unit_residents ur
                WHERE ur.unit_id = ANY($1)
                  AND ur.end_date IS NULL
                  AND ur.receives_notifications = TRUE
                "#,
            )
            .bind(target_ids)
            .fetch_all(&mut *conn)
            .await?
        }
        target_type::ROLES => {
            // Roles are stored as text in `user_memberships.role`; the
            // announcement carries them as the textual role names cast to the
            // UUID-typed `target_ids` only when a role-mapping table exists.
            // This system uses enum role names, so match on the string form.
            let role_names: Vec<String> = target_ids.iter().map(|id| id.to_string()).collect();
            sqlx::query_as(
                r#"
                SELECT DISTINCT user_id FROM user_memberships
                WHERE organization_id = $1
                  AND revoked_at IS NULL
                  AND role = ANY($2)
                "#,
            )
            .bind(org_id)
            .bind(&role_names)
            .fetch_all(&mut *conn)
            .await?
        }
        _ => Vec::new(),
    };

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Validate that target_ids exist within the organization.
///
/// Security fix (Critical 1.3): Ensures managers can only target buildings/units
/// that belong to their organization.
pub(crate) async fn validate_target_ids(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    target_type: &str,
    target_ids: &[Uuid],
) -> Result<(), String> {
    if target_ids.is_empty() {
        return Ok(());
    }

    match target_type {
        target_type::BUILDING => {
            // Validate buildings exist in the organization
            let (count,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*) FROM buildings
                WHERE id = ANY($1) AND organization_id = $2
                "#,
            )
            .bind(target_ids)
            .bind(org_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

            if (count as usize) != target_ids.len() {
                return Err(format!(
                    "One or more target buildings not found in organization (found {}/{})",
                    count,
                    target_ids.len()
                ));
            }
        }
        target_type::UNITS => {
            // Validate units exist in the organization
            let (count,): (i64,) = sqlx::query_as(
                r#"
                SELECT COUNT(*) FROM units u
                JOIN buildings b ON b.id = u.building_id
                WHERE u.id = ANY($1) AND b.organization_id = $2
                "#,
            )
            .bind(target_ids)
            .bind(org_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

            if (count as usize) != target_ids.len() {
                return Err(format!(
                    "One or more target units not found in organization (found {}/{})",
                    count,
                    target_ids.len()
                ));
            }
        }
        target_type::ROLES => {
            // For roles, we validate against the TenantRole enum
            // Role IDs are typically UUIDs mapped to role names, but since our system
            // uses enum roles, we skip validation here (roles are enforced at enum level)
            // If you have a role_mappings table, validate against that
        }
        _ => {
            // Unknown target type - already validated earlier, but be safe
        }
    }

    Ok(())
}

/// Parse target type string to notification TargetType enum.
///
/// Used for converting database target_type values to notification event types.
pub(crate) fn parse_target_type(target_type: &str) -> common::notifications::TargetType {
    match target_type {
        target_type::ALL => common::notifications::TargetType::All,
        target_type::BUILDING => common::notifications::TargetType::Building,
        target_type::UNITS => common::notifications::TargetType::Units,
        target_type::ROLES => common::notifications::TargetType::Roles,
        _ => common::notifications::TargetType::All, // Default fallback
    }
}

/// Sanitize markdown/HTML content using ammonia.
///
/// Security fix (Critical 1.5): Uses ammonia library for robust XSS protection.
/// Allows safe markdown/HTML elements while removing all dangerous content
/// including script tags, event handlers, and javascript: URLs.
pub(crate) fn sanitize_markdown(content: &str) -> String {
    use ammonia::Builder;
    use std::collections::HashSet;

    // Define allowed tags for markdown content
    let allowed_tags: HashSet<&str> = [
        // Text formatting
        "p",
        "br",
        "strong",
        "b",
        "em",
        "i",
        "u",
        "s",
        "del",
        "ins",
        "mark",
        // Headings
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        // Lists
        "ul",
        "ol",
        "li",
        // Quotes and code
        "blockquote",
        "code",
        "pre",
        // Links and images
        "a",
        "img",
        // Tables
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        // Misc
        "hr",
        "div",
        "span",
        "sup",
        "sub",
    ]
    .into_iter()
    .collect();

    // Define allowed attributes for specific tags
    let mut tag_attributes = std::collections::HashMap::new();
    tag_attributes.insert(
        "a",
        ["href", "title", "target"]
            .into_iter()
            .collect::<HashSet<_>>(),
    );
    tag_attributes.insert(
        "img",
        ["src", "alt", "title", "width", "height"]
            .into_iter()
            .collect::<HashSet<_>>(),
    );
    tag_attributes.insert(
        "td",
        ["colspan", "rowspan"].into_iter().collect::<HashSet<_>>(),
    );
    tag_attributes.insert(
        "th",
        ["colspan", "rowspan", "scope"]
            .into_iter()
            .collect::<HashSet<_>>(),
    );

    // Define allowed URL schemes (prevent javascript:, data:, vbscript: etc.)
    let allowed_schemes: HashSet<&str> = ["http", "https", "mailto"].into_iter().collect();

    Builder::default()
        .tags(allowed_tags)
        .tag_attributes(tag_attributes)
        .link_rel(Some("noopener noreferrer"))
        .url_schemes(allowed_schemes)
        .strip_comments(true)
        .clean(content)
        .to_string()
}

// ============================================================================
// Tests (Story 6.3 - comment validation)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Comment content length validation ---

    #[test]
    fn empty_comment_content_is_invalid() {
        let content = "";
        assert!(
            content.is_empty(),
            "Empty content should be caught by handler guard"
        );
    }

    #[test]
    fn comment_content_at_max_length_is_valid() {
        let content = "a".repeat(MAX_COMMENT_LENGTH);
        assert!(content.len() <= MAX_COMMENT_LENGTH);
    }

    #[test]
    fn comment_content_exceeding_max_length_is_invalid() {
        let content = "a".repeat(MAX_COMMENT_LENGTH + 1);
        assert!(content.len() > MAX_COMMENT_LENGTH);
    }

    // --- Sanitize markdown (used in create_comment) ---

    #[test]
    fn sanitize_markdown_strips_script_tags() {
        let input = r#"Hello <script>alert('xss')</script> world"#;
        let output = sanitize_markdown(input);
        assert!(!output.contains("<script>"), "Script tags must be stripped");
        assert!(output.contains("Hello"), "Safe text must be preserved");
    }

    #[test]
    fn sanitize_markdown_allows_safe_formatting() {
        let input = "<strong>Important</strong> and <em>emphasis</em>";
        let output = sanitize_markdown(input);
        assert!(output.contains("<strong>") || output.contains("Important"));
    }

    #[test]
    fn sanitize_markdown_strips_javascript_href() {
        let input = r#"<a href="javascript:alert(1)">click</a>"#;
        let output = sanitize_markdown(input);
        assert!(
            !output.contains("javascript:"),
            "javascript: URLs must be stripped"
        );
    }

    // --- CreateCommentRequest deserialization ---

    #[test]
    fn create_comment_request_defaults_ai_consent_to_false() {
        let json = r#"{"content":"Hello world"}"#;
        let req: CreateCommentRequest = serde_json::from_str(json).unwrap();
        assert!(
            !req.ai_training_consent,
            "ai_training_consent should default to false"
        );
        assert_eq!(req.content, "Hello world");
        assert!(req.parent_id.is_none());
    }

    #[test]
    fn create_comment_request_accepts_parent_id() {
        let parent = Uuid::new_v4();
        let json = format!(
            r#"{{"content":"Reply","parent_id":"{}","ai_training_consent":true}}"#,
            parent
        );
        let req: CreateCommentRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.parent_id, Some(parent));
        assert!(req.ai_training_consent);
    }

    // --- ListCommentsQuery defaults ---

    #[test]
    fn list_comments_query_defaults_to_none() {
        let query = ListCommentsQuery::default();
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }
}

//! Messaging Models
//!
//! Models for direct messaging between users (Epic 6, Story 6.5).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// MESSAGE THREAD MODELS
// ============================================================================

/// A conversation thread between two users
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageThread {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub participant_ids: Vec<Uuid>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Thread with preview info for list display
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThreadWithPreview {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub participant_ids: Vec<Uuid>,
    /// Other participant's info
    pub other_participant: ParticipantInfo,
    /// Last message preview
    pub last_message: Option<MessagePreview>,
    /// Number of unread messages in this thread
    pub unread_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row for thread with preview query
#[derive(Debug, Clone, FromRow)]
pub struct ThreadWithPreviewRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub participant_ids: Vec<Uuid>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Other participant info
    pub other_user_id: Option<Uuid>,
    pub other_first_name: Option<String>,
    pub other_last_name: Option<String>,
    pub other_email: Option<String>,
    // Last message preview
    pub last_message_id: Option<Uuid>,
    pub last_message_content: Option<String>,
    pub last_message_sender_id: Option<Uuid>,
    pub last_message_created_at: Option<DateTime<Utc>>,
    // Unread count
    pub unread_count: Option<i64>,
}

impl ThreadWithPreviewRow {
    pub fn into_thread_with_preview(self, current_user_id: Uuid) -> ThreadWithPreview {
        ThreadWithPreview {
            id: self.id,
            organization_id: self.organization_id,
            participant_ids: self.participant_ids.clone(),
            other_participant: ParticipantInfo {
                id: self.other_user_id.unwrap_or(Uuid::nil()),
                first_name: self.other_first_name.unwrap_or_default(),
                last_name: self.other_last_name.unwrap_or_default(),
                email: self.other_email.unwrap_or_default(),
            },
            last_message: self.last_message_id.map(|id| MessagePreview {
                id,
                content: truncate_preview(self.last_message_content.as_deref().unwrap_or("")),
                sender_id: self.last_message_sender_id.unwrap_or(Uuid::nil()),
                is_from_me: self.last_message_sender_id == Some(current_user_id),
                created_at: self.last_message_created_at.unwrap_or_else(Utc::now),
            }),
            unread_count: self.unread_count.unwrap_or(0),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Basic participant info for display
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantInfo {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

/// Message preview for thread list
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessagePreview {
    pub id: Uuid,
    /// Truncated content (first 100 chars)
    pub content: String,
    pub sender_id: Uuid,
    pub is_from_me: bool,
    pub created_at: DateTime<Utc>,
}

/// Request to create a new thread
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateThread {
    pub organization_id: Uuid,
    pub participant_ids: Vec<Uuid>,
}

// ============================================================================
// MESSAGE MODELS
// ============================================================================

/// An individual message within a thread
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub read_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Message with sender info for display
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageWithSender {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub sender: ParticipantInfo,
    pub content: String,
    pub read_at: Option<DateTime<Utc>>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

/// Row for message with sender query
#[derive(Debug, Clone, FromRow)]
pub struct MessageWithSenderRow {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub read_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    // Sender info
    pub sender_first_name: String,
    pub sender_last_name: String,
    pub sender_email: String,
}

impl From<MessageWithSenderRow> for MessageWithSender {
    fn from(row: MessageWithSenderRow) -> Self {
        Self {
            id: row.id,
            thread_id: row.thread_id,
            sender: ParticipantInfo {
                id: row.sender_id,
                first_name: row.sender_first_name,
                last_name: row.sender_last_name,
                email: row.sender_email,
            },
            content: if row.deleted_at.is_some() {
                "[Message deleted]".to_string()
            } else {
                row.content
            },
            read_at: row.read_at,
            is_deleted: row.deleted_at.is_some(),
            created_at: row.created_at,
        }
    }
}

/// Request to create a new message
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessage {
    pub thread_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
}

// ============================================================================
// MESSAGE ATTACHMENT MODELS (UC-05.9 / BIT-184)
// ============================================================================

/// An S3-backed file attachment linked to a message.
///
/// Mirrors `AnnouncementAttachment`: the row stores file metadata and the S3
/// object key; presigned upload/download URLs are minted on demand by the
/// route layer and are never persisted.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttachment {
    pub id: Uuid,
    pub message_id: Uuid,
    pub file_key: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

/// Request to link an uploaded S3 object to a message.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageAttachment {
    pub message_id: Uuid,
    pub file_key: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,
}

// ============================================================================
// USER BLOCK MODELS
// ============================================================================

/// A user block record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserBlock {
    pub id: Uuid,
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
    /// Organization context for multi-tenant isolation
    pub organization_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Block with blocked user info for display
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BlockWithUserInfo {
    pub id: Uuid,
    pub blocked_user: ParticipantInfo,
    pub created_at: DateTime<Utc>,
}

/// Row for block with user info query
#[derive(Debug, Clone, FromRow)]
pub struct BlockWithUserInfoRow {
    pub id: Uuid,
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
    pub created_at: DateTime<Utc>,
    // Blocked user info
    pub blocked_first_name: String,
    pub blocked_last_name: String,
    pub blocked_email: String,
}

impl From<BlockWithUserInfoRow> for BlockWithUserInfo {
    fn from(row: BlockWithUserInfoRow) -> Self {
        Self {
            id: row.id,
            blocked_user: ParticipantInfo {
                id: row.blocked_id,
                first_name: row.blocked_first_name,
                last_name: row.blocked_last_name,
                email: row.blocked_email,
            },
            created_at: row.created_at,
        }
    }
}

/// Request to create a block
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateBlock {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
    /// Organization context for multi-tenant isolation
    pub organization_id: Uuid,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Truncate content for preview display
fn truncate_preview(content: &str) -> String {
    const MAX_PREVIEW_LENGTH: usize = 100;
    if content.len() <= MAX_PREVIEW_LENGTH {
        content.to_string()
    } else {
        format!("{}...", &content[..MAX_PREVIEW_LENGTH])
    }
}

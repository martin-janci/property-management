//! Document model (Epic 7A: Basic Document Management, Epic 7B: Document Versioning).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum file size in bytes (50MB).
pub const MAX_FILE_SIZE: i64 = 50 * 1024 * 1024;

/// Allowed MIME types for document upload.
pub const ALLOWED_MIME_TYPES: &[&str] = &[
    // Documents
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "text/plain",
    "text/csv",
    // Images
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
];

/// Document category enum values.
pub mod document_category {
    pub const CONTRACTS: &str = "contracts";
    pub const INVOICES: &str = "invoices";
    pub const REPORTS: &str = "reports";
    pub const MANUALS: &str = "manuals";
    pub const CERTIFICATES: &str = "certificates";
    pub const OTHER: &str = "other";

    pub const ALL: &[&str] = &[CONTRACTS, INVOICES, REPORTS, MANUALS, CERTIFICATES, OTHER];
}

/// Access scope enum values.
pub mod access_scope {
    pub const ORGANIZATION: &str = "organization";
    pub const BUILDING: &str = "building";
    pub const UNIT: &str = "unit";
    pub const ROLE: &str = "role";
    pub const USERS: &str = "users";

    pub const ALL: &[&str] = &[ORGANIZATION, BUILDING, UNIT, ROLE, USERS];
}

/// Share type enum values.
pub mod share_type {
    pub const USER: &str = "user";
    pub const ROLE: &str = "role";
    pub const BUILDING: &str = "building";
    pub const LINK: &str = "link";

    pub const ALL: &[&str] = &[USER, ROLE, BUILDING, LINK];
}

// ============================================================================
// Document Folder (Story 7A.2)
// ============================================================================

/// Document folder entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentFolder {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl DocumentFolder {
    /// Check if folder is deleted (soft-deleted).
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Check if this is a root folder.
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Folder with document count for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FolderWithCount {
    #[serde(flatten)]
    pub folder: DocumentFolder,
    pub document_count: i64,
    pub subfolder_count: i64,
}

/// Folder tree node for hierarchical display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FolderTreeNode {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub document_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FolderTreeNode>>,
}

/// Data for creating a folder.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFolder {
    pub organization_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub created_by: Uuid,
}

/// Data for updating a folder.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateFolder {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

// ============================================================================
// Document (Story 7A.1)
// ============================================================================

/// Document entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Document {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub access_scope: String,
    pub access_target_ids: serde_json::Value,
    pub access_roles: serde_json::Value,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    // Version fields (Story 7B.1)
    pub version_number: i32,
    pub parent_document_id: Option<Uuid>,
    pub is_current_version: bool,
    // Template fields (Story 7B.2)
    pub template_id: Option<Uuid>,
    pub generation_metadata: Option<serde_json::Value>,
}

impl Document {
    /// Check if document is deleted (soft-deleted).
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Check if MIME type supports inline preview.
    ///
    /// Delegates to [`common::supports_inline_preview`] — the single source of
    /// truth shared with the storage presigner
    /// (`integrations::storage::supports_inline_preview`) so the preview gate
    /// and the `Content-Disposition: inline` decision cannot diverge (GH #1413).
    pub fn supports_preview(&self) -> bool {
        common::supports_inline_preview(&self.mime_type)
    }

    /// Check if this is an image file.
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    /// Check if this is a PDF file.
    pub fn is_pdf(&self) -> bool {
        self.mime_type == "application/pdf"
    }

    /// Get file extension from file name.
    pub fn extension(&self) -> Option<&str> {
        self.file_name.rsplit('.').next()
    }

    /// Check if this is the original version (first version).
    pub fn is_original_version(&self) -> bool {
        self.parent_document_id.is_none()
    }

    /// Get the root document ID for this version chain.
    pub fn root_document_id(&self) -> Uuid {
        self.parent_document_id.unwrap_or(self.id)
    }
}

/// Document summary for list display.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentSummary {
    pub id: Uuid,
    pub title: String,
    pub category: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub folder_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Document with additional details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentWithDetails {
    #[serde(flatten)]
    pub document: Document,
    pub created_by_name: String,
    pub folder_name: Option<String>,
    pub share_count: i64,
}

/// Data for creating a document.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDocument {
    pub organization_id: Uuid,
    pub folder_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub access_scope: Option<String>,
    pub access_target_ids: Option<Vec<Uuid>>,
    pub access_roles: Option<Vec<String>>,
    pub created_by: Uuid,
}

/// Data for updating a document.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateDocument {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub folder_id: Option<Uuid>,
    pub access_scope: Option<String>,
    pub access_target_ids: Option<Vec<Uuid>>,
    pub access_roles: Option<Vec<String>>,
}

/// Query for listing documents.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct DocumentListQuery {
    pub folder_id: Option<Uuid>,
    pub category: Option<String>,
    pub created_by: Option<Uuid>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Data for moving a document to a folder.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MoveDocument {
    pub document_id: Uuid,
    pub folder_id: Option<Uuid>,
}

// ============================================================================
// Document Share (Story 7A.5)
// ============================================================================

/// Document share entity from database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentShare {
    pub id: Uuid,
    pub document_id: Uuid,
    pub share_type: String,
    pub target_id: Option<Uuid>,
    pub target_role: Option<String>,
    pub shared_by: Uuid,
    pub share_token: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl DocumentShare {
    /// Check if share is active (not revoked and not expired).
    pub fn is_active(&self) -> bool {
        if self.revoked_at.is_some() {
            return false;
        }
        if let Some(expires_at) = self.expires_at {
            if expires_at < Utc::now() {
                return false;
            }
        }
        true
    }

    /// Check if share is a link share.
    pub fn is_link_share(&self) -> bool {
        self.share_type == share_type::LINK
    }

    /// Check if share has password protection.
    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }
}

/// Share with document info for display.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShareWithDocument {
    #[serde(flatten)]
    pub share: DocumentShare,
    pub document_title: String,
    pub shared_by_name: String,
}

/// Data for creating a share.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateShare {
    pub document_id: Uuid,
    pub share_type: String,
    pub target_id: Option<Uuid>,
    pub target_role: Option<String>,
    pub shared_by: Uuid,
    pub password: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Data for revoking a share.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeShare {
    pub share_id: Uuid,
}

// ============================================================================
// Document Share Access Log
// ============================================================================

/// Access log entry for document shares.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ShareAccessLog {
    pub id: Uuid,
    pub share_id: Uuid,
    pub accessed_by: Option<Uuid>,
    pub accessed_at: DateTime<Utc>,
    pub ip_address: Option<String>,
}

/// Data for logging share access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogShareAccess {
    pub share_id: Uuid,
    pub accessed_by: Option<Uuid>,
    pub ip_address: Option<String>,
}

// ============================================================================
// Document Versioning (Story 7B.1)
// ============================================================================

/// Document version information for version history display.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentVersion {
    pub id: Uuid,
    pub version_number: i32,
    pub is_current_version: bool,
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_by: Uuid,
    pub created_by_name: String,
    pub created_at: DateTime<Utc>,
}

/// Full version history for a document.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentVersionHistory {
    pub document_id: Uuid,
    pub title: String,
    pub total_versions: i32,
    pub versions: Vec<DocumentVersion>,
}

/// Request to upload a new version of a document.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDocumentVersion {
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_by: Uuid,
}

/// Request to restore a previous version.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RestoreVersionRequest {
    /// User performing the restore.
    pub restored_by: Uuid,
}

/// Response after creating a new version.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVersionResponse {
    pub id: Uuid,
    pub version_number: i32,
    pub message: String,
}

/// Response after restoring a version.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RestoreVersionResponse {
    pub id: Uuid,
    pub version_number: i32,
    pub message: String,
}

// ============================================================================
// Document Intelligence (Epic 28)
// ============================================================================

/// OCR processing status values.
pub mod ocr_status {
    pub const PENDING: &str = "pending";
    pub const PROCESSING: &str = "processing";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const NOT_APPLICABLE: &str = "not_applicable";
    pub const SKIPPED: &str = "skipped";

    pub const ALL: &[&str] = &[
        PENDING,
        PROCESSING,
        COMPLETED,
        FAILED,
        NOT_APPLICABLE,
        SKIPPED,
    ];
}

/// OCR queue entry for async processing.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentOcrQueue {
    pub id: Uuid,
    pub document_id: Uuid,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DocumentOcrQueue {
    /// Check if queue item has exhausted retry attempts.
    pub fn is_exhausted(&self) -> bool {
        self.attempts >= self.max_attempts
    }

    /// Check if queue item is ready for processing.
    pub fn is_ready(&self) -> bool {
        !self.is_exhausted() && self.next_attempt_at <= Utc::now()
    }
}

/// Classification history entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentClassificationHistory {
    pub id: Uuid,
    pub document_id: Uuid,
    pub predicted_category: String,
    pub confidence: f64,
    pub actual_category: Option<String>,
    pub was_accepted: Option<bool>,
    pub feedback_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Summarization queue entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentSummarizationQueue {
    pub id: Uuid,
    pub document_id: Uuid,
    pub priority: i32,
    pub requested_by: Option<Uuid>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Document intelligence processing statistics.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct DocumentIntelligenceStats {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub date: chrono::NaiveDate,
    pub documents_processed: i32,
    pub ocr_completed: i32,
    pub ocr_failed: i32,
    pub classifications_completed: i32,
    pub classifications_accepted: i32,
    pub summaries_generated: i32,
    pub total_pages_processed: i32,
    pub avg_ocr_confidence: Option<f64>,
    pub avg_classification_confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Document with OCR and intelligence fields.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentWithIntelligence {
    #[serde(flatten)]
    pub document: Document,
    // OCR fields
    pub extracted_text: Option<String>,
    pub ocr_status: String,
    pub ocr_processed_at: Option<DateTime<Utc>>,
    pub ocr_error: Option<String>,
    pub ocr_page_count: Option<i32>,
    pub ocr_confidence: Option<f64>,
    // Classification fields
    pub ai_predicted_category: Option<String>,
    pub ai_classification_confidence: Option<f64>,
    pub ai_classification_at: Option<DateTime<Utc>>,
    pub ai_classification_accepted: Option<bool>,
    // Summarization fields (from Epic 13)
    pub ai_summary: Option<String>,
    pub ai_summary_generated_at: Option<DateTime<Utc>>,
    pub ai_key_points: Option<serde_json::Value>,
    pub ai_action_items: Option<serde_json::Value>,
    pub ai_topics: Option<serde_json::Value>,
    pub word_count: Option<i32>,
    pub language_detected: Option<String>,
}

/// Request to submit classification feedback.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClassificationFeedback {
    pub document_id: Uuid,
    pub accepted: bool,
    pub correct_category: Option<String>,
    pub feedback_by: Uuid,
}

/// Request to generate document summary.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateSummaryRequest {
    pub document_id: Uuid,
    pub requested_by: Uuid,
    pub priority: Option<i32>,
}

/// Full-text search request.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentSearchRequest {
    pub query: String,
    pub organization_id: Uuid,
    #[serde(default)]
    pub include_content: bool,
    pub folder_id: Option<Uuid>,
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Search result with relevance ranking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentSearchResult {
    pub document: DocumentSummary,
    pub rank: f32,
    pub headline: Option<String>,
    pub matched_fields: Vec<String>,
}

/// Search response with pagination.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocumentSearchResponse {
    pub results: Vec<DocumentSearchResult>,
    pub total: i64,
    pub query: String,
}

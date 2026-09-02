//! GDPR data export models (Epic 9, Story 9.3).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Data export request status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "data_export_status", rename_all = "snake_case")]
pub enum DataExportStatus {
    Pending,
    Processing,
    Ready,
    Downloaded,
    Expired,
    Failed,
}

impl DataExportStatus {
    /// Stable snake_case string form — matches the `data_export_status`
    /// Postgres enum labels and the serde representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Ready => "ready",
            Self::Downloaded => "downloaded",
            Self::Expired => "expired",
            Self::Failed => "failed",
        }
    }
}

/// Export format options.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
}

/// A data export request.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DataExportRequest {
    /// Request ID
    pub id: Uuid,
    /// User who requested the export
    pub user_id: Uuid,
    /// Current status
    pub status: DataExportStatus,
    /// Export format
    pub format: String,
    /// Categories to include (null = all)
    pub include_categories: Option<serde_json::Value>,
    /// Path to export file
    pub file_path: Option<String>,
    /// File size in bytes
    pub file_size_bytes: Option<i64>,
    /// SHA-256 hash of file
    pub file_hash: Option<String>,
    /// Secure download token
    pub download_token: Option<Uuid>,
    /// When file was downloaded
    pub downloaded_at: Option<DateTime<Utc>>,
    /// Number of times downloaded
    pub download_count: i32,
    /// When the download expires
    pub expires_at: DateTime<Utc>,
    /// When processing started
    pub started_at: Option<DateTime<Utc>>,
    /// When processing completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// When request was created
    pub created_at: DateTime<Utc>,
    /// When request was last updated
    pub updated_at: DateTime<Utc>,
}

/// Aggregate figures for the platform GDPR data-export compliance report.
///
/// Returned by [`crate::repositories::data_export::DataExportRepository::report_summary`].
/// All counts are computed in a single pass over `data_export_requests` so they
/// stay mutually consistent — the compliance endpoint previously reported
/// hard-coded zeros for the completed/downloaded figures, materially
/// understating GDPR activity to platform admins.
#[derive(Debug, Clone)]
pub struct DataExportReportSummary {
    /// All export requests in range.
    pub total_requests: i64,
    /// Requests still in flight (`pending` or `processing`).
    pub pending_count: i64,
    /// Requests whose export file was successfully generated at some point
    /// (`ready`, `downloaded`, or `expired`).
    pub completed_count: i64,
    /// Requests that were actually downloaded at least once (`downloaded_at`
    /// set), regardless of current status.
    pub downloaded_count: i64,
    /// Most-recent requests in range (bounded by the caller's limit).
    pub entries: Vec<DataExportRequest>,
}

/// Data for creating a new export request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDataExportRequest {
    /// User requesting the export
    pub user_id: Uuid,
    /// Export format (json or csv)
    pub format: ExportFormat,
    /// Optional categories to include
    pub include_categories: Option<Vec<String>>,
}

/// Response for requesting data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExportRequestResponse {
    /// Request ID for tracking
    pub request_id: Uuid,
    /// Estimated time (human readable)
    pub estimated_time: String,
    /// Current status
    pub status: DataExportStatus,
}

/// Response for checking export status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExportStatusResponse {
    /// Request ID
    pub request_id: Uuid,
    /// Current status
    pub status: DataExportStatus,
    /// When the export will be ready (if processing)
    pub estimated_ready_at: Option<DateTime<Utc>>,
    /// Download URL (only if ready)
    pub download_url: Option<String>,
    /// File size in bytes (if ready)
    pub file_size_bytes: Option<i64>,
    /// When download expires
    pub expires_at: DateTime<Utc>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Available data categories for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCategories {
    /// Available categories
    pub categories: Vec<ExportCategory>,
}

/// A single export category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCategory {
    /// Category identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether included by default
    pub default_included: bool,
}

/// The actual exported user data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataExport {
    /// Export metadata
    pub metadata: ExportMetadata,
    /// Profile information
    pub profile: Option<ProfileExport>,
    /// Organization memberships
    pub organizations: Option<Vec<OrganizationExport>>,
    /// Unit residencies
    pub residencies: Option<Vec<ResidencyExport>>,
    /// Activity history
    pub activity: Option<Vec<ActivityExport>>,
    /// Documents
    pub documents: Option<Vec<DocumentExport>>,
    /// Votes cast
    pub votes: Option<Vec<VoteExport>>,
    /// Faults reported
    pub faults: Option<Vec<FaultExport>>,
    /// Messages sent
    pub messages: Option<Vec<MessageExport>>,
    /// Announcements created
    pub announcements: Option<Vec<AnnouncementExport>>,
}

/// Export metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// When export was generated
    pub generated_at: DateTime<Utc>,
    /// Export format
    pub format: String,
    /// Categories included
    pub categories_included: Vec<String>,
    /// User ID
    pub user_id: Uuid,
    /// User email
    pub user_email: String,
}

/// Profile data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExport {
    pub id: Uuid,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone_number: Option<String>,
    pub language: String,
    pub profile_visibility: String,
    pub show_contact_info: bool,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization membership export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationExport {
    pub organization_id: Uuid,
    pub organization_name: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

/// Residency data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidencyExport {
    pub unit_id: Uuid,
    pub building_name: String,
    pub unit_number: String,
    pub resident_type: String,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
}

/// Activity log export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityExport {
    pub action: String,
    pub resource_type: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Document data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentExport {
    pub id: Uuid,
    pub title: String,
    pub category: String,
    pub file_type: String,
    pub file_size: i64,
    pub created_at: DateTime<Utc>,
}

/// Vote data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteExport {
    pub vote_id: Uuid,
    pub vote_title: String,
    pub question: String,
    pub response: String,
    pub voted_at: DateTime<Utc>,
}

/// Fault report export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultExport {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Message data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageExport {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub content: String,
    pub sent_at: DateTime<Utc>,
}

/// Announcement data export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncementExport {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

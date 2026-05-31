//! Advanced Compliance (AML/DSA) models (Epic 67).
//!
//! Types for Anti-Money Laundering risk assessment, Enhanced Due Diligence,
//! Digital Services Act transparency reporting, and content moderation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// STORY 67.1: AML RISK ASSESSMENT
// ============================================================================

/// AML risk level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "aml_risk_level", rename_all = "snake_case")]
pub enum AmlRiskLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// AML assessment status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "aml_assessment_status", rename_all = "snake_case")]
pub enum AmlAssessmentStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    RequiresReview,
    Approved,
    Rejected,
}

/// Country risk rating for AML purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "country_risk_rating", rename_all = "snake_case")]
pub enum CountryRiskRating {
    Low,
    Medium,
    High,
    Sanctioned,
}

impl CountryRiskRating {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            CountryRiskRating::Low => "low",
            CountryRiskRating::Medium => "medium",
            CountryRiskRating::High => "high",
            CountryRiskRating::Sanctioned => "sanctioned",
        }
    }
}

impl std::fmt::Display for CountryRiskRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An AML risk assessment record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AmlRiskAssessment {
    /// Assessment ID
    pub id: Uuid,
    /// Organization performing the assessment
    pub organization_id: Uuid,
    /// Transaction being assessed (if applicable)
    pub transaction_id: Option<Uuid>,
    /// Party being assessed (user/entity ID)
    pub party_id: Uuid,
    /// Type of party (individual, company, trust)
    pub party_type: String,
    /// Transaction amount in cents
    pub transaction_amount_cents: Option<i64>,
    /// Currency code (EUR, USD, etc.)
    pub currency: Option<String>,
    /// Calculated risk score (0-100)
    pub risk_score: i32,
    /// Risk level based on score
    pub risk_level: AmlRiskLevel,
    /// Assessment status
    pub status: AmlAssessmentStatus,
    /// Risk factors identified (JSON array)
    pub risk_factors: Option<serde_json::Value>,
    /// Country of party
    pub country_code: Option<String>,
    /// Country risk rating
    pub country_risk: Option<CountryRiskRating>,
    /// Whether ID verification is complete
    pub id_verified: bool,
    /// Whether source of funds is documented
    pub source_of_funds_documented: bool,
    /// Whether PEP (Politically Exposed Person) check done
    pub pep_check_completed: bool,
    /// PEP check result
    pub is_pep: Option<bool>,
    /// Whether sanctions screening done
    pub sanctions_check_completed: bool,
    /// Sanctions screening result
    pub sanctions_match: Option<bool>,
    /// Notes from assessor
    pub assessor_notes: Option<String>,
    /// User who performed assessment
    pub assessed_by: Option<Uuid>,
    /// When assessment was completed
    pub assessed_at: Option<DateTime<Utc>>,
    /// Whether flagged for review
    pub flagged_for_review: bool,
    /// Review reason if flagged
    pub review_reason: Option<String>,
    /// When record was created
    pub created_at: DateTime<Utc>,
    /// When record was last updated
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new AML risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAmlRiskAssessment {
    /// Organization performing the assessment
    pub organization_id: Uuid,
    /// Transaction being assessed (if applicable)
    pub transaction_id: Option<Uuid>,
    /// Party being assessed
    pub party_id: Uuid,
    /// Type of party
    pub party_type: String,
    /// Transaction amount in cents
    pub transaction_amount_cents: Option<i64>,
    /// Currency code
    pub currency: Option<String>,
    /// Country of party
    pub country_code: Option<String>,
}

/// Response for AML risk assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmlRiskAssessmentResponse {
    /// Assessment ID
    pub id: Uuid,
    /// Party ID
    pub party_id: Uuid,
    /// Calculated risk score (0-100)
    pub risk_score: i32,
    /// Risk level
    pub risk_level: AmlRiskLevel,
    /// Status
    pub status: AmlAssessmentStatus,
    /// Risk factors identified
    pub risk_factors: Vec<RiskFactor>,
    /// Whether flagged for manual review
    pub flagged_for_review: bool,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// A risk factor identified during assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Factor type
    pub factor_type: String,
    /// Factor description
    pub description: String,
    /// Weight/impact on score (0-100)
    pub weight: i32,
    /// Whether mitigated
    pub mitigated: bool,
}

/// Country risk database entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CountryRisk {
    /// Country ISO code
    pub country_code: String,
    /// Country name
    pub country_name: String,
    /// Risk rating
    pub risk_rating: CountryRiskRating,
    /// Whether on sanctions list
    pub is_sanctioned: bool,
    /// FATF status (grey list, black list, etc.)
    pub fatf_status: Option<String>,
    /// Notes about the country
    pub notes: Option<String>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// STORY 67.2: ENHANCED DUE DILIGENCE (EDD)
// ============================================================================

/// EDD status tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "edd_status", rename_all = "snake_case")]
pub enum EddStatus {
    #[default]
    NotRequired,
    Required,
    InProgress,
    PendingDocuments,
    UnderReview,
    Completed,
    Expired,
}

/// Document verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "document_verification_status", rename_all = "snake_case")]
pub enum DocumentVerificationStatus {
    Pending,
    Verified,
    Rejected,
    Expired,
}

impl DocumentVerificationStatus {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentVerificationStatus::Pending => "pending",
            DocumentVerificationStatus::Verified => "verified",
            DocumentVerificationStatus::Rejected => "rejected",
            DocumentVerificationStatus::Expired => "expired",
        }
    }
}

impl std::fmt::Display for DocumentVerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An Enhanced Due Diligence record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EnhancedDueDiligence {
    /// EDD record ID
    pub id: Uuid,
    /// Related AML assessment ID
    pub aml_assessment_id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Party being assessed
    pub party_id: Uuid,
    /// EDD status
    pub status: EddStatus,
    /// Source of wealth documented
    pub source_of_wealth: Option<String>,
    /// Source of funds documented
    pub source_of_funds: Option<String>,
    /// Beneficial ownership documented (JSON)
    pub beneficial_ownership: Option<serde_json::Value>,
    /// Business relationship purpose
    pub relationship_purpose: Option<String>,
    /// Expected transaction patterns
    pub expected_transaction_patterns: Option<String>,
    /// Documents requested (JSON array of document types)
    pub documents_requested: Option<serde_json::Value>,
    /// Compliance notes (immutable once added)
    pub compliance_notes: Option<serde_json::Value>,
    /// When EDD was initiated
    pub initiated_at: DateTime<Utc>,
    /// User who initiated EDD
    pub initiated_by: Uuid,
    /// When EDD was completed
    pub completed_at: Option<DateTime<Utc>>,
    /// User who completed EDD
    pub completed_by: Option<Uuid>,
    /// Next review date
    pub next_review_date: Option<DateTime<Utc>>,
    /// Record hash for immutability verification
    pub record_hash: Option<String>,
    /// When record was created
    pub created_at: DateTime<Utc>,
    /// When record was last updated
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new EDD record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnhancedDueDiligence {
    /// Related AML assessment ID
    pub aml_assessment_id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Party being assessed
    pub party_id: Uuid,
    /// User initiating EDD
    pub initiated_by: Uuid,
    /// Documents to request
    pub documents_requested: Option<Vec<String>>,
}

/// EDD document record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EddDocument {
    /// Document ID
    pub id: Uuid,
    /// Related EDD record ID
    pub edd_id: Uuid,
    /// Document type (passport, utility_bill, bank_statement, etc.)
    pub document_type: String,
    /// Document file path/URL
    pub file_path: String,
    /// Original filename
    pub original_filename: String,
    /// File size in bytes
    pub file_size_bytes: i64,
    /// MIME type
    pub mime_type: String,
    /// Verification status
    pub verification_status: DocumentVerificationStatus,
    /// Verified by user ID
    pub verified_by: Option<Uuid>,
    /// When verified
    pub verified_at: Option<DateTime<Utc>>,
    /// Rejection reason if rejected
    pub rejection_reason: Option<String>,
    /// Document expiry date (for ID documents)
    pub expiry_date: Option<DateTime<Utc>>,
    /// When uploaded
    pub uploaded_at: DateTime<Utc>,
    /// Uploaded by user ID
    pub uploaded_by: Uuid,
}

/// Data for adding an EDD document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEddDocument {
    /// Related EDD record ID
    pub edd_id: Uuid,
    /// Document type
    pub document_type: String,
    /// Document file path/URL
    pub file_path: String,
    /// Original filename
    pub original_filename: String,
    /// File size in bytes
    pub file_size_bytes: i64,
    /// MIME type
    pub mime_type: String,
    /// Uploaded by user ID
    pub uploaded_by: Uuid,
    /// Document expiry date
    pub expiry_date: Option<DateTime<Utc>>,
}

/// Compliance note entry (immutable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceNote {
    /// Note ID
    pub id: Uuid,
    /// Note content
    pub content: String,
    /// Added by user ID
    pub added_by: Uuid,
    /// Added by user name
    pub added_by_name: String,
    /// When added
    pub added_at: DateTime<Utc>,
}

/// Request to add a compliance note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddComplianceNote {
    /// Note content
    pub content: String,
}

// ============================================================================
// STORY 67.3: DSA TRANSPARENCY REPORTS
// ============================================================================

/// DSA report status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "dsa_report_status", rename_all = "snake_case")]
pub enum DsaReportStatus {
    Draft,
    Generated,
    Published,
    Archived,
}

/// A DSA transparency report.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DsaTransparencyReport {
    /// Report ID
    pub id: Uuid,
    /// Reporting period start
    pub period_start: DateTime<Utc>,
    /// Reporting period end
    pub period_end: DateTime<Utc>,
    /// Report status
    pub status: DsaReportStatus,
    /// Total content moderation actions
    pub total_moderation_actions: i64,
    /// Content removed count
    pub content_removed_count: i64,
    /// Content restricted count
    pub content_restricted_count: i64,
    /// Warnings issued count
    pub warnings_issued_count: i64,
    /// User reports received
    pub user_reports_received: i64,
    /// User reports resolved
    pub user_reports_resolved: i64,
    /// Average resolution time in hours
    pub avg_resolution_time_hours: Option<f64>,
    /// Automated decisions count
    pub automated_decisions_count: i64,
    /// Automated decisions overturned
    pub automated_decisions_overturned: i64,
    /// Appeals received
    pub appeals_received: i64,
    /// Appeals upheld (in favor of user)
    pub appeals_upheld: i64,
    /// Appeals rejected
    pub appeals_rejected: i64,
    /// Breakdown by content type (JSON)
    pub content_type_breakdown: Option<serde_json::Value>,
    /// Breakdown by violation type (JSON)
    pub violation_type_breakdown: Option<serde_json::Value>,
    /// Generated report file path
    pub report_file_path: Option<String>,
    /// When generated
    pub generated_at: Option<DateTime<Utc>>,
    /// Generated by user ID
    pub generated_by: Option<Uuid>,
    /// When published
    pub published_at: Option<DateTime<Utc>>,
    /// When record was created
    pub created_at: DateTime<Utc>,
    /// When record was last updated
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a DSA transparency report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDsaTransparencyReport {
    /// Reporting period start
    pub period_start: DateTime<Utc>,
    /// Reporting period end
    pub period_end: DateTime<Utc>,
}

/// DSA transparency report response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsaTransparencyReportResponse {
    /// Report ID
    pub id: Uuid,
    /// Reporting period
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Status
    pub status: DsaReportStatus,
    /// Summary statistics
    pub summary: DsaReportSummary,
    /// Content type breakdown
    pub content_type_breakdown: Vec<ContentTypeCount>,
    /// Violation type breakdown
    pub violation_type_breakdown: Vec<ViolationTypeCount>,
    /// Download URL (if generated)
    pub download_url: Option<String>,
    /// When generated
    pub generated_at: Option<DateTime<Utc>>,
}

/// Summary statistics for DSA report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsaReportSummary {
    /// Total moderation actions
    pub total_moderation_actions: i64,
    /// Content removed
    pub content_removed: i64,
    /// Content restricted
    pub content_restricted: i64,
    /// Warnings issued
    pub warnings_issued: i64,
    /// User reports received
    pub user_reports_received: i64,
    /// User reports resolved
    pub user_reports_resolved: i64,
    /// Average resolution time
    pub avg_resolution_time_hours: Option<f64>,
    /// Automated decisions
    pub automated_decisions: i64,
    /// Automated decisions overturned
    pub automated_decisions_overturned: i64,
    /// Appeals statistics
    pub appeals_received: i64,
    pub appeals_upheld: i64,
    pub appeals_rejected: i64,
}

/// Content type count for breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTypeCount {
    pub content_type: String,
    pub count: i64,
}

/// Violation type count for breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationTypeCount {
    pub violation_type: String,
    pub count: i64,
}

// ============================================================================
// STORY 67.4: CONTENT MODERATION DASHBOARD
// ============================================================================

/// Content moderation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "moderation_status", rename_all = "snake_case")]
pub enum ModerationStatus {
    #[default]
    Pending,
    UnderReview,
    Approved,
    Removed,
    Restricted,
    Warned,
    Appealed,
    AppealApproved,
    AppealRejected,
}

/// Content type being moderated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "moderated_content_type", rename_all = "snake_case")]
pub enum ModeratedContentType {
    Listing,
    ListingPhoto,
    UserProfile,
    Review,
    Comment,
    Message,
    Announcement,
    Document,
    CommunityPost,
}

/// Violation type categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "violation_type", rename_all = "snake_case")]
pub enum ViolationType {
    Spam,
    Harassment,
    HateSpeech,
    Violence,
    IllegalContent,
    Misinformation,
    Fraud,
    Privacy,
    IntellectualProperty,
    InappropriateContent,
    Other,
}

/// Moderation action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "moderation_action_type", rename_all = "snake_case")]
pub enum ModerationActionType {
    Remove,
    Restrict,
    Warn,
    Approve,
    Ignore,
    Escalate,
}

/// Report source (who reported the content).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "report_source", rename_all = "snake_case")]
pub enum ReportSource {
    User,
    Automated,
    Staff,
    External,
}

impl ReportSource {
    /// Stable snake_case name. Use instead of `{:?}` in audit/log records.
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportSource::User => "user",
            ReportSource::Automated => "automated",
            ReportSource::Staff => "staff",
            ReportSource::External => "external",
        }
    }
}

impl std::fmt::Display for ReportSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A content moderation case.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ModerationCase {
    /// Case ID
    pub id: Uuid,
    /// Content type
    pub content_type: ModeratedContentType,
    /// Content ID (reference to actual content)
    pub content_id: Uuid,
    /// Content preview/excerpt
    pub content_preview: Option<String>,
    /// Content owner user ID
    pub content_owner_id: Uuid,
    /// Organization ID (if applicable)
    pub organization_id: Option<Uuid>,
    /// How the content was flagged
    pub report_source: ReportSource,
    /// User who reported (if user report)
    pub reported_by: Option<Uuid>,
    /// Automated detection confidence (0-100)
    pub automated_confidence: Option<i32>,
    /// Suspected violation type
    pub violation_type: Option<ViolationType>,
    /// Report reason/description
    pub report_reason: Option<String>,
    /// Current status
    pub status: ModerationStatus,
    /// Priority (1-5, 1 being highest)
    pub priority: i32,
    /// Assigned moderator
    pub assigned_to: Option<Uuid>,
    /// When assigned
    pub assigned_at: Option<DateTime<Utc>>,
    /// Decision made
    pub decision: Option<ModerationActionType>,
    /// Decision rationale
    pub decision_rationale: Option<String>,
    /// Action template used (if any)
    pub action_template_id: Option<Uuid>,
    /// Moderator who made decision
    pub decided_by: Option<Uuid>,
    /// When decision was made
    pub decided_at: Option<DateTime<Utc>>,
    /// Appeal filed
    pub appeal_filed: bool,
    /// Appeal reason
    pub appeal_reason: Option<String>,
    /// Appeal filed at
    pub appeal_filed_at: Option<DateTime<Utc>>,
    /// Appeal decision
    pub appeal_decision: Option<String>,
    /// Appeal decided by
    pub appeal_decided_by: Option<Uuid>,
    /// Appeal decided at
    pub appeal_decided_at: Option<DateTime<Utc>>,
    /// When case was created
    pub created_at: DateTime<Utc>,
    /// When case was last updated
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a moderation case (user report).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModerationCase {
    /// Content type
    pub content_type: ModeratedContentType,
    /// Content ID
    pub content_id: Uuid,
    /// Suspected violation type
    pub violation_type: Option<ViolationType>,
    /// Report reason
    pub report_reason: Option<String>,
}

/// Data for taking moderation action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeModerationAction {
    /// Action to take
    pub action: ModerationActionType,
    /// Decision rationale
    pub rationale: String,
    /// Action template ID (if using template)
    pub template_id: Option<Uuid>,
}

/// Moderation case list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationCaseResponse {
    /// Case ID
    pub id: Uuid,
    /// Content type
    pub content_type: ModeratedContentType,
    /// Content ID
    pub content_id: Uuid,
    /// Content preview
    pub content_preview: Option<String>,
    /// Content owner info
    pub content_owner: ContentOwnerInfo,
    /// Report source
    pub report_source: ReportSource,
    /// Violation type
    pub violation_type: Option<ViolationType>,
    /// Report reason
    pub report_reason: Option<String>,
    /// Status
    pub status: ModerationStatus,
    /// Priority
    pub priority: i32,
    /// Assigned moderator name
    pub assigned_to_name: Option<String>,
    /// Decision
    pub decision: Option<ModerationActionType>,
    /// Appeal status
    pub appeal_filed: bool,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Time since creation (for SLA tracking)
    pub age_hours: f64,
}

/// Content owner information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentOwnerInfo {
    /// User ID
    pub user_id: Uuid,
    /// User name
    pub name: String,
    /// Previous violations count
    pub previous_violations: i32,
}

/// Moderation action template.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ModerationActionTemplate {
    /// Template ID
    pub id: Uuid,
    /// Template name
    pub name: String,
    /// Violation type this applies to
    pub violation_type: ViolationType,
    /// Action type
    pub action_type: ModerationActionType,
    /// Template rationale text
    pub rationale_template: String,
    /// Whether to notify content owner
    pub notify_owner: bool,
    /// Notification template
    pub notification_template: Option<String>,
    /// Is active
    pub is_active: bool,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Moderation queue query parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationQueueQuery {
    /// Filter by status
    pub status: Option<ModerationStatus>,
    /// Filter by content type
    pub content_type: Option<ModeratedContentType>,
    /// Filter by violation type
    pub violation_type: Option<ViolationType>,
    /// Filter by priority
    pub priority: Option<i32>,
    /// Filter by assigned moderator
    pub assigned_to: Option<Uuid>,
    /// Show only unassigned
    pub unassigned_only: Option<bool>,
    /// Sort by (created_at, priority, age)
    pub sort_by: Option<String>,
    /// Sort order (asc, desc)
    pub sort_order: Option<String>,
    /// Page limit
    pub limit: Option<i64>,
    /// Page offset
    pub offset: Option<i64>,
}

/// Moderation queue statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationQueueStats {
    /// Total pending cases
    pub pending_count: i64,
    /// Cases under review
    pub under_review_count: i64,
    /// Cases by priority
    pub by_priority: Vec<PriorityCount>,
    /// Cases by violation type
    pub by_violation_type: Vec<ViolationTypeCount>,
    /// Average resolution time (hours)
    pub avg_resolution_time_hours: f64,
    /// Cases older than SLA (24 hours)
    pub overdue_count: i64,
}

/// Count by priority level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityCount {
    pub priority: i32,
    pub count: i64,
}

/// Appeal request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAppeal {
    /// Appeal reason
    pub reason: String,
}

/// Appeal decision request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecideAppeal {
    /// Decision (upheld or rejected)
    pub decision: String,
    /// Decision rationale
    pub rationale: String,
}

// ============================================================================
// SUSPICIOUS ACTIVITY DETECTION
// ============================================================================

/// Suspicious activity record for AML monitoring.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SuspiciousActivity {
    /// Record ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Related party ID
    pub party_id: Uuid,
    /// Activity type
    pub activity_type: String,
    /// Activity description
    pub description: String,
    /// Detection method (automated/manual)
    pub detection_method: String,
    /// Risk indicators (JSON)
    pub risk_indicators: Option<serde_json::Value>,
    /// Transaction patterns (JSON)
    pub transaction_patterns: Option<serde_json::Value>,
    /// Amount involved (if monetary)
    pub amount_cents: Option<i64>,
    /// Currency
    pub currency: Option<String>,
    /// Whether reported to authorities
    pub reported_to_authorities: bool,
    /// When reported
    pub reported_at: Option<DateTime<Utc>>,
    /// SAR reference number (if filed)
    pub sar_reference: Option<String>,
    /// Investigation status
    pub investigation_status: String,
    /// Investigation notes
    pub investigation_notes: Option<String>,
    /// Investigated by
    pub investigated_by: Option<Uuid>,
    /// When detected
    pub detected_at: DateTime<Utc>,
    /// When record was created
    pub created_at: DateTime<Utc>,
    /// When record was last updated
    pub updated_at: DateTime<Utc>,
}

/// AML threshold configuration.
pub const AML_THRESHOLD_EUR_CENTS: i64 = 1_000_000; // 10,000 EUR in cents

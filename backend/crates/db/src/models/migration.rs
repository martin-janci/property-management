//! Platform Migration & Data Import models (Epic 66).
//!
//! Types for import templates, bulk import, data export, and validation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// STORY 66.1: Import Templates
// ============================================================================

/// Type of data being imported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "import_data_type", rename_all = "snake_case")]
// serde casing must match the wire contract (frontend `ImportDataType` is
// lowercase, e.g. "buildings") — without this the HTTP body/query would
// (de)serialize as PascalCase and reject valid client payloads with 422.
#[serde(rename_all = "snake_case")]
pub enum ImportDataType {
    Buildings,
    Units,
    Residents,
    Financials,
    Faults,
    Documents,
    Meters,
    Votes,
    Custom,
}

impl std::fmt::Display for ImportDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buildings => write!(f, "buildings"),
            Self::Units => write!(f, "units"),
            Self::Residents => write!(f, "residents"),
            Self::Financials => write!(f, "financials"),
            Self::Faults => write!(f, "faults"),
            Self::Documents => write!(f, "documents"),
            Self::Meters => write!(f, "meters"),
            Self::Votes => write!(f, "votes"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

/// Field data type for validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldDataType {
    String,
    Integer,
    Decimal,
    Boolean,
    Date,
    DateTime,
    Email,
    Phone,
    Uuid,
    Enum,
    Json,
}

/// Validation rule for a field.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct FieldValidation {
    /// Field is required
    #[serde(default)]
    pub required: bool,
    /// Minimum length (for strings)
    #[serde(default)]
    pub min_length: Option<usize>,
    /// Maximum length (for strings)
    #[serde(default)]
    pub max_length: Option<usize>,
    /// Minimum value (for numbers)
    #[serde(default)]
    pub min_value: Option<f64>,
    /// Maximum value (for numbers)
    #[serde(default)]
    pub max_value: Option<f64>,
    /// Regex pattern to match
    #[serde(default)]
    pub pattern: Option<String>,
    /// Allowed values for enum type
    #[serde(default)]
    pub allowed_values: Option<Vec<String>>,
    /// Custom validation message
    #[serde(default)]
    pub message: Option<String>,
}

/// A field mapping in an import template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportFieldMapping {
    /// Field name in the system
    pub field_name: String,
    /// Display label for the field
    pub display_label: String,
    /// Column header in CSV/Excel
    pub column_header: String,
    /// Data type of the field
    pub data_type: FieldDataType,
    /// Validation rules
    pub validation: FieldValidation,
    /// Example value for the template
    pub example_value: Option<String>,
    /// Description/help text
    pub description: Option<String>,
    /// Target table column (for database mapping)
    pub target_column: Option<String>,
    /// Transformation function name (e.g., "parse_date", "normalize_phone")
    pub transformation: Option<String>,
}

/// An import template definition.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ImportTemplate {
    /// Template ID
    pub id: Uuid,
    /// Organization that created the template (null for system templates)
    pub organization_id: Option<Uuid>,
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Data type this template imports
    pub data_type: ImportDataType,
    /// Field mappings (JSON)
    pub field_mappings: serde_json::Value,
    /// Whether this is a system-provided template
    pub is_system_template: bool,
    /// Template version
    pub version: i32,
    /// Whether the template is active
    pub is_active: bool,
    /// Created by user
    pub created_by: Option<Uuid>,
    /// When the template was created
    pub created_at: DateTime<Utc>,
    /// When the template was last updated
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new import template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateImportTemplate {
    /// Organization ID (null for system templates)
    pub organization_id: Option<Uuid>,
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Data type this template imports
    pub data_type: ImportDataType,
    /// Field mappings
    pub field_mappings: Vec<ImportFieldMapping>,
    /// Created by user
    pub created_by: Option<Uuid>,
}

/// Request to update an import template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateImportTemplate {
    /// Template name
    pub name: Option<String>,
    /// Template description
    pub description: Option<String>,
    /// Field mappings
    pub field_mappings: Option<Vec<ImportFieldMapping>>,
    /// Whether the template is active
    pub is_active: Option<bool>,
}

/// Response for listing templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplateListResponse {
    /// Templates
    pub templates: Vec<ImportTemplateSummary>,
    /// Total count
    pub total: i64,
}

/// Summary of an import template for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplateSummary {
    /// Template ID
    pub id: Uuid,
    /// Template name
    pub name: String,
    /// Data type
    pub data_type: ImportDataType,
    /// Description
    pub description: Option<String>,
    /// Whether system template
    pub is_system_template: bool,
    /// Number of fields
    pub field_count: usize,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

/// Template download format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TemplateFormat {
    #[default]
    Csv,
    Xlsx,
}

// ============================================================================
// STORY 66.2: Bulk Data Import
// ============================================================================

/// Import job status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "import_job_status", rename_all = "snake_case")]
pub enum ImportJobStatus {
    Pending,
    Validating,
    Validated,
    ValidationFailed,
    Importing,
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ImportJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Validating => write!(f, "validating"),
            Self::Validated => write!(f, "validated"),
            Self::ValidationFailed => write!(f, "validation_failed"),
            Self::Importing => write!(f, "importing"),
            Self::Completed => write!(f, "completed"),
            Self::PartiallyCompleted => write!(f, "partially_completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// An import job record.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ImportJob {
    /// Job ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Template used for import
    pub template_id: Uuid,
    /// Job status
    pub status: ImportJobStatus,
    /// Original filename
    pub original_filename: String,
    /// File path in storage
    pub file_path: String,
    /// File size in bytes
    pub file_size_bytes: i64,
    /// Total rows in the file
    pub total_rows: Option<i32>,
    /// Rows processed so far
    pub processed_rows: i32,
    /// Successfully imported rows
    pub successful_rows: i32,
    /// Failed rows
    pub failed_rows: i32,
    /// Skipped rows (e.g., duplicates)
    pub skipped_rows: i32,
    /// Validation errors (JSON array)
    pub validation_errors: Option<serde_json::Value>,
    /// Import errors (JSON array)
    pub import_errors: Option<serde_json::Value>,
    /// Import options (e.g., skip duplicates, update existing)
    pub options: Option<serde_json::Value>,
    /// User who initiated the import
    pub created_by: Uuid,
    /// When import started
    pub started_at: Option<DateTime<Utc>>,
    /// When import completed
    pub completed_at: Option<DateTime<Utc>>,
    /// When record was created
    pub created_at: DateTime<Utc>,
    /// When record was last updated
    pub updated_at: DateTime<Utc>,
}

/// Options for import behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportOptions {
    /// Skip rows with validation errors instead of failing
    pub skip_errors: bool,
    /// Update existing records if found (by key field)
    pub update_existing: bool,
    /// Key field for duplicate detection
    pub key_field: Option<String>,
    /// Dry run - validate only, don't import
    pub dry_run: bool,
    /// Batch size for processing
    pub batch_size: Option<i32>,
    /// Continue from a specific row (for resumable imports)
    pub start_row: Option<i32>,
}

/// Request to start a new import job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateImportJob {
    /// Organization ID
    pub organization_id: Uuid,
    /// Template ID to use
    pub template_id: Uuid,
    /// Original filename
    pub original_filename: String,
    /// File path in storage
    pub file_path: String,
    /// File size in bytes
    pub file_size_bytes: i64,
    /// Import options
    pub options: Option<ImportOptions>,
    /// User starting the import
    pub created_by: Uuid,
}

/// Response for import job status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportJobStatusResponse {
    /// Job ID
    pub id: Uuid,
    /// Current status
    pub status: ImportJobStatus,
    /// Original filename
    pub filename: String,
    /// Template name
    pub template_name: String,
    /// Progress percentage (0-100)
    pub progress_percent: i32,
    /// Total rows
    pub total_rows: Option<i32>,
    /// Rows processed
    pub processed_rows: i32,
    /// Successful rows
    pub successful_rows: i32,
    /// Failed rows
    pub failed_rows: i32,
    /// Skipped rows
    pub skipped_rows: i32,
    /// Error summary (first few errors)
    pub error_summary: Option<Vec<ImportRowError>>,
    /// When import started
    pub started_at: Option<DateTime<Utc>>,
    /// When import completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Estimated time remaining (seconds)
    pub estimated_remaining_seconds: Option<i32>,
}

/// An error for a specific row.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ImportRowError {
    /// Row number (1-indexed)
    pub row_number: i32,
    /// Column name with error
    pub column: Option<String>,
    /// Error message
    pub message: String,
    /// Error code for categorization
    pub error_code: String,
    /// Original value that caused the error
    pub original_value: Option<String>,
}

/// Import job history entry.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ImportJobHistory {
    /// Job ID
    pub id: Uuid,
    /// Status
    pub status: ImportJobStatus,
    /// Filename
    pub filename: String,
    /// Data type imported
    pub data_type: ImportDataType,
    /// Records imported
    pub records_imported: i32,
    /// Records failed
    pub records_failed: i32,
    /// Who ran the import
    pub created_by_name: String,
    /// When import was created
    pub created_at: DateTime<Utc>,
    /// When import completed
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// STORY 66.3: Data Export for Migration
// ============================================================================

/// Migration export status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "migration_export_status", rename_all = "snake_case")]
pub enum MigrationExportStatus {
    Pending,
    Processing,
    Ready,
    Downloaded,
    Expired,
    Failed,
}

/// Data categories available for export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportDataCategory {
    Buildings,
    Units,
    Residents,
    Financials,
    Faults,
    Documents,
    Votes,
    Announcements,
    Meters,
    Leases,
    Vendors,
    WorkOrders,
}

impl std::fmt::Display for ExportDataCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buildings => write!(f, "buildings"),
            Self::Units => write!(f, "units"),
            Self::Residents => write!(f, "residents"),
            Self::Financials => write!(f, "financials"),
            Self::Faults => write!(f, "faults"),
            Self::Documents => write!(f, "documents"),
            Self::Votes => write!(f, "votes"),
            Self::Announcements => write!(f, "announcements"),
            Self::Meters => write!(f, "meters"),
            Self::Leases => write!(f, "leases"),
            Self::Vendors => write!(f, "vendors"),
            Self::WorkOrders => write!(f, "work_orders"),
        }
    }
}

/// Privacy/anonymization options for export.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExportPrivacyOptions {
    /// Anonymize personal data (names, emails, phones)
    pub anonymize_personal_data: bool,
    /// Remove financial account numbers
    pub mask_financial_data: bool,
    /// Remove document contents (export metadata only)
    pub exclude_document_contents: bool,
    /// Hash identifiers instead of using real IDs
    pub hash_identifiers: bool,
}

/// A migration export request.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MigrationExport {
    /// Export ID
    pub id: Uuid,
    /// Organization ID
    pub organization_id: Uuid,
    /// Current status
    pub status: MigrationExportStatus,
    /// Categories to include
    pub categories: serde_json::Value,
    /// Privacy options
    pub privacy_options: serde_json::Value,
    /// Output format (zip with CSVs)
    pub output_format: String,
    /// Path to export file
    pub file_path: Option<String>,
    /// File size in bytes
    pub file_size_bytes: Option<i64>,
    /// SHA-256 hash of file
    pub file_hash: Option<String>,
    /// Secure download token
    pub download_token: Option<Uuid>,
    /// Number of times downloaded
    pub download_count: i32,
    /// When file was downloaded
    pub downloaded_at: Option<DateTime<Utc>>,
    /// When download expires
    pub expires_at: DateTime<Utc>,
    /// When processing started
    pub started_at: Option<DateTime<Utc>>,
    /// When processing completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// User who requested the export
    pub created_by: Uuid,
    /// When request was created
    pub created_at: DateTime<Utc>,
    /// When record was last updated
    pub updated_at: DateTime<Utc>,
}

/// Request to create a migration export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMigrationExport {
    /// Organization ID
    pub organization_id: Uuid,
    /// Categories to include
    pub categories: Vec<ExportDataCategory>,
    /// Privacy options
    pub privacy_options: ExportPrivacyOptions,
    /// User requesting the export
    pub created_by: Uuid,
}

/// Response for migration export request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationExportResponse {
    /// Export ID for tracking
    pub export_id: Uuid,
    /// Current status
    pub status: MigrationExportStatus,
    /// Estimated time (human readable)
    pub estimated_time: String,
    /// Categories being exported
    pub categories: Vec<ExportDataCategory>,
}

/// Response for migration export status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationExportStatusResponse {
    /// Export ID
    pub export_id: Uuid,
    /// Current status
    pub status: MigrationExportStatus,
    /// Categories exported
    pub categories: Vec<String>,
    /// Download URL (if ready)
    pub download_url: Option<String>,
    /// File size in bytes (if ready)
    pub file_size_bytes: Option<i64>,
    /// When download expires
    pub expires_at: DateTime<Utc>,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Record counts by category
    pub record_counts: Option<serde_json::Value>,
}

/// Exported data schema metadata (included in export ZIP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSchemaMetadata {
    /// Schema version
    pub schema_version: String,
    /// Export timestamp
    pub exported_at: DateTime<Utc>,
    /// Platform version
    pub platform_version: String,
    /// Organization ID
    pub organization_id: Uuid,
    /// Organization name
    pub organization_name: String,
    /// Categories included
    pub categories: Vec<String>,
    /// Privacy options used
    pub privacy_options: ExportPrivacyOptions,
    /// File manifest
    pub files: Vec<ExportFileEntry>,
}

/// Entry in the export file manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFileEntry {
    /// Filename in the ZIP
    pub filename: String,
    /// Data category
    pub category: String,
    /// Number of records
    pub record_count: i64,
    /// File size in bytes
    pub size_bytes: i64,
    /// Column definitions
    pub columns: Vec<ExportColumnDefinition>,
}

/// Column definition for export schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportColumnDefinition {
    /// Column name
    pub name: String,
    /// Data type
    pub data_type: String,
    /// Whether nullable
    pub nullable: bool,
    /// Description
    pub description: Option<String>,
    /// Foreign key reference (if any)
    pub foreign_key: Option<String>,
}

// ============================================================================
// STORY 66.4: Import Validation & Preview
// ============================================================================

/// Validation result severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// A validation issue found during import preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Row number (1-indexed, null for file-level issues)
    pub row_number: Option<i32>,
    /// Column name
    pub column: Option<String>,
    /// Severity
    pub severity: ValidationSeverity,
    /// Issue code
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Original value
    pub original_value: Option<String>,
    /// Suggested fix
    pub suggested_value: Option<String>,
}

/// Duplicate detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateRecord {
    /// Row number in import file
    pub import_row: i32,
    /// Matching existing record ID
    pub existing_id: Uuid,
    /// Key fields that matched
    pub matched_fields: Vec<String>,
    /// Confidence score (0-100)
    pub confidence: i32,
    /// Comparison of differing fields
    pub differences: Vec<FieldDifference>,
}

/// Difference between import and existing record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDifference {
    /// Field name
    pub field: String,
    /// Value in import file
    pub import_value: Option<String>,
    /// Value in existing record
    pub existing_value: Option<String>,
}

/// Import preview/validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewResult {
    /// Job ID
    pub job_id: Uuid,
    /// Whether validation passed (no errors, only warnings/info)
    pub is_valid: bool,
    /// Total rows in file
    pub total_rows: i32,
    /// Rows that will be imported
    pub importable_rows: i32,
    /// Rows with errors
    pub error_rows: i32,
    /// Rows with warnings
    pub warning_rows: i32,
    /// Count by record type being created
    pub record_counts: RecordTypeCounts,
    /// Validation issues (limited to first N)
    pub issues: Vec<ValidationIssue>,
    /// Total issue count (may exceed returned issues)
    pub total_issue_count: i32,
    /// Detected duplicates
    pub duplicates: Vec<DuplicateRecord>,
    /// Sample records (first 10)
    pub sample_records: Vec<serde_json::Value>,
    /// Column mapping summary
    pub column_mapping: Vec<ColumnMappingStatus>,
}

/// Count of records by type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordTypeCounts {
    /// New records to create
    pub new_records: i32,
    /// Existing records to update
    pub updates: i32,
    /// Records to skip
    pub skipped: i32,
}

/// Status of a column mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMappingStatus {
    /// Column header in file
    pub source_column: String,
    /// Target field in system
    pub target_field: Option<String>,
    /// Whether mapping was found
    pub is_mapped: bool,
    /// Whether field is required
    pub is_required: bool,
    /// Sample values from file
    pub sample_values: Vec<String>,
}

/// Request to approve and execute import after preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveImportRequest {
    /// Job ID
    pub job_id: Uuid,
    /// Override options (optional, to change from preview)
    pub options: Option<ImportOptions>,
    /// Acknowledge warnings
    pub acknowledge_warnings: bool,
}

/// Import approval response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveImportResponse {
    /// Job ID
    pub job_id: Uuid,
    /// New status
    pub status: ImportJobStatus,
    /// Message
    pub message: String,
    /// Estimated completion time (seconds)
    pub estimated_seconds: Option<i32>,
}

// ============================================================================
// Common Types
// ============================================================================

/// Pagination parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationPagination {
    /// Page number (1-indexed)
    pub page: Option<i32>,
    /// Items per page
    pub per_page: Option<i32>,
}

/// Filter parameters for import job list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportJobFilter {
    /// Filter by status
    pub status: Option<ImportJobStatus>,
    /// Filter by data type
    pub data_type: Option<ImportDataType>,
    /// Filter by date range start
    pub from_date: Option<DateTime<Utc>>,
    /// Filter by date range end
    pub to_date: Option<DateTime<Utc>>,
}

/// Available import categories response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportCategoriesResponse {
    /// Available categories
    pub categories: Vec<ImportCategoryInfo>,
}

/// Information about an import category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportCategoryInfo {
    /// Category identifier
    pub id: ImportDataType,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether a system template exists
    pub has_system_template: bool,
    /// Dependencies (must import first)
    pub dependencies: Vec<ImportDataType>,
}

/// Available export categories response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCategoriesResponse {
    /// Available categories
    pub categories: Vec<ExportCategoryInfo>,
}

/// Information about an export category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCategoryInfo {
    /// Category identifier
    pub id: ExportDataCategory,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Approximate record count for the organization
    pub record_count: i64,
    /// Whether category contains personal data
    pub contains_personal_data: bool,
}

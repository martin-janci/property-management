//! Form management models for Epic 54.
//!
//! Supports form templates, field definitions, submissions, and digital signatures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Status Constants
// ============================================================================

/// Form status values.
pub mod form_status {
    pub const DRAFT: &str = "draft";
    pub const PUBLISHED: &str = "published";
    pub const ARCHIVED: &str = "archived";
}

/// Form field type values.
pub mod field_type {
    pub const TEXT: &str = "text";
    pub const TEXTAREA: &str = "textarea";
    pub const NUMBER: &str = "number";
    pub const EMAIL: &str = "email";
    pub const PHONE: &str = "phone";
    pub const DATE: &str = "date";
    pub const DATETIME: &str = "datetime";
    pub const CHECKBOX: &str = "checkbox";
    pub const RADIO: &str = "radio";
    pub const SELECT: &str = "select";
    pub const MULTISELECT: &str = "multiselect";
    pub const FILE: &str = "file";
    pub const SIGNATURE: &str = "signature";

    /// Every value of the `form_field_type` Postgres enum (migration 00066).
    pub const ALL: &[&str] = &[
        TEXT,
        TEXTAREA,
        NUMBER,
        EMAIL,
        PHONE,
        DATE,
        DATETIME,
        CHECKBOX,
        RADIO,
        SELECT,
        MULTISELECT,
        FILE,
        SIGNATURE,
    ];

    /// Whether `value` is a valid `form_field_type`. Handlers check this so an
    /// unknown type is a 400, not a 500 from the enum cast.
    pub fn is_valid(value: &str) -> bool {
        ALL.contains(&value)
    }
}

/// Form submission status values.
pub mod submission_status {
    pub const PENDING: &str = "pending";
    pub const REVIEWED: &str = "reviewed";
    pub const APPROVED: &str = "approved";
    pub const REJECTED: &str = "rejected";
}

/// Form target type values.
pub mod target_type {
    pub const ALL: &str = "all";
    pub const BUILDING: &str = "building";
    pub const UNITS: &str = "units";
    pub const ROLES: &str = "roles";
}

// ============================================================================
// Form Entity
// ============================================================================

/// A form template that can be filled out by users.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct Form {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub status: String,
    pub version: i32,
    pub target_type: Option<String>,
    pub target_ids: serde_json::Value,
    pub require_signatures: bool,
    pub allow_multiple_submissions: bool,
    pub submission_deadline: Option<DateTime<Utc>>,
    pub confirmation_message: Option<String>,
    pub pdf_template_settings: serde_json::Value,
    pub created_by: Uuid,
    pub updated_by: Option<Uuid>,
    pub published_by: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Summary view of a form for list responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormSummary {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub status: String,
    pub target_type: Option<String>,
    pub require_signatures: bool,
    pub submission_deadline: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub submission_count: i64,
    pub created_by_name: Option<String>,
}

/// Form with all related data for detail view.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormWithDetails {
    #[serde(flatten)]
    pub form: Form,
    pub fields: Vec<FormField>,
    pub created_by_name: Option<String>,
    pub published_by_name: Option<String>,
    pub submission_count: i64,
}

// ============================================================================
// Form Field Entity
// ============================================================================

/// A field definition within a form.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FormField {
    pub id: Uuid,
    pub form_id: Uuid,
    pub field_key: String,
    pub label: String,
    pub field_type: String,
    pub required: bool,
    pub help_text: Option<String>,
    pub placeholder: Option<String>,
    pub default_value: Option<String>,
    pub validation_rules: serde_json::Value,
    pub options: serde_json::Value,
    pub field_order: i32,
    pub width: String,
    pub section: Option<String>,
    pub conditional_display: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Field option for select/radio/checkbox fields.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

/// Validation rules for a form field.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ValidationRules {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_message: Option<String>,
}

/// Conditional display configuration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConditionalDisplay {
    pub field: String,
    pub operator: String, // "equals", "not_equals", "contains", "not_empty"
    pub value: serde_json::Value,
}

// ============================================================================
// Form Submission Entity
// ============================================================================

/// A submitted form response.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FormSubmission {
    pub id: Uuid,
    pub form_id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub submitted_by: Uuid,
    pub submitted_at: DateTime<Utc>,
    pub data: serde_json::Value,
    pub attachments: serde_json::Value,
    pub signature_data: Option<serde_json::Value>,
    pub status: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub review_notes: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary view of a form submission.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormSubmissionSummary {
    pub id: Uuid,
    pub form_id: Uuid,
    pub form_title: String,
    pub submitted_by: Uuid,
    pub submitted_by_name: String,
    pub submitted_at: DateTime<Utc>,
    pub status: String,
    pub has_signature: bool,
    pub unit_number: Option<String>,
    pub building_name: Option<String>,
}

/// Form submission with full details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormSubmissionWithDetails {
    #[serde(flatten)]
    pub submission: FormSubmission,
    pub form_title: String,
    pub submitted_by_name: String,
    pub reviewed_by_name: Option<String>,
    pub unit_number: Option<String>,
    pub building_name: Option<String>,
}

/// Digital signature data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignatureData {
    pub signature_image: String, // Base64 encoded image
    pub signed_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

// ============================================================================
// Form Download Entity
// ============================================================================

/// Tracks form PDF downloads.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct FormDownload {
    pub id: Uuid,
    pub form_id: Uuid,
    pub downloaded_by: Uuid,
    pub downloaded_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

// ============================================================================
// Create/Update DTOs
// ============================================================================

/// Request to create a new form.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateForm {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub building_id: Option<Uuid>,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_ids: Option<Vec<Uuid>>,
    #[serde(default)]
    pub require_signatures: bool,
    #[serde(default)]
    pub allow_multiple_submissions: bool,
    #[serde(default)]
    pub submission_deadline: Option<DateTime<Utc>>,
    #[serde(default)]
    pub confirmation_message: Option<String>,
    #[serde(default)]
    pub fields: Vec<CreateFormField>,
}

/// Request to update an existing form.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateForm {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub building_id: Option<Uuid>,
    #[serde(default)]
    pub target_type: Option<String>,
    #[serde(default)]
    pub target_ids: Option<Vec<Uuid>>,
    #[serde(default)]
    pub require_signatures: Option<bool>,
    #[serde(default)]
    pub allow_multiple_submissions: Option<bool>,
    #[serde(default)]
    pub submission_deadline: Option<DateTime<Utc>>,
    #[serde(default)]
    pub confirmation_message: Option<String>,
}

/// Request to create a form field.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFormField {
    pub field_key: String,
    pub label: String,
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub help_text: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub validation_rules: Option<ValidationRules>,
    #[serde(default)]
    pub options: Option<Vec<FieldOption>>,
    #[serde(default)]
    pub field_order: i32,
    #[serde(default = "default_width")]
    pub width: String,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub conditional_display: Option<ConditionalDisplay>,
}

fn default_width() -> String {
    "full".to_string()
}

/// Request to update a form field.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateFormField {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub field_type: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub help_text: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub validation_rules: Option<ValidationRules>,
    #[serde(default)]
    pub options: Option<Vec<FieldOption>>,
    #[serde(default)]
    pub field_order: Option<i32>,
    #[serde(default)]
    pub width: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub conditional_display: Option<ConditionalDisplay>,
}

/// Request to submit a form.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitForm {
    pub data: serde_json::Value,
    #[serde(default)]
    pub attachments: Option<Vec<FormAttachment>>,
    #[serde(default)]
    pub signature_data: Option<SignatureData>,
}

/// Parameters for submitting a form (used internally by repository).
#[derive(Debug, Clone)]
pub struct FormSubmissionParams {
    pub org_id: Uuid,
    pub form_id: Uuid,
    pub user_id: Uuid,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub data: SubmitForm,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Form attachment reference.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormAttachment {
    pub field_key: String,
    pub file_id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
}

/// Request to review a submission.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewSubmission {
    pub status: String, // "approved" or "rejected"
    #[serde(default)]
    pub review_notes: Option<String>,
}

// ============================================================================
// Query Parameters
// ============================================================================

/// Query parameters for listing forms.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct FormListQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub building_id: Option<Uuid>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_order: Option<String>,
}

/// Query parameters for listing submissions.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct SubmissionListQuery {
    #[serde(default)]
    pub form_id: Option<Uuid>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub building_id: Option<Uuid>,
    #[serde(default)]
    pub unit_id: Option<Uuid>,
    #[serde(default)]
    pub submitted_by: Option<Uuid>,
    #[serde(default)]
    pub from_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub page: Option<i64>,
    #[serde(default)]
    pub per_page: Option<i64>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for form list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormListResponse {
    pub forms: Vec<FormSummary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Response for submission list endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmissionListResponse {
    pub submissions: Vec<FormSubmissionSummary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Response for creating a form.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFormResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for submitting a form.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitFormResponse {
    pub id: Uuid,
    pub message: String,
    pub confirmation_message: Option<String>,
}

/// Form statistics.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormStatistics {
    pub total_forms: i64,
    pub published_forms: i64,
    pub draft_forms: i64,
    pub archived_forms: i64,
    pub total_submissions: i64,
    pub pending_submissions: i64,
    pub approved_submissions: i64,
    pub rejected_submissions: i64,
}

/// Export format options.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Excel,
    Pdf,
}

/// Request to export submissions.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExportSubmissionsRequest {
    pub format: ExportFormat,
    #[serde(default)]
    pub from_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub to_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub status: Option<String>,
}

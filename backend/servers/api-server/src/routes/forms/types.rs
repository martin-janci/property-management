//! Request/response/query types, constants, and validation helpers for the
//! forms routes.

use axum::{http::StatusCode, Json};
use common::errors::ErrorResponse;
use db::models::{
    Form, FormField, FormStatistics, FormSubmissionWithDetails, FormSummary, FormWithDetails,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed title length (characters).
pub(super) const MAX_TITLE_LENGTH: usize = 500;

/// Maximum allowed description length (characters).
pub(super) const MAX_DESCRIPTION_LENGTH: usize = 5000;

/// Maximum number of fields per form.
pub(super) const MAX_FIELDS_PER_FORM: usize = 100;

/// Maximum signature image size (1MB in bytes).
pub(super) const MAX_SIGNATURE_SIZE: usize = 1024 * 1024;

// ============================================================================
// Helper Functions
// ============================================================================

/// Validates base64-encoded signature image data.
/// Returns error if invalid format, too large, or unsupported image type.
pub(super) fn validate_signature_image(
    signature_b64: &str,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    // Check size (base64 is ~33% larger than binary, so divide by 0.75)
    let estimated_size = (signature_b64.len() * 3) / 4;
    if estimated_size > MAX_SIGNATURE_SIZE {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "SIGNATURE_TOO_LARGE",
                format!(
                    "Signature image must be less than {}KB",
                    MAX_SIGNATURE_SIZE / 1024
                ),
            )),
        ));
    }

    // Validate base64 format and decode
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature_b64)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_BASE64",
                    "Signature image must be valid base64-encoded data",
                )),
            )
        })?;

    // Check if it's a valid image by checking magic bytes
    let is_png = decoded.starts_with(b"\x89PNG\r\n\x1a\n");
    let is_jpeg = decoded.starts_with(&[0xFF, 0xD8, 0xFF]);
    let is_webp = decoded.len() > 12 && &decoded[0..4] == b"RIFF" && &decoded[8..12] == b"WEBP";

    if !is_png && !is_jpeg && !is_webp {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_IMAGE_TYPE",
                "Signature image must be PNG, JPEG, or WebP format",
            )),
        ));
    }

    Ok(())
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for form detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FormDetailResponse {
    pub form: FormWithDetails,
}

/// Response for form action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FormActionResponse {
    pub message: String,
    pub form: Form,
}

/// Response for form statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FormStatisticsResponse {
    pub statistics: FormStatistics,
}

/// Response for form fields.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldsResponse {
    pub fields: Vec<FormField>,
}

/// Response for form field action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldActionResponse {
    pub message: String,
    pub field: FormField,
}

/// Response for submission detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmissionDetailResponse {
    pub submission: FormSubmissionWithDetails,
}

/// Response for available forms (for users).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AvailableFormsResponse {
    pub forms: Vec<FormSummary>,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating a form.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateFormRequest {
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
    pub submission_deadline: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub confirmation_message: Option<String>,
    #[serde(default)]
    pub fields: Vec<CreateFormFieldRequest>,
}

/// Request for creating a form field.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateFormFieldRequest {
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
    pub validation_rules: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Option<Vec<FieldOptionRequest>>,
    #[serde(default)]
    pub field_order: i32,
    #[serde(default = "default_width")]
    pub width: String,
    #[serde(default)]
    pub section: Option<String>,
}

fn default_width() -> String {
    "full".to_string()
}

/// Field option for select/radio fields.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldOptionRequest {
    pub value: String,
    pub label: String,
}

/// Request for updating a form.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateFormRequest {
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
    pub submission_deadline: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub confirmation_message: Option<String>,
}

/// Request for updating a form field.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateFormFieldRequest {
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
    pub validation_rules: Option<serde_json::Value>,
    #[serde(default)]
    pub options: Option<Vec<FieldOptionRequest>>,
    #[serde(default)]
    pub field_order: Option<i32>,
    #[serde(default)]
    pub width: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
}

/// Request for submitting a form.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmitFormRequest {
    pub data: serde_json::Value,
    #[serde(default)]
    pub attachments: Option<Vec<AttachmentRequest>>,
    #[serde(default)]
    pub signature_data: Option<SignatureDataRequest>,
}

/// Attachment reference in submission.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AttachmentRequest {
    pub field_key: String,
    pub file_id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
}

/// Digital signature data.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SignatureDataRequest {
    pub signature_image: String,
}

/// Request for reviewing a submission.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReviewSubmissionRequest {
    pub status: String,
    #[serde(default)]
    pub review_notes: Option<String>,
}

/// Request for reordering fields.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReorderFieldsRequest {
    pub field_orders: Vec<FieldOrderItem>,
}

/// Field order item.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FieldOrderItem {
    pub field_id: Uuid,
    pub order: i32,
}

// ============================================================================
// Query Types
// ============================================================================

/// Query for listing forms.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListFormsQuery {
    pub status: Option<String>,
    pub category: Option<String>,
    pub building_id: Option<Uuid>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

/// Query for listing submissions.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListSubmissionsQuery {
    pub status: Option<String>,
    pub building_id: Option<Uuid>,
    pub unit_id: Option<Uuid>,
    pub submitted_by: Option<Uuid>,
    pub from_date: Option<chrono::DateTime<chrono::Utc>>,
    pub to_date: Option<chrono::DateTime<chrono::Utc>>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

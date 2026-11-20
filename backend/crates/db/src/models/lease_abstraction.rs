//! Lease abstraction models (Epic 133: AI Lease Abstraction & Document Intelligence).
//! Provides automated extraction of key terms from lease documents.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// =============================================================================
// ENUMS
// =============================================================================

pub mod document_status {
    pub const PENDING: &str = "pending";
    pub const PROCESSING: &str = "processing";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const REVIEW_REQUIRED: &str = "review_required";
}

pub mod review_status {
    pub const PENDING: &str = "pending";
    pub const APPROVED: &str = "approved";
    pub const REJECTED: &str = "rejected";
}

pub mod import_status {
    pub const PENDING: &str = "pending";
    pub const IMPORTED: &str = "imported";
    pub const FAILED: &str = "failed";
    pub const CANCELLED: &str = "cancelled";
}

// =============================================================================
// LEASE DOCUMENTS (Story 133.1)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LeaseDocument {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub uploaded_by: Uuid,

    pub file_name: String,
    pub file_size_bytes: i32,
    pub mime_type: String,
    pub storage_path: String,

    pub status: String,
    pub unit_id: Option<Uuid>,

    pub processing_started_at: Option<DateTime<Utc>>,
    pub processing_completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub page_count: Option<i32>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateLeaseDocument {
    pub file_name: String,
    pub file_size_bytes: i32,
    pub mime_type: String,
    pub storage_path: String,
    pub unit_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LeaseDocumentSummary {
    pub id: Uuid,
    pub file_name: String,
    pub status: String,
    pub page_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub has_extraction: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LeaseDocumentQuery {
    pub status: Option<String>,
    pub unit_id: Option<Uuid>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

// =============================================================================
// LEASE EXTRACTIONS (Story 133.2)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LeaseExtraction {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version: i32,

    pub tenant_name: Option<String>,
    pub tenant_name_confidence: Option<Decimal>,
    pub tenant_name_location: Option<JsonValue>,

    pub landlord_name: Option<String>,
    pub landlord_name_confidence: Option<Decimal>,
    pub landlord_name_location: Option<JsonValue>,

    pub property_address: Option<String>,
    pub property_address_confidence: Option<Decimal>,
    pub property_address_location: Option<JsonValue>,

    pub lease_start_date: Option<NaiveDate>,
    pub lease_start_date_confidence: Option<Decimal>,
    pub lease_start_date_location: Option<JsonValue>,

    pub lease_end_date: Option<NaiveDate>,
    pub lease_end_date_confidence: Option<Decimal>,
    pub lease_end_date_location: Option<JsonValue>,

    pub monthly_rent: Option<Decimal>,
    pub monthly_rent_confidence: Option<Decimal>,
    pub monthly_rent_location: Option<JsonValue>,
    pub rent_currency: Option<String>,

    pub security_deposit: Option<Decimal>,
    pub security_deposit_confidence: Option<Decimal>,
    pub security_deposit_location: Option<JsonValue>,

    pub payment_due_day: Option<i32>,
    pub payment_due_day_confidence: Option<Decimal>,
    pub payment_due_day_location: Option<JsonValue>,

    pub special_clauses: Option<JsonValue>,

    pub overall_confidence: Option<Decimal>,
    pub fields_extracted: i32,
    pub fields_flagged: i32,

    pub model_used: Option<String>,
    pub extraction_duration_ms: Option<i32>,

    pub review_status: String,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
}

/// Extracted field with value, confidence, and location
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractedField {
    pub name: String,
    pub value: Option<String>,
    pub confidence: Option<Decimal>,
    pub location: Option<JsonValue>,
    pub needs_review: bool,
}

/// Summary view of an extraction
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractionSummary {
    pub id: Uuid,
    pub document_id: Uuid,
    pub overall_confidence: Option<Decimal>,
    pub fields_extracted: i32,
    pub fields_flagged: i32,
    pub review_status: String,
    pub created_at: DateTime<Utc>,
}

/// Full extraction with all fields expanded for UI
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ExtractionWithFields {
    pub extraction: LeaseExtraction,
    pub fields: Vec<ExtractedField>,
    pub document: LeaseDocumentSummary,
}

/// Request to create a new extraction (AI processing result)
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateLeaseExtraction {
    pub document_id: Uuid,

    pub tenant_name: Option<String>,
    pub tenant_name_confidence: Option<Decimal>,
    pub tenant_name_location: Option<JsonValue>,

    pub landlord_name: Option<String>,
    pub landlord_name_confidence: Option<Decimal>,
    pub landlord_name_location: Option<JsonValue>,

    pub property_address: Option<String>,
    pub property_address_confidence: Option<Decimal>,
    pub property_address_location: Option<JsonValue>,

    pub lease_start_date: Option<NaiveDate>,
    pub lease_start_date_confidence: Option<Decimal>,
    pub lease_start_date_location: Option<JsonValue>,

    pub lease_end_date: Option<NaiveDate>,
    pub lease_end_date_confidence: Option<Decimal>,
    pub lease_end_date_location: Option<JsonValue>,

    pub monthly_rent: Option<Decimal>,
    pub monthly_rent_confidence: Option<Decimal>,
    pub monthly_rent_location: Option<JsonValue>,
    #[serde(default = "default_currency")]
    pub rent_currency: String,

    pub security_deposit: Option<Decimal>,
    pub security_deposit_confidence: Option<Decimal>,
    pub security_deposit_location: Option<JsonValue>,

    pub payment_due_day: Option<i32>,
    pub payment_due_day_confidence: Option<Decimal>,
    pub payment_due_day_location: Option<JsonValue>,

    #[serde(default)]
    pub special_clauses: JsonValue,

    pub model_used: Option<String>,
    pub extraction_duration_ms: Option<i32>,
}

fn default_currency() -> String {
    "EUR".to_string()
}

// =============================================================================
// EXTRACTION CORRECTIONS (Story 133.3)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ExtractionCorrection {
    pub id: Uuid,
    pub extraction_id: Uuid,
    pub corrected_by: Uuid,
    pub field_name: String,
    pub original_value: Option<String>,
    pub corrected_value: String,
    pub correction_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateExtractionCorrection {
    pub field_name: String,
    pub original_value: Option<String>,
    pub corrected_value: String,
    pub correction_reason: Option<String>,
}

/// Approve extraction with optional corrections
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ApproveExtraction {
    #[serde(default)]
    pub corrections: Vec<CreateExtractionCorrection>,
}

/// Reject extraction with reason
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RejectExtraction {
    pub reason: String,
}

// =============================================================================
// LEASE IMPORTS (Story 133.4)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct LeaseImport {
    pub id: Uuid,
    pub extraction_id: Uuid,
    pub lease_id: Option<Uuid>,
    pub status: String,
    pub imported_by: Option<Uuid>,
    pub imported_at: Option<DateTime<Utc>>,
    pub validation_errors: Option<JsonValue>,
    pub validation_warnings: Option<JsonValue>,
    pub fields_imported: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ImportExtractionRequest {
    /// The unit to link the lease to
    pub unit_id: Uuid,
    /// Optional overrides for extracted values
    #[serde(default)]
    pub overrides: JsonValue,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ImportValidationResult {
    pub can_import: bool,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
    pub severity: String, // error, warning
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ImportResult {
    pub success: bool,
    pub lease_id: Option<Uuid>,
    pub fields_imported: Vec<String>,
    pub errors: Vec<String>,
}

// =============================================================================
// PROCESSING REQUEST
// =============================================================================

/// Request to process a document with AI
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ProcessDocumentRequest {
    /// Ignored: the document is the `{id}` path segment. Kept optional so
    /// existing clients that still send it keep working; an empty body is valid.
    #[serde(default)]
    pub document_id: Option<Uuid>,
    /// Force reprocessing even if already processed
    #[serde(default)]
    pub force: bool,
}

/// Processing status response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProcessingStatus {
    pub document_id: Uuid,
    pub status: String,
    pub extraction_id: Option<Uuid>,
    pub error: Option<String>,
}

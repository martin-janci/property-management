//! Form routes (Epic 54: Forms Management).
//!
//! Provides endpoints for form templates, fields, and submissions.

use crate::state::AppState;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    form_status, CreateForm, CreateFormField, CreateFormResponse, Form, FormField, FormListQuery,
    FormListResponse, FormStatistics, FormSubmissionParams, FormSubmissionWithDetails, FormSummary,
    FormWithDetails, ReviewSubmission, SubmissionListQuery, SubmissionListResponse, SubmitForm,
    SubmitFormResponse, UpdateForm, UpdateFormField,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed title length (characters).
const MAX_TITLE_LENGTH: usize = 500;

/// Maximum allowed description length (characters).
const MAX_DESCRIPTION_LENGTH: usize = 5000;

/// Maximum number of fields per form.
const MAX_FIELDS_PER_FORM: usize = 100;

/// Maximum signature image size (1MB in bytes).
const MAX_SIGNATURE_SIZE: usize = 1024 * 1024;

// ============================================================================
// Helper Functions
// ============================================================================

/// Validates base64-encoded signature image data.
/// Returns error if invalid format, too large, or unsupported image type.
fn validate_signature_image(signature_b64: &str) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
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

// ============================================================================
// Router
// ============================================================================

/// Create forms router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Form CRUD
        .route("/", post(create_form))
        .route("/", get(list_forms))
        .route("/available", get(list_available_forms))
        .route("/statistics", get(get_statistics))
        .route("/{id}", get(get_form))
        .route("/{id}", put(update_form))
        .route("/{id}", delete(delete_form))
        // Publishing
        .route("/{id}/publish", post(publish_form))
        .route("/{id}/archive", post(archive_form))
        // Fields
        .route("/{id}/fields", get(list_fields))
        .route("/{id}/fields", post(add_field))
        .route("/{id}/fields/reorder", post(reorder_fields))
        .route("/{id}/fields/{field_id}", put(update_field))
        .route("/{id}/fields/{field_id}", delete(delete_field))
        // Submissions
        .route("/{id}/submit", post(submit_form))
        .route("/{id}/submissions", get(list_submissions))
        .route("/{id}/submissions/{submission_id}", get(get_submission))
        .route(
            "/{id}/submissions/{submission_id}/review",
            post(review_submission),
        )
        // Download tracking
        .route("/{id}/download", post(record_download))
}

// ============================================================================
// Form CRUD Handlers
// ============================================================================

/// Create a new form (Story 54.1).
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/forms",
    request_body = CreateFormRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Form created", body = CreateFormResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn create_form(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Json(req): Json<CreateFormRequest>,
) -> Result<(StatusCode, Json<CreateFormResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can create forms",
            )),
        ));
    }

    // Validate title
    if req.title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Title is required")),
        ));
    }
    if req.title.len() > MAX_TITLE_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Title exceeds maximum length of {} characters",
                    MAX_TITLE_LENGTH
                ),
            )),
        ));
    }

    // Validate description
    if let Some(ref desc) = req.description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Description exceeds maximum length of {} characters",
                        MAX_DESCRIPTION_LENGTH
                    ),
                )),
            ));
        }
    }

    // Validate fields count
    if req.fields.len() > MAX_FIELDS_PER_FORM {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!("Maximum {} fields per form", MAX_FIELDS_PER_FORM),
            )),
        ));
    }

    let repo = &state.form_repo;

    // Validate and convert fields, returning error on malformed JSON
    let mut fields = Vec::new();
    for f in req.fields {
        // Validate validation_rules JSON if present
        let validation_rules = match f.validation_rules {
            Some(v) => Some(serde_json::from_value(v).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "INVALID_VALIDATION_RULES",
                        format!(
                            "Invalid validation rules JSON for field '{}': {}",
                            f.field_key, e
                        ),
                    )),
                )
            })?),
            None => None,
        };

        fields.push(CreateFormField {
            field_key: f.field_key,
            label: f.label,
            field_type: f.field_type,
            required: f.required,
            help_text: f.help_text,
            placeholder: f.placeholder,
            default_value: f.default_value,
            validation_rules,
            options: f.options.map(|opts| {
                opts.into_iter()
                    .map(|o| db::models::FieldOption {
                        value: o.value,
                        label: o.label,
                    })
                    .collect()
            }),
            field_order: f.field_order,
            width: f.width,
            section: f.section,
            conditional_display: None,
        });
    }

    // Convert request to domain model
    let create_data = CreateForm {
        title: req.title,
        description: req.description,
        category: req.category,
        building_id: req.building_id,
        target_type: req.target_type,
        target_ids: req.target_ids,
        require_signatures: req.require_signatures,
        allow_multiple_submissions: req.allow_multiple_submissions,
        submission_deadline: req.submission_deadline,
        confirmation_message: req.confirmation_message,
        fields,
    };

    let form = repo
        .create(tenant.tenant_id, auth.user_id, create_data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create form",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateFormResponse {
            id: form.id,
            message: "Form created successfully".to_string(),
        }),
    ))
}

/// List all forms (Story 54.1).
#[utoipa::path(
    get,
    path = "/api/v1/forms",
    params(ListFormsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of forms", body = FormListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn list_forms(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Query(query): Query<ListFormsQuery>,
) -> Result<Json<FormListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only managers can see all forms including drafts
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can list all forms",
            )),
        ));
    }

    let repo = &state.form_repo;

    let form_query = FormListQuery {
        status: query.status,
        category: query.category,
        building_id: query.building_id,
        search: query.search,
        page: query.page,
        per_page: query.per_page,
        sort_by: query.sort_by,
        sort_order: query.sort_order,
    };

    let (forms, total) = repo.list(tenant.tenant_id, form_query).await.map_err(|e| {
        tracing::error!("Failed to list forms: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list forms")),
        )
    })?;

    Ok(Json(FormListResponse {
        forms,
        total,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(20),
    }))
}

/// List available forms for users (Story 54.2).
#[utoipa::path(
    get,
    path = "/api/v1/forms/available",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of available forms", body = AvailableFormsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn list_available_forms(
    State(state): State<AppState>,
    tenant: TenantExtractor,
) -> Result<Json<AvailableFormsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = &state.form_repo;

    let forms = repo
        .list_available_forms(tenant.tenant_id, None, "")
        .await
        .map_err(|e| {
            tracing::error!("Failed to list available forms: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list available forms",
                )),
            )
        })?;

    Ok(Json(AvailableFormsResponse { forms }))
}

/// Get form details.
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form details", body = FormDetailResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn get_form(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<FormDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = &state.form_repo;

    let form = repo
        .get_with_details(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
            )
        })?;

    // Non-managers can only see published forms
    if !tenant.role.is_manager() && form.form.status != form_status::PUBLISHED {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        ));
    }

    Ok(Json(FormDetailResponse { form }))
}

/// Update a form.
#[utoipa::path(
    put,
    path = "/api/v1/forms/{id}",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = UpdateFormRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form updated", body = FormActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn update_form(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFormRequest>,
) -> Result<Json<FormActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update forms",
            )),
        ));
    }

    let repo = &state.form_repo;

    // Check if form exists and is editable
    let existing = repo.get(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to get form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
        )
    })?;

    let existing = existing.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        )
    })?;

    if existing.status != form_status::DRAFT {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Only draft forms can be edited",
            )),
        ));
    }

    let update_data = UpdateForm {
        title: req.title,
        description: req.description,
        category: req.category,
        building_id: req.building_id,
        target_type: req.target_type,
        target_ids: req.target_ids,
        require_signatures: req.require_signatures,
        allow_multiple_submissions: req.allow_multiple_submissions,
        submission_deadline: req.submission_deadline,
        confirmation_message: req.confirmation_message,
    };

    let form = repo
        .update(tenant.tenant_id, id, auth.user_id, update_data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update form",
                )),
            )
        })?;

    Ok(Json(FormActionResponse {
        message: "Form updated successfully".to_string(),
        form,
    }))
}

/// Delete a form.
#[utoipa::path(
    delete,
    path = "/api/v1/forms/{id}",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form deleted"),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn delete_form(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete forms",
            )),
        ));
    }

    let repo = &state.form_repo;

    repo.delete(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to delete form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to delete form",
            )),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Publish a form.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/publish",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form published", body = FormActionResponse),
        (status = 400, description = "Cannot publish form", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn publish_form(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<FormActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can publish forms",
            )),
        ));
    }

    let repo = &state.form_repo;

    // Check that form has at least one field
    let fields = repo.get_fields(id).await.map_err(|e| {
        tracing::error!("Failed to get form fields: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to get form fields",
            )),
        )
    })?;

    if fields.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Cannot publish form without fields",
            )),
        ));
    }

    let form = repo
        .publish(tenant.tenant_id, id, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to publish form: {:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Cannot publish form - it may already be published",
                )),
            )
        })?;

    Ok(Json(FormActionResponse {
        message: "Form published successfully".to_string(),
        form,
    }))
}

/// Archive a form.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/archive",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form archived", body = FormActionResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn archive_form(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<FormActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can archive forms",
            )),
        ));
    }

    let repo = &state.form_repo;

    let form = repo.archive(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to archive form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to archive form",
            )),
        )
    })?;

    Ok(Json(FormActionResponse {
        message: "Form archived successfully".to_string(),
        form,
    }))
}

/// Get form statistics.
#[utoipa::path(
    get,
    path = "/api/v1/forms/statistics",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form statistics", body = FormStatisticsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn get_statistics(
    State(state): State<AppState>,
    tenant: TenantExtractor,
) -> Result<Json<FormStatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can view statistics",
            )),
        ));
    }

    let repo = &state.form_repo;

    let statistics = repo.get_statistics(tenant.tenant_id).await.map_err(|e| {
        tracing::error!("Failed to get statistics: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to get statistics",
            )),
        )
    })?;

    Ok(Json(FormStatisticsResponse { statistics }))
}

// ============================================================================
// Field Handlers
// ============================================================================

/// List form fields.
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}/fields",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form fields", body = FieldsResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn list_fields(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<FieldsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = &state.form_repo;

    // Verify form exists and user has access
    let form = repo.get(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to get form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
        )
    })?;

    if form.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        ));
    }

    let fields = repo.get_fields(id).await.map_err(|e| {
        tracing::error!("Failed to get fields: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get fields")),
        )
    })?;

    Ok(Json(FieldsResponse { fields }))
}

/// Add a field to a form.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/fields",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = CreateFormFieldRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Field added", body = FieldActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn add_field(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateFormFieldRequest>,
) -> Result<(StatusCode, Json<FieldActionResponse>), (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can add fields",
            )),
        ));
    }

    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = repo.get(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to get form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
        )
    })?;

    let form = form.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        )
    })?;

    if form.status != form_status::DRAFT {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only add fields to draft forms",
            )),
        ));
    }

    let field_data = CreateFormField {
        field_key: req.field_key,
        label: req.label,
        field_type: req.field_type,
        required: req.required,
        help_text: req.help_text,
        placeholder: req.placeholder,
        default_value: req.default_value,
        validation_rules: req
            .validation_rules
            .map(|v| serde_json::from_value(v).unwrap_or_default()),
        options: req.options.map(|opts| {
            opts.into_iter()
                .map(|o| db::models::FieldOption {
                    value: o.value,
                    label: o.label,
                })
                .collect()
        }),
        field_order: req.field_order,
        width: req.width,
        section: req.section,
        conditional_display: None,
    };

    let field = repo
        .create_field(id, field_data, req.field_order)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add field: {:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Failed to add field - field key may already exist",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(FieldActionResponse {
            message: "Field added successfully".to_string(),
            field,
        }),
    ))
}

/// Update a form field.
#[utoipa::path(
    put,
    path = "/api/v1/forms/{id}/fields/{field_id}",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("field_id" = Uuid, Path, description = "Field ID")
    ),
    request_body = UpdateFormFieldRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Field updated", body = FieldActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Field not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn update_field(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((id, field_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateFormFieldRequest>,
) -> Result<Json<FieldActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update fields",
            )),
        ));
    }

    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = repo.get(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to get form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
        )
    })?;

    let form = form.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        )
    })?;

    if form.status != form_status::DRAFT {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only update fields in draft forms",
            )),
        ));
    }

    let update_data = UpdateFormField {
        label: req.label,
        field_type: req.field_type,
        required: req.required,
        help_text: req.help_text,
        placeholder: req.placeholder,
        default_value: req.default_value,
        validation_rules: req
            .validation_rules
            .map(|v| serde_json::from_value(v).unwrap_or_default()),
        options: req.options.map(|opts| {
            opts.into_iter()
                .map(|o| db::models::FieldOption {
                    value: o.value,
                    label: o.label,
                })
                .collect()
        }),
        field_order: req.field_order,
        width: req.width,
        section: req.section,
        conditional_display: None,
    };

    let field = repo
        .update_field(id, field_id, update_data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update field: {:?}", e);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Field not found")),
            )
        })?;

    Ok(Json(FieldActionResponse {
        message: "Field updated successfully".to_string(),
        field,
    }))
}

/// Delete a form field.
#[utoipa::path(
    delete,
    path = "/api/v1/forms/{id}/fields/{field_id}",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("field_id" = Uuid, Path, description = "Field ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Field deleted"),
        (status = 404, description = "Field not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn delete_field(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((id, field_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete fields",
            )),
        ));
    }

    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = repo.get(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to get form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
        )
    })?;

    let form = form.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        )
    })?;

    if form.status != form_status::DRAFT {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only delete fields from draft forms",
            )),
        ));
    }

    repo.delete_field(id, field_id).await.map_err(|e| {
        tracing::error!("Failed to delete field: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to delete field",
            )),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reorder form fields.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/fields/reorder",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = ReorderFieldsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fields reordered"),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn reorder_fields(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(req): Json<ReorderFieldsRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can reorder fields",
            )),
        ));
    }

    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = repo.get(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to get form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
        )
    })?;

    let form = form.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        )
    })?;

    if form.status != form_status::DRAFT {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only reorder fields in draft forms",
            )),
        ));
    }

    let field_orders: Vec<(Uuid, i32)> = req
        .field_orders
        .into_iter()
        .map(|fo| (fo.field_id, fo.order))
        .collect();

    repo.reorder_fields(id, field_orders).await.map_err(|e| {
        tracing::error!("Failed to reorder fields: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "INTERNAL_ERROR",
                "Failed to reorder fields",
            )),
        )
    })?;

    Ok(StatusCode::OK)
}

// ============================================================================
// Submission Handlers
// ============================================================================

/// Submit a form (Story 54.3).
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/submit",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = SubmitFormRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Form submitted", body = SubmitFormResponse),
        (status = 400, description = "Invalid submission", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn submit_form(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Path(id): Path<Uuid>,
    Json(req): Json<SubmitFormRequest>,
) -> Result<(StatusCode, Json<SubmitFormResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Extract IP from X-Forwarded-For header or connection address
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| Some(addr.ip().to_string()));

    // Extract User-Agent from headers
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let repo = &state.form_repo;

    // Get form details
    let form = repo
        .get_with_details(tenant.tenant_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
            )
        })?;

    // Verify form is published
    if form.form.status != form_status::PUBLISHED {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only submit published forms",
            )),
        ));
    }

    // Check if user already submitted and multiple submissions not allowed
    if !form.form.allow_multiple_submissions {
        let has_submitted = repo
            .has_user_submitted(id, auth.user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check submission: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to check submission",
                    )),
                )
            })?;

        if has_submitted {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "You have already submitted this form",
                )),
            ));
        }
    }

    // Check deadline
    if let Some(deadline) = form.form.submission_deadline {
        if chrono::Utc::now() > deadline {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Submission deadline has passed",
                )),
            ));
        }
    }

    // Validate required fields
    for field in &form.fields {
        if field.required {
            let field_value = req.data.get(&field.field_key);
            if field_value.is_none()
                || field_value.map(|v| v.is_null()).unwrap_or(true)
                || field_value
                    .and_then(|v| v.as_str())
                    .map(|s| s.is_empty())
                    .unwrap_or(false)
            {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "BAD_REQUEST",
                        format!("Field '{}' is required", field.label),
                    )),
                ));
            }
        }
    }

    // Check signature requirement
    if form.form.require_signatures && req.signature_data.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Signature is required for this form",
            )),
        ));
    }

    let submit_data = SubmitForm {
        data: req.data,
        attachments: req.attachments.map(|atts| {
            atts.into_iter()
                .map(|a| db::models::FormAttachment {
                    field_key: a.field_key,
                    file_id: a.file_id,
                    filename: a.filename,
                    mime_type: a.mime_type,
                    size: a.size,
                })
                .collect()
        }),
        signature_data: match req.signature_data {
            Some(s) => {
                // Validate signature image before processing
                validate_signature_image(&s.signature_image)?;
                Some(db::models::SignatureData {
                    signature_image: s.signature_image,
                    signed_at: chrono::Utc::now(),
                    ip_address: ip_address.clone(),
                    user_agent: user_agent.clone(),
                })
            }
            None => None,
        },
    };

    let submission = repo
        .submit(FormSubmissionParams {
            org_id: tenant.tenant_id,
            form_id: id,
            user_id: auth.user_id,
            building_id: None, // could be extracted from user context if needed
            unit_id: None,     // could be extracted from user context if needed
            data: submit_data,
            ip_address: ip_address.clone(),
            user_agent: user_agent.clone(),
        })
        .await
        .map_err(|e| {
            tracing::error!("Failed to submit form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to submit form",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(SubmitFormResponse {
            id: submission.id,
            message: "Form submitted successfully".to_string(),
            confirmation_message: form.form.confirmation_message,
        }),
    ))
}

/// List submissions for a form (Story 54.4).
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}/submissions",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ListSubmissionsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of submissions", body = SubmissionListResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn list_submissions(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Query(query): Query<ListSubmissionsQuery>,
) -> Result<Json<SubmissionListResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can view submissions",
            )),
        ));
    }

    let repo = &state.form_repo;

    let sub_query = SubmissionListQuery {
        form_id: Some(id),
        status: query.status,
        building_id: query.building_id,
        unit_id: query.unit_id,
        submitted_by: query.submitted_by,
        from_date: query.from_date,
        to_date: query.to_date,
        page: query.page,
        per_page: query.per_page,
    };

    let (submissions, total) = repo
        .list_submissions(tenant.tenant_id, sub_query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list submissions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list submissions",
                )),
            )
        })?;

    Ok(Json(SubmissionListResponse {
        submissions,
        total,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(20),
    }))
}

/// Get submission details.
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}/submissions/{submission_id}",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("submission_id" = Uuid, Path, description = "Submission ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Submission details", body = SubmissionDetailResponse),
        (status = 404, description = "Submission not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn get_submission(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path((_id, submission_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SubmissionDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let repo = &state.form_repo;

    let submission = repo
        .get_submission(tenant.tenant_id, submission_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get submission: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get submission",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Submission not found")),
            )
        })?;

    // Non-managers can only view their own submissions
    if !tenant.role.is_manager() && submission.submission.submitted_by != auth.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "You can only view your own submissions",
            )),
        ));
    }

    Ok(Json(SubmissionDetailResponse { submission }))
}

/// Review a submission (approve/reject).
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/submissions/{submission_id}/review",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("submission_id" = Uuid, Path, description = "Submission ID")
    ),
    request_body = ReviewSubmissionRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Submission reviewed", body = SubmissionDetailResponse),
        (status = 400, description = "Invalid review", body = ErrorResponse),
        (status = 404, description = "Submission not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn review_submission(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path((_id, submission_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ReviewSubmissionRequest>,
) -> Result<Json<SubmissionDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can review submissions",
            )),
        ));
    }

    // Validate status
    if req.status != "approved" && req.status != "rejected" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Status must be 'approved' or 'rejected'",
            )),
        ));
    }

    let repo = &state.form_repo;

    let review_data = ReviewSubmission {
        status: req.status,
        review_notes: req.review_notes,
    };

    repo.review_submission(tenant.tenant_id, submission_id, auth.user_id, review_data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to review submission: {:?}", e);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Submission not found")),
            )
        })?;

    // Get updated submission
    let submission = repo
        .get_submission(tenant.tenant_id, submission_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get submission: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get submission",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Submission not found")),
            )
        })?;

    Ok(Json(SubmissionDetailResponse { submission }))
}

/// Record form download (Story 54.2).
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/download",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Download recorded"),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
async fn record_download(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let repo = &state.form_repo;

    // Verify form exists
    let form = repo.get(tenant.tenant_id, id).await.map_err(|e| {
        tracing::error!("Failed to get form: {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
        )
    })?;

    if form.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
        ));
    }

    repo.record_download(id, auth.user_id, None, None)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record download: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to record download",
                )),
            )
        })?;

    Ok(StatusCode::OK)
}

//! Platform Migration & Data Import routes (Epic 66, Stories 66.1-66.4).
//!
//! Handles import templates, bulk import, data export, and validation.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_extra::extract::Multipart;
use db::models::{
    ApproveImportRequest, ApproveImportResponse, ExportCategoriesResponse, ExportCategoryInfo,
    ExportDataCategory, ExportPrivacyOptions, FieldDataType, FieldValidation,
    ImportCategoriesResponse, ImportCategoryInfo, ImportDataType, ImportFieldMapping,
    ImportJobHistory, ImportJobStatus, ImportJobStatusResponse, ImportPreviewResult,
    ImportRowError, ImportTemplateSummary, MigrationExportResponse, MigrationExportStatus,
    MigrationExportStatusResponse, MigrationPagination, RecordTypeCounts, TemplateFormat,
    UpdateImportTemplate, ValidationIssue, ValidationSeverity,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Create the migration router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Story 66.1: Import Templates
        .route("/templates", get(list_templates))
        .route("/templates", post(create_template))
        .route("/templates/system", get(list_system_templates))
        .route("/templates/{template_id}", get(get_template))
        .route("/templates/{template_id}", put(update_template))
        .route("/templates/{template_id}", delete(delete_template))
        .route("/templates/{template_id}/download", get(download_template))
        .route(
            "/templates/{template_id}/duplicate",
            post(duplicate_template),
        )
        .route("/categories/import", get(get_import_categories))
        // Story 66.2: Bulk Data Import
        .route("/import/upload", post(upload_import_file))
        .route("/import/jobs", get(list_import_jobs))
        .route("/import/jobs/{job_id}", get(get_import_job_status))
        .route("/import/jobs/{job_id}/cancel", post(cancel_import_job))
        .route("/import/jobs/{job_id}/retry", post(retry_import_job))
        .route("/import/jobs/{job_id}/errors", get(get_import_job_errors))
        // Story 66.3: Data Export for Migration
        .route("/export", post(request_migration_export))
        .route("/export/{export_id}", get(get_export_status))
        .route("/export/{export_id}/download", get(download_export))
        .route("/export/history", get(get_export_history))
        .route("/categories/export", get(get_export_categories))
        // Story 66.4: Import Validation & Preview
        .route("/import/jobs/{job_id}/preview", get(get_import_preview))
        .route("/import/jobs/{job_id}/approve", post(approve_import))
        .route("/import/jobs/{job_id}/validate", post(validate_import))
}

// ============================================================================
// STORY 66.1: Import Templates
// ============================================================================

/// Query parameters for listing templates.
#[derive(Debug, Deserialize)]
pub struct ListTemplatesQuery {
    /// Filter by data type
    pub data_type: Option<ImportDataType>,
    /// Include system templates
    #[serde(default = "default_true")]
    pub include_system: bool,
    /// Page number
    pub page: Option<i32>,
    /// Items per page
    pub per_page: Option<i32>,
}

fn default_true() -> bool {
    true
}

/// Response for listing templates.
#[derive(Debug, Serialize)]
pub struct ListTemplatesResponse {
    pub templates: Vec<ImportTemplateSummary>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

/// List import templates for the organization.
async fn list_templates(
    State(_state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListTemplatesQuery>,
) -> Result<Json<ListTemplatesResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, query the database
    let templates = generate_sample_templates(org_id, query.data_type, query.include_system);

    Ok(Json(ListTemplatesResponse {
        total: templates.len() as i64,
        templates,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(20),
    }))
}

/// List system-provided templates.
async fn list_system_templates(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<ImportTemplateSummary>>, (StatusCode, String)> {
    let templates: Vec<ImportTemplateSummary> = vec![
        ImportTemplateSummary {
            id: Uuid::new_v4(),
            name: "Buildings Import".to_string(),
            data_type: ImportDataType::Buildings,
            description: Some("Import building master data".to_string()),
            is_system_template: true,
            field_count: 12,
            updated_at: chrono::Utc::now(),
        },
        ImportTemplateSummary {
            id: Uuid::new_v4(),
            name: "Units Import".to_string(),
            data_type: ImportDataType::Units,
            description: Some("Import unit data with building references".to_string()),
            is_system_template: true,
            field_count: 15,
            updated_at: chrono::Utc::now(),
        },
        ImportTemplateSummary {
            id: Uuid::new_v4(),
            name: "Residents Import".to_string(),
            data_type: ImportDataType::Residents,
            description: Some("Import resident data with unit assignments".to_string()),
            is_system_template: true,
            field_count: 18,
            updated_at: chrono::Utc::now(),
        },
        ImportTemplateSummary {
            id: Uuid::new_v4(),
            name: "Financials Import".to_string(),
            data_type: ImportDataType::Financials,
            description: Some("Import financial transactions and balances".to_string()),
            is_system_template: true,
            field_count: 14,
            updated_at: chrono::Utc::now(),
        },
    ];

    Ok(Json(templates))
}

/// Request to create a new template.
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub data_type: ImportDataType,
    pub field_mappings: Vec<ImportFieldMapping>,
}

/// Create a new import template.
async fn create_template(
    State(_state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<Json<ImportTemplateSummary>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // Validate field mappings
    if req.field_mappings.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "At least one field mapping is required".to_string(),
        ));
    }

    // In a real implementation, save to database
    let template = ImportTemplateSummary {
        id: Uuid::new_v4(),
        name: req.name,
        data_type: req.data_type,
        description: req.description,
        is_system_template: false,
        field_count: req.field_mappings.len(),
        updated_at: chrono::Utc::now(),
    };

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        template_id = %template.id,
        "Created import template"
    );

    Ok(Json(template))
}

/// Template detail response.
#[derive(Debug, Serialize)]
pub struct TemplateDetailResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub data_type: ImportDataType,
    pub field_mappings: Vec<ImportFieldMapping>,
    pub is_system_template: bool,
    pub version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Get a specific template.
async fn get_template(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(template_id): Path<Uuid>,
) -> Result<Json<TemplateDetailResponse>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, fetch from database
    // For now, return a sample buildings template
    let template = TemplateDetailResponse {
        id: template_id,
        name: "Buildings Import".to_string(),
        description: Some("Import building master data".to_string()),
        data_type: ImportDataType::Buildings,
        field_mappings: get_buildings_template_fields(),
        is_system_template: true,
        version: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    Ok(Json(template))
}

/// Update an import template.
async fn update_template(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(template_id): Path<Uuid>,
    Json(req): Json<UpdateImportTemplate>,
) -> Result<Json<ImportTemplateSummary>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, fetch and update in database
    // System templates cannot be modified
    let template = ImportTemplateSummary {
        id: template_id,
        name: req.name.unwrap_or_else(|| "Updated Template".to_string()),
        data_type: ImportDataType::Buildings,
        description: None,
        is_system_template: false,
        field_count: req.field_mappings.map(|f| f.len()).unwrap_or(10),
        updated_at: chrono::Utc::now(),
    };

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        template_id = %template_id,
        "Updated import template"
    );

    Ok(Json(template))
}

/// Delete an import template.
async fn delete_template(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(template_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation:
    // 1. Check if template exists and belongs to org
    // 2. Check if template is not a system template
    // 3. Delete from database

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        template_id = %template_id,
        "Deleted import template"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// Query parameters for template download.
#[derive(Debug, Deserialize)]
pub struct DownloadTemplateQuery {
    #[serde(default)]
    pub format: TemplateFormat,
    /// Include example data rows
    #[serde(default)]
    pub include_examples: bool,
}

/// Download response with file info.
#[derive(Debug, Serialize)]
pub struct TemplateDownloadResponse {
    pub filename: String,
    pub content_type: String,
    pub download_url: String,
}

/// Download a template as CSV/Excel.
async fn download_template(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(template_id): Path<Uuid>,
    Query(query): Query<DownloadTemplateQuery>,
) -> Result<Json<TemplateDownloadResponse>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let (filename, content_type) = match query.format {
        TemplateFormat::Csv => ("template.csv".to_string(), "text/csv"),
        TemplateFormat::Xlsx => (
            "template.xlsx".to_string(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
    };

    // In a real implementation, generate the file and return a signed URL
    let download_url = format!(
        "/api/v1/migration/templates/{}/file?format={:?}&token={}",
        template_id,
        query.format,
        Uuid::new_v4()
    );

    Ok(Json(TemplateDownloadResponse {
        filename,
        content_type: content_type.to_string(),
        download_url,
    }))
}

/// Duplicate a template.
async fn duplicate_template(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(template_id): Path<Uuid>,
) -> Result<Json<ImportTemplateSummary>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, copy the template with a new ID
    let new_template = ImportTemplateSummary {
        id: Uuid::new_v4(),
        name: "Buildings Import (Copy)".to_string(),
        data_type: ImportDataType::Buildings,
        description: Some("Copy of system template".to_string()),
        is_system_template: false,
        field_count: 12,
        updated_at: chrono::Utc::now(),
    };

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        source_template_id = %template_id,
        new_template_id = %new_template.id,
        "Duplicated import template"
    );

    Ok(Json(new_template))
}

/// Get available import categories.
async fn get_import_categories(
    State(_state): State<AppState>,
    _user: AuthUser,
) -> Json<ImportCategoriesResponse> {
    Json(ImportCategoriesResponse {
        categories: vec![
            ImportCategoryInfo {
                id: ImportDataType::Buildings,
                name: "Buildings".to_string(),
                description: "Building master data including address and details".to_string(),
                has_system_template: true,
                dependencies: vec![],
            },
            ImportCategoryInfo {
                id: ImportDataType::Units,
                name: "Units".to_string(),
                description: "Individual units within buildings".to_string(),
                has_system_template: true,
                dependencies: vec![ImportDataType::Buildings],
            },
            ImportCategoryInfo {
                id: ImportDataType::Residents,
                name: "Residents".to_string(),
                description: "Resident and owner information".to_string(),
                has_system_template: true,
                dependencies: vec![ImportDataType::Buildings, ImportDataType::Units],
            },
            ImportCategoryInfo {
                id: ImportDataType::Financials,
                name: "Financials".to_string(),
                description: "Financial transactions, fees, and balances".to_string(),
                has_system_template: true,
                dependencies: vec![ImportDataType::Buildings, ImportDataType::Units],
            },
            ImportCategoryInfo {
                id: ImportDataType::Faults,
                name: "Faults".to_string(),
                description: "Fault reports and maintenance issues".to_string(),
                has_system_template: true,
                dependencies: vec![ImportDataType::Buildings],
            },
            ImportCategoryInfo {
                id: ImportDataType::Meters,
                name: "Meters".to_string(),
                description: "Utility meters and readings".to_string(),
                has_system_template: true,
                dependencies: vec![ImportDataType::Buildings, ImportDataType::Units],
            },
        ],
    })
}

// ============================================================================
// STORY 66.2: Bulk Data Import
// ============================================================================

/// Upload response.
#[derive(Debug, Serialize)]
pub struct UploadImportResponse {
    pub job_id: Uuid,
    pub status: ImportJobStatus,
    pub filename: String,
    pub file_size_bytes: i64,
    pub message: String,
}

/// Upload an import file.
async fn upload_import_file(
    State(_state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<UploadImportResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    let mut template_id: Option<Uuid> = None;
    let mut filename: Option<String> = None;
    let mut file_size: i64 = 0;
    let mut _file_data: Vec<u8> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "template_id" => {
                let value = field
                    .text()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                template_id =
                    Some(Uuid::parse_str(&value).map_err(|_| {
                        (StatusCode::BAD_REQUEST, "Invalid template_id".to_string())
                    })?);
            }
            "file" => {
                filename = field.file_name().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
                file_size = data.len() as i64;
                _file_data = data.to_vec();
            }
            _ => {}
        }
    }

    let template_id = template_id.ok_or((
        StatusCode::BAD_REQUEST,
        "template_id is required".to_string(),
    ))?;

    let filename = filename.ok_or((StatusCode::BAD_REQUEST, "File is required".to_string()))?;

    // Validate file size
    const MAX_IMPORT_SIZE: i64 = 100 * 1024 * 1024; // 100MB
    if file_size > MAX_IMPORT_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "File size exceeds maximum of {}MB",
                MAX_IMPORT_SIZE / 1024 / 1024
            ),
        ));
    }

    // In a real implementation:
    // 1. Save file to storage
    // 2. Create import job record
    // 3. Queue job for processing

    let job_id = Uuid::new_v4();

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        job_id = %job_id,
        template_id = %template_id,
        filename = %filename,
        file_size = file_size,
        "Created import job"
    );

    Ok(Json(UploadImportResponse {
        job_id,
        status: ImportJobStatus::Pending,
        filename,
        file_size_bytes: file_size,
        message: "File uploaded successfully. Validation will begin shortly.".to_string(),
    }))
}

/// Query parameters for listing import jobs.
#[derive(Debug, Deserialize)]
pub struct ListImportJobsQuery {
    pub status: Option<ImportJobStatus>,
    pub data_type: Option<ImportDataType>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

/// Response for listing import jobs.
#[derive(Debug, Serialize)]
pub struct ListImportJobsResponse {
    pub jobs: Vec<ImportJobHistory>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

/// List import jobs for the organization.
async fn list_import_jobs(
    State(_state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListImportJobsQuery>,
) -> Result<Json<ListImportJobsResponse>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, query the database
    let jobs = vec![
        ImportJobHistory {
            id: Uuid::new_v4(),
            status: ImportJobStatus::Completed,
            filename: "buildings_2024.csv".to_string(),
            data_type: ImportDataType::Buildings,
            records_imported: 45,
            records_failed: 0,
            created_by_name: "John Manager".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::days(1),
            completed_at: Some(chrono::Utc::now() - chrono::Duration::days(1)),
        },
        ImportJobHistory {
            id: Uuid::new_v4(),
            status: ImportJobStatus::PartiallyCompleted,
            filename: "residents_import.xlsx".to_string(),
            data_type: ImportDataType::Residents,
            records_imported: 120,
            records_failed: 5,
            created_by_name: "John Manager".to_string(),
            created_at: chrono::Utc::now() - chrono::Duration::hours(6),
            completed_at: Some(chrono::Utc::now() - chrono::Duration::hours(6)),
        },
    ];

    Ok(Json(ListImportJobsResponse {
        total: jobs.len() as i64,
        jobs,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(20),
    }))
}

/// Get import job status.
async fn get_import_job_status(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ImportJobStatusResponse>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, fetch from database
    let status = ImportJobStatusResponse {
        id: job_id,
        status: ImportJobStatus::Importing,
        filename: "residents_import.csv".to_string(),
        template_name: "Residents Import".to_string(),
        progress_percent: 65,
        total_rows: Some(200),
        processed_rows: 130,
        successful_rows: 125,
        failed_rows: 5,
        skipped_rows: 0,
        error_summary: Some(vec![ImportRowError {
            row_number: 45,
            column: Some("email".to_string()),
            message: "Invalid email format".to_string(),
            error_code: "INVALID_EMAIL".to_string(),
            original_value: Some("not-an-email".to_string()),
        }]),
        started_at: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
        completed_at: None,
        estimated_remaining_seconds: Some(120),
    };

    Ok(Json(status))
}

/// Cancel an import job.
async fn cancel_import_job(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ImportJobStatusResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, update job status and stop processing
    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        job_id = %job_id,
        "Cancelled import job"
    );

    let status = ImportJobStatusResponse {
        id: job_id,
        status: ImportJobStatus::Cancelled,
        filename: "import_file.csv".to_string(),
        template_name: "Template".to_string(),
        progress_percent: 65,
        total_rows: Some(200),
        processed_rows: 130,
        successful_rows: 125,
        failed_rows: 5,
        skipped_rows: 0,
        error_summary: None,
        started_at: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
        completed_at: Some(chrono::Utc::now()),
        estimated_remaining_seconds: None,
    };

    Ok(Json(status))
}

/// Retry a failed import job.
async fn retry_import_job(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ImportJobStatusResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        job_id = %job_id,
        "Retrying import job"
    );

    let status = ImportJobStatusResponse {
        id: job_id,
        status: ImportJobStatus::Pending,
        filename: "import_file.csv".to_string(),
        template_name: "Template".to_string(),
        progress_percent: 0,
        total_rows: Some(200),
        processed_rows: 0,
        successful_rows: 0,
        failed_rows: 0,
        skipped_rows: 0,
        error_summary: None,
        started_at: None,
        completed_at: None,
        estimated_remaining_seconds: Some(300),
    };

    Ok(Json(status))
}

/// Response with detailed errors.
#[derive(Debug, Serialize)]
pub struct ImportJobErrorsResponse {
    pub job_id: Uuid,
    pub total_errors: i32,
    pub errors: Vec<ImportRowError>,
    pub page: i32,
    pub per_page: i32,
}

/// Get detailed errors for an import job.
async fn get_import_job_errors(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(job_id): Path<Uuid>,
    Query(pagination): Query<MigrationPagination>,
) -> Result<Json<ImportJobErrorsResponse>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, fetch errors from database
    let errors = vec![
        ImportRowError {
            row_number: 45,
            column: Some("email".to_string()),
            message: "Invalid email format".to_string(),
            error_code: "INVALID_EMAIL".to_string(),
            original_value: Some("not-an-email".to_string()),
        },
        ImportRowError {
            row_number: 78,
            column: Some("phone".to_string()),
            message: "Phone number too short".to_string(),
            error_code: "INVALID_PHONE".to_string(),
            original_value: Some("123".to_string()),
        },
        ImportRowError {
            row_number: 102,
            column: Some("unit_id".to_string()),
            message: "Referenced unit not found".to_string(),
            error_code: "FOREIGN_KEY_NOT_FOUND".to_string(),
            original_value: Some("UNIT-999".to_string()),
        },
    ];

    Ok(Json(ImportJobErrorsResponse {
        job_id,
        total_errors: errors.len() as i32,
        errors,
        page: pagination.page.unwrap_or(1),
        per_page: pagination.per_page.unwrap_or(50),
    }))
}

// ============================================================================
// STORY 66.3: Data Export for Migration
// ============================================================================

/// Request to create a migration export.
#[derive(Debug, Deserialize)]
pub struct RequestMigrationExportRequest {
    pub categories: Vec<ExportDataCategory>,
    #[serde(default)]
    pub privacy_options: ExportPrivacyOptions,
}

/// Request a full data export for migration.
async fn request_migration_export(
    State(_state): State<AppState>,
    user: AuthUser,
    Json(req): Json<RequestMigrationExportRequest>,
) -> Result<Json<MigrationExportResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    if req.categories.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "At least one category must be selected".to_string(),
        ));
    }

    // In a real implementation:
    // 1. Check for existing pending exports
    // 2. Create export record
    // 3. Queue background job

    let export_id = Uuid::new_v4();

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        export_id = %export_id,
        categories = ?req.categories,
        "Created migration export request"
    );

    Ok(Json(MigrationExportResponse {
        export_id,
        status: MigrationExportStatus::Pending,
        estimated_time: "10-15 minutes".to_string(),
        categories: req.categories,
    }))
}

/// Get migration export status.
async fn get_export_status(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(export_id): Path<Uuid>,
) -> Result<Json<MigrationExportStatusResponse>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, fetch from database
    let status = MigrationExportStatusResponse {
        export_id,
        status: MigrationExportStatus::Ready,
        categories: vec![
            "buildings".to_string(),
            "units".to_string(),
            "residents".to_string(),
        ],
        download_url: Some(format!(
            "/api/v1/migration/export/{}/download?token={}",
            export_id,
            Uuid::new_v4()
        )),
        file_size_bytes: Some(15_234_567),
        expires_at: chrono::Utc::now() + chrono::Duration::days(7),
        error_message: None,
        record_counts: Some(serde_json::json!({
            "buildings": 45,
            "units": 320,
            "residents": 580
        })),
    };

    Ok(Json(status))
}

/// Download response.
#[derive(Debug, Serialize)]
pub struct ExportDownloadResponse {
    pub filename: String,
    pub content_type: String,
    pub download_url: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Download a migration export.
async fn download_export(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(export_id): Path<Uuid>,
) -> Result<Json<ExportDownloadResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        export_id = %export_id,
        "Downloading migration export"
    );

    // In a real implementation:
    // 1. Verify export belongs to organization
    // 2. Check if export is ready
    // 3. Generate signed download URL
    // 4. Increment download count

    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

    Ok(Json(ExportDownloadResponse {
        filename: format!("migration_export_{}.zip", export_id),
        content_type: "application/zip".to_string(),
        download_url: format!(
            "https://storage.example.com/exports/{}.zip?token={}",
            export_id,
            Uuid::new_v4()
        ),
        expires_at,
    }))
}

/// Export history entry.
#[derive(Debug, Serialize)]
pub struct ExportHistoryEntry {
    pub id: Uuid,
    pub status: MigrationExportStatus,
    pub categories: Vec<String>,
    pub file_size_bytes: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub download_count: i32,
}

/// Get export history.
async fn get_export_history(
    State(_state): State<AppState>,
    user: AuthUser,
    Query(_pagination): Query<MigrationPagination>,
) -> Result<Json<Vec<ExportHistoryEntry>>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, fetch from database
    let history = vec![
        ExportHistoryEntry {
            id: Uuid::new_v4(),
            status: MigrationExportStatus::Ready,
            categories: vec!["buildings".to_string(), "units".to_string()],
            file_size_bytes: Some(5_234_567),
            created_at: chrono::Utc::now() - chrono::Duration::days(3),
            completed_at: Some(chrono::Utc::now() - chrono::Duration::days(3)),
            expires_at: chrono::Utc::now() + chrono::Duration::days(4),
            download_count: 2,
        },
        ExportHistoryEntry {
            id: Uuid::new_v4(),
            status: MigrationExportStatus::Expired,
            categories: vec![
                "buildings".to_string(),
                "units".to_string(),
                "residents".to_string(),
                "financials".to_string(),
            ],
            file_size_bytes: Some(25_678_901),
            created_at: chrono::Utc::now() - chrono::Duration::days(14),
            completed_at: Some(chrono::Utc::now() - chrono::Duration::days(14)),
            expires_at: chrono::Utc::now() - chrono::Duration::days(7),
            download_count: 1,
        },
    ];

    Ok(Json(history))
}

/// Get available export categories with record counts.
async fn get_export_categories(
    State(_state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ExportCategoriesResponse>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, query actual record counts
    Ok(Json(ExportCategoriesResponse {
        categories: vec![
            ExportCategoryInfo {
                id: ExportDataCategory::Buildings,
                name: "Buildings".to_string(),
                description: "Building master data".to_string(),
                record_count: 45,
                contains_personal_data: false,
            },
            ExportCategoryInfo {
                id: ExportDataCategory::Units,
                name: "Units".to_string(),
                description: "Unit details within buildings".to_string(),
                record_count: 320,
                contains_personal_data: false,
            },
            ExportCategoryInfo {
                id: ExportDataCategory::Residents,
                name: "Residents".to_string(),
                description: "Resident and owner information".to_string(),
                record_count: 580,
                contains_personal_data: true,
            },
            ExportCategoryInfo {
                id: ExportDataCategory::Financials,
                name: "Financials".to_string(),
                description: "Financial transactions and balances".to_string(),
                record_count: 12500,
                contains_personal_data: true,
            },
            ExportCategoryInfo {
                id: ExportDataCategory::Faults,
                name: "Faults".to_string(),
                description: "Fault reports and maintenance issues".to_string(),
                record_count: 890,
                contains_personal_data: false,
            },
            ExportCategoryInfo {
                id: ExportDataCategory::Documents,
                name: "Documents".to_string(),
                description: "Document metadata (not file contents)".to_string(),
                record_count: 2340,
                contains_personal_data: true,
            },
            ExportCategoryInfo {
                id: ExportDataCategory::Votes,
                name: "Votes".to_string(),
                description: "Voting history and results".to_string(),
                record_count: 156,
                contains_personal_data: true,
            },
            ExportCategoryInfo {
                id: ExportDataCategory::Meters,
                name: "Meters".to_string(),
                description: "Utility meters and readings".to_string(),
                record_count: 640,
                contains_personal_data: false,
            },
        ],
    }))
}

// ============================================================================
// STORY 66.4: Import Validation & Preview
// ============================================================================

/// Get import preview/validation results.
async fn get_import_preview(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ImportPreviewResult>, (StatusCode, String)> {
    let _org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation, fetch validation results from database/cache
    let preview = ImportPreviewResult {
        job_id,
        is_valid: true,
        total_rows: 150,
        importable_rows: 145,
        error_rows: 3,
        warning_rows: 7,
        record_counts: RecordTypeCounts {
            new_records: 120,
            updates: 25,
            skipped: 5,
        },
        issues: vec![
            ValidationIssue {
                row_number: Some(23),
                column: Some("email".to_string()),
                severity: ValidationSeverity::Error,
                code: "INVALID_EMAIL".to_string(),
                message: "Invalid email format".to_string(),
                original_value: Some("not.an".to_string()),
                suggested_value: None,
            },
            ValidationIssue {
                row_number: Some(45),
                column: Some("phone".to_string()),
                severity: ValidationSeverity::Warning,
                code: "PHONE_FORMAT".to_string(),
                message: "Phone number missing country code, will use default (+421)".to_string(),
                original_value: Some("0901234567".to_string()),
                suggested_value: Some("+421901234567".to_string()),
            },
            ValidationIssue {
                row_number: None,
                column: None,
                severity: ValidationSeverity::Info,
                code: "DUPLICATE_CHECK".to_string(),
                message: "5 records matched existing entries and will be updated".to_string(),
                original_value: None,
                suggested_value: None,
            },
        ],
        total_issue_count: 15,
        duplicates: vec![],
        sample_records: vec![
            serde_json::json!({
                "name": "Building A",
                "address": "123 Main St",
                "units": 24
            }),
            serde_json::json!({
                "name": "Building B",
                "address": "456 Oak Ave",
                "units": 36
            }),
        ],
        column_mapping: vec![
            db::models::ColumnMappingStatus {
                source_column: "Building Name".to_string(),
                target_field: Some("name".to_string()),
                is_mapped: true,
                is_required: true,
                sample_values: vec!["Building A".to_string(), "Building B".to_string()],
            },
            db::models::ColumnMappingStatus {
                source_column: "Street Address".to_string(),
                target_field: Some("address".to_string()),
                is_mapped: true,
                is_required: true,
                sample_values: vec!["123 Main St".to_string(), "456 Oak Ave".to_string()],
            },
            db::models::ColumnMappingStatus {
                source_column: "Extra Column".to_string(),
                target_field: None,
                is_mapped: false,
                is_required: false,
                sample_values: vec!["value1".to_string(), "value2".to_string()],
            },
        ],
    };

    Ok(Json(preview))
}

/// Approve and execute import.
async fn approve_import(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(job_id): Path<Uuid>,
    Json(req): Json<ApproveImportRequest>,
) -> Result<Json<ApproveImportResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    // In a real implementation:
    // 1. Verify job is in Validated status
    // 2. Check for errors (fail if acknowledge_warnings is false and has warnings)
    // 3. Update job status to Importing
    // 4. Queue import execution

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        job_id = %job_id,
        acknowledge_warnings = req.acknowledge_warnings,
        "Approved import job for execution"
    );

    Ok(Json(ApproveImportResponse {
        job_id,
        status: ImportJobStatus::Importing,
        message: "Import approved and started. You will be notified when complete.".to_string(),
        estimated_seconds: Some(180),
    }))
}

/// Trigger validation for an import job.
async fn validate_import(
    State(_state): State<AppState>,
    user: AuthUser,
    Path(job_id): Path<Uuid>,
) -> Result<Json<ImportJobStatusResponse>, (StatusCode, String)> {
    let org_id = user.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "Organization context required".to_string(),
    ))?;

    tracing::info!(
        org_id = %org_id,
        user_id = %user.user_id,
        job_id = %job_id,
        "Triggered import validation"
    );

    // In a real implementation, queue validation job
    let status = ImportJobStatusResponse {
        id: job_id,
        status: ImportJobStatus::Validating,
        filename: "import_file.csv".to_string(),
        template_name: "Template".to_string(),
        progress_percent: 0,
        total_rows: Some(150),
        processed_rows: 0,
        successful_rows: 0,
        failed_rows: 0,
        skipped_rows: 0,
        error_summary: None,
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
        estimated_remaining_seconds: Some(30),
    };

    Ok(Json(status))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate sample template data for demonstration.
/// Note: In production, this would query actual templates from the database.
fn generate_sample_templates(
    _org_id: Uuid,
    data_type: Option<ImportDataType>,
    include_system: bool,
) -> Vec<ImportTemplateSummary> {
    let mut templates = Vec::new();

    if include_system {
        if data_type.is_none() || data_type == Some(ImportDataType::Buildings) {
            templates.push(ImportTemplateSummary {
                id: Uuid::new_v4(),
                name: "Buildings Import".to_string(),
                data_type: ImportDataType::Buildings,
                description: Some("Import building master data".to_string()),
                is_system_template: true,
                field_count: 12,
                updated_at: chrono::Utc::now(),
            });
        }
        if data_type.is_none() || data_type == Some(ImportDataType::Units) {
            templates.push(ImportTemplateSummary {
                id: Uuid::new_v4(),
                name: "Units Import".to_string(),
                data_type: ImportDataType::Units,
                description: Some("Import unit data".to_string()),
                is_system_template: true,
                field_count: 15,
                updated_at: chrono::Utc::now(),
            });
        }
        if data_type.is_none() || data_type == Some(ImportDataType::Residents) {
            templates.push(ImportTemplateSummary {
                id: Uuid::new_v4(),
                name: "Residents Import".to_string(),
                data_type: ImportDataType::Residents,
                description: Some("Import resident data".to_string()),
                is_system_template: true,
                field_count: 18,
                updated_at: chrono::Utc::now(),
            });
        }
    }

    templates
}

fn get_buildings_template_fields() -> Vec<ImportFieldMapping> {
    vec![
        ImportFieldMapping {
            field_name: "name".to_string(),
            display_label: "Building Name".to_string(),
            column_header: "Building Name".to_string(),
            data_type: FieldDataType::String,
            validation: FieldValidation {
                required: true,
                min_length: Some(1),
                max_length: Some(255),
                ..Default::default()
            },
            example_value: Some("Residential Building A".to_string()),
            description: Some("Official name of the building".to_string()),
            target_column: Some("name".to_string()),
            transformation: None,
        },
        ImportFieldMapping {
            field_name: "address_street".to_string(),
            display_label: "Street Address".to_string(),
            column_header: "Street Address".to_string(),
            data_type: FieldDataType::String,
            validation: FieldValidation {
                required: true,
                min_length: Some(1),
                max_length: Some(500),
                ..Default::default()
            },
            example_value: Some("123 Main Street".to_string()),
            description: Some("Street name and number".to_string()),
            target_column: Some("address_street".to_string()),
            transformation: None,
        },
        ImportFieldMapping {
            field_name: "address_city".to_string(),
            display_label: "City".to_string(),
            column_header: "City".to_string(),
            data_type: FieldDataType::String,
            validation: FieldValidation {
                required: true,
                min_length: Some(1),
                max_length: Some(100),
                ..Default::default()
            },
            example_value: Some("Bratislava".to_string()),
            description: Some("City name".to_string()),
            target_column: Some("address_city".to_string()),
            transformation: None,
        },
        ImportFieldMapping {
            field_name: "address_postal_code".to_string(),
            display_label: "Postal Code".to_string(),
            column_header: "Postal Code".to_string(),
            data_type: FieldDataType::String,
            validation: FieldValidation {
                required: true,
                pattern: Some(r"^\d{3}\s?\d{2}$".to_string()),
                message: Some("Postal code must be in format XXXXX or XXX XX".to_string()),
                ..Default::default()
            },
            example_value: Some("831 02".to_string()),
            description: Some("Postal/ZIP code".to_string()),
            target_column: Some("address_postal_code".to_string()),
            transformation: Some("normalize_postal_code".to_string()),
        },
        ImportFieldMapping {
            field_name: "total_units".to_string(),
            display_label: "Total Units".to_string(),
            column_header: "Total Units".to_string(),
            data_type: FieldDataType::Integer,
            validation: FieldValidation {
                required: false,
                min_value: Some(1.0),
                max_value: Some(10000.0),
                ..Default::default()
            },
            example_value: Some("24".to_string()),
            description: Some("Number of units in the building".to_string()),
            target_column: Some("total_units".to_string()),
            transformation: None,
        },
        ImportFieldMapping {
            field_name: "year_built".to_string(),
            display_label: "Year Built".to_string(),
            column_header: "Year Built".to_string(),
            data_type: FieldDataType::Integer,
            validation: FieldValidation {
                required: false,
                min_value: Some(1800.0),
                max_value: Some(2100.0),
                ..Default::default()
            },
            example_value: Some("1985".to_string()),
            description: Some("Year the building was constructed".to_string()),
            target_column: Some("year_built".to_string()),
            transformation: None,
        },
    ]
}

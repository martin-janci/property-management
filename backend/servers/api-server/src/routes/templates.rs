//! Document template routes (Epic 7B: Story 7B.2 - Document Templates & Generation).

use crate::state::AppState;
use api_core::{AuthUser, RlsConnection, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    document_category, placeholder_type, template_type, CreateTemplate, DocumentTemplate,
    GenerateDocumentRequest, GenerateDocumentResponse, TemplateListQuery, TemplatePlaceholder,
    TemplateSummary, TemplateWithDetails, UpdateTemplate,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed template name length.
const MAX_TEMPLATE_NAME_LENGTH: usize = 255;

/// Maximum allowed template content length (1MB).
const MAX_TEMPLATE_CONTENT_LENGTH: usize = 1024 * 1024;

// ============================================================================
// Response Types
// ============================================================================

/// Response for template creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for template list with pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateListResponse {
    pub templates: Vec<TemplateSummary>,
    pub count: usize,
    pub total: i64,
}

/// Response for template details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateDetailResponse {
    pub template: TemplateWithDetails,
}

/// Response for template action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateActionResponse {
    pub message: String,
    pub template: DocumentTemplate,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating a template.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub description: Option<String>,
    pub template_type: String,
    pub content: String,
    pub placeholders: Vec<TemplatePlaceholder>,
}

/// Request for updating a template.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub template_type: Option<String>,
    pub content: Option<String>,
    pub placeholders: Option<Vec<TemplatePlaceholder>>,
}

/// Query for listing templates.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListTemplatesQuery {
    pub template_type: Option<String>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Router
// ============================================================================

/// Create templates router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_template))
        .route("/", get(list_templates))
        .route("/{id}", get(get_template))
        .route("/{id}", put(update_template))
        .route("/{id}", delete(delete_template))
        .route("/{id}/generate", post(generate_document))
}

// ============================================================================
// Handlers
// ============================================================================

/// Create a new template (Story 7B.2).
#[utoipa::path(
    post,
    path = "/api/v1/templates",
    request_body = CreateTemplateRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Template created", body = CreateTemplateResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Templates"
)]
async fn create_template(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<CreateTemplateResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;
    let org_id = tenant.tenant_id;

    // Only managers can create templates
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can create templates",
            )),
        ));
    }

    // Validate name
    if req.name.is_empty() || req.name.len() > MAX_TEMPLATE_NAME_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Template name must be 1-{} characters",
                    MAX_TEMPLATE_NAME_LENGTH
                ),
            )),
        ));
    }

    // Validate content length
    if req.content.len() > MAX_TEMPLATE_CONTENT_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Template content exceeds maximum length of 1MB",
            )),
        ));
    }

    // Validate template type
    if !template_type::ALL.contains(&req.template_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Invalid template_type")),
        ));
    }

    // Validate placeholders
    for placeholder in &req.placeholders {
        if placeholder.name.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Placeholder name cannot be empty",
                )),
            ));
        }
        if !placeholder_type::ALL.contains(&placeholder.placeholder_type.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Invalid placeholder type '{}' for '{}'",
                        placeholder.placeholder_type, placeholder.name
                    ),
                )),
            ));
        }
    }

    // Check for duplicate name
    if state
        .document_template_repo
        .name_exists(org_id, &req.name, None)
        .await
        .unwrap_or(false)
    {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "DUPLICATE_NAME",
                "A template with this name already exists",
            )),
        ));
    }

    let data = CreateTemplate {
        organization_id: org_id,
        name: req.name,
        description: req.description,
        template_type: req.template_type,
        content: req.content,
        placeholders: req.placeholders,
        created_by: user_id,
    };

    match state.document_template_repo.create(data).await {
        Ok(template) => Ok((
            StatusCode::CREATED,
            Json(CreateTemplateResponse {
                id: template.id,
                message: "Template created successfully".to_string(),
            }),
        )),
        Err(e) => {
            tracing::error!("Failed to create template: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create template",
                )),
            ))
        }
    }
}

/// List templates.
#[utoipa::path(
    get,
    path = "/api/v1/templates",
    params(ListTemplatesQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Template list", body = TemplateListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Templates"
)]
async fn list_templates(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Query(query): Query<ListTemplatesQuery>,
) -> Result<Json<TemplateListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;

    let list_query = TemplateListQuery {
        template_type: query.template_type,
        search: query.search,
        limit: query.limit,
        offset: query.offset,
    };

    let templates = state
        .document_template_repo
        .list(org_id, list_query.clone())
        .await
        .unwrap_or_default();

    let total = state
        .document_template_repo
        .count(org_id, list_query)
        .await
        .unwrap_or(0);

    let count = templates.len();
    Ok(Json(TemplateListResponse {
        templates,
        count,
        total,
    }))
}

/// Get template details.
#[utoipa::path(
    get,
    path = "/api/v1/templates/{id}",
    params(
        ("id" = Uuid, Path, description = "Template ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Template details", body = TemplateDetailResponse),
        (status = 404, description = "Template not found", body = ErrorResponse),
    ),
    tag = "Templates"
)]
async fn get_template(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<TemplateDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .document_template_repo
        .find_by_id_with_details(id, tenant.tenant_id)
        .await
    {
        Ok(Some(template)) => {
            // Defense-in-depth: the query is already org-scoped (PAP-63), so a
            // cross-tenant row can never reach here. Keep the guard as a belt.
            if template.template.organization_id != tenant.tenant_id {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
                ));
            }
            Ok(Json(TemplateDetailResponse { template }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get template: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get template",
                )),
            ))
        }
    }
}

/// Update a template.
#[utoipa::path(
    put,
    path = "/api/v1/templates/{id}",
    params(
        ("id" = Uuid, Path, description = "Template ID")
    ),
    request_body = UpdateTemplateRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Template updated", body = TemplateActionResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Template not found", body = ErrorResponse),
    ),
    tag = "Templates"
)]
async fn update_template(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTemplateRequest>,
) -> Result<Json<TemplateActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only managers can update templates
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update templates",
            )),
        ));
    }

    // Check template exists within the caller's organization (PAP-63)
    let existing = match state
        .document_template_repo
        .find_by_id(id, tenant.tenant_id)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find template: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find template",
                )),
            ));
        }
    };

    // Validate name if provided
    if let Some(ref name) = req.name {
        if name.is_empty() || name.len() > MAX_TEMPLATE_NAME_LENGTH {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Template name must be 1-{} characters",
                        MAX_TEMPLATE_NAME_LENGTH
                    ),
                )),
            ));
        }

        // Check for duplicate name (excluding current template)
        if state
            .document_template_repo
            .name_exists(existing.organization_id, name, Some(id))
            .await
            .unwrap_or(false)
        {
            return Err((
                StatusCode::CONFLICT,
                Json(ErrorResponse::new(
                    "DUPLICATE_NAME",
                    "A template with this name already exists",
                )),
            ));
        }
    }

    // Validate template type if provided
    if let Some(ref t_type) = req.template_type {
        if !template_type::ALL.contains(&t_type.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", "Invalid template_type")),
            ));
        }
    }

    let data = UpdateTemplate {
        name: req.name,
        description: req.description,
        template_type: req.template_type,
        content: req.content,
        placeholders: req.placeholders,
    };

    match state
        .document_template_repo
        .update(id, tenant.tenant_id, data)
        .await
    {
        Ok(template) => Ok(Json(TemplateActionResponse {
            message: "Template updated".to_string(),
            template,
        })),
        Err(e) => {
            tracing::error!("Failed to update template: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update template",
                )),
            ))
        }
    }
}

/// Delete a template (soft delete).
#[utoipa::path(
    delete,
    path = "/api/v1/templates/{id}",
    params(
        ("id" = Uuid, Path, description = "Template ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Template not found", body = ErrorResponse),
    ),
    tag = "Templates"
)]
async fn delete_template(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Only managers can delete templates
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete templates",
            )),
        ));
    }

    // Check template exists within the caller's organization (PAP-63)
    match state
        .document_template_repo
        .find_by_id(id, tenant.tenant_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find template: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find template",
                )),
            ));
        }
    }

    match state
        .document_template_repo
        .delete(id, tenant.tenant_id)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            tracing::error!("Failed to delete template: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete template",
                )),
            ))
        }
    }
}

/// Generate a document from a template.
#[utoipa::path(
    post,
    path = "/api/v1/templates/{id}/generate",
    params(
        ("id" = Uuid, Path, description = "Template ID")
    ),
    request_body = GenerateDocumentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Document generated", body = GenerateDocumentResponse),
        (status = 400, description = "Invalid request - missing required placeholders", body = ErrorResponse),
        (status = 404, description = "Template not found", body = ErrorResponse),
    ),
    tag = "Templates"
)]
async fn generate_document(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<GenerateDocumentRequest>,
) -> Result<(StatusCode, Json<GenerateDocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;
    let org_id = tenant.tenant_id;

    // Get template (scoped to the caller's organization — PAP-63)
    let template = match state.document_template_repo.find_by_id(id, org_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find template: {}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find template",
                )),
            ));
        }
    };

    // Validate required placeholders
    if let Err(missing) = template.validate_values(&req.values) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "MISSING_PLACEHOLDERS",
                format!(
                    "Missing required placeholder values: {}",
                    missing.join(", ")
                ),
            )),
        ));
    }

    // Validate category
    if !document_category::ALL.contains(&req.category.as_str()) {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Invalid category")),
        ));
    }

    // Generate content by replacing placeholders
    let generated_content = template.generate_content(&req.values);

    // Create a document with the generated content
    // Note: In a full implementation, this would create an actual file (e.g., PDF)
    // For now, we store the content as a text file reference
    let file_key = format!("generated/{}/{}.md", org_id, uuid::Uuid::new_v4());
    let file_name = format!("{}.md", req.title.replace(['/', '\\'], "_"));

    let create_doc = db::models::CreateDocument {
        organization_id: org_id,
        folder_id: req.folder_id,
        title: req.title,
        description: req.description,
        category: req.category,
        file_key,
        file_name,
        mime_type: "text/markdown".to_string(),
        size_bytes: generated_content.len() as i64,
        access_scope: None,
        access_target_ids: None,
        access_roles: None,
        created_by: user_id,
    };

    let create_result = state
        .document_repo
        .create_rls(&mut **rls.conn(), create_doc)
        .await;
    match create_result {
        Ok(document) => {
            rls.release().await;
            Ok((
                StatusCode::CREATED,
                Json(GenerateDocumentResponse {
                    document_id: document.id,
                    message: format!("Document generated from template '{}'", template.name),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to create generated document: {}", e);
            rls.release().await;
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to generate document",
                )),
            ))
        }
    }
}

//! Document core routes — CRUD, download, preview, move, and access control.

//! Document routes (Epic 7A: Basic Document Management, Epic 7B: Document Versioning, Epic 28: Document Intelligence, Epic 92: Intelligent Document Generation).

use crate::state::AppState;
use api_core::{extractors::RlsConnection, AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use db::models::{
    access_scope, document_category, CreateDocument, Document, DocumentClassificationHistory,
    DocumentFolder, DocumentIntelligenceStats, DocumentListQuery, DocumentSummary, DocumentVersion, DocumentVersionHistory,
    DocumentWithDetails, FolderTreeNode, FolderWithCount,
    MoveDocument, ShareWithDocument, UpdateDocument, ALLOWED_MIME_TYPES,
    MAX_FILE_SIZE,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed title length (characters).
pub const MAX_TITLE_LENGTH: usize = 500;

/// Maximum allowed description length (characters).
pub const MAX_DESCRIPTION_LENGTH: usize = 5000;

/// Maximum allowed folder name length (characters).
pub const MAX_FOLDER_NAME_LENGTH: usize = 255;

// ============================================================================
// Response Types
// ============================================================================

/// Response for document creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateDocumentResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for document list with pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentSummary>,
    pub count: usize,
    pub total: i64,
}

/// Response for document details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentDetailResponse {
    pub document: DocumentWithDetails,
}

/// Response for document action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DocumentActionResponse {
    pub message: String,
    pub document: Document,
}

/// Response for folder creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateFolderResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for folder list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FolderListResponse {
    pub folders: Vec<FolderWithCount>,
}

/// Response for folder tree.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FolderTreeResponse {
    pub tree: Vec<FolderTreeNode>,
}

/// Response for folder details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FolderDetailResponse {
    pub folder: DocumentFolder,
}

/// Response for folder action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FolderActionResponse {
    pub message: String,
    pub folder: DocumentFolder,
}

/// Response for share creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateShareResponse {
    pub id: Uuid,
    pub share_token: Option<String>,
    pub share_url: Option<String>,
    pub message: String,
}

/// Response for share list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ShareListResponse {
    pub shares: Vec<ShareWithDocument>,
}

/// Response for download/preview URL.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UrlResponse {
    pub url: String,
    pub expires_at: DateTime<Utc>,
}

/// Response for shared document access (no auth required).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SharedDocumentResponse {
    pub document: DocumentSummary,
    pub download_url: String,
    pub preview_url: Option<String>,
}

/// Response for version list (Story 7B.1).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VersionHistoryResponse {
    pub history: DocumentVersionHistory,
}

/// Response for single version (Story 7B.1).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VersionResponse {
    pub version: DocumentVersion,
}

/// Response for creating/restoring a version (Story 7B.1).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateVersionResponse {
    pub id: Uuid,
    pub version_number: i32,
    pub message: String,
}

// ============================================================================
// Document Intelligence Response Types (Epic 28)
// ============================================================================

/// Response for OCR reprocess request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OcrReprocessResponse {
    pub message: String,
    pub queue_id: Option<Uuid>,
}

/// Response for document classification.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClassificationResponse {
    pub document_id: Uuid,
    pub predicted_category: Option<String>,
    pub confidence: Option<f64>,
    pub classified_at: Option<DateTime<Utc>>,
    pub accepted: Option<bool>,
}

/// Response for classification history.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClassificationHistoryResponse {
    pub history: Vec<DocumentClassificationHistory>,
}

/// Response for summarization request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SummarizationResponse {
    pub message: String,
    pub queue_id: Uuid,
}

/// Response for intelligence stats.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct IntelligenceStatsResponse {
    pub stats: Vec<DocumentIntelligenceStats>,
}

// ============================================================================
// Epic 92.3: AI Document Summarization Types
// ============================================================================

/// Request for AI-powered document summarization.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AiSummarizeRequest {
    /// Target summary length (short, medium, long)
    pub summary_length: Option<String>,
    /// Language for the summary (sk, cs, de, en)
    pub language: Option<String>,
    /// Whether to extract key points
    #[serde(default = "default_true")]
    pub extract_key_points: bool,
}

fn default_true() -> bool {
    true
}

/// Response for AI-powered document summarization.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AiSummarizeResponse {
    pub document_id: Uuid,
    pub summary: String,
    pub key_points: Vec<String>,
    pub word_count: usize,
    pub tokens_used: i32,
    pub processing_time_ms: u64,
    pub provider: String,
    pub model: String,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating a document.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub folder_id: Option<Uuid>,
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub access_scope: Option<String>,
    pub access_target_ids: Option<Vec<Uuid>>,
    pub access_roles: Option<Vec<String>>,
}

/// Request for updating a document.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateDocumentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub folder_id: Option<Uuid>,
    pub access_scope: Option<String>,
    pub access_target_ids: Option<Vec<Uuid>>,
    pub access_roles: Option<Vec<String>>,
}

/// Request for updating document access.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateAccessRequest {
    pub access_scope: String,
    pub access_target_ids: Option<Vec<Uuid>>,
    pub access_roles: Option<Vec<String>>,
}

/// Request for moving a document.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MoveDocumentRequest {
    pub folder_id: Option<Uuid>,
}

/// Request for creating a folder.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateFolderRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

/// Request for updating a folder.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
}

/// Request for deleting a folder.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteFolderRequest {
    #[serde(default)]
    pub cascade: bool,
}

/// Request for creating a share.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateShareRequest {
    pub share_type: String,
    pub target_id: Option<Uuid>,
    pub target_role: Option<String>,
    pub password: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request for accessing a password-protected share.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccessShareRequest {
    pub password: String,
}

/// Request for uploading a new document version (Story 7B.1).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadVersionRequest {
    pub file_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
}

/// Query for listing documents.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListDocumentsQuery {
    pub folder_id: Option<Uuid>,
    pub category: Option<String>,
    pub created_by: Option<Uuid>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query for listing folders.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct ListFoldersQuery {
    pub parent_id: Option<Uuid>,
}

// ============================================================================
// Document Intelligence Request Types (Epic 28)
// ============================================================================

/// Request for full-text document search.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchDocumentsRequest {
    pub query: String,
    pub folder_id: Option<Uuid>,
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Request for classification feedback.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ClassificationFeedbackRequest {
    pub accepted: bool,
    pub correct_category: Option<String>,
}

/// Query for intelligence stats.
#[derive(Debug, Serialize, Deserialize, ToSchema, Default, utoipa::IntoParams)]
pub struct IntelligenceStatsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

// ============================================================================
// Router
// ============================================================================

/// Create documents core router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Document CRUD
        .route("/", post(create_document))
        .route("/", get(list_documents))
        .route("/{id}", get(get_document))
        .route("/{id}", put(update_document))
        .route("/{id}", delete(delete_document))
        // Document actions
        .route("/{id}/move", post(move_document))
        .route("/{id}/access", put(update_document_access))
        // Download/Preview (Story 7A.4)
        .route("/{id}/download", get(get_download_url))
        .route("/{id}/preview", get(get_preview_url))
}

// ============================================================================
// Document Handlers
// ============================================================================

/// Create a new document (Story 7A.1).
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    request_body = CreateDocumentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Document created", body = CreateDocumentResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn create_document(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<CreateDocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;
    let org_id = tenant.tenant_id;

    // Validate title length
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

    // Validate description length
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

    // Validate file size
    if req.size_bytes > MAX_FILE_SIZE {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "FILE_TOO_LARGE",
                format!(
                    "File exceeds maximum size of {} bytes (50MB)",
                    MAX_FILE_SIZE
                ),
            )),
        ));
    }

    // Validate MIME type
    if !ALLOWED_MIME_TYPES.contains(&req.mime_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "UNSUPPORTED_FILE_TYPE",
                format!(
                    "File type '{}' is not supported. Allowed types: PDF, DOC, DOCX, XLS, XLSX, PNG, JPG, GIF, WEBP, TXT",
                    req.mime_type
                ),
            )),
        ));
    }

    // Validate category
    if !document_category::ALL.contains(&req.category.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Invalid category")),
        ));
    }

    // Validate access scope if provided
    if let Some(ref scope) = req.access_scope {
        if !access_scope::ALL.contains(&scope.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", "Invalid access_scope")),
            ));
        }

        // Validate access_target_ids are required for non-organization scope
        if scope != access_scope::ORGANIZATION
            && scope != access_scope::ROLE
            && req.access_target_ids.as_ref().is_none_or(|v| v.is_empty())
        {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "access_target_ids required for building, unit, or users scope",
                )),
            ));
        }

        // Validate access_roles are required for role scope
        if scope == access_scope::ROLE && req.access_roles.as_ref().is_none_or(|v| v.is_empty()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "access_roles required for role scope",
                )),
            ));
        }
    }

    let data = CreateDocument {
        organization_id: org_id,
        folder_id: req.folder_id,
        title: req.title,
        description: req.description,
        category: req.category,
        file_key: req.file_key,
        file_name: req.file_name,
        mime_type: req.mime_type,
        size_bytes: req.size_bytes,
        access_scope: req.access_scope,
        access_target_ids: req.access_target_ids,
        access_roles: req.access_roles,
        created_by: user_id,
    };

    match state
        .document_repo
        .create_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(document) => {
            rls.release().await;
            Ok((
                StatusCode::CREATED,
                Json(CreateDocumentResponse {
                    id: document.id,
                    message: "Document created successfully".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to create document: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create document",
                )),
            ))
        }
    }
}

/// List documents with filters.
#[utoipa::path(
    get,
    path = "/api/v1/documents",
    params(ListDocumentsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Document list", body = DocumentListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn list_documents(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<DocumentListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;
    let user_id = auth.user_id;

    // For managers, show all documents; for others, show only accessible documents
    let is_manager = tenant.role.is_manager();

    let list_query = DocumentListQuery {
        folder_id: query.folder_id,
        category: query.category.clone(),
        created_by: query.created_by,
        search: query.search.clone(),
        limit: query.limit,
        offset: query.offset,
    };

    let (documents, total) = if is_manager {
        let docs = state
            .document_repo
            .list_rls(&mut **rls.conn(), org_id, list_query.clone())
            .await
            .unwrap_or_default();
        let total = state
            .document_repo
            .count_rls(&mut **rls.conn(), org_id, list_query)
            .await
            .unwrap_or(0);
        (docs, total)
    } else {
        // Use simplified access control for non-managers
        // Shows: org-wide documents + own documents + role-based documents
        // TODO: Full implementation needs building/unit context from TenantContext
        let user_role = tenant.role.to_string().to_lowercase().replace(' ', "_");
        let docs = state
            .document_repo
            .list_accessible_simple_rls(
                &mut **rls.conn(),
                org_id,
                user_id,
                &user_role,
                list_query.clone(),
            )
            .await
            .unwrap_or_default();
        let total = state
            .document_repo
            .count_accessible_simple_rls(&mut **rls.conn(), org_id, user_id, &user_role, list_query)
            .await
            .unwrap_or(0);
        (docs, total)
    };

    let count = documents.len();
    rls.release().await;
    Ok(Json(DocumentListResponse {
        documents,
        count,
        total,
    }))
}

/// Get document details.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Document details", body = DocumentDetailResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn get_document(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<DocumentDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .document_repo
        .find_by_id_with_details_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(document)) => {
            rls.release().await;
            Ok(Json(DocumentDetailResponse { document }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get document: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get document",
                )),
            ))
        }
    }
}

/// Update a document.
#[utoipa::path(
    put,
    path = "/api/v1/documents/{id}",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = UpdateDocumentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Document updated", body = DocumentActionResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn update_document(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDocumentRequest>,
) -> Result<Json<DocumentActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check document exists
    let existing = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Only creator or manager can update
    if existing.created_by != auth.user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can update",
            )),
        ));
    }

    // Validate inputs
    if let Some(ref title) = req.title {
        if title.len() > MAX_TITLE_LENGTH {
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
    }

    if let Some(ref category) = req.category {
        if !document_category::ALL.contains(&category.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", "Invalid category")),
            ));
        }
    }

    let data = UpdateDocument {
        title: req.title,
        description: req.description,
        category: req.category,
        folder_id: req.folder_id,
        access_scope: req.access_scope,
        access_target_ids: req.access_target_ids,
        access_roles: req.access_roles,
    };

    match state
        .document_repo
        .update_rls(&mut **rls.conn(), id, data)
        .await
    {
        Ok(document) => {
            rls.release().await;
            Ok(Json(DocumentActionResponse {
                message: "Document updated".to_string(),
                document,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to update document: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update document",
                )),
            ))
        }
    }
}

/// Delete a document (soft delete).
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{id}",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Document deleted"),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn delete_document(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Check document exists
    let existing = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Only creator or manager can delete
    if existing.created_by != auth.user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can delete",
            )),
        ));
    }

    match state.document_repo.delete_rls(&mut **rls.conn(), id).await {
        Ok(_) => {
            rls.release().await;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!("Failed to delete document: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete document",
                )),
            ))
        }
    }
}

/// Move a document to a folder.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/move",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = MoveDocumentRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Document moved", body = DocumentActionResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn move_document(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<MoveDocumentRequest>,
) -> Result<Json<DocumentActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check document exists
    let existing = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Only creator or manager can move
    if existing.created_by != auth.user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can move",
            )),
        ));
    }

    let data = MoveDocument {
        document_id: id,
        folder_id: req.folder_id,
    };

    match state
        .document_repo
        .move_document_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(document) => {
            rls.release().await;
            Ok(Json(DocumentActionResponse {
                message: "Document moved".to_string(),
                document,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to move document: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to move document",
                )),
            ))
        }
    }
}

/// Update document access permissions.
#[utoipa::path(
    put,
    path = "/api/v1/documents/{id}/access",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = UpdateAccessRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Access updated", body = DocumentActionResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn update_document_access(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAccessRequest>,
) -> Result<Json<DocumentActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Check document exists
    let existing = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Only creator or manager can update access
    if existing.created_by != auth.user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can update access",
            )),
        ));
    }

    // Validate access scope
    if !access_scope::ALL.contains(&req.access_scope.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Invalid access_scope")),
        ));
    }

    let data = UpdateDocument {
        title: None,
        description: None,
        category: None,
        folder_id: None,
        access_scope: Some(req.access_scope),
        access_target_ids: req.access_target_ids,
        access_roles: req.access_roles,
    };

    match state
        .document_repo
        .update_rls(&mut **rls.conn(), id, data)
        .await
    {
        Ok(document) => {
            rls.release().await;
            Ok(Json(DocumentActionResponse {
                message: "Document access updated".to_string(),
                document,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to update document access: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update document access",
                )),
            ))
        }
    }
}

// ============================================================================
// Download/Preview Handlers (Story 7A.4)
// ============================================================================

/// Get download URL for a document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/download",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Download URL", body = UrlResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn get_download_url(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<UrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Story 84.1: Generate S3 presigned URL for download
    // Security: Storage service must be configured to serve documents
    let (url, expires_at) = match integrations::StorageService::from_env() {
        Ok(storage) => {
            match storage
                .generate_download_url(
                    &document.file_key,
                    &document.file_name,
                    &document.mime_type,
                    None, // Use default 15 minute expiration
                )
                .await
            {
                Ok(presigned) => (presigned.url, presigned.expires_at),
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        file_key = %document.file_key,
                        "Failed to generate presigned URL"
                    );
                    return Err((
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(ErrorResponse::new(
                            "STORAGE_ERROR",
                            "Unable to generate download URL. Please try again later.",
                        )),
                    ));
                }
            }
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                "Storage service not configured - document downloads unavailable"
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "STORAGE_NOT_CONFIGURED",
                    "Document storage is not configured. Please contact support.",
                )),
            ));
        }
    };

    rls.release().await;
    Ok(Json(UrlResponse { url, expires_at }))
}

/// Get preview URL for a document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/preview",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Preview URL", body = UrlResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn get_preview_url(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<UrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    if !document.supports_preview() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PREVIEW_NOT_SUPPORTED",
                "Preview is not supported for this file type. Use download instead.",
            )),
        ));
    }

    // Story 84.1: Generate S3 presigned URL for inline preview
    // Security: Storage service must be configured to serve previews
    let (url, expires_at) = match integrations::StorageService::from_env() {
        Ok(storage) => {
            // For preview, we use a longer expiration time
            match storage
                .generate_download_url(
                    &document.file_key,
                    &document.file_name,
                    &document.mime_type,
                    Some(3600), // 1 hour for preview
                )
                .await
            {
                Ok(presigned) => (presigned.url, presigned.expires_at),
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        file_key = %document.file_key,
                        "Failed to generate presigned preview URL"
                    );
                    return Err((
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(ErrorResponse::new(
                            "STORAGE_ERROR",
                            "Unable to generate preview URL. Please try again later.",
                        )),
                    ));
                }
            }
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                "Storage service not configured - document previews unavailable"
            );
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "STORAGE_NOT_CONFIGURED",
                    "Document storage is not configured. Please contact support.",
                )),
            ));
        }
    };

    rls.release().await;
    Ok(Json(UrlResponse { url, expires_at }))
}

// ============================================================================

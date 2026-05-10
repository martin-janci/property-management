//! Document routes (Epic 7A: Basic Document Management, Epic 7B: Document Versioning, Epic 28: Document Intelligence, Epic 92: Intelligent Document Generation).

use crate::state::AppState;
use api_core::{extractors::RlsConnection, AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use common::errors::ErrorResponse;
use db::models::{
    access_scope, document_category, share_type, ClassificationFeedback, CreateDocument,
    CreateDocumentVersion, CreateFolder, CreateShare, Document, DocumentClassificationHistory,
    DocumentFolder, DocumentIntelligenceStats, DocumentListQuery, DocumentSearchRequest,
    DocumentSearchResponse, DocumentSummary, DocumentVersion, DocumentVersionHistory,
    DocumentWithDetails, FolderTreeNode, FolderWithCount, GenerateSummaryRequest, LogShareAccess,
    MoveDocument, ShareWithDocument, UpdateDocument, UpdateFolder, ALLOWED_MIME_TYPES,
    MAX_FILE_SIZE,
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

/// Maximum allowed folder name length (characters).
const MAX_FOLDER_NAME_LENGTH: usize = 255;

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

/// Create documents router.
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
        // Versioning (Story 7B.1)
        .route("/{id}/versions", get(get_version_history))
        .route("/{id}/versions", post(create_version))
        .route("/{id}/versions/{version_id}", get(get_version))
        .route("/{id}/versions/{version_id}/restore", post(restore_version))
        // Shares (Story 7A.5)
        .route("/{id}/shares", get(list_shares))
        .route("/{id}/shares", post(create_share))
        .route("/{id}/shares/{share_id}", delete(revoke_share))
        // Folders (Story 7A.2)
        .route("/folders", get(list_folders))
        .route("/folders", post(create_folder))
        .route("/folders/tree", get(get_folder_tree))
        .route("/folders/{id}", get(get_folder))
        .route("/folders/{id}", put(update_folder))
        .route("/folders/{id}", delete(delete_folder))
        // Document Intelligence (Epic 28)
        // Story 28.1: OCR
        .route("/{id}/ocr/reprocess", post(reprocess_ocr))
        // Story 28.2: Full-text search
        .route("/search", post(search_documents))
        // Story 28.3: Auto-classification
        .route("/{id}/classification", get(get_classification))
        .route(
            "/{id}/classification/feedback",
            post(submit_classification_feedback),
        )
        .route(
            "/{id}/classification/history",
            get(get_classification_history),
        )
        // Story 28.4: Summarization
        .route("/{id}/summarize", post(request_summarization))
        // Story 92.3: LLM-powered summarization (sync)
        .route("/{id}/ai-summarize", post(ai_summarize_document))
        // Intelligence stats
        .route("/intelligence/stats", get(get_intelligence_stats))
    // Public shared document access (no auth required - separate route in main.rs)
    // .route("/shared/{token}", get(access_shared_document))
    // .route("/shared/{token}/access", post(access_protected_share))
}

/// Create public routes for shared documents (no auth required).
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/shared/{token}", get(access_shared_document))
        .route("/shared/{token}/access", post(access_protected_share))
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
            match storage.generate_download_url(
                &document.file_key,
                &document.file_name,
                &document.mime_type,
                None, // Use default 15 minute expiration
            ) {
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
            match storage.generate_download_url(
                &document.file_key,
                &document.file_name,
                &document.mime_type,
                Some(3600), // 1 hour for preview
            ) {
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
// Version Handlers (Story 7B.1)
// ============================================================================

/// Get version history for a document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/versions",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Version history", body = VersionHistoryResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn get_version_history(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<VersionHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .document_repo
        .get_version_history_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(history) => {
            rls.release().await;
            Ok(Json(VersionHistoryResponse { history }))
        }
        Err(sqlx::Error::RowNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get version history: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get version history",
                )),
            ))
        }
    }
}

/// Upload a new version of a document.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/versions",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = UploadVersionRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Version created", body = CreateVersionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn create_version(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UploadVersionRequest>,
) -> Result<(StatusCode, Json<CreateVersionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

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

    // Only creator or manager can upload new versions
    if existing.created_by != user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can upload new versions",
            )),
        ));
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
                format!("File type '{}' is not supported", req.mime_type),
            )),
        ));
    }

    let data = CreateDocumentVersion {
        file_key: req.file_key,
        file_name: req.file_name,
        mime_type: req.mime_type,
        size_bytes: req.size_bytes,
        created_by: user_id,
    };

    match state
        .document_repo
        .create_version_rls(&mut **rls.conn(), id, data)
        .await
    {
        Ok(new_version) => {
            rls.release().await;
            Ok((
                StatusCode::CREATED,
                Json(CreateVersionResponse {
                    id: new_version.id,
                    version_number: new_version.version_number,
                    message: format!(
                        "Version {} created successfully",
                        new_version.version_number
                    ),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to create version: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create version",
                )),
            ))
        }
    }
}

/// Get a specific version of a document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/versions/{version_id}",
    params(
        ("id" = Uuid, Path, description = "Document ID"),
        ("version_id" = Uuid, Path, description = "Version ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Version details", body = VersionResponse),
        (status = 404, description = "Version not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn get_version(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path((id, version_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<VersionResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .document_repo
        .get_version_rls(&mut **rls.conn(), id, version_id)
        .await
    {
        Ok(Some(version)) => {
            rls.release().await;
            Ok(Json(VersionResponse { version }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Version not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get version: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get version",
                )),
            ))
        }
    }
}

/// Restore a previous version to become the current version.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/versions/{version_id}/restore",
    params(
        ("id" = Uuid, Path, description = "Document ID"),
        ("version_id" = Uuid, Path, description = "Version ID to restore")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Version restored", body = CreateVersionResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Version not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn restore_version(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path((id, version_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<CreateVersionResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;

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

    // Only creator or manager can restore versions
    if existing.created_by != user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can restore versions",
            )),
        ));
    }

    // Check version exists in the same chain
    match state
        .document_repo
        .get_version_rls(&mut **rls.conn(), id, version_id)
        .await
    {
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Version not found in this document",
                )),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find version: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find version",
                )),
            ));
        }
        Ok(Some(_)) => {}
    }

    // TODO: Migrate restore_version to RLS pattern
    #[allow(deprecated)]
    match state
        .document_repo
        .restore_version(id, version_id, user_id)
        .await
    {
        Ok(new_version) => {
            rls.release().await;
            Ok((
                StatusCode::CREATED,
                Json(CreateVersionResponse {
                    id: new_version.id,
                    version_number: new_version.version_number,
                    message: format!(
                        "Version restored successfully as version {}",
                        new_version.version_number
                    ),
                }),
            ))
        }
        Err(sqlx::Error::RowNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Version not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to restore version: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to restore version",
                )),
            ))
        }
    }
}

// ============================================================================
// Folder Handlers (Story 7A.2)
// ============================================================================

/// List folders.
#[utoipa::path(
    get,
    path = "/api/v1/documents/folders",
    params(ListFoldersQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Folder list", body = FolderListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn list_folders(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<ListFoldersQuery>,
) -> Result<Json<FolderListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;

    match state
        .document_repo
        .get_folders_rls(&mut **rls.conn(), org_id, query.parent_id)
        .await
    {
        Ok(folders) => {
            rls.release().await;
            Ok(Json(FolderListResponse { folders }))
        }
        Err(e) => {
            tracing::error!("Failed to list folders: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list folders",
                )),
            ))
        }
    }
}

/// Get folder tree.
#[utoipa::path(
    get,
    path = "/api/v1/documents/folders/tree",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Folder tree", body = FolderTreeResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn get_folder_tree(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
) -> Result<Json<FolderTreeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;

    match state
        .document_repo
        .get_folder_tree_rls(&mut **rls.conn(), org_id)
        .await
    {
        Ok(tree) => {
            rls.release().await;
            Ok(Json(FolderTreeResponse { tree }))
        }
        Err(e) => {
            tracing::error!("Failed to get folder tree: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get folder tree",
                )),
            ))
        }
    }
}

/// Create a folder.
#[utoipa::path(
    post,
    path = "/api/v1/documents/folders",
    request_body = CreateFolderRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Folder created", body = CreateFolderResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn create_folder(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<CreateFolderRequest>,
) -> Result<(StatusCode, Json<CreateFolderResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Only managers can create folders
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can create folders",
            )),
        ));
    }

    // Validate name
    if req.name.is_empty() || req.name.len() > MAX_FOLDER_NAME_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Folder name must be 1-{} characters",
                    MAX_FOLDER_NAME_LENGTH
                ),
            )),
        ));
    }

    let org_id = tenant.tenant_id;
    let user_id = auth.user_id;

    let data = CreateFolder {
        organization_id: org_id,
        parent_id: req.parent_id,
        name: req.name,
        description: req.description,
        created_by: user_id,
    };

    match state
        .document_repo
        .create_folder_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(folder) => {
            rls.release().await;
            Ok((
                StatusCode::CREATED,
                Json(CreateFolderResponse {
                    id: folder.id,
                    message: "Folder created successfully".to_string(),
                }),
            ))
        }
        Err(e) => {
            // Check for depth violation
            if e.to_string().contains("Maximum folder depth") {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "MAX_DEPTH_EXCEEDED",
                        "Maximum folder depth of 5 levels exceeded",
                    )),
                ));
            }
            tracing::error!("Failed to create folder: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create folder",
                )),
            ))
        }
    }
}

/// Get folder details.
#[utoipa::path(
    get,
    path = "/api/v1/documents/folders/{id}",
    params(
        ("id" = Uuid, Path, description = "Folder ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Folder details", body = FolderDetailResponse),
        (status = 404, description = "Folder not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn get_folder(
    State(state): State<AppState>,
    _auth: AuthUser,
    _tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<FolderDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .document_repo
        .find_folder_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(folder)) => {
            rls.release().await;
            Ok(Json(FolderDetailResponse { folder }))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Folder not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get folder: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get folder")),
            ))
        }
    }
}

/// Update a folder.
#[utoipa::path(
    put,
    path = "/api/v1/documents/folders/{id}",
    params(
        ("id" = Uuid, Path, description = "Folder ID")
    ),
    request_body = UpdateFolderRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Folder updated", body = FolderActionResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Folder not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn update_folder(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFolderRequest>,
) -> Result<Json<FolderActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only managers can update folders
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update folders",
            )),
        ));
    }

    // Check folder exists
    match state
        .document_repo
        .find_folder_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Folder not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find folder: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find folder",
                )),
            ));
        }
    }

    // Validate name if provided
    if let Some(ref name) = req.name {
        if name.is_empty() || name.len() > MAX_FOLDER_NAME_LENGTH {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Folder name must be 1-{} characters",
                        MAX_FOLDER_NAME_LENGTH
                    ),
                )),
            ));
        }
    }

    // Validate parent_id to prevent circular references
    if let Some(new_parent_id) = req.parent_id {
        // Cannot set a folder as its own parent
        if new_parent_id == id {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "CIRCULAR_REFERENCE",
                    "A folder cannot be its own parent",
                )),
            ));
        }

        // Check that new parent is not a descendant of this folder
        match state
            .document_repo
            .is_descendant_of_rls(&mut **rls.conn(), new_parent_id, id)
            .await
        {
            Ok(true) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "CIRCULAR_REFERENCE",
                        "Cannot move a folder into one of its descendants",
                    )),
                ));
            }
            Ok(false) => {}
            Err(e) => {
                tracing::error!("Failed to check folder hierarchy: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to validate folder hierarchy",
                    )),
                ));
            }
        }
    }

    let data = UpdateFolder {
        name: req.name,
        description: req.description,
        parent_id: req.parent_id,
    };

    match state
        .document_repo
        .update_folder_rls(&mut **rls.conn(), id, data)
        .await
    {
        Ok(folder) => {
            rls.release().await;
            Ok(Json(FolderActionResponse {
                message: "Folder updated".to_string(),
                folder,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to update folder: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update folder",
                )),
            ))
        }
    }
}

/// Delete a folder.
#[utoipa::path(
    delete,
    path = "/api/v1/documents/folders/{id}",
    params(
        ("id" = Uuid, Path, description = "Folder ID")
    ),
    request_body = Option<DeleteFolderRequest>,
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Folder deleted"),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
        (status = 404, description = "Folder not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn delete_folder(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    body: Option<Json<DeleteFolderRequest>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Only managers can delete folders
    if !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete folders",
            )),
        ));
    }

    // Check folder exists
    match state
        .document_repo
        .find_folder_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Folder not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find folder: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find folder",
                )),
            ));
        }
    }

    let cascade = body.map(|b| b.cascade).unwrap_or(false);

    // Check if folder has documents and warn if not cascading
    if !cascade {
        let doc_count = state
            .document_repo
            .count_documents_in_folder_rls(&mut **rls.conn(), id)
            .await
            .unwrap_or(0);
        if doc_count > 0 {
            // Documents will be moved to root
            tracing::info!(
                folder_id = %id,
                document_count = doc_count,
                "Moving documents to root folder before deleting folder"
            );
        }
    }

    match state
        .document_repo
        .delete_folder_rls(&mut **rls.conn(), id, cascade)
        .await
    {
        Ok(_) => {
            rls.release().await;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!("Failed to delete folder: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete folder",
                )),
            ))
        }
    }
}

// ============================================================================
// Share Handlers (Story 7A.5)
// ============================================================================

/// List shares for a document.
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/shares",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Share list", body = ShareListResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn list_shares(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ShareListResponse>, (StatusCode, Json<ErrorResponse>)> {
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

    // Only creator or manager can view shares
    if existing.created_by != auth.user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can view shares",
            )),
        ));
    }

    match state
        .document_repo
        .get_shares_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(shares) => {
            rls.release().await;
            Ok(Json(ShareListResponse { shares }))
        }
        Err(e) => {
            tracing::error!("Failed to list shares: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list shares",
                )),
            ))
        }
    }
}

/// Create a share.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/shares",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = CreateShareRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Share created", body = CreateShareResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn create_share(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateShareRequest>,
) -> Result<(StatusCode, Json<CreateShareResponse>), (StatusCode, Json<ErrorResponse>)> {
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

    // Only creator or manager can create shares
    if existing.created_by != auth.user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can create shares",
            )),
        ));
    }

    // Validate share type
    if !share_type::ALL.contains(&req.share_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Invalid share_type")),
        ));
    }

    // Validate target_id for non-link shares
    if req.share_type != share_type::LINK && req.target_id.is_none() && req.target_role.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "target_id or target_role required for non-link shares",
            )),
        ));
    }

    let data = CreateShare {
        document_id: id,
        share_type: req.share_type.clone(),
        target_id: req.target_id,
        target_role: req.target_role,
        shared_by: auth.user_id,
        password: req.password,
        expires_at: req.expires_at,
    };

    match state
        .document_repo
        .create_share_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(share) => {
            let share_url = share
                .share_token
                .as_ref()
                .map(|token| format!("/documents/shared/{}", token));

            rls.release().await;
            Ok((
                StatusCode::CREATED,
                Json(CreateShareResponse {
                    id: share.id,
                    share_token: share.share_token,
                    share_url,
                    message: "Share created successfully".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to create share: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create share",
                )),
            ))
        }
    }
}

/// Revoke a share.
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{id}/shares/{share_id}",
    params(
        ("id" = Uuid, Path, description = "Document ID"),
        ("share_id" = Uuid, Path, description = "Share ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Share revoked"),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Share not found", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn revoke_share(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path((id, share_id)): Path<(Uuid, Uuid)>,
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

    // Only creator or manager can revoke shares
    if existing.created_by != auth.user_id && !tenant.role.is_manager() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only the document creator or managers can revoke shares",
            )),
        ));
    }

    // Check share exists
    match state
        .document_repo
        .find_share_by_id_rls(&mut **rls.conn(), share_id)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Share not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to find share: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to find share")),
            ));
        }
    }

    match state
        .document_repo
        .revoke_share_rls(&mut **rls.conn(), share_id)
        .await
    {
        Ok(_) => {
            rls.release().await;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!("Failed to revoke share: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to revoke share",
                )),
            ))
        }
    }
}

// ============================================================================
// Public Share Access (No Auth Required)
// ============================================================================

/// Access a shared document via token.
#[utoipa::path(
    get,
    path = "/api/v1/documents/shared/{token}",
    params(
        ("token" = String, Path, description = "Share token")
    ),
    responses(
        (status = 200, description = "Shared document access", body = SharedDocumentResponse),
        (status = 401, description = "Password required", body = ErrorResponse),
        (status = 404, description = "Share not found or expired", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn access_shared_document(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Path(token): Path<String>,
) -> Result<Json<SharedDocumentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ip_address = addr.ip().to_string();
    // Find share by token
    // TODO: This endpoint is public, no RLS context available
    #[allow(deprecated)]
    let share = match state.document_repo.find_share_by_token(&token).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Share not found or expired",
                )),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find share: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to access share",
                )),
            ));
        }
    };

    // Check if password protected
    if share.has_password() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new(
                "PASSWORD_REQUIRED",
                "This share is password protected",
            )),
        ));
    }

    // Get document - public endpoint, no RLS context
    #[allow(deprecated)]
    let document = match state.document_repo.find_by_id(share.document_id).await {
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
                    "Failed to access document",
                )),
            ));
        }
    };

    // Log access - important for audit trail but shouldn't fail the request
    // Public endpoint, no RLS context
    #[allow(deprecated)]
    if let Err(e) = state
        .document_repo
        .log_share_access(LogShareAccess {
            share_id: share.id,
            accessed_by: None,
            ip_address: Some(ip_address.clone()),
        })
        .await
    {
        tracing::warn!(
            share_id = %share.id,
            ip_address = %ip_address,
            error = %e,
            "Failed to log share access"
        );
    }

    // Generate URLs
    let download_url = format!("/api/v1/storage/{}", document.file_key);
    let preview_url = if document.supports_preview() {
        Some(format!("/api/v1/storage/preview/{}", document.file_key))
    } else {
        None
    };

    Ok(Json(SharedDocumentResponse {
        document: DocumentSummary {
            id: document.id,
            title: document.title,
            category: document.category,
            file_name: document.file_name,
            mime_type: document.mime_type,
            size_bytes: document.size_bytes,
            folder_id: document.folder_id,
            created_at: document.created_at,
        },
        download_url,
        preview_url,
    }))
}

/// Access a password-protected shared document.
#[utoipa::path(
    post,
    path = "/api/v1/documents/shared/{token}/access",
    params(
        ("token" = String, Path, description = "Share token")
    ),
    request_body = AccessShareRequest,
    responses(
        (status = 200, description = "Shared document access", body = SharedDocumentResponse),
        (status = 401, description = "Invalid password", body = ErrorResponse),
        (status = 404, description = "Share not found or expired", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn access_protected_share(
    State(state): State<AppState>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Path(token): Path<String>,
    Json(req): Json<AccessShareRequest>,
) -> Result<Json<SharedDocumentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ip_address = addr.ip().to_string();
    // Find share by token
    // TODO: This endpoint is public, no RLS context available
    #[allow(deprecated)]
    let share = match state.document_repo.find_share_by_token(&token).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Share not found or expired",
                )),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find share: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to access share",
                )),
            ));
        }
    };

    // Verify password
    if !state
        .document_repo
        .verify_share_password(share.id, &req.password)
        .await
        .unwrap_or(false)
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_PASSWORD", "Invalid password")),
        ));
    }

    // Get document - public endpoint, no RLS context
    #[allow(deprecated)]
    let document = match state.document_repo.find_by_id(share.document_id).await {
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
                    "Failed to access document",
                )),
            ));
        }
    };

    // Log access - important for audit trail but shouldn't fail the request
    // Public endpoint, no RLS context
    #[allow(deprecated)]
    if let Err(e) = state
        .document_repo
        .log_share_access(LogShareAccess {
            share_id: share.id,
            accessed_by: None,
            ip_address: Some(ip_address.clone()),
        })
        .await
    {
        tracing::warn!(
            share_id = %share.id,
            ip_address = %ip_address,
            error = %e,
            "Failed to log share access"
        );
    }

    // Generate URLs
    let download_url = format!("/api/v1/storage/{}", document.file_key);
    let preview_url = if document.supports_preview() {
        Some(format!("/api/v1/storage/preview/{}", document.file_key))
    } else {
        None
    };

    Ok(Json(SharedDocumentResponse {
        document: DocumentSummary {
            id: document.id,
            title: document.title,
            category: document.category,
            file_name: document.file_name,
            mime_type: document.mime_type,
            size_bytes: document.size_bytes,
            folder_id: document.folder_id,
            created_at: document.created_at,
        },
        download_url,
        preview_url,
    }))
}

// ============================================================================
// Document Intelligence Handlers (Epic 28)
// ============================================================================

/// Reprocess OCR for a document (Story 28.1).
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/ocr/reprocess",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "OCR reprocess queued", body = OcrReprocessResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn reprocess_ocr(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OcrReprocessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists and user has access
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

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Queue for OCR
    match state.document_repo.queue_for_ocr(id, Some(1)).await {
        Ok(queue_id) => {
            rls.release().await;
            Ok(Json(OcrReprocessResponse {
                message: "Document queued for OCR processing".to_string(),
                queue_id,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to queue document for OCR: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to queue document for OCR",
                )),
            ))
        }
    }
}

/// Full-text search across documents (Story 28.2).
#[utoipa::path(
    post,
    path = "/api/v1/documents/search",
    request_body = SearchDocumentsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Search results", body = DocumentSearchResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn search_documents(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<SearchDocumentsRequest>,
) -> Result<Json<DocumentSearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate query
    if req.query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Search query cannot be empty",
            )),
        ));
    }

    if req.query.len() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Search query too long (max 500 characters)",
            )),
        ));
    }

    let search_request = DocumentSearchRequest {
        query: req.query,
        organization_id: tenant.tenant_id,
        include_content: true,
        folder_id: req.folder_id,
        category: req.category,
        limit: req.limit,
        offset: req.offset,
    };

    match state.document_repo.full_text_search(search_request).await {
        Ok(response) => {
            rls.release().await;
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to search documents: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to search documents",
                )),
            ))
        }
    }
}

/// Get document classification (Story 28.3).
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/classification",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Classification info", body = ClassificationResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn get_classification(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ClassificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Find document with details
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

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Return response based on the document's category
    rls.release().await;
    Ok(Json(ClassificationResponse {
        document_id: id,
        predicted_category: Some(document.category.clone()),
        confidence: None,
        classified_at: None,
        accepted: None,
    }))
}

/// Submit classification feedback (Story 28.3).
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/classification/feedback",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = ClassificationFeedbackRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Feedback submitted", body = MessageResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn submit_classification_feedback(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<ClassificationFeedbackRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists
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

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Validate category if provided
    if let Some(ref cat) = req.correct_category {
        if !document_category::ALL.contains(&cat.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", "Invalid category")),
            ));
        }
    }

    let feedback = ClassificationFeedback {
        document_id: id,
        accepted: req.accepted,
        correct_category: req.correct_category,
        feedback_by: auth.user_id,
    };

    match state
        .document_repo
        .submit_classification_feedback(feedback)
        .await
    {
        Ok(()) => {
            rls.release().await;
            Ok(Json(MessageResponse {
                message: "Classification feedback submitted".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to submit classification feedback: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to submit feedback",
                )),
            ))
        }
    }
}

/// Get classification history for a document (Story 28.3).
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/classification/history",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Classification history", body = ClassificationHistoryResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn get_classification_history(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ClassificationHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists and belongs to tenant
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

    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    match state.document_repo.get_classification_history(id).await {
        Ok(history) => {
            rls.release().await;
            Ok(Json(ClassificationHistoryResponse { history }))
        }
        Err(e) => {
            tracing::error!("Failed to get classification history: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get classification history",
                )),
            ))
        }
    }
}

/// Request document summarization (Story 28.4).
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/summarize",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Summarization queued", body = SummarizationResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn request_summarization(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SummarizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists
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

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    let request = GenerateSummaryRequest {
        document_id: id,
        requested_by: auth.user_id,
        priority: Some(1),
    };

    match state.document_repo.queue_for_summarization(request).await {
        Ok(queue_id) => {
            rls.release().await;
            Ok(Json(SummarizationResponse {
                message: "Document queued for summarization".to_string(),
                queue_id,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to queue document for summarization: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to queue document for summarization",
                )),
            ))
        }
    }
}

/// Get document intelligence statistics.
#[utoipa::path(
    get,
    path = "/api/v1/documents/intelligence/stats",
    params(IntelligenceStatsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Intelligence statistics", body = IntelligenceStatsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn get_intelligence_stats(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<IntelligenceStatsQuery>,
) -> Result<Json<IntelligenceStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Parse dates, default to last 30 days
    let end_date = query
        .end_date
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Utc::now().date_naive());

    let start_date = query
        .start_date
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| end_date - chrono::Duration::days(30));

    match state
        .document_repo
        .get_intelligence_stats(tenant.tenant_id, start_date, end_date)
        .await
    {
        Ok(stats) => {
            rls.release().await;
            Ok(Json(IntelligenceStatsResponse { stats }))
        }
        Err(e) => {
            tracing::error!("Failed to get intelligence stats: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get statistics",
                )),
            ))
        }
    }
}

/// Simple message response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Epic 92.3: AI Document Summarization Handler
// ============================================================================

/// AI-powered document summarization (Story 92.3).
///
/// Generates a concise summary of the document using LLM.
/// Supports PDF, DOCX, and TXT documents.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/ai-summarize",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = AiSummarizeRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Document summarized", body = AiSummarizeResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 400, description = "Unsupported document type", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn ai_summarize_document(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<AiSummarizeRequest>,
) -> Result<Json<AiSummarizeResponse>, (StatusCode, Json<ErrorResponse>)> {
    use std::time::Instant;

    let start_time = Instant::now();

    // Verify document exists
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

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Check if document type is supported
    let supported_mime_types = [
        "application/pdf",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "text/plain",
        "text/markdown",
        "application/rtf",
    ];

    if !supported_mime_types.contains(&document.mime_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "UNSUPPORTED_FORMAT",
                format!(
                    "Document type '{}' is not supported for summarization. Supported: PDF, DOCX, DOC, TXT, MD, RTF",
                    document.mime_type
                ),
            )),
        ));
    }

    // Get document content (extracted text)
    let content = match state.document_repo.get_extracted_text(id).await {
        Ok(Some(text)) if !text.trim().is_empty() => text,
        Ok(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "NO_CONTENT",
                    "Document has no extracted text. Please run OCR first using /{id}/ocr/reprocess",
                )),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get document content: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to retrieve document content",
                )),
            ));
        }
    };

    // Determine provider and model
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o".to_string(),
        "azure_openai" => "gpt-4o".to_string(),
        _ => "claude-3-5-sonnet-20241022".to_string(),
    });

    // Build the summarization prompt
    let language = req.language.as_deref().unwrap_or("en");
    let summary_length = req.summary_length.as_deref().unwrap_or("medium");

    let word_target = match summary_length {
        "short" => "50-100 words",
        "long" => "300-500 words",
        _ => "150-250 words",
    };

    let language_name = match language {
        "sk" => "Slovak",
        "cs" => "Czech",
        "de" => "German",
        _ => "English",
    };

    let system_prompt = format!(
        r#"You are an expert document analyst and summarizer.

Your task is to create a concise summary of the provided document in {} language.

Requirements:
1. Generate a summary of {} (target length)
2. Extract 3-7 key points as bullet points
3. Maintain the document's core message and important details
4. Use clear, professional language
5. Do not add information not present in the original document

Format your response as:
SUMMARY:
[Your summary here]

KEY POINTS:
- [Key point 1]
- [Key point 2]
..."#,
        language_name, word_target
    );

    // For very long documents, truncate
    let content_tokens_estimate = content.len() / 4;
    let max_context = 100_000;

    let content_to_summarize = if content_tokens_estimate > max_context {
        let char_limit = max_context * 4;
        let half_limit = char_limit / 2;
        let start = &content[..half_limit.min(content.len())];
        let end_start = content.len().saturating_sub(half_limit);
        let end = &content[end_start..];

        format!(
            "[Document truncated for length]\n\n{}\n\n[...]\n\n{}",
            start, end
        )
    } else {
        content.clone()
    };

    // Call LLM
    let llm_request = integrations::ChatCompletionRequest {
        model: model.clone(),
        messages: vec![
            integrations::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            integrations::ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Please summarize the following document:\n\n{}",
                    content_to_summarize
                ),
            },
        ],
        temperature: Some(0.3),
        max_tokens: Some(2000),
    };

    let response = state
        .llm_client
        .chat(&provider, &llm_request)
        .await
        .map_err(|e| {
            tracing::error!("LLM summarization failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LLM_ERROR",
                    format!("Failed to generate summary: {}", e),
                )),
            )
        })?;

    let response_content = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    // Parse the response
    let (summary, key_points) = parse_summary_response(&response_content);

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Store the summary in the document
    let key_points_json = serde_json::to_value(&key_points).ok();
    let word_count = summary.split_whitespace().count() as i32;
    let _ = state
        .document_repo
        .update_summary(
            id,
            &summary,
            key_points_json,
            None, // action_items
            None, // topics
            Some(word_count),
            Some(language),
        )
        .await;

    tracing::info!(
        "Document {} summarized (tokens: {}, latency: {}ms)",
        id,
        response.usage.total_tokens,
        processing_time_ms
    );

    rls.release().await;
    Ok(Json(AiSummarizeResponse {
        document_id: id,
        summary: summary.clone(),
        key_points,
        word_count: summary.split_whitespace().count(),
        tokens_used: response.usage.total_tokens,
        processing_time_ms,
        provider,
        model,
    }))
}

/// Parse the summary response from LLM.
fn parse_summary_response(content: &str) -> (String, Vec<String>) {
    let mut summary = String::new();
    let mut key_points = Vec::new();
    let mut in_summary = false;
    let mut in_key_points = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.to_uppercase().starts_with("SUMMARY:") {
            in_summary = true;
            in_key_points = false;
            let after_label = trimmed
                .strip_prefix("SUMMARY:")
                .or_else(|| trimmed.strip_prefix("Summary:"))
                .unwrap_or("");
            if !after_label.trim().is_empty() {
                summary.push_str(after_label.trim());
                summary.push(' ');
            }
            continue;
        }

        if trimmed.to_uppercase().starts_with("KEY POINTS:")
            || trimmed.to_uppercase().starts_with("KEY_POINTS:")
        {
            in_summary = false;
            in_key_points = true;
            continue;
        }

        if in_summary && !trimmed.is_empty() {
            summary.push_str(trimmed);
            summary.push(' ');
        }

        if in_key_points && trimmed.starts_with('-') {
            let point = trimmed.trim_start_matches('-').trim();
            if !point.is_empty() {
                key_points.push(point.to_string());
            }
        }
    }

    // If parsing failed, use the whole content as summary
    if summary.is_empty() {
        summary = content.to_string();
    }

    (summary.trim().to_string(), key_points)
}

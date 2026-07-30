//! Document core routes — CRUD, download, preview, move, and access control.

//! Document routes (Epic 7A: Basic Document Management, Epic 7B: Document Versioning, Epic 28: Document Intelligence, Epic 92: Intelligent Document Generation).

use crate::state::AppState;
use api_core::{extractors::RlsConnection, AuthUser, TenantExtractor};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_extra::extract::Multipart;
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use db::models::{
    access_scope, document_category, CreateDocument, Document, DocumentClassificationHistory,
    DocumentFolder, DocumentIntelligenceStats, DocumentListQuery, DocumentSummary, DocumentVersion,
    DocumentVersionHistory, DocumentWithDetails, FolderTreeNode, FolderWithCount, MoveDocument,
    ShareWithDocument, UpdateDocument, ALLOWED_MIME_TYPES, MAX_FILE_SIZE,
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

/// Deserialize a field as a double `Option` so a handler can tell "absent"
/// (`None`) apart from "explicitly null" (`Some(None)`) and "set to a value"
/// (`Some(Some(v))`). Paired with `#[serde(default)]`, which yields `None` when
/// the key is absent; when present (null OR a value) this runs and wraps the
/// inner `Option` in `Some`. (#1589)
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Request for updating a folder.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    /// New parent folder. Omit to leave unchanged; send `null` to detach the
    /// folder to the top level; send an id to move it under that folder. The
    /// double-`Option` lets the handler distinguish absent from explicit-null
    /// (#1589). The wire shape is unchanged (optional, nullable UUID).
    #[serde(default, deserialize_with = "double_option")]
    #[schema(value_type = Option<Uuid>, nullable)]
    pub parent_id: Option<Option<Uuid>>,
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

/// Response for the multipart upload endpoint (Story 7A.1).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadDocumentResponse {
    pub id: Uuid,
    pub file_key: String,
    pub message: String,
}

/// Request for a presigned direct-to-S3 upload URL (gap-84-1).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUploadUrlRequest {
    /// Original filename — used to derive the storage key and, when
    /// `mime_type` is blank, to detect the content type from the extension.
    pub file_name: String,
    /// MIME type the client will upload with. It MUST be sent as the
    /// `Content-Type` header on the subsequent PUT, because the presigned URL
    /// is signed for exactly this content type.
    pub mime_type: String,
    /// Exact size of the upload in bytes. REQUIRED (GH #2320): it is validated
    /// against the 50 MiB cap up-front and signed into the presigned URL as
    /// `Content-Length`, so S3 rejects a PUT whose body length differs. The
    /// client MUST send exactly this many bytes. Kept `Option` at the serde
    /// layer only so a missing field yields a clear 400 instead of a 422.
    #[serde(default)]
    pub size_bytes: Option<i64>,
}

/// Response carrying a presigned PUT URL for direct client-to-S3 upload
/// (gap-84-1). The client uploads bytes straight to `url`, then registers the
/// document via `POST /api/v1/documents` with the returned `file_key`.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateUploadUrlResponse {
    /// Presigned S3 PUT URL. The client uploads bytes directly here, bypassing
    /// the api-server byte proxy.
    pub url: String,
    /// Storage key the object will live at. Echo this back to
    /// `POST /api/v1/documents` to register the document record once the PUT
    /// completes.
    pub file_key: String,
    /// MIME type the client MUST set as the `Content-Type` header on the PUT so
    /// the request matches the presigned signature.
    pub content_type: String,
    /// HTTP method to use for the upload (always `PUT`).
    pub method: String,
    /// When the presigned URL expires.
    pub expires_at: DateTime<Utc>,
}

/// Query parameters for deleting a storage object by its `file_key` (#2564).
///
/// Used by the direct-to-S3 orphan-cleanup route. The `file_key` is carried as
/// a query parameter (not a body) so the request stays a plain `DELETE` that
/// intermediaries don't strip a body from.
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct DeleteByFileKeyQuery {
    /// Storage key of the object to delete. MUST lie inside the caller's own
    /// org namespace (`{org_id}/…`) — enforced by `validate_file_key_org_scope`.
    pub file_key: String,
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
    // The /upload route needs a raised body limit (50 MiB) to accept real
    // files.  The global cap in main.rs is 16 MiB so we override it here
    // via a per-route sub-router, matching the pattern used for the
    // admin restore and migration import endpoints.
    let upload_router = Router::new()
        .route("/upload", post(upload_document))
        .layer(DefaultBodyLimit::max(52_428_800)); // exactly 50 MiB

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
        // Presigned direct-to-S3 upload URL (gap-84-1). No body-limit override
        // needed — only a small JSON request/response crosses the api-server;
        // the file bytes go straight to S3.
        .route("/upload-url", post(create_upload_url))
        // Best-effort orphan cleanup for the direct-to-S3 path (#2564): delete a
        // bucket object by file_key when registration (step 3) failed after the
        // bytes were already PUT (step 2). Org-scoped, no body needed.
        .route("/by-file-key", delete(delete_by_file_key))
        // Multipart upload (Story 7A.1)
        .merge(upload_router)
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
                    "File type '{}' is not supported. Allowed types: PDF, DOC, DOCX, XLS, XLSX, PNG, JPG, GIF, WEBP, TXT, CSV",
                    req.mime_type
                ),
            )),
        ));
    }

    // SECURITY (#2320): file_key must lie inside the caller's org namespace.
    // Both key producers (the presigned upload-url mint and the multipart
    // upload path) emit `{org_id}/{year}/{month}/{uuid}_{name}` via
    // `integrations::generate_storage_key`. Without this guard, any org member
    // could register a "document" pointing at an arbitrary bucket object
    // (another org's files, or a `messages/{thread_id}/…` attachment) and then
    // exfiltrate it through the presigned download/preview handlers — the same
    // bucket-wide IDOR fixed for messaging in #1791/#1770.
    validate_file_key_org_scope(&req.file_key, org_id)?;

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
        // Full access control for non-managers (GH #1413). Shows org-wide docs,
        // own docs, role-based docs, plus building/unit-scoped docs the caller is
        // a member of. Building/unit membership is resolved via the same
        // `user_scope_memberships_rls` resolver the download/preview gate uses, so
        // the list and the gate agree: a building/unit doc the caller can open is
        // also listed (no openable-but-not-listed divergence). A DB error fails
        // closed (no memberships → building/unit scopes omitted, never widened).
        let user_role = tenant.role.to_string().to_lowercase().replace(' ', "_");
        let (building_ids, unit_ids) = state
            .document_repo
            .user_scope_memberships_rls(&mut **rls.conn(), user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to resolve user scope memberships");
                (Vec::new(), Vec::new())
            });
        let roles = [user_role];
        let docs = state
            .document_repo
            .list_accessible_rls(
                &mut **rls.conn(),
                org_id,
                user_id,
                &building_ids,
                &unit_ids,
                &roles,
                list_query.clone(),
            )
            .await
            .unwrap_or_default();
        let total = state
            .document_repo
            .count_accessible_rls(
                &mut **rls.conn(),
                org_id,
                user_id,
                &building_ids,
                &unit_ids,
                &roles,
                list_query,
            )
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
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<DocumentDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .document_repo
        .find_by_id_with_details_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(document)) => {
            // Defense-in-depth cross-tenant guard. `find_by_id_with_details_rls`
            // relies on the connection's `app.current_org_id` GUC + the
            // `documents` FORCE-RLS policy to scope by org, but that lookup takes
            // no org argument and a BYPASSRLS/superuser pool is not bound by
            // FORCE. Re-check the org explicitly so a foreign-org document is
            // invisible (404, not an existence leak), matching the sibling read
            // handlers (`get_download_url`, `get_preview_url`, intelligence.rs).
            validate_document_org_scope(document.document.organization_id, tenant.tenant_id)?;
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
/// Returns `true` when a non-manager user is permitted to access `doc`.
///
/// Managers bypass this check entirely (their RLS policy already grants
/// full org-wide access). For everyone else the caller's `user_id` and
/// normalised `user_role` are matched against the document's `access_scope`.
///
/// This resolves only the membership-free scopes (creator / organization /
/// role / users). `building` and `unit` scope resolution needs the caller's
/// building/unit memberships and lives in [`scope_membership_allows`]; both
/// are OR'd at the call site so a building/unit member is granted access.
fn document_access_allowed(doc: &Document, user_id: Uuid, user_role: &str) -> bool {
    doc.created_by == user_id
        || doc.access_scope == "organization"
        || (doc.access_scope == "role"
            && doc
                .access_roles
                .as_array()
                .is_some_and(|arr| arr.iter().any(|r| r.as_str() == Some(user_role))))
        || (doc.access_scope == "users"
            && doc.access_target_ids.as_array().is_some_and(|arr| {
                arr.iter()
                    .any(|id| id.as_str() == Some(&user_id.to_string()))
            }))
}

/// Returns `true` when a `building`- or `unit`-scoped document targets a
/// building/unit the caller is a member of.
///
/// `user_building_ids` / `user_unit_ids` are the caller's active owner/resident
/// memberships (see [`DocumentRepository::user_scope_memberships_rls`]). This
/// mirrors the `building`/`unit` branches of the SQL gate
/// (`DocumentRepository::check_access_rls`); without it a building/unit
/// resident would see a scoped document in their list but get a 404 on
/// download/preview (GH #1413).
fn scope_membership_allows(
    doc: &Document,
    user_building_ids: &[Uuid],
    user_unit_ids: &[Uuid],
) -> bool {
    match doc.access_scope.as_str() {
        "building" => target_ids_intersect(&doc.access_target_ids, user_building_ids),
        "unit" => target_ids_intersect(&doc.access_target_ids, user_unit_ids),
        _ => false,
    }
}

/// True when any `id` in `member_ids` appears (as its string form) in the
/// document's `access_target_ids` JSON array. Target ids are stored as JSON
/// strings, matching the `users`-scope convention in [`document_access_allowed`].
fn target_ids_intersect(access_target_ids: &serde_json::Value, member_ids: &[Uuid]) -> bool {
    if member_ids.is_empty() {
        return false;
    }
    access_target_ids.as_array().is_some_and(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str())
            .any(|target| member_ids.iter().any(|id| id.to_string() == target))
    })
}

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
    auth: AuthUser,
    tenant: TenantExtractor,
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

    // Defense-in-depth cross-tenant guard. `find_by_id_rls` relies on the
    // connection's `app.current_org_id` GUC + the `documents` FORCE-RLS policy
    // to scope by org, but that lookup takes no org argument and a
    // BYPASSRLS/superuser pool (e.g. the integration-test pool) is not bound by
    // FORCE. Re-check the org explicitly so a foreign-org document is invisible
    // (404, not a 503 existence leak) regardless of how the connection is
    // privileged — matching the repository-layer `... AND organization_id = $N`
    // convention used across the cross-tenant IDOR fixes.
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    if !tenant.role.is_manager() {
        let user_role = tenant.role.to_string().to_lowercase().replace(' ', "_");
        // Resolve building/unit memberships so scoped documents the caller can
        // see in their list are also downloadable/previewable (GH #1413). A DB
        // error fails closed (no memberships → deny building/unit scopes).
        let (building_ids, unit_ids) = state
            .document_repo
            .user_scope_memberships_rls(&mut **rls.conn(), auth.user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to resolve user scope memberships");
                (Vec::new(), Vec::new())
            });
        let allowed = document_access_allowed(&document, auth.user_id, &user_role)
            || scope_membership_allows(&document, &building_ids, &unit_ids);
        if !allowed {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ));
        }
    }

    // Story 7A.4: Generate presigned S3 URL with Content-Disposition: attachment.
    // Use state.storage_service (initialised at startup with the real S3 client).
    // StorageService::from_env() is sync and does NOT set up the S3 client, so
    // presigning would always fail — that was the bug in the original stub.
    let storage = state.storage_service.as_ref().ok_or_else(|| {
        tracing::error!("Storage service not configured — document downloads unavailable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "STORAGE_NOT_CONFIGURED",
                "Document storage is not configured. Please contact support.",
            )),
        )
    })?;

    let presigned = storage
        .generate_download_url(
            &document.file_key,
            &document.file_name,
            &document.mime_type,
            // Short-lived TTL driven by the S3_PRESIGNED_URL_TTL_SECS config
            // knob (defaults to 15 minutes). gap-84-1.
            Some(storage.download_ttl_secs()),
        )
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                file_key = %document.file_key,
                "Failed to generate presigned download URL"
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "STORAGE_ERROR",
                    "Unable to generate download URL. Please try again later.",
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(UrlResponse {
        url: presigned.url,
        expires_at: presigned.expires_at,
    }))
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
    auth: AuthUser,
    tenant: TenantExtractor,
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

    // Defense-in-depth cross-tenant guard (see `get_download_url`): a foreign-org
    // document must be invisible (404) even on a BYPASSRLS/superuser pool that
    // FORCE RLS does not bind.
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    if !tenant.role.is_manager() {
        let user_role = tenant.role.to_string().to_lowercase().replace(' ', "_");
        // Resolve building/unit memberships so scoped documents the caller can
        // see in their list are also downloadable/previewable (GH #1413). A DB
        // error fails closed (no memberships → deny building/unit scopes).
        let (building_ids, unit_ids) = state
            .document_repo
            .user_scope_memberships_rls(&mut **rls.conn(), auth.user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(error = %e, "Failed to resolve user scope memberships");
                (Vec::new(), Vec::new())
            });
        let allowed = document_access_allowed(&document, auth.user_id, &user_role)
            || scope_membership_allows(&document, &building_ids, &unit_ids);
        if !allowed {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ));
        }
    }

    if !document.supports_preview() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "PREVIEW_NOT_SUPPORTED",
                "Preview is not supported for this file type. Use download instead.",
            )),
        ));
    }

    // Story 7A.4: Generate presigned S3 URL with Content-Disposition: inline.
    // generate_preview_url (new method) sets inline disposition so the browser
    // renders the file rather than downloading it.
    // Use state.storage_service (same reason as get_download_url).
    let storage = state.storage_service.as_ref().ok_or_else(|| {
        tracing::error!("Storage service not configured — document previews unavailable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "STORAGE_NOT_CONFIGURED",
                "Document storage is not configured. Please contact support.",
            )),
        )
    })?;

    let presigned = storage
        .generate_preview_url(
            &document.file_key,
            &document.mime_type,
            None, // Default: 1-hour expiration for inline preview
        )
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                file_key = %document.file_key,
                "Failed to generate presigned preview URL"
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "STORAGE_ERROR",
                    "Unable to generate preview URL. Please try again later.",
                )),
            )
        })?;

    rls.release().await;
    Ok(Json(UrlResponse {
        url: presigned.url,
        expires_at: presigned.expires_at,
    }))
}

// ============================================================================
// Presigned Upload-URL Handler (gap-84-1)
// ============================================================================

/// Validate that a client-supplied `file_key` lies inside the caller's org
/// namespace (GH #2320).
///
/// Mirrors the messaging guard from #1791/#1770 (`link_message_attachment`):
/// the key must start with the tenant's own `{org_id}/` prefix (the shape both
/// key producers emit via `integrations::generate_storage_key`) and must not
/// contain a `..` component. Kept as a pure function so the accept/reject
/// contract can be unit-tested without a DB.
pub(crate) fn validate_file_key_org_scope(
    file_key: &str,
    org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let expected_prefix = format!("{org_id}/");
    if !file_key.starts_with(&expected_prefix) || file_key.contains("..") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_FILE_KEY",
                "file_key must reference an object uploaded for this organization",
            )),
        ));
    }
    Ok(())
}

/// Application-layer org guard for a single fetched document (#2422).
///
/// `get_document` reads via `find_by_id_with_details_rls`, whose SQL scopes
/// only by `id` and relies on the `documents` FORCE-RLS policy + the
/// connection's `app.current_org_id` GUC to enforce org isolation. That is the
/// primary control, but it fails open on a BYPASSRLS/superuser pool (e.g. the
/// integration-test pool) which FORCE does not bind. Re-check the org here so a
/// foreign-org document is indistinguishable from a missing one (404, not an
/// existence leak), mirroring the inline guard in the sibling read handlers
/// (`get_download_url`, `get_preview_url`, `intelligence.rs`). Kept as a pure
/// function so the accept/reject contract can be unit-tested without a DB.
pub(crate) fn validate_document_org_scope(
    document_org_id: Uuid,
    tenant_org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if document_org_id != tenant_org_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }
    Ok(())
}

/// Validate a presigned-upload request before minting a URL (gap-84-1).
///
/// Mirrors the eager checks the multipart [`upload_document`] handler runs on
/// received bytes, but applied up-front to the client's *declared* `mime_type`
/// / `size_bytes` so an invalid request is rejected before a presigned URL is
/// handed out. Kept as a pure function so it can be unit-tested without a live
/// S3 client or DB.
///
/// GH #2320: `size_bytes` is now REQUIRED — it is signed into the presigned
/// PUT URL as `Content-Length`, which is what actually enforces the 50 MiB
/// cap on the direct-to-S3 path. Returns the validated size on success, or
/// the client error tuple on failure.
fn validate_upload_url_request(
    mime_type: &str,
    size_bytes: Option<i64>,
) -> Result<i64, (StatusCode, Json<ErrorResponse>)> {
    if !ALLOWED_MIME_TYPES.contains(&mime_type) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "UNSUPPORTED_FILE_TYPE",
                format!(
                    "File type '{mime_type}' is not supported. \
                    Allowed: PDF, DOC, DOCX, XLS, XLSX, PNG, JPG, GIF, WEBP, TXT, CSV"
                ),
            )),
        ));
    }

    let Some(size) = size_bytes else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "size_bytes is required — it is signed into the upload URL as Content-Length",
            )),
        ));
    };
    if size < 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "size_bytes must be non-negative",
            )),
        ));
    }
    if size > MAX_FILE_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse::new(
                "FILE_TOO_LARGE",
                format!("File exceeds maximum size of {MAX_FILE_SIZE} bytes (50 MiB)"),
            )),
        ));
    }

    Ok(size)
}

/// Mint a presigned PUT URL for direct client-to-S3 upload (gap-84-1).
///
/// Lets clients upload file bytes straight to S3-compatible storage instead of
/// proxying them through the api-server multipart `/upload` route. Flow:
///   1. `POST /api/v1/documents/upload-url` → this handler returns a short-lived
///      presigned PUT URL plus the storage `file_key`.
///   2. Client `PUT`s the bytes directly to that URL, setting
///      `Content-Type: <content_type>` and a `Content-Length` equal to the
///      declared `size_bytes` — both are signed headers (GH #2320), so a
///      mismatched request fails S3 signature validation.
///   3. Client `POST`s `/api/v1/documents` with the `file_key` to register the
///      document record.
///
/// Any authenticated org member may request an upload URL — same authorization
/// posture as the existing multipart upload handler (`AuthUser` + tenant
/// context, no extra capability gate). The URL is scoped to a tenant-specific
/// key and expires in 5 minutes.
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload-url",
    request_body = CreateUploadUrlRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Presigned upload URL", body = CreateUploadUrlResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
        (status = 503, description = "Storage unavailable", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn create_upload_url(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    Json(req): Json<CreateUploadUrlRequest>,
) -> Result<Json<CreateUploadUrlResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;

    // Resolve MIME type: prefer the client's declared type, fall back to
    // extension-based detection when it's blank (parity with upload_document).
    let content_type = if req.mime_type.trim().is_empty() {
        integrations::get_content_type(&req.file_name).to_string()
    } else {
        req.mime_type.clone()
    };

    let size_bytes = validate_upload_url_request(&content_type, req.size_bytes)?;

    // Presigning needs a real S3 client. StorageService::from_env() (sync) does
    // not create one, so gate on has_s3_client() — same 503 contract the
    // download handler uses.
    let storage = state.storage_service.as_ref().ok_or_else(|| {
        tracing::error!("Storage service not configured — presigned uploads unavailable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "STORAGE_NOT_CONFIGURED",
                "Document storage is not configured. Please contact support.",
            )),
        )
    })?;
    if !storage.has_s3_client() {
        tracing::error!("Storage service present but S3 client not initialised");
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "STORAGE_NOT_CONFIGURED",
                "Document storage is not configured. Please contact support.",
            )),
        ));
    }

    // Tenant-scoped storage key ({org_id}/{year}/{month}/{uuid}_{filename}).
    let file_key = integrations::generate_storage_key(org_id, &req.file_name);

    // Default TTL (5 min) is applied by generate_upload_url when None is passed.
    // size_bytes is signed into the URL as Content-Length (GH #2320) so S3
    // rejects a PUT whose body length differs from the declared size.
    let presigned = storage
        .generate_upload_url(&file_key, &content_type, size_bytes, None)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                file_key = %file_key,
                "Failed to generate presigned upload URL"
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "STORAGE_ERROR",
                    "Unable to generate upload URL. Please try again later.",
                )),
            )
        })?;

    tracing::info!(
        file_key = %file_key,
        org_id = %org_id,
        content_type = %content_type,
        "Issued presigned direct-to-S3 upload URL"
    );

    Ok(Json(CreateUploadUrlResponse {
        url: presigned.url,
        file_key,
        content_type,
        method: "PUT".to_string(),
        expires_at: presigned.expires_at,
    }))
}

/// Delete a storage object by `file_key` — direct-upload orphan cleanup (#2564).
///
/// Compensating action for the direct-to-S3 upload flow (gap-84-1). That flow
/// PUTs the bytes straight to S3 (step 2) *before* registering the document
/// record via `POST /api/v1/documents` (step 3). If registration fails, the
/// object is left in the bucket with no `documents` row referencing it — a
/// leaked orphan. The client calls this route best-effort in its catch block to
/// reap the orphan immediately, rather than depending solely on the ~1-day
/// bucket lifecycle sweep (the authorised alternative in #2534).
///
/// Authorization mirrors the rest of the direct-upload flow: any authenticated
/// org member (`AuthUser` + tenant context, no extra capability gate — same
/// posture as `create_upload_url` / `create_document`). The `file_key` MUST lie
/// inside the caller's own org namespace: `validate_file_key_org_scope` rejects
/// cross-org keys, `messages/…` attachments, and `..` traversal with 400, so
/// this route cannot be used to delete another tenant's objects (the same guard
/// `create_document` applies to a client-supplied key, #2320).
///
/// The route reaps *orphaned* (never-registered) keys only. Before touching
/// storage it asserts that no live `documents` row references the key within
/// the caller's org and rejects with 409 `FILE_KEY_IN_USE` otherwise (#2573):
/// without this guard any authenticated org member could pass another member's
/// registered `file_key` (same `{org_id}/…` prefix) and delete the underlying
/// object out from under a live document row, leaving a dangling reference.
///
/// Returns 204 No Content on success. S3 `DeleteObject` is idempotent, so
/// deleting an already-absent key is a no-op that still returns 204. 409 if the
/// key is still referenced by a live document. 503 if storage is not configured.
#[utoipa::path(
    delete,
    path = "/api/v1/documents/by-file-key",
    params(DeleteByFileKeyQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Object deleted (or already absent)"),
        (status = 400, description = "Invalid file_key", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 409, description = "file_key still referenced by a document", body = ErrorResponse),
        (status = 503, description = "Storage unavailable", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn delete_by_file_key(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<DeleteByFileKeyQuery>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = tenant.tenant_id;

    // Org-scope guard: the caller may only delete objects under its own
    // `{org_id}/` prefix. Rejects cross-org keys, message attachments, and `..`
    // traversal (400) — same guard create_document applies (#2320).
    validate_file_key_org_scope(&query.file_key, org_id)?;

    // Referenced-object guard (#2573): this route reaps *orphaned* keys only.
    // Refuse to delete an object whose bytes are still referenced by a live
    // `documents` row — otherwise any authenticated org member could delete a
    // registered document's underlying object and leave a dangling reference.
    let in_use = state
        .document_repo
        .exists_by_file_key_rls(&mut **rls.conn(), org_id, &query.file_key)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                file_key = %query.file_key,
                "Failed to check whether file_key is still referenced"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Unable to verify file_key state. Please try again later.",
                )),
            )
        })?;
    if in_use {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse::new(
                "FILE_KEY_IN_USE",
                "Refusing to delete a file_key still referenced by a document",
            )),
        ));
    }

    let storage = state.storage_service.as_ref().ok_or_else(|| {
        tracing::error!("Storage service not configured — orphan cleanup unavailable");
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "STORAGE_NOT_CONFIGURED",
                "Document storage is not configured. Please contact support.",
            )),
        )
    })?;

    storage.delete(&query.file_key).await.map_err(|e| {
        tracing::error!(
            error = %e,
            file_key = %query.file_key,
            "Failed to delete object from storage (direct-upload orphan cleanup)"
        );
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse::new(
                "STORAGE_ERROR",
                "Unable to delete object. Please try again later.",
            )),
        )
    })?;

    tracing::info!(
        file_key = %query.file_key,
        org_id = %org_id,
        "Deleted object by file_key (direct-upload orphan cleanup)"
    );

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Upload Handler (Story 7A.1)
// ============================================================================

/// Upload a document via multipart/form-data (Story 7A.1).
///
/// Accepts a `multipart/form-data` body with the following fields:
/// - `file` (required) — the binary file data
/// - `title` (required) — human-readable document title
/// - `category` (required) — document category
/// - `description` (optional)
/// - `folder_id` (optional) — target folder UUID
///
/// The handler:
/// 1. Validates MIME type and file size against repository limits.
/// 2. Uploads the file bytes to S3-compatible storage (if configured) using
///    `integrations::generate_storage_key` for a stable, tenant-scoped key.
/// 3. Inserts a document record via `document_repo.create_rls` (RLS-aware).
/// 4. Returns `{ id, file_key, message }` on success.
#[utoipa::path(
    post,
    path = "/api/v1/documents/upload",
    request_body(content_type = "multipart/form-data"),
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Document uploaded", body = UploadDocumentResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 413, description = "File too large", body = ErrorResponse),
        (status = 503, description = "Storage unavailable", body = ErrorResponse),
    ),
    tag = "Documents"
)]
async fn upload_document(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadDocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let user_id = auth.user_id;
    let org_id = tenant.tenant_id;

    // --- Parse multipart fields ---
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut mime_type: Option<String> = None;
    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut category: Option<String> = None;
    let mut folder_id: Option<Uuid> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::warn!(error = %e, "Failed to read multipart field");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!("Failed to read multipart field: {e}"),
            )),
        )
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                // Capture filename and content-type from part headers before
                // consuming the field via `.bytes()`.
                file_name = field
                    .file_name()
                    .map(|s| s.to_string())
                    .or(Some("upload".to_string()));
                mime_type = field.content_type().map(|ct| ct.to_string());

                let data = field.bytes().await.map_err(|e| {
                    tracing::warn!(error = %e, "Failed to read file bytes");
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "BAD_REQUEST",
                            format!("Failed to read file data: {e}"),
                        )),
                    )
                })?;
                file_bytes = Some(data.to_vec());
            }
            "title" => {
                title = Some(field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "BAD_REQUEST",
                            format!("Failed to read title field: {e}"),
                        )),
                    )
                })?);
            }
            "description" => {
                let text = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "BAD_REQUEST",
                            format!("Failed to read description field: {e}"),
                        )),
                    )
                })?;
                if !text.is_empty() {
                    description = Some(text);
                }
            }
            "category" => {
                category = Some(field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "BAD_REQUEST",
                            format!("Failed to read category field: {e}"),
                        )),
                    )
                })?);
            }
            "folder_id" => {
                let text = field.text().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::new(
                            "BAD_REQUEST",
                            format!("Failed to read folder_id field: {e}"),
                        )),
                    )
                })?;
                if !text.is_empty() {
                    folder_id = text.parse::<Uuid>().ok();
                }
            }
            // Unknown fields (e.g. building_id sent by frontend) are drained
            // and ignored so the multipart parser can advance safely.
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    // --- Require file part ---
    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Missing 'file' field in multipart body",
            )),
        )
    })?;

    // --- Require title ---
    let title = title.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Missing 'title' field")),
        )
    })?;

    // --- Require category ---
    let category = category.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Missing 'category' field",
            )),
        )
    })?;

    let file_name = file_name.unwrap_or_else(|| "upload".to_string());

    // Resolve MIME type: prefer Content-Type from the part header,
    // fall back to extension-based detection.
    let resolved_mime =
        mime_type.unwrap_or_else(|| integrations::get_content_type(&file_name).to_string());

    // --- Validate title length ---
    if title.len() > MAX_TITLE_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!("Title exceeds maximum length of {MAX_TITLE_LENGTH} characters"),
            )),
        ));
    }

    // --- Validate description length ---
    if let Some(ref desc) = description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Description exceeds maximum length of {MAX_DESCRIPTION_LENGTH} characters"
                    ),
                )),
            ));
        }
    }

    // --- Validate file size ---
    let size_bytes = bytes.len() as i64;
    if size_bytes > MAX_FILE_SIZE {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ErrorResponse::new(
                "FILE_TOO_LARGE",
                format!("File exceeds maximum size of {MAX_FILE_SIZE} bytes (50 MiB)"),
            )),
        ));
    }

    // --- Validate MIME type ---
    if !ALLOWED_MIME_TYPES.contains(&resolved_mime.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "UNSUPPORTED_FILE_TYPE",
                format!(
                    "File type '{resolved_mime}' is not supported. \
                    Allowed: PDF, DOC, DOCX, XLS, XLSX, PNG, JPG, GIF, WEBP, TXT, CSV"
                ),
            )),
        ));
    }

    // --- Validate category ---
    if !document_category::ALL.contains(&category.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Invalid category")),
        ));
    }

    // --- Generate S3-compatible storage key ---
    let file_key = integrations::generate_storage_key(org_id, &file_name);

    // --- Upload bytes to S3 (when the storage service is ready) ---
    if let Some(ref storage_service) = state.storage_service {
        if storage_service.has_s3_client() {
            storage_service
                .upload(&file_key, bytes, &resolved_mime)
                .await
                .map_err(|e| {
                    tracing::error!(
                        error = %e,
                        file_key = %file_key,
                        "Failed to upload document bytes to S3"
                    );
                    (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(ErrorResponse::new(
                            "STORAGE_ERROR",
                            "Failed to upload file to storage. Please try again.",
                        )),
                    )
                })?;

            tracing::info!(
                file_key = %file_key,
                size_bytes,
                mime_type = %resolved_mime,
                "Uploaded document bytes to S3"
            );
        } else {
            tracing::warn!(
                file_key = %file_key,
                "Storage service present but S3 client not initialised — skipping S3 upload"
            );
        }
    } else {
        tracing::warn!(
            file_key = %file_key,
            "No storage service configured — document bytes not persisted to S3"
        );
    }

    // --- Create document record (RLS-aware) ---
    let data = db::models::CreateDocument {
        organization_id: org_id,
        folder_id,
        title,
        description,
        category,
        file_key: file_key.clone(),
        file_name,
        mime_type: resolved_mime,
        size_bytes,
        // Default to organization-wide access; callers may update via PUT /{id}/access
        access_scope: None,
        access_target_ids: None,
        access_roles: None,
        created_by: user_id,
    };

    match state
        .document_repo
        .create_rls(&mut **rls.conn(), data)
        .await
    {
        Ok(document) => {
            rls.release().await;
            tracing::info!(
                document_id = %document.id,
                file_key = %file_key,
                org_id = %org_id,
                user_id = %user_id,
                "Document upload complete"
            );
            Ok((
                StatusCode::CREATED,
                Json(UploadDocumentResponse {
                    id: document.id,
                    file_key,
                    message: "Document uploaded successfully".to_string(),
                }),
            ))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create document record after upload");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create document record",
                )),
            ))
        }
    }
}

#[cfg(test)]
#[path = "document_access_test.rs"]
mod document_access_test;

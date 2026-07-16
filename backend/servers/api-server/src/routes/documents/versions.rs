//! Document version routes (Story 7B.1).

use crate::state::AppState;
use api_core::{extractors::RlsConnection, AuthUser, TenantExtractor};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{CreateDocumentVersion, ALLOWED_MIME_TYPES, MAX_FILE_SIZE};
use uuid::Uuid;

use super::core::*;

/// Create documents versions router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/versions", get(get_version_history))
        .route("/{id}/versions", post(create_version))
        .route("/{id}/versions/{version_id}", get(get_version))
        .route("/{id}/versions/{version_id}/restore", post(restore_version))
}

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

    // SECURITY (#2320): the version's file_key must lie inside the caller's org
    // namespace, same as document registration — otherwise a member could point
    // a new version at another org's bucket object and exfiltrate it through the
    // version download/preview presign.
    validate_file_key_org_scope(&req.file_key, tenant.tenant_id)?;

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

    // RLS-scoped restore: runs on the request's org-scoped connection so it
    // stays correct under FORCE ROW LEVEL SECURITY on `documents` (#754).
    match state
        .document_repo
        .restore_version_rls(rls.conn(), id, version_id, user_id)
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

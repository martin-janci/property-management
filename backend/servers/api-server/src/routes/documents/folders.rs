//! Document folder routes (Story 7A.2).

use crate::state::AppState;
use api_core::{extractors::RlsConnection, AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{CreateFolder, UpdateFolder};
use uuid::Uuid;

use super::core::*;

/// Create documents folders router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/folders", get(list_folders))
        .route("/folders", post(create_folder))
        .route("/folders/tree", get(get_folder_tree))
        .route("/folders/{id}", get(get_folder))
        .route("/folders/{id}", put(update_folder))
        .route("/folders/{id}", delete(delete_folder))
}

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
    let org_id = rls.tenant_id();
    match state
        .document_repo
        .find_folder_by_id_rls(&mut **rls.conn(), id, org_id)
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

    // Check folder exists (org-scoped: cross-org rows are invisible even under
    // a superuser connection that bypasses RLS — see find_folder_by_id_rls).
    let org_id = rls.tenant_id();
    match state
        .document_repo
        .find_folder_by_id_rls(&mut **rls.conn(), id, org_id)
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

    // Check folder exists (org-scoped: cross-org rows are invisible even under
    // a superuser connection that bypasses RLS — see find_folder_by_id_rls).
    let org_id = rls.tenant_id();
    match state
        .document_repo
        .find_folder_by_id_rls(&mut **rls.conn(), id, org_id)
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

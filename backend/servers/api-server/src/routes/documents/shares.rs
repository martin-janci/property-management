//! Document sharing routes (Story 7A.5).

use crate::state::AppState;
use api_core::{extractors::RlsConnection, AuthUser, TenantExtractor};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use common::notifications::{Notification, NotificationCategory};
use db::models::{share_type, CreateShare, DocumentSummary, LogShareAccess};
use uuid::Uuid;

use super::core::*;

/// Create authenticated documents shares router.
pub fn authenticated_router() -> Router<AppState> {
    Router::new()
        .route("/{id}/shares", get(list_shares))
        .route("/{id}/shares", post(create_share))
        .route("/{id}/shares/{share_id}", delete(revoke_share))
}

/// Create public (no-auth) share access router.
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/shared/{token}", get(access_shared_document))
        .route("/shared/{token}/access", post(access_protected_share))
}

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

            // Best-effort: notify the recipients of a non-link share (#973.2).
            // LINK shares are pull-based (anyone with the URL) so there is no
            // concrete recipient to notify. Recipient resolution runs on the
            // same RLS connection so the reads are authorised under the
            // manager's org context, mirroring the announcement publish path.
            let recipients = resolve_share_recipients(
                rls.conn(),
                tenant.tenant_id,
                &share.share_type,
                share.target_id,
                share.target_role.as_deref(),
            )
            .await;

            rls.release().await;

            if !recipients.is_empty() {
                let notification = Notification::new(
                    Uuid::nil(),
                    NotificationCategory::Documents,
                    "Document shared with you",
                    format!("\"{}\" has been shared with you", existing.title),
                )
                .with_action_url(format!("/documents/{}", existing.id))
                .with_data(serde_json::json!({
                    "document_id": existing.id,
                    "share_id": share.id,
                    "share_type": share.share_type,
                    "shared_by": auth.user_id,
                }));

                let (sent, skipped, failed) = state
                    .notification_pipeline
                    .broadcast(&recipients, &notification, Some(existing.id))
                    .await;

                tracing::info!(
                    document_id = %existing.id,
                    share_id = %share.id,
                    share_type = %share.share_type,
                    recipients = recipients.len(),
                    channels_sent = sent,
                    channels_skipped = skipped,
                    channels_failed = failed,
                    "Document share created — recipient notifications dispatched"
                );
            }

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
    let share = match state
        .document_repo
        .find_share_by_id_rls(&mut **rls.conn(), share_id)
        .await
    {
        Ok(Some(s)) => s,
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
    };

    // The share must belong to the document named in the path. Authorization
    // above was checked against the path document `id`, but the share is looked
    // up purely by `share_id`; RLS only scopes to the tenant. Without this
    // guard a creator of document A could revoke a share of document B in the
    // same tenant by passing A's id (to pass the creator check) plus B's
    // share_id (IDOR). Return 404 to avoid leaking the existence of the
    // cross-document share, mirroring the "Share not found" shape above.
    //
    // This explicit check is now belt-and-suspenders: `revoke_share_rls` is
    // itself scoped to `document_id = id` at the data layer (#2197), so even if
    // a future refactor drops this guard the UPDATE affects 0 rows for a
    // cross-document share and the `Ok(0) => 404` branch below still holds.
    if share.document_id != id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Share not found")),
        ));
    }

    match state
        .document_repo
        .revoke_share_rls(&mut **rls.conn(), share_id, id)
        .await
    {
        // 0 rows affected means no share with this id belongs to this document
        // (cross-document mismatch or already gone). Map to 404 without leaking
        // cross-document existence, matching the guard above.
        Ok(0) => {
            rls.release().await;
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Share not found")),
            ))
        }
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
    // Find share by token. This public endpoint has no caller org context, so
    // the validated token is the authorization grant: the lookup runs with
    // super-admin RLS context on a dedicated connection (see
    // `find_share_by_token_for_share`) so it stays correct under FORCE RLS on
    // `document_shares` (#754 / PAP-21).
    let share = match state
        .document_repo
        .find_share_by_token_for_share(&token)
        .await
    {
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

    // Get document via the share-token path. The validated token is the
    // authorization grant; the lookup runs with super-admin RLS context on a
    // dedicated connection so it stays correct under FORCE RLS on `documents`
    // (#754).
    let document = match state
        .document_repo
        .find_by_id_for_share(share.document_id)
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
                    "Failed to access document",
                )),
            ));
        }
    };

    // Log access - important for audit trail but shouldn't fail the request.
    // Public endpoint: the validated token authorizes the write, which runs
    // with super-admin RLS context on a dedicated connection (FORCE RLS on
    // `document_share_access_log`, #754 / PAP-21).
    if let Err(e) = state
        .document_repo
        .log_share_access_for_share(LogShareAccess {
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
    // Find share by token. This public endpoint has no caller org context, so
    // the validated token is the authorization grant: the lookup runs with
    // super-admin RLS context on a dedicated connection (see
    // `find_share_by_token_for_share`) so it stays correct under FORCE RLS on
    // `document_shares` (#754 / PAP-21).
    let share = match state
        .document_repo
        .find_share_by_token_for_share(&token)
        .await
    {
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

    // Verify password. The share row is under FORCE RLS, so the lookup needed
    // to read its password hash runs with super-admin RLS context on a
    // dedicated connection (#754 / PAP-21).
    if !state
        .document_repo
        .verify_share_password_for_share(share.id, &req.password)
        .await
        .unwrap_or(false)
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_PASSWORD", "Invalid password")),
        ));
    }

    // Get document via the share-token path. The validated token is the
    // authorization grant; the lookup runs with super-admin RLS context on a
    // dedicated connection so it stays correct under FORCE RLS on `documents`
    // (#754).
    let document = match state
        .document_repo
        .find_by_id_for_share(share.document_id)
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
                    "Failed to access document",
                )),
            ));
        }
    };

    // Log access - important for audit trail but shouldn't fail the request.
    // Public endpoint: the validated token authorizes the write, which runs
    // with super-admin RLS context on a dedicated connection (FORCE RLS on
    // `document_share_access_log`, #754 / PAP-21).
    if let Err(e) = state
        .document_repo
        .log_share_access_for_share(LogShareAccess {
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
// Helper Functions
// ============================================================================

/// Resolve a document share's targeting into the concrete set of recipient
/// user ids (#973.2, Epic 7A.5 share-notification fan-out).
///
/// Must be called on a connection whose RLS context is the sharing manager's
/// org, so the membership / unit / resident reads are authorised. Mirrors the
/// announcement publish fan-out (`resolve_announcement_recipients`).
///
/// | share_type | recipients |
/// |------------|------------|
/// | `link`     | none (pull-based; anyone with the URL) |
/// | `user`     | the single target user |
/// | `role`     | active org members holding the target role |
/// | `building` | active residents of units in the target building |
///
/// Resolution is best-effort: a query error is logged and treated as "no
/// recipients" so a transient read failure never blocks the share itself.
async fn resolve_share_recipients(
    conn: &mut sqlx::PgConnection,
    org_id: Uuid,
    share_type_str: &str,
    target_id: Option<Uuid>,
    target_role: Option<&str>,
) -> Vec<Uuid> {
    let result: Result<Vec<(Uuid,)>, sqlx::Error> = match share_type_str {
        share_type::USER => match target_id {
            Some(uid) => Ok(vec![(uid,)]),
            None => Ok(Vec::new()),
        },
        share_type::ROLE => match target_role {
            Some(role) => {
                sqlx::query_as(
                    // Issue #1000: exclude expired memberships, matching the
                    // announcement targeting predicate in `announcement.rs`
                    // (`list_published_rls`/`count_published_rls`). A user whose
                    // membership lapsed (but was never explicitly revoked) no
                    // longer has org access and must not be fanned out to.
                    r#"
                    SELECT DISTINCT user_id FROM user_memberships
                    WHERE organization_id = $1
                      AND revoked_at IS NULL
                      AND (expires_at IS NULL OR expires_at > NOW())
                      AND role = $2
                    "#,
                )
                .bind(org_id)
                .bind(role)
                .fetch_all(&mut *conn)
                .await
            }
            None => Ok(Vec::new()),
        },
        share_type::BUILDING => match target_id {
            Some(building_id) => {
                sqlx::query_as(
                    r#"
                    SELECT DISTINCT ur.user_id
                    FROM unit_residents ur
                    JOIN units u ON u.id = ur.unit_id
                    WHERE u.building_id = $1
                      AND ur.end_date IS NULL
                      AND ur.receives_notifications = TRUE
                    "#,
                )
                .bind(building_id)
                .fetch_all(&mut *conn)
                .await
            }
            None => Ok(Vec::new()),
        },
        // LINK (and anything unexpected) has no concrete recipient.
        _ => Ok(Vec::new()),
    };

    match result {
        Ok(rows) => rows.into_iter().map(|(id,)| id).collect(),
        Err(e) => {
            tracing::error!(
                share_type = %share_type_str,
                error = %e,
                "Failed to resolve share recipients; share created without notification fan-out"
            );
            Vec::new()
        }
    }
}

// ============================================================================

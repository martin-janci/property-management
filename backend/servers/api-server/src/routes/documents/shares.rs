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
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use uuid::Uuid;

use crate::routes::rate_limit::{rate_limit_allowed, InProcessRateLimiter};

use super::core::*;

// ── Brute-force throttle for password-protected share access ────────────────
//
// `POST /api/v1/documents/shared/{token}/access` is a fully public endpoint
// that verifies a share password. Without a limiter an attacker who has the
// (guessable-length) token can brute-force the password at request speed. We
// reuse the shared sliding-window limiter (`routes::rate_limit`) — the same
// algorithm that guards the MFA verify + `/forgot-password` surfaces — keyed
// on `token:ip` so one source IP hammering one share is blocked after
// `SHARE_ACCESS_MAX_ATTEMPTS` failures in the window, without locking that IP
// out of unrelated shares or locking the share out for other IPs. A correct
// password clears the key so a legitimate viewer who first mistyped is never
// penalised.
const SHARE_ACCESS_MAX_ATTEMPTS: u32 = 10;
const SHARE_ACCESS_WINDOW: Duration = Duration::from_secs(900);
/// Only sweep expired limiter entries once the map exceeds this size, bounding
/// memory against a token/IP-fanned flood (mirrors the MFA limiter tuning).
const SHARE_ACCESS_SWEEP_THRESHOLD: usize = 1024;

static SHARE_ACCESS_RATE_LIMITER: LazyLock<InProcessRateLimiter<String>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Build the per-share, per-IP limiter key.
fn share_access_key(token: &str, ip: &str) -> String {
    format!("{token}:{ip}")
}

/// Record a share-access attempt for `token`+`ip`; `false` once more than
/// `SHARE_ACCESS_MAX_ATTEMPTS` attempts occur in a rolling
/// `SHARE_ACCESS_WINDOW`.
fn share_access_attempt_allowed(token: &str, ip: &str) -> bool {
    rate_limit_allowed(
        &SHARE_ACCESS_RATE_LIMITER,
        share_access_key(token, ip),
        SHARE_ACCESS_MAX_ATTEMPTS,
        SHARE_ACCESS_WINDOW,
        SHARE_ACCESS_SWEEP_THRESHOLD,
    )
}

/// Clear the attempt counter for a `token`+`ip` after a successful password
/// verification (and used by tests to reset the process-global limiter).
fn share_access_attempts_reset(token: &str, ip: &str) {
    if let Ok(mut map) = SHARE_ACCESS_RATE_LIMITER.lock() {
        map.remove(&share_access_key(token, ip));
    }
}

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
        (status = 429, description = "Too many password attempts", body = ErrorResponse),
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

    // Brute-force throttle: this public endpoint verifies a share password, so
    // cap attempts per `token`+`ip` within a rolling window before doing the
    // (deliberately expensive) password hash comparison. The (N+1)th attempt in
    // the window is rejected with 429; a correct password below clears the
    // counter. Runs before the verify so a flood cannot amplify into hashing
    // load.
    if !share_access_attempt_allowed(&token, &ip_address) {
        tracing::warn!(
            share_id = %share.id,
            ip_address = %ip_address,
            "protected share access rate limited",
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse::new(
                "RATE_LIMITED",
                "Too many password attempts. Please try again later.",
            )),
        ));
    }

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

    // Correct password: clear this key's counter so a legitimate viewer who
    // first mistyped is never penalised by the earlier failed attempts.
    share_access_attempts_reset(&token, &ip_address);

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

#[cfg(test)]
mod share_access_throttle_tests {
    use super::{
        share_access_attempt_allowed, share_access_attempts_reset, SHARE_ACCESS_MAX_ATTEMPTS,
    };

    // Distinct token/IP per test so the process-global limiter can't leak state
    // between them (LazyLock static is shared across the whole test binary).

    #[test]
    fn blocks_after_max_attempts_for_same_token_and_ip() {
        let token = "throttle-tok-block";
        let ip = "203.0.113.1";
        for i in 0..SHARE_ACCESS_MAX_ATTEMPTS {
            assert!(
                share_access_attempt_allowed(token, ip),
                "attempt {i} within the limit must be allowed"
            );
        }
        assert!(
            !share_access_attempt_allowed(token, ip),
            "the (max + 1)th attempt in the window must be blocked"
        );
    }

    #[test]
    fn different_ip_on_same_token_is_independent() {
        let token = "throttle-tok-shared";
        let attacker_ip = "203.0.113.2";
        // Exhaust the attacker IP against the share.
        for _ in 0..=SHARE_ACCESS_MAX_ATTEMPTS {
            share_access_attempt_allowed(token, attacker_ip);
        }
        assert!(
            !share_access_attempt_allowed(token, attacker_ip),
            "attacker IP must stay blocked on this token"
        );
        // A legitimate viewer from another IP is not locked out of the share.
        assert!(
            share_access_attempt_allowed(token, "198.51.100.9"),
            "a different IP on the same token must be unaffected"
        );
    }

    #[test]
    fn same_ip_different_token_is_independent() {
        let ip = "203.0.113.3";
        for _ in 0..=SHARE_ACCESS_MAX_ATTEMPTS {
            share_access_attempt_allowed("throttle-tok-a", ip);
        }
        assert!(
            !share_access_attempt_allowed("throttle-tok-a", ip),
            "the exhausted token must stay blocked for this IP"
        );
        assert!(
            share_access_attempt_allowed("throttle-tok-b", ip),
            "the same IP against a different share must be unaffected"
        );
    }

    #[test]
    fn reset_clears_counter_after_success() {
        let token = "throttle-tok-reset";
        let ip = "203.0.113.4";
        for _ in 0..SHARE_ACCESS_MAX_ATTEMPTS {
            assert!(share_access_attempt_allowed(token, ip));
        }
        // Simulate a correct password clearing the window.
        share_access_attempts_reset(token, ip);
        assert!(
            share_access_attempt_allowed(token, ip),
            "after a reset the key must be allowed to attempt again"
        );
    }
}

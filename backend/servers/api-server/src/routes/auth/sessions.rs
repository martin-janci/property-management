//! Session-management routes (list / revoke / revoke-all).
//!
//! Split out of `routes::auth` as a mechanical, behavior-preserving refactor
//! (Story 1.5). Shared helpers (rate limiting, `AuthService`, token/cookie
//! utilities), models, and Axum/serde/utoipa imports resolve via `use super::*`
//! against the parent `auth` module — a child module can see its parent's
//! private items, so no helper visibility had to change.

use super::*;
// ==================== Session Management (Story 1.5) ====================

/// Session info returned to clients.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    /// Session ID
    pub id: String,
    /// Device info (if available)
    pub device_info: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
    /// When the session was created
    pub created_at: String,
    /// When the session was last used
    pub last_used_at: String,
    /// Whether this is the current session
    pub is_current: bool,
}

/// List sessions response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSessionsResponse {
    /// Active sessions
    pub sessions: Vec<SessionInfo>,
}

/// List active sessions endpoint.
#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Sessions retrieved", body = ListSessionsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ListSessionsResponse>, AuthError> {
    let user_id = authenticated_user_id(&state, &headers)?;

    // Identify the caller's current session. Prefer the HttpOnly cookie,
    // fall back to the `X-Refresh-Token` header (see
    // `resolve_current_session_id`). Cookie-based ppt-web clients used to
    // resolve to `None` here, so no row could ever be marked `isCurrent`.
    let current_session_id = resolve_current_session_id(&state, &headers, user_id).await;

    // Get all active sessions for user
    let sessions = match state.session_repo.find_user_sessions(user_id).await {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch user sessions");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to fetch sessions",
            ));
        }
    };

    let session_infos: Vec<SessionInfo> = sessions
        .into_iter()
        .map(|s| {
            let is_current = current_session_id
                .as_ref()
                .map(|id| id == &s.id)
                .unwrap_or(false);

            SessionInfo {
                id: s.id.to_string(),
                device_info: s.device_info,
                ip_address: s.ip_address,
                user_agent: s.user_agent,
                created_at: s.created_at.to_rfc3339(),
                last_used_at: s.last_used_at.to_rfc3339(),
                is_current,
            }
        })
        .collect();

    Ok(Json(ListSessionsResponse {
        sessions: session_infos,
    }))
}

/// Revoke session request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeSessionRequest {
    /// Session ID to revoke
    pub session_id: String,
}

/// Revoke session response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokeSessionResponse {
    /// Success message
    pub message: String,
}

/// Revoke a specific session.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sessions/revoke",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    request_body = RevokeSessionRequest,
    responses(
        (status = 200, description = "Session revoked", body = RevokeSessionResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "Session not found", body = ErrorResponse)
    )
)]
pub async fn revoke_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RevokeSessionRequest>,
) -> Result<Json<RevokeSessionResponse>, AuthError> {
    let user_id = authenticated_user_id(&state, &headers)?;

    // Parse session ID
    let session_id: uuid::Uuid = req.session_id.parse().map_err(|_| {
        err_response(
            StatusCode::BAD_REQUEST,
            "INVALID_SESSION_ID",
            "Invalid session ID format",
        )
    })?;

    // Verify session belongs to this user
    let sessions = match state.session_repo.find_user_sessions(user_id).await {
        Ok(sessions) => sessions,
        Err(e) => {
            tracing::error!(error = %e, "Failed to fetch user sessions");
            return Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to verify session",
            ));
        }
    };

    let session_exists = sessions.iter().any(|s| s.id == session_id);
    if !session_exists {
        return Err(err_response(
            StatusCode::NOT_FOUND,
            "SESSION_NOT_FOUND",
            "Session not found",
        ));
    }

    // Revoke the session
    match state.session_repo.revoke_token(session_id).await {
        Ok(true) => {
            tracing::info!(user_id = %user_id, session_id = %session_id, "Session revoked");
            Ok(Json(RevokeSessionResponse {
                message: "Session revoked successfully".to_string(),
            }))
        }
        Ok(false) => Err(err_response(
            StatusCode::NOT_FOUND,
            "SESSION_NOT_FOUND",
            "Session already revoked",
        )),
        Err(e) => {
            tracing::error!(error = %e, "Failed to revoke session");
            Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to revoke session",
            ))
        }
    }
}

/// Revoke all sessions response.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RevokeAllSessionsResponse {
    /// Success message
    pub message: String,
    /// Number of sessions revoked
    pub revoked_count: u64,
}

/// Revoke all sessions except current.
#[utoipa::path(
    post,
    path = "/api/v1/auth/sessions/revoke-all",
    tag = "Authentication",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Sessions revoked", body = RevokeAllSessionsResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn revoke_all_sessions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<RevokeAllSessionsResponse>, AuthError> {
    let user_id = authenticated_user_id(&state, &headers)?;

    // Get current session to exclude. Prefer the HttpOnly cookie, fall back
    // to the `X-Refresh-Token` header (see `resolve_current_session_id`).
    // Cookie-based ppt-web clients used to resolve to `None` here, which made
    // `revoke_all_user_tokens(user_id, None)` revoke the caller's OWN live
    // session — "sign out other devices" signed the caller out too.
    let current_session_id = resolve_current_session_id(&state, &headers, user_id).await;

    // Revoke all sessions except current
    match state
        .session_repo
        .revoke_all_user_tokens(user_id, current_session_id)
        .await
    {
        Ok(count) => {
            tracing::info!(
                user_id = %user_id,
                revoked_count = count,
                "All other sessions revoked"
            );
            Ok(Json(RevokeAllSessionsResponse {
                message: format!("{} session(s) revoked", count),
                revoked_count: count,
            }))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to revoke sessions");
            Err(err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to revoke sessions",
            ))
        }
    }
}

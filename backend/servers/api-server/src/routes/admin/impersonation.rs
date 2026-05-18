//! Admin endpoints for impersonation. Phase 5.
//!
//! Issuing a token sets a cookie / returns the opaque token; the frontend
//! shows a sticky banner whenever it sees the token. The token lives 15
//! minutes (see `admin_core::IMPERSONATION_TTL`).

use admin_core::{
    require_capability, AuditOutcome, AuditWriter, Capability, ImpersonationService,
    IssuedImpersonationToken, RequireCapability,
};
use api_core::AuthUser;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/active",
            get(list_active).layer(require_capability(Capability::UsersImpersonate)),
        )
        .route(
            "/start",
            post(start).layer(require_capability(Capability::UsersImpersonate)),
        )
        .route(
            "/{token_id}",
            delete(stop).layer(require_capability(Capability::UsersImpersonate)),
        )
}

// ====================================================================
// GET /admin/impersonation/active
// ====================================================================
//
// Lists active impersonation sessions — rows where `ended_at IS NULL` and
// `expires_at > NOW()`. Joins `users` twice to surface actor + target email
// for the dashboard UI. Capability: `UsersImpersonate` (closest existing
// cap — the codebase has no separate `ImpersonationRead`).

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ActiveImpersonationRow {
    pub id: Uuid,
    pub actor_user_id: Uuid,
    pub actor_email: String,
    pub target_user_id: Uuid,
    pub target_email: String,
    pub started_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// The capability key that justified the impersonation. The
    /// `impersonation_tokens` table has no free-form `reason` column, so we
    /// surface `justifying_capability` here under the same field name the
    /// dashboard expects.
    pub reason: String,
}

async fn list_active(
    _cap: RequireCapability,
    auth: AuthUser,
    State(state): State<AppState>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ActiveImpersonationRow>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, ActiveImpersonationRow>(
        r#"
        SELECT t.id,
               t.actor_id          AS actor_user_id,
               actor.email         AS actor_email,
               t.target_user_id,
               target.email        AS target_email,
               t.issued_at         AS started_at,
               t.expires_at,
               t.justifying_capability AS reason
          FROM impersonation_tokens t
          JOIN users actor  ON actor.id  = t.actor_id
          JOIN users target ON target.id = t.target_user_id
         WHERE t.ended_at IS NULL
           AND t.expires_at > NOW()
         ORDER BY t.issued_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ip = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok());
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok());
    if let Err(e) = audit
        .record(
            Some(auth.user_id),
            Capability::UsersImpersonate,
            AuditOutcome::Allowed,
            Some("impersonation_active"),
            None,
            None,
            ip,
            ua,
        )
        .await
    {
        tracing::warn!(error = %e, "audit write for impersonation/active failed");
    }

    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct StartBody {
    pub target_user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct StartResponse {
    pub token_id: Uuid,
    pub plain_token: String,
    pub expires_at: DateTime<Utc>,
    pub target_user_id: Uuid,
}

impl From<IssuedImpersonationToken> for StartResponse {
    fn from(t: IssuedImpersonationToken) -> Self {
        Self {
            token_id: t.record.id,
            plain_token: t.plain_token,
            expires_at: t.record.expires_at,
            target_user_id: t.record.target_user_id,
        }
    }
}

/// POST /admin/impersonation/start
async fn start(
    _cap: RequireCapability,
    auth: AuthUser,
    Extension(svc): Extension<Arc<dyn ImpersonationService>>,
    Json(body): Json<StartBody>,
) -> Result<Json<StartResponse>, (StatusCode, String)> {
    let issued = svc
        .start_impersonation(
            auth.user_id,
            body.target_user_id,
            Capability::UsersImpersonate,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(issued.into()))
}

/// DELETE /admin/impersonation/:token_id
async fn stop(
    _cap: RequireCapability,
    auth: AuthUser,
    Extension(svc): Extension<Arc<dyn ImpersonationService>>,
    Path(token_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    svc.end_impersonation(token_id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

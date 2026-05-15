//! Admin endpoints for impersonation. Phase 5.
//!
//! Issuing a token sets a cookie / returns the opaque token; the frontend
//! shows a sticky banner whenever it sees the token. The token lives 15
//! minutes (see `admin_core::IMPERSONATION_TTL`).

use admin_core::{
    require_capability, Capability, ImpersonationService, IssuedImpersonationToken,
    RequireCapability,
};
use api_core::AuthUser;
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{delete, post},
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
            "/start",
            post(start).layer(require_capability(Capability::UsersImpersonate)),
        )
        .route(
            "/{token_id}",
            delete(stop).layer(require_capability(Capability::UsersImpersonate)),
        )
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
        .start_impersonation(auth.user_id, body.target_user_id, Capability::UsersImpersonate)
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

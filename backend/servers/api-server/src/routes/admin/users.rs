//! Admin endpoints for global user search & principal_kind transitions. Phase 5.
//!
//! `principal_kind` lives in Phase 2; we stub the transition call so this
//! handler compiles before Phase 2 lands. The capability gating (Phase 5
//! responsibility) is fully wired.

use admin_core::{require_capability, Capability, RequireCapability};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(search_users).layer(require_capability(Capability::UsersRead)),
        )
        .route(
            "/{id}",
            get(get_user).layer(require_capability(Capability::UsersRead)),
        )
        .route(
            "/{id}/principal-kind",
            post(set_principal_kind)
                .layer(require_capability(Capability::PrincipalKindEscalate)),
        )
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UserSummary {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
}

/// GET /admin/users?q=...
async fn search_users(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Vec<UserSummary>>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(20).min(100) as i64;
    let (users, _total) = state
        .platform_admin_repo
        .search_users_for_support(q.q.as_deref(), None, limit, 0)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(
        users
            .into_iter()
            .map(|u| UserSummary {
                id: u.id,
                email: u.email,
                display_name: u.display_name,
            })
            .collect(),
    ))
}

#[derive(Debug, Serialize)]
pub struct UserDetail {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
}

/// GET /admin/users/:id
async fn get_user(
    _cap: RequireCapability,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserDetail>, (StatusCode, String)> {
    let u = state
        .platform_admin_repo
        .get_user_for_support(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "user not found".into()))?;
    Ok(Json(UserDetail {
        id: u.id,
        email: u.email,
        display_name: u.display_name,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SetPrincipalKindBody {
    /// New principal kind: "public" | "staff" | "platform". Phase 2 owns the
    /// enum; we accept a string here and let the SECURITY DEFINER function
    /// validate.
    pub kind: String,
    pub reason: String,
}

/// POST /admin/users/:id/principal-kind
///
/// Stub. Phase 2 owns `set_principal_kind(uuid, text, text)` SECURITY DEFINER
/// SQL function. Once that lands, swap the body for `SELECT
/// set_principal_kind($1, $2, $3)`. The capability gate
/// (`PrincipalKindEscalate`) is the Phase 5 contribution.
async fn set_principal_kind(
    _cap: RequireCapability,
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(body): Json<SetPrincipalKindBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !matches!(body.kind.as_str(), "public" | "staff" | "platform") {
        return Err((StatusCode::BAD_REQUEST, "invalid principal kind".into()));
    }
    // TODO(Phase 2): SELECT set_principal_kind($1, $2::principal_kind, $3)
    Ok(StatusCode::NOT_IMPLEMENTED)
}

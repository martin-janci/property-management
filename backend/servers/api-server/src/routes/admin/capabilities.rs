//! Admin endpoints for capability grants. Phase 5.

use admin_core::{
    require_capability, Capability, CapabilityGrant, CapabilityGrantsRepository, RequireCapability,
};
use api_core::AuthUser;
use axum::{
    extract::Path,
    http::StatusCode,
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
            "/registry",
            get(list_registry).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/users/{user_id}",
            get(list_for_user).layer(require_capability(Capability::AuditRead)),
        )
        .route(
            "/grant",
            post(grant_capability).layer(require_capability(Capability::MembershipsGrant)),
        )
        .route(
            "/{grant_id}",
            delete(revoke_capability).layer(require_capability(Capability::MembershipsRevoke)),
        )
}

#[derive(Debug, Serialize)]
pub struct RegistryEntry {
    pub key: &'static str,
    pub registered: bool,
}

/// GET /admin/capabilities/registry
async fn list_registry(_cap: RequireCapability) -> Json<Vec<RegistryEntry>> {
    use admin_core::CapabilityRegistry;
    let registered = CapabilityRegistry::registered();
    Json(
        Capability::ALL
            .iter()
            .map(|c| RegistryEntry {
                key: c.as_str(),
                registered: registered.contains(c),
            })
            .collect(),
    )
}

/// GET /admin/capabilities/users/:user_id
async fn list_for_user(
    _cap: RequireCapability,
    Extension(grants): Extension<Arc<dyn CapabilityGrantsRepository>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<CapabilityGrant>>, (StatusCode, String)> {
    let rows = grants
        .list_for_user(user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct GrantBody {
    pub user_id: Uuid,
    pub capability: Capability,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default = "default_true")]
    pub mfa_required: bool,
    pub note: Option<String>,
}

fn default_true() -> bool {
    true
}

/// POST /admin/capabilities/grant
///
/// Defenses:
///   * Application layer rejects `granted_by == user_id` (no self-grant) —
///     leak #21. Enforced by `PgCapabilityGrantsRepository::grant`.
///   * Granting `PrincipalKindEscalate` requires the granter ALSO holds
///     `PrincipalKindEscalate` — leak #12.
async fn grant_capability(
    _cap: RequireCapability,
    auth: AuthUser,
    Extension(grants): Extension<Arc<dyn CapabilityGrantsRepository>>,
    Json(body): Json<GrantBody>,
) -> Result<Json<CapabilityGrant>, (StatusCode, String)> {
    if body.capability == Capability::PrincipalKindEscalate {
        let has = grants
            .user_has(auth.user_id, Capability::PrincipalKindEscalate)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if !has {
            return Err((
                StatusCode::FORBIDDEN,
                "only PrincipalKindEscalate holders can grant PrincipalKindEscalate".into(),
            ));
        }
    }

    let row = grants
        .grant(
            body.user_id,
            body.capability,
            auth.user_id,
            body.expires_at,
            body.mfa_required,
            body.note,
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(row))
}

/// DELETE /admin/capabilities/:grant_id
async fn revoke_capability(
    _cap: RequireCapability,
    auth: AuthUser,
    Extension(grants): Extension<Arc<dyn CapabilityGrantsRepository>>,
    Path(grant_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    grants
        .revoke(grant_id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

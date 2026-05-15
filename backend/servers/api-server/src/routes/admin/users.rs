//! Admin endpoints for global user search & principal_kind transitions. Phase 5.
//!
//! `principal_kind` lives in Phase 2; we stub the transition call so this
//! handler compiles before Phase 2 lands. The capability gating (Phase 5
//! responsibility) is fully wired.

use admin_core::{require_capability, Capability, RequireCapability};
use api_core::extractors::principal::RequestPrincipal;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::{AuthPolicyEnforcer, AuthPolicyError};
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
/// Wired to the Phase 2 `set_principal_kind(uuid, varchar, uuid, text)`
/// SECURITY DEFINER SQL function (migrations 00129 + 00143). The capability
/// gate (`PrincipalKindEscalate`) is the Phase 5 contribution. The N3 actor
/// check (added in 00143) enforces that we pass `principal.user_id` as the
/// `actor` argument — the SQL function rejects anything else.
///
/// We deliberately route through `RlsConnection` so `app.current_user_id`
/// is set on the connection that runs the function — the SECURITY DEFINER
/// body reads that GUC and aborts if it does not equal `actor`.
async fn set_principal_kind(
    _cap: RequireCapability,
    principal: RequestPrincipal,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(target_id): Path<Uuid>,
    Json(body): Json<SetPrincipalKindBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !matches!(body.kind.as_str(), "public" | "staff" | "platform") {
        rls.release().await;
        return Err((StatusCode::BAD_REQUEST, "invalid principal kind".into()));
    }

    // N2: re-evaluate the target's effective per-org auth policy before the
    // transition. Defends leak #13 — a corrupt or wrong-tenant policy aborts
    // the transition before the DB function fires.
    let enforcer = AuthPolicyEnforcer::new(state.db.clone());
    if let Err(err) = enforcer.check_principal_kind_change(target_id).await {
        rls.release().await;
        return Err(map_auth_policy_error(err));
    }

    // N3: pass the authenticated principal as `actor`. The SECURITY DEFINER
    // function asserts `actor == app.current_user_id` (set by RlsConnection),
    // so a forged actor argument would be rejected at the DB layer.
    let result = sqlx::query("SELECT set_principal_kind($1, $2, $3, $4)")
        .bind(target_id)
        .bind(&body.kind)
        .bind(principal.user_id)
        .bind(&body.reason)
        .execute(rls.conn())
        .await;

    rls.release().await;

    match result {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            tracing::error!(
                error = %e,
                target = %target_id,
                actor = %principal.user_id,
                "set_principal_kind failed"
            );
            Err((StatusCode::BAD_REQUEST, e.to_string()))
        }
    }
}

/// Map an AuthPolicyError onto the (StatusCode, String) shape this module's
/// handlers return.
fn map_auth_policy_error(err: AuthPolicyError) -> (StatusCode, String) {
    tracing::warn!(error = %err, "auth policy enforcement rejected principal_kind change (N2)");
    match err {
        AuthPolicyError::EmailNotVerified => (
            StatusCode::FORBIDDEN,
            "org policy requires verified email".into(),
        ),
        AuthPolicyError::PasswordPolicy(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "password does not satisfy org policy".into(),
        ),
        AuthPolicyError::MfaRequired(_) => {
            (StatusCode::FORBIDDEN, "MFA required by org policy".into())
        }
        AuthPolicyError::UserNotFound(_) => {
            (StatusCode::NOT_FOUND, "target user not found".into())
        }
        AuthPolicyError::Lookup(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

//! Admin endpoints for global user search & principal_kind transitions. Phase 5.
//!
//! `principal_kind` lives in Phase 2; we stub the transition call so this
//! handler compiles before Phase 2 lands. The capability gating (Phase 5
//! responsibility) is fully wired.

use admin_core::{require_capability, AuditOutcome, AuditWriter, Capability, RequireCapability};
use api_core::extractors::principal::RequestPrincipal;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
            post(set_principal_kind).layer(require_capability(Capability::PrincipalKindEscalate)),
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
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    mut rls: RlsConnection,
    Path(target_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<SetPrincipalKindBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    let ip = super::audit_ip_from_headers(&headers);
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok());

    if !matches!(body.kind.as_str(), "public" | "staff" | "platform") {
        rls.release().await;
        // Failure audit row: invalid body. Outcome::Denied with target_id so
        // the trail records "who tried to change user X's principal kind" —
        // closes the M2 gap (capability extractor's "allowed" row carries no
        // resource id).
        let payload = serde_json::json!({
            "kind": body.kind,
            "error": "invalid_kind",
        });
        if let Err(e) = audit
            .record(
                Some(principal.user_id),
                Capability::PrincipalKindEscalate,
                AuditOutcome::Denied,
                Some("principal_kind"),
                Some(target_id),
                Some(&payload),
                ip,
                ua,
            )
            .await
        {
            tracing::warn!(error = %e, "audit write for set_principal_kind (invalid) failed");
        }
        return Err((StatusCode::BAD_REQUEST, "invalid principal kind".into()));
    }

    // N2: re-evaluate the target's effective per-org auth policy before the
    // transition. Defends leak #13 — a corrupt or wrong-tenant policy aborts
    // the transition before the DB function fires.
    let enforcer = AuthPolicyEnforcer::new(state.db.clone());
    if let Err(err) = enforcer.check_principal_kind_change(target_id).await {
        rls.release().await;
        let payload = serde_json::json!({
            "kind": body.kind,
            "error": "auth_policy_rejected",
        });
        if let Err(e) = audit
            .record(
                Some(principal.user_id),
                Capability::PrincipalKindEscalate,
                AuditOutcome::Denied,
                Some("principal_kind"),
                Some(target_id),
                Some(&payload),
                ip,
                ua,
            )
            .await
        {
            tracing::warn!(error = %e, "audit write for set_principal_kind (policy) failed");
        }
        return Err(map_auth_policy_error(err));
    }

    // Best-effort: snapshot the prior principal_kind so the audit payload can
    // surface a before/after pair. A read failure here must NOT block the
    // mutation — we degrade to `prior_kind: null` rather than erroring out.
    // The read runs on the RLS-scoped connection so the same security context
    // applies.
    let prior_kind: Option<String> =
        match sqlx::query_scalar::<_, String>("SELECT principal_kind FROM users WHERE id = $1")
            .bind(target_id)
            .fetch_optional(&mut **rls.conn())
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    target = %target_id,
                    "failed to read prior principal_kind for audit payload"
                );
                None
            }
        };

    // N3: pass the authenticated principal as `actor`. The SECURITY DEFINER
    // function asserts `actor == app.current_user_id` (set by RlsConnection),
    // so a forged actor argument would be rejected at the DB layer.
    let result = sqlx::query("SELECT set_principal_kind($1, $2, $3, $4)")
        .bind(target_id)
        .bind(&body.kind)
        .bind(principal.user_id)
        .bind(&body.reason)
        .execute(&mut **rls.conn())
        .await;

    rls.release().await;

    match result {
        Ok(_) => {
            // Success audit row carrying target_id + before/after payload.
            // `PgAuditWriter` SHA-256-hashes the payload before persistence
            // (pre-existing limitation tracked in #300 — fix happens there,
            // not here). Emitting consistently is the M2 deliverable.
            let payload = serde_json::json!({
                "kind": body.kind,
                "prior_kind": prior_kind,
            });
            if let Err(e) = audit
                .record(
                    Some(principal.user_id),
                    Capability::PrincipalKindEscalate,
                    AuditOutcome::Allowed,
                    Some("principal_kind"),
                    Some(target_id),
                    Some(&payload),
                    ip,
                    ua,
                )
                .await
            {
                tracing::warn!(error = %e, "audit write for set_principal_kind (success) failed");
            }
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                target = %target_id,
                actor = %principal.user_id,
                "set_principal_kind failed"
            );
            // Failure audit row: SECURITY DEFINER function rejected the
            // transition. We DO NOT include the DB error string in the
            // payload (could echo PII); we record the attempted target_kind
            // and a stable error tag.
            let payload = serde_json::json!({
                "kind": body.kind,
                "prior_kind": prior_kind,
                "error": "db_function_rejected",
            });
            if let Err(audit_err) = audit
                .record(
                    Some(principal.user_id),
                    Capability::PrincipalKindEscalate,
                    AuditOutcome::Denied,
                    Some("principal_kind"),
                    Some(target_id),
                    Some(&payload),
                    ip,
                    ua,
                )
                .await
            {
                tracing::warn!(error = %audit_err, "audit write for set_principal_kind (failure) failed");
            }
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
        AuthPolicyError::UserNotFound(_) => (StatusCode::NOT_FOUND, "target user not found".into()),
        AuthPolicyError::Lookup(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

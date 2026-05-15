//! Admin endpoints for capability grants. Phase 5.
//!
//! Routes:
//!   * `GET    /registry`           — list all known capabilities (gated by `AuditRead`).
//!   * `GET    /me`                 — N10 bootstrap: caller introspects their own
//!                                    `principal_kind` + active grants. NOT gated by
//!                                    a capability — only by `RequestPrincipal`.
//!                                    Without this endpoint, a fresh platform
//!                                    principal cannot self-introspect because
//!                                    `/users/{id}` requires `AuditRead`, which
//!                                    they may not yet hold (deadlock).
//!   * `GET    /users/{user_id}`    — list another principal's grants (gated by `AuditRead`).
//!   * `POST   /grant`              — issue a grant (gated by `MembershipsGrant`).
//!   * `DELETE /{grant_id}`         — revoke a grant (gated by `MembershipsRevoke`).

use admin_core::{
    require_capability, AuditOutcome, AuditWriter, Capability, CapabilityGrant,
    CapabilityGrantsRepository, RequireCapability,
};
use api_core::extractors::principal::RequestPrincipal;
use api_core::AuthUser;
use axum::{
    extract::Path,
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
            "/registry",
            get(list_registry).layer(require_capability(Capability::AuditRead)),
        )
        // N10: bootstrap endpoint. Gated only by `RequestPrincipal` so a fresh
        // platform principal can self-introspect WITHOUT first holding
        // `AuditRead` — that would be a chicken-and-egg lockout.
        .route("/me", get(list_for_me))
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

/// Response shape for `GET /admin/capabilities/me` — the N10 bootstrap.
#[derive(Debug, Serialize)]
pub struct MyCapabilitiesResponse {
    pub user_id: Uuid,
    /// `public` / `staff` / `platform` — re-derived server-side from the
    /// trusted `users.principal_kind` column on every request.
    pub principal_kind: &'static str,
    /// Active (non-revoked, non-expired) capability grants for the caller.
    pub capabilities: Vec<CapabilityGrant>,
}

/// GET /admin/capabilities/me
///
/// N10 bootstrap. A fresh platform principal cannot self-introspect via
/// `/admin/capabilities/users/{id}` (that requires `AuditRead`, which they
/// may not yet hold). This endpoint is gated only by `RequestPrincipal` so
/// the caller can always discover what they currently hold — the frontend
/// `useCapability` hook calls this on login.
///
/// Read-of-self is still audited (Phase 5 design contract: every capability
/// surface emits an audit row). The audit action is `AuditRead` because the
/// data being read is a slice of the audit / capabilities surface, even
/// though no `AuditRead` capability is required to invoke this endpoint.
async fn list_for_me(
    principal: RequestPrincipal,
    Extension(grants): Extension<Arc<dyn CapabilityGrantsRepository>>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    headers: HeaderMap,
) -> Result<Json<MyCapabilitiesResponse>, (StatusCode, String)> {
    let rows = grants
        .list_for_user(principal.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Audit the read-of-self best-effort. We swallow audit-store errors so
    // an audit-store outage cannot brick the bootstrap path itself — the
    // alternative is a frontend that cannot recover from a DB blip.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok());
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok());
    if let Err(e) = audit
        .record(
            Some(principal.user_id),
            Capability::AuditRead,
            AuditOutcome::Allowed,
            Some("capabilities_self"),
            Some(principal.user_id),
            None,
            ip,
            ua,
        )
        .await
    {
        tracing::warn!(error = %e, user_id = %principal.user_id, "audit write for capabilities/me failed");
    }

    Ok(Json(MyCapabilitiesResponse {
        user_id: principal.user_id,
        principal_kind: principal.kind.as_str(),
        capabilities: rows,
    }))
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

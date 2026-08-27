//! Admin endpoints for capability grants. Phase 5.
//!
//! Routes:
//!   * `GET    /registry`                          — list all known capabilities (gated by `AuditRead`).
//!   * `GET    /me`                                — N10 bootstrap: caller introspects their own
//!                                                   `principal_kind` + active grants. NOT gated by
//!                                                   a capability — only by `RequestPrincipal`.
//!                                                   Without this endpoint, a fresh platform
//!                                                   principal cannot self-introspect because
//!                                                   `/users/{id}` requires `AuditRead`, which
//!                                                   they may not yet hold (deadlock).
//!   * `GET    /users/{user_id}`                   — list another principal's grants (gated by `AuditRead`).
//!   * `POST   /users/{user_id}/grant`             — issue a grant (gated by `MembershipsGrant`).
//!                                                   `user_id` is extracted from the path (source of
//!                                                   truth); the body no longer contains `user_id`.
//!   * `DELETE /users/{user_id}/grant/{grant_id}`  — revoke a grant (gated by `MembershipsRevoke`).
//!                                                   Validates that `grant_id` belongs to `user_id`
//!                                                   before deleting; returns 404 if ownership fails.

use admin_core::{
    require_capability, AdminDeps, AuditOutcome, AuditWriter, Capability, CapabilityGrant,
    CapabilityGrantsRepository, RequireCapability,
};
use api_core::extractors::principal::RequestPrincipal;
use api_core::{AuthUser, TenantMembershipProvider};
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

use crate::services::{AuthPolicyEnforcer, AuthPolicyError};
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
        // RESTful nested-resource convention: user_id in path is the source of
        // truth; body no longer contains user_id (body-injection defence).
        .route(
            "/users/{user_id}/grant",
            post(grant_capability).layer(require_capability(Capability::MembershipsGrant)),
        )
        // Validates grant ownership (grant_id must belong to user_id) before delete.
        .route(
            "/users/{user_id}/grant/{grant_id}",
            delete(revoke_capability).layer(require_capability(Capability::MembershipsRevoke)),
        )
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RegistryEntry {
    pub capability: String,
    pub description: String,
    pub risk_level: String,
    pub holder_count: i64,
}

/// GET /admin/capabilities/registry
///
/// Returns each capability enriched with its canonical description and risk
/// level from `capability_descriptions` (seeded by migration 00152) plus the
/// count of current active holders from `capability_grants`.
async fn list_registry(
    _cap: RequireCapability,
    State(state): State<AppState>,
) -> Result<Json<Vec<RegistryEntry>>, (StatusCode, String)> {
    // Runtime-checked sqlx (not `query!`) — CI runs with SQLX_OFFLINE=true and
    // no `.sqlx` offline cache is committed, so compile-time checked macros
    // would fail to build. Matches the convention in `audit.rs`, `agencies.rs`.
    let rows = sqlx::query_as::<_, RegistryEntry>(
        r#"
        SELECT
            cd.capability AS capability,
            cd.description AS description,
            cd.risk_level AS risk_level,
            COUNT(cg.id) FILTER (
                WHERE cg.revoked_at IS NULL
                  AND (cg.expires_at IS NULL OR cg.expires_at > NOW())
            ) AS holder_count
        FROM capability_descriptions cd
        LEFT JOIN capability_grants cg ON cg.capability = cd.capability
        GROUP BY cd.capability, cd.description, cd.risk_level
        ORDER BY cd.capability
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(rows))
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
    /// Timestamp of the caller's most recent MFA verification, if any and
    /// still within the validity window. `null` means never verified or the
    /// window has already expired.
    ///
    /// Frontend uses this (together with `mfa_window_seconds`) to compute the
    /// live countdown on the `MfaWindowChip` header chip.
    pub mfa_verified_at: Option<DateTime<Utc>>,
    /// Length of the MFA validity window in seconds. Currently hardcoded to
    /// 900 (15 min) — matching `RECENT_MFA_WINDOW` in `admin_core::mfa`.
    ///
    /// TODO(config): expose via `AppConfig::mfa_window_seconds` and thread it
    /// into both `PgMfaRecency` and this field so a single value drives both.
    pub mfa_window_seconds: i64,
}

/// The MFA validity window in seconds. Must stay in sync with
/// `admin_core::mfa::RECENT_MFA_WINDOW` (both are 15 minutes = 900 s).
///
/// TODO(config): thread this through `AppConfig` so a single value drives
/// both `PgMfaRecency` and this constant.
const MFA_WINDOW_SECONDS: i64 = 900;

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
///
/// The response also includes `mfa_verified_at` (the timestamp of the
/// caller's most-recent MFA verification, or `null` if none within the
/// window) and `mfa_window_seconds` so the frontend can render a live
/// countdown chip without decoding the JWT.
async fn list_for_me(
    principal: RequestPrincipal,
    State(state): State<AppState>,
    Extension(grants): Extension<Arc<dyn CapabilityGrantsRepository>>,
    Extension(audit): Extension<Arc<dyn AuditWriter>>,
    headers: HeaderMap,
) -> Result<Json<MyCapabilitiesResponse>, (StatusCode, String)> {
    let rows = grants
        .list_for_user(principal.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch the most-recent MFA verification timestamp for this user, if any
    // within the validity window. We return `None` if the user has never
    // verified or if the last verification is older than MFA_WINDOW_SECONDS —
    // that way the frontend chip stays in the `unverified` state even when the
    // DB row exists but is stale.
    //
    // Best-effort: a DB error here must NOT block the bootstrap response.
    // The chip falls back to `unverified` — acceptable degradation.
    // TODO(refactor): move into MfaRecency trait to dedupe with
    // PgMfaRecency::is_recent — add `latest_verification(user_id)` to the
    // trait + Pg impl + tests so the raw SQL lives in one place.
    // Bind the MFA window from the single-source-of-truth constant
    // (`MFA_WINDOW_SECONDS`) instead of duplicating the magic number as a
    // SQL literal. `make_interval(secs => $2)` keeps the type explicit on
    // the Postgres side. See item 5 of the admin backend audit follow-ups.
    let mfa_verified_at: Option<DateTime<Utc>> = match sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        SELECT verified_at
        FROM two_factor_auth_verifications
        WHERE user_id = $1
          AND verified_at > NOW() - make_interval(secs => $2)
        ORDER BY verified_at DESC
        LIMIT 1
        "#,
    )
    .bind(principal.user_id)
    .bind(MFA_WINDOW_SECONDS as f64)
    .fetch_optional(state.db_pool())
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(user_id = %principal.user_id, error = %e, "failed to load latest MFA verification");
            None
        }
    };

    // Audit the read-of-self best-effort. We swallow audit-store errors so
    // an audit-store outage cannot brick the bootstrap path itself — the
    // alternative is a frontend that cannot recover from a DB blip.
    let ip = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok());
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
        mfa_verified_at,
        mfa_window_seconds: MFA_WINDOW_SECONDS,
    }))
}

#[derive(Debug, Deserialize)]
pub struct GrantBody {
    /// `user_id` is taken from the URL path (`/users/{user_id}/grant`).
    /// Removed from the body to prevent path/body mismatch injection.
    pub capability: Capability,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default = "default_true")]
    pub mfa_required: bool,
    pub note: Option<String>,
    /// Free-form justification supplied by the granter. Surfaced into the
    /// audit-log payload so the trail records *why* the grant was issued
    /// (distinct from `note`, which is operator-facing and persists on the
    /// grant row itself).
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RevokeBody {
    /// Free-form justification supplied by the revoker. Plumbed into the
    /// audit-log payload. Optional and defaulted because some clients
    /// (curl one-liners, ops tooling) issue DELETE with no body — those
    /// fall through to `None` rather than failing the request.
    #[serde(default)]
    pub reason: Option<String>,
}

fn default_true() -> bool {
    true
}

/// POST /admin/capabilities/users/:user_id/grant
///
/// Defenses:
///   * `user_id` is extracted from the URL path — body injection is not
///     possible because the body no longer carries a `user_id` field.
///   * Application layer rejects `granted_by == user_id` (no self-grant) —
///     leak #21. Enforced by `PgCapabilityGrantsRepository::grant`.
///   * Granting `PrincipalKindEscalate` requires the granter ALSO holds
///     `PrincipalKindEscalate` — leak #12.
///   * N2 (leak #13): the grantee's effective per-org auth policy is
///     re-evaluated before any grant row is written. The
///     `check_capability_grant_for_user` helper resolves the strictest
///     policy across ALL of the grantee's active memberships and rejects
///     the grant if ANY org requires `require_email_verification` (tighten,
///     never loosen — independent of row order); platform-only principals
///     fall through (governed by platform defaults, which do not require
///     verification).
async fn grant_capability(
    _cap: RequireCapability,
    auth: AuthUser,
    // Both `grants` and `audit` live inside the already-attached `AdminDeps`
    // bundle (`lib.rs::attach_admin_extensions`). Using the bundle directly
    // keeps this handler under clippy's `too_many_arguments` 7-arg limit
    // without dropping any extractor — see PR #300 review.
    Extension(deps): Extension<AdminDeps>,
    axum::extract::State(state): axum::extract::State<AppState>,
    Path(user_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<GrantBody>,
) -> Result<Json<CapabilityGrant>, (StatusCode, String)> {
    let grants = &deps.grants;
    let audit = &deps.audit;
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

    // N2: re-evaluate the grantee's effective auth policy at the moment of
    // the grant. Defends leak #13 (policy resolved under wrong tenant) — a
    // grant that violates the grantee org's email-verification rule is
    // rejected before the capability row is written.
    let enforcer = AuthPolicyEnforcer::new(state.db.clone());
    if let Err(err) = enforcer.check_capability_grant_for_user(user_id).await {
        return Err(map_auth_policy_error(err));
    }

    let row = grants
        .grant(
            user_id,
            body.capability,
            auth.user_id,
            body.expires_at,
            body.mfa_required,
            body.note.clone(),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    // Emit a route-level audit row that carries the operator-supplied
    // `reason` in the hashed payload (the baseline audit from
    // `RequireCapability` only sees the capability name, not the body).
    let ip = super::audit_ip_from_headers(&headers);
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok());
    let payload = serde_json::json!({
        "grant_id": row.id,
        "user_id": user_id,
        "capability": body.capability.as_str(),
        "reason": body.reason,
        "expires_at": body.expires_at,
        "mfa_required": body.mfa_required,
    });
    if let Err(e) = audit
        .record(
            Some(auth.user_id),
            Capability::MembershipsGrant,
            AuditOutcome::Allowed,
            Some("capability_grant"),
            Some(row.id),
            Some(&payload),
            ip,
            ua,
        )
        .await
    {
        tracing::warn!(error = %e, "audit write for capability grant (reason) failed");
    }

    Ok(Json(row))
}

/// DELETE /admin/capabilities/users/:user_id/grant/:grant_id
///
/// D2.1: capability rows are PLATFORM-scoped (no `organization_id` column);
/// see `docs/multitenancy/decisions/capability-platform-scope.md`. The
/// `RequireCapability` extractor on this route has already enforced the
/// platform invariants (principal_kind == Platform + recent MFA + active
/// `MembershipsRevoke` grant) by the time we get here. We still call
/// `check_capability_revoke` so the policy-load liveness check is symmetric
/// with `check_capability_grant_for_user` on the grant path — a corrupted
/// policy-resolution layer aborts the revoke instead of silently proceeding.
///
/// Ownership validation: the grant row's `user_id` must match the path
/// `user_id` parameter. Returns 404 (not 403) if ownership fails — leaking
/// whether a grant_id exists at all is harmless only to callers with
/// `MembershipsRevoke`, but we still prefer the lookup-consistent 404.
///
/// E3 hardening: the underlying repo UPDATE writes
/// `revoked_with_mfa_at = NOW()` and the DB-layer trigger
/// `capability_grants_revoke_mfa_check` (migration 00145) rejects any
/// revoke whose MFA timestamp is missing or stale. This is defense in
/// depth — even if a future code path bypasses the enforcer above, the
/// trigger refuses the write.
async fn revoke_capability(
    _cap: RequireCapability,
    auth: AuthUser,
    // See `grant_capability` — collapsing `grants` + `audit` into the
    // `AdminDeps` bundle keeps the signature under clippy's 7-arg limit.
    Extension(deps): Extension<AdminDeps>,
    axum::extract::State(state): axum::extract::State<AppState>,
    Path((user_id, grant_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    // DELETE requests historically arrive without a body; `Option<Json<_>>`
    // makes the body optional so callers that do supply a JSON `reason`
    // get it plumbed into the audit log, while body-less callers still 204.
    body: Option<Json<RevokeBody>>,
) -> Result<StatusCode, (StatusCode, String)> {
    let grants = &deps.grants;
    let audit = &deps.audit;
    let enforcer = AuthPolicyEnforcer::new(state.db.clone());
    if let Err(err) = enforcer.check_capability_revoke(auth.user_id).await {
        return Err(map_auth_policy_error(err));
    }

    // Ownership validation: verify the grant belongs to the claimed user_id.
    let user_grants = grants
        .list_for_user(user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !user_grants.iter().any(|g| g.id == grant_id) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("grant {grant_id} not found for user {user_id}"),
        ));
    }

    grants
        .revoke(grant_id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Emit a route-level audit row that carries the operator-supplied
    // `reason` (if any) in the hashed payload. Mirrors the grant path.
    let reason = body.and_then(|Json(b)| b.reason);
    let ip = super::audit_ip_from_headers(&headers);
    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok());
    let payload = serde_json::json!({
        "grant_id": grant_id,
        "user_id": user_id,
        "reason": reason,
    });
    if let Err(e) = audit
        .record(
            Some(auth.user_id),
            Capability::MembershipsRevoke,
            AuditOutcome::Allowed,
            Some("capability_revoke"),
            Some(grant_id),
            Some(&payload),
            ip,
            ua,
        )
        .await
    {
        tracing::warn!(error = %e, "audit write for capability revoke (reason) failed");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Map an AuthPolicyError onto the (StatusCode, String) shape this module's
/// handlers return.
fn map_auth_policy_error(err: AuthPolicyError) -> (StatusCode, String) {
    tracing::warn!(error = %err, "auth policy enforcement rejected capability grant (N2)");
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

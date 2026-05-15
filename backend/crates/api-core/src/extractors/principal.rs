//! Request-principal extractor (Phase 2 — Identity Unification).
//!
//! `RequestPrincipal` is the canonical authority object for an authenticated
//! request under the new model. It replaces the JWT-claim-driven authorization
//! that came before.
//!
//! # Algorithm
//!
//! 1. Decode the bearer JWT — but the ONLY claim that matters is `sub`. Token-
//!    carried `tenant_id` / `role` claims are accepted for backward-compat
//!    deserialization but are NEVER trusted server-side (defends leaks #10
//!    and #11 — "stale authz in a live token" and "skeleton-key token").
//! 2. Read the `ResolvedTenant` injected by `host_tenant_middleware`. If
//!    present, the request's effective org is that org.
//! 3. Look up the user's `principal_kind` and active `user_memberships`. If
//!    `ResolvedTenant` is set and the user has NO active membership in that
//!    org AND the kind is not `Platform`, return **403** (defends leak #11
//!    again, this time on the read path).
//! 4. Otherwise, build a `RequestPrincipal` with `effective_org` set to either
//!    the resolved org (membership case) or `None` (platform-host case for
//!    a platform principal).

use crate::extractors::tenant::TenantMembershipProvider;
use crate::middleware::host_tenant::ResolvedTenant;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use db::models::PrincipalKind;
use db::repositories::MembershipRepository;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Permissive claim deserialization: only `sub` is *required*; everything else
/// is best-effort and IGNORED for authorization. This shape exists so we can
/// accept tokens minted by the legacy issuance paths during the one-phase
/// transition window without breaking deserialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalClaims {
    /// Subject — the user id. The ONLY trustworthy claim.
    pub sub: Uuid,
    /// Issued-at (validated by `jsonwebtoken` for `exp`/`nbf`; we do not use it).
    #[serde(default)]
    pub iat: i64,
    /// Expiration (validated by `jsonwebtoken`).
    #[serde(default)]
    pub exp: i64,
    /// Phase 2 *intent* claim — informational only. Server re-derives kind
    /// from the `users` table (defense in depth; never trusted as authority).
    #[serde(default)]
    pub kind: Option<String>,
}

/// The authoritative per-request principal. Built fresh on every request from
/// trusted server-side state.
#[derive(Debug, Clone, Copy)]
pub struct RequestPrincipal {
    pub user_id: Uuid,
    pub kind: PrincipalKind,
    /// Resolved organization for this request. `None` only for `Platform`
    /// principals reaching a platform host (no `ResolvedTenant` present).
    pub effective_org: Option<Uuid>,
}

impl RequestPrincipal {
    pub fn is_platform(&self) -> bool {
        matches!(self.kind, PrincipalKind::Platform)
    }
}

impl<S> FromRequestParts<S> for RequestPrincipal
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // ----- (1) Decode JWT — sub is the only trustworthy claim. -----
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

        let token = auth_header.strip_prefix("Bearer ").ok_or((
            StatusCode::UNAUTHORIZED,
            "Invalid Authorization header format",
        ))?;

        let secret = std::env::var("JWT_SECRET").map_err(|_| {
            tracing::error!("JWT_SECRET environment variable not set");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server configuration error",
            )
        })?;

        // We deliberately use the default `Validation` (HS256, exp+nbf checked).
        // We do NOT enforce any audience/issuer here — those are issuance
        // concerns that a future hardening pass can layer on.
        let token_data = decode::<PrincipalClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;
        let user_id = token_data.claims.sub;

        // ----- (2) Lookup the principal_kind from the trusted users table. -----
        let pool = state.db_pool();
        let kind_str: Option<String> = sqlx::query_scalar(
            r#"SELECT principal_kind FROM users WHERE id = $1 AND status != 'deleted'"#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "principal_kind lookup failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to resolve principal",
            )
        })?;

        let Some(kind_str) = kind_str else {
            tracing::warn!(user_id = %user_id, "RequestPrincipal: user not found / deleted");
            return Err((StatusCode::UNAUTHORIZED, "Unknown principal"));
        };
        let kind = PrincipalKind::parse(&kind_str);

        // ----- (3) Read host-resolved tenant (may be absent on platform host). -----
        let resolved = parts.extensions.get::<ResolvedTenant>().copied();

        let effective_org = match (resolved, kind) {
            // Host-resolved org + non-platform kind → require an active membership.
            (Some(rt), kind) if kind != PrincipalKind::Platform => {
                let repo = MembershipRepository::new(pool.clone());
                let active = repo
                    .is_active(user_id, rt.organization_id)
                    .await
                    .map_err(|e| {
                        tracing::error!(error = %e, user_id = %user_id, org_id = %rt.organization_id, "membership lookup failed");
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to verify membership",
                        )
                    })?;
                if !active {
                    // Defends leak #11: a token does not grant access to *any*
                    // org — only to orgs the user has an active membership in.
                    tracing::warn!(
                        user_id = %user_id,
                        host_org = %rt.organization_id,
                        "RequestPrincipal: no active membership in host-resolved org"
                    );
                    return Err((
                        StatusCode::FORBIDDEN,
                        "no active membership in this organization",
                    ));
                }
                Some(rt.organization_id)
            }
            // Platform kind on a tenant host: allowed; effective_org is the host's org.
            (Some(rt), PrincipalKind::Platform) => Some(rt.organization_id),
            // No host resolution at all (platform host): allowed only for platform principals.
            (None, PrincipalKind::Platform) => None,
            (None, _) => {
                // A non-platform user reached a path with no resolved tenant.
                // Under Phase 2, that means the legacy header-based path is
                // not in play here — we fail closed. Callers that genuinely
                // do not need a tenant should use a different extractor.
                tracing::warn!(
                    user_id = %user_id,
                    kind = %kind_str,
                    "RequestPrincipal: no ResolvedTenant for non-platform principal"
                );
                return Err((StatusCode::FORBIDDEN, "tenant not resolved"));
            }
        };

        // Stash the user_id in extensions so other extractors that piggy-back
        // on `AuthUser`-style state still find it.
        parts.extensions.insert(user_id);

        Ok(RequestPrincipal {
            user_id,
            kind,
            effective_org,
        })
    }
}

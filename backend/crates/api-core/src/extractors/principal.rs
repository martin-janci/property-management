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
use crate::middleware::host_tenant::{ResolvedTenant, TenantSource};
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
    /// Token discriminator. Access tokens carry `token_type: "access"`;
    /// refresh tokens carry `"refresh"`. Unlike `kind`, this IS enforced:
    /// a refresh token must never be accepted on an access-only route
    /// (mirrors the `auth.rs` RUST-002 fix). Kept `Option` + `serde(default)`
    /// so any legacy token that omits the claim still decodes and is treated
    /// as access — only an *explicit* non-access value is rejected.
    #[serde(default)]
    pub token_type: Option<String>,
}

/// The authoritative per-request principal. Built fresh on every request from
/// trusted server-side state.
///
/// Issue #528 (4): marked `#[must_use]` so any expression returning a
/// `RequestPrincipal` whose value is dropped triggers `unused_must_use`
/// at compile time. (Note: parameter-binding discards still fall through
/// to the shell-based `check-discarded-principal` lint — see
/// `backend/scripts/lints/check-discarded-principal.sh` — until the
/// linear-typing wrapper in the issue's option 2 lands.)
#[derive(Debug, Clone, Copy)]
#[must_use = "RequestPrincipal must be used for authz/tenant-scoping — see backend/scripts/lints/check-discarded-principal.sh"]
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

        // Reject refresh tokens (and any other non-access discriminator)
        // BEFORE touching the DB. A token that omits `token_type` is treated
        // as access for backward-compat; an explicit "refresh" is denied so
        // a refresh token cannot be replayed against an access-only route.
        if let Some(token_type) = token_data.claims.token_type.as_deref() {
            if token_type != "access" {
                tracing::warn!(
                    token_type = %token_type,
                    "RequestPrincipal: rejected non-access token on access-only route"
                );
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid token type for this endpoint",
                ));
            }
        }

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
            // Phase 4 — PlatformHost wiring.
            //
            // The PlatformHost variant carries `organization_id == Uuid::nil()`
            // as a sentinel meaning "no specific tenant — global read context".
            // Using that nil id as a real org id (e.g. for membership lookup)
            // would either 403-with-misleading-error (no one has membership in
            // the nil org) or, worse, leak across tenants downstream. Branch on
            // `source == PlatformHost` BEFORE the membership-checking arms.
            //
            //   * Platform principal on PlatformHost → allowed, `effective_org = None`.
            //   * Any other principal kind on PlatformHost → 403; the platform
            //     host is platform-only by definition.
            (Some(rt), PrincipalKind::Platform) if rt.source == TenantSource::PlatformHost => None,
            (Some(rt), _) if rt.source == TenantSource::PlatformHost => {
                tracing::warn!(
                    user_id = %user_id,
                    kind = %kind_str,
                    "RequestPrincipal: non-platform principal on platform host"
                );
                return Err((
                    StatusCode::FORBIDDEN,
                    "platform host requires platform principal",
                ));
            }
            // Host-resolved org + non-platform kind → require an active membership.
            (Some(rt), PrincipalKind::Public) | (Some(rt), PrincipalKind::Staff) => {
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

/// Optional variant of [`RequestPrincipal`] for endpoints that accept BOTH
/// authenticated and anonymous traffic (e.g. a public listing detail that
/// surfaces extra fields when the caller is logged in).
///
/// # Branching contract
///
/// Three explicit branches, encoded so the call-site can match on
/// `Some`/`None` without juggling errors:
///
/// 1. **No `Authorization` header** → `Self(None)`. Truly anonymous request.
/// 2. **`Authorization` present and resolves cleanly** → `Self(Some(principal))`.
///    Hand the principal to the handler.
/// 3. **`Authorization` present but JWT is invalid OR membership lookup
///    yields a 403** → propagate the rejection. We MUST NOT swallow
///    security failures here: an attacker presenting a stolen-but-revoked
///    token, or a token for an org they no longer belong to, must be told
///    "no", not silently demoted to "anonymous". (Defends leaks #10 and
///    #11 in the brainstorming notes — same as the underlying extractor.)
///
/// In short: presence of the `Authorization` header is a *commitment* by
/// the client to "I am authenticated"; the server honors that commitment
/// strictly. Absence of the header is the only path to the `None` branch.
#[derive(Debug, Clone, Copy)]
pub struct OptionalRequestPrincipal(pub Option<RequestPrincipal>);

impl<S> FromRequestParts<S> for OptionalRequestPrincipal
where
    S: TenantMembershipProvider,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Branch (1): no `Authorization` header → anonymous.
        //
        // We deliberately gate on the *presence* of the header rather than
        // delegating to `RequestPrincipal::from_request_parts` and matching
        // on its rejection: a missing-header rejection is conceptually the
        // same as "not authenticated", but a malformed-header or invalid-
        // token rejection is a security signal we must NOT swallow. Doing
        // the check here lets us cleanly distinguish branch (1) from branch
        // (3) without inspecting error strings.
        if parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .is_none()
        {
            return Ok(Self(None));
        }

        // Branches (2) and (3): a header is present — let the underlying
        // extractor do the work, and propagate any error it returns.
        // 401 (invalid token), 403 (membership/principal mismatch), 500
        // (DB lookup failure) all surface unchanged.
        let principal = RequestPrincipal::from_request_parts(parts, state).await?;
        Ok(Self(Some(principal)))
    }
}

/// Authenticated principal for **reality-portal** endpoints (GH #1300).
///
/// `RequestPrincipal` is the api-server / tenant-scoped extractor: its only
/// arm that admits a `public` principal requires an active `organization_members`
/// row in the host-resolved org. Reality-portal agencies are `principal_kind =
/// 'public'`, never have `organization_members` rows (their membership lives in
/// `reality_agency_members`), and reach the portal on a `PlatformHost`. So
/// `RequestPrincipal` 403s every real reality agency on its own resources.
///
/// `PortalPrincipal` is the "different extractor" the `RequestPrincipal` docs
/// point callers at: it authenticates the JWT and re-derives `principal_kind`
/// from the trusted `users` table (identical defense-in-depth to
/// `RequestPrincipal`), but does **not** require a `ResolvedTenant` or any
/// `organization_members` membership. It yields only `user_id` (+ the derived
/// `kind`); per-resource authorization stays enforced because every reality
/// repo call is keyed on `principal.user_id` (the IDOR fix from PR #1297).
///
/// It admits any authenticated, non-deleted principal kind (a platform admin
/// may legitimately reach these endpoints); cross-user isolation is the
/// repo-layer `user_id` keying, not a kind gate.
#[derive(Debug, Clone, Copy)]
#[must_use = "PortalPrincipal must be used for authz/scoping — see GH #1300"]
pub struct PortalPrincipal {
    pub user_id: Uuid,
    pub kind: PrincipalKind,
}

impl<S> FromRequestParts<S> for PortalPrincipal
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

        let token_data = decode::<PrincipalClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

        // Reject refresh (and any non-access) tokens before touching the DB —
        // mirrors `RequestPrincipal`.
        if let Some(token_type) = token_data.claims.token_type.as_deref() {
            if token_type != "access" {
                tracing::warn!(
                    token_type = %token_type,
                    "PortalPrincipal: rejected non-access token on access-only route"
                );
                return Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid token type for this endpoint",
                ));
            }
        }

        let user_id = token_data.claims.sub;

        // ----- (2) Re-derive principal_kind from the trusted users table. -----
        let pool = state.db_pool();
        let kind_str: Option<String> = sqlx::query_scalar(
            r#"SELECT principal_kind FROM users WHERE id = $1 AND status != 'deleted'"#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "PortalPrincipal: principal_kind lookup failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to resolve principal",
            )
        })?;

        let Some(kind_str) = kind_str else {
            tracing::warn!(user_id = %user_id, "PortalPrincipal: user not found / deleted");
            return Err((StatusCode::UNAUTHORIZED, "Unknown principal"));
        };

        Ok(PortalPrincipal {
            user_id,
            kind: PrincipalKind::parse(&kind_str),
        })
    }
}
